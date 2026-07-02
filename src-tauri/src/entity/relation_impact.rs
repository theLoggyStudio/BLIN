//! Impact stock / compteur via liaisons entité (incrément / décrément, idempotent, workflows).

use std::path::Path;

use chrono::Utc;
use rusqlite::params;
use serde_json::{Map, Value};
use uuid::Uuid;

use super::attr_types::is_reserved_attribute;
use super::child_table;
use super::compteur::{self, is_compteur_attr};
use super::embed;
use super::registry::{EntityAttribute, EntityDef, EntityRegistry};
use super::record_signature;
use super::schema::{attr_column, table_has_column, table_name};
use crate::db::Database;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImpactAction {
    Increment,
    Decrement,
}

impl ImpactAction {
    fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_lowercase().as_str() {
            "increment" | "incrementer" | "incrémenter" => Some(Self::Increment),
            "decrement" | "decrementer" | "décrémenter" => Some(Self::Decrement),
            _ => None,
        }
    }
}

pub fn is_numeric_impactable(attr_type: &str) -> bool {
    matches!(
        attr_type,
        "stock" | "number" | "integer" | "float" | "compteur" | "matricule"
    )
}

fn has_impact_config(attr: &EntityAttribute) -> bool {
    attr.attr_type == "entity"
        && attr
            .relation_impact_source
            .as_deref()
            .is_some_and(|s| !s.trim().is_empty())
        && attr
            .relation_impact_target
            .as_deref()
            .is_some_and(|s| !s.trim().is_empty())
        && attr
            .relation_impact_action
            .as_deref()
            .and_then(|s| ImpactAction::parse(s))
            .is_some()
}

fn embed_item_label(item: &Map<String, Value>) -> String {
    for key in ["libelle", "nom", "titre", "reference", "intitule"] {
        if let Some(s) = item.get(key).and_then(|v| v.as_str()) {
            let t = s.trim();
            if !t.is_empty() {
                return t.to_string();
            }
        }
    }
    "article".into()
}

fn stock_insufficient_message(
    article_label: &str,
    qty: f64,
    cap: f64,
    attr_label: &str,
    line: usize,
) -> String {
    if cap <= 0.0 {
        format!(
            "Impossible de finaliser la signature : le stock de « {article_label} » est épuisé.\n\
             Loggy : créez une demande d'achat pour réapprovisionner cet article avant de signer."
        )
    } else {
        format!(
            "« {attr_label} » ligne {line} : quantité demandée ({qty}) supérieure au stock disponible ({cap}) pour « {article_label} ».\n\
             Loggy : réduisez la quantité ou lancez une demande d'achat pour compléter le stock."
        )
    }
}

pub fn json_to_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().replace(',', ".").parse().ok(),
        Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        _ => None,
    }
}

fn read_numeric_at_path(map: &Map<String, Value>, path: &str) -> Option<f64> {
    let parts: Vec<&str> = path.split('.').filter(|p| !p.is_empty()).collect();
    if parts.is_empty() {
        return None;
    }
    let mut cur = Value::Object(map.clone());
    for part in parts {
        let Value::Object(obj) = &cur else {
            return None;
        };
        cur = obj.get(part)?.clone();
    }
    json_to_f64(&cur)
}

fn flat_embed_key(parent_attr: &EntityAttribute, path: &str) -> String {
    let prefix = embed::embedded_prefix(parent_attr);
    if path.contains('.') {
        format!("{prefix}_{}", path.replace('.', "_"))
    } else {
        format!("{prefix}_{path}")
    }
}

/// Découpe `article.qte_initial` → relations [`article`], champ `qte_initial`.
fn split_dotted_path(path: &str) -> (Vec<String>, String) {
    let parts: Vec<String> = path
        .split('.')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect();
    if parts.is_empty() {
        return (vec![], String::new());
    }
    if parts.len() == 1 {
        return (vec![], parts[0].clone());
    }
    let field = parts.last().cloned().unwrap_or_default();
    let relations = parts[..parts.len() - 1].to_vec();
    (relations, field)
}

fn navigate_embed_chain(
    registry: &EntityRegistry,
    start_ent: &EntityDef,
    start_data: Map<String, Value>,
    relations: &[String],
) -> Result<(EntityDef, Map<String, Value>), String> {
    let mut ent = start_ent;
    let mut data = start_data;
    for rel in relations {
        let attr = ent
            .attributs
            .iter()
            .find(|a| a.nom == *rel && a.attr_type == "entity")
            .ok_or_else(|| format!("Liaison « {rel} » introuvable sur {}.", ent.nom))?;
        let next_ent = embed::resolve_child(registry, attr)
            .ok_or_else(|| format!("Entité cible « {rel} » introuvable."))?;
        data = if let Some(Value::Object(nested)) = data.get(rel) {
            nested.clone()
        } else if let Some(Value::Array(items)) = data.get(rel) {
            items
                .first()
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default()
        } else {
            extract_embed_one_to_one(attr, next_ent, &data)
        };
        ent = next_ent;
    }
    Ok((ent.clone(), data))
}

fn resolve_impact_target(
    registry: &EntityRegistry,
    link_child: &EntityDef,
    target_path: &str,
    link_attr: &EntityAttribute,
    parent_row: &Map<String, Value>,
    line_data: Option<&Map<String, Value>>,
) -> Result<(EntityDef, String, Map<String, Value>), String> {
    let (relations, field) = split_dotted_path(target_path);
    if field.is_empty() {
        return Err("Champ cible vide.".into());
    }
    let base_data = line_data.cloned().unwrap_or_else(|| {
        if link_attr.relation_multiple {
            Map::new()
        } else {
            extract_embed_one_to_one(link_attr, link_child, parent_row)
        }
    });
    if relations.is_empty() {
        return Ok((link_child.clone(), field, base_data));
    }
    let (ent, data) = navigate_embed_chain(registry, link_child, base_data, &relations)?;
    Ok((ent, field, data))
}

fn read_source_delta(
    source_field: &str,
    parent_row: &Map<String, Value>,
    line_data: Option<&Map<String, Value>>,
    parent_attr: &EntityAttribute,
) -> Option<f64> {
    if let Some(line) = line_data {
        if let Some(n) = read_numeric_at_path(line, source_field) {
            return Some(n);
        }
        if let Some(n) = line.get(source_field).and_then(json_to_f64) {
            return Some(n);
        }
        let leaf = source_field.rsplit('.').next().unwrap_or(source_field);
        if leaf != source_field {
            if let Some(n) = line.get(leaf).and_then(json_to_f64) {
                return Some(n);
            }
        }
    }
    if let Some(n) = read_numeric_at_path(parent_row, source_field) {
        return Some(n);
    }
    if let Some(n) = parent_row.get(source_field).and_then(json_to_f64) {
        return Some(n);
    }
    let flat = flat_embed_key(parent_attr, source_field);
    if let Some(n) = parent_row.get(&flat).and_then(json_to_f64) {
        return Some(n);
    }
    let prefix = embed::embedded_prefix(parent_attr);
    let prefixed = format!("{prefix}_{source_field}");
    parent_row.get(&prefixed).and_then(json_to_f64)
}

fn extract_embed_one_to_one(
    parent_attr: &EntityAttribute,
    child: &EntityDef,
    parent_row: &Map<String, Value>,
) -> Map<String, Value> {
    let mut out = Map::new();
    for child_attr in embed::copyable_child_attributes(child) {
        let col = embed::embedded_column_name(parent_attr, child_attr);
        if let Some(v) = parent_row.get(&col) {
            out.insert(child_attr.nom.clone(), v.clone());
        }
        if is_compteur_attr(child_attr) {
            for suffix in ["_libelle", "_jjmmaaaa", "_numero"] {
                let sub = format!("{col}{suffix}");
                if let Some(v) = parent_row.get(&sub) {
                    out.insert(format!("{}{}", child_attr.nom, suffix), v.clone());
                }
            }
        }
    }
    out
}

fn record_exists(db: &Database, table: &str, id: &str) -> Result<bool, String> {
    let sql = format!("SELECT 1 FROM {table} WHERE id = ?1 LIMIT 1");
    let found: Option<i64> = db.conn.query_row(&sql, params![id], |row| row.get(0)).ok();
    Ok(found.is_some())
}

/// Recherche un enregistrement fille par référence unique.
pub fn lookup_child_id_by_reference(
    db: &Database,
    table: &str,
    reference: &str,
) -> Result<Option<String>, String> {
    lookup_unique_string_id(db, table, "reference", reference)
}

fn lookup_unique_string_id(
    db: &Database,
    table: &str,
    col: &str,
    value: &str,
) -> Result<Option<String>, String> {
    if value.is_empty() || !table_has_column(db, table, col)? {
        return Ok(None);
    }
    let sql = format!("SELECT id FROM {table} WHERE {col} = ?1 COLLATE NOCASE LIMIT 2");
    let mut stmt = db.conn.prepare(&sql).map_err(|e| e.to_string())?;
    let ids: Vec<String> = stmt
        .query_map(params![value], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    if ids.len() == 1 {
        Ok(Some(ids[0].clone()))
    } else {
        Ok(None)
    }
}

fn find_child_record_id(
    db: &Database,
    child: &EntityDef,
    embed_data: &Map<String, Value>,
) -> Result<Option<String>, String> {
    let table = table_name(&child.nom);
    if !table_exists(db, &table)? {
        return Ok(None);
    }

    for key in [embed::EMBED_SOURCE_RECORD_ID, "id"] {
        if let Some(id) = embed_data
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            if record_exists(db, &table, id)? {
                return Ok(Some(id.to_string()));
            }
        }
    }

    if let Some(reference) = embed_data
        .get("reference")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if let Some(id) = lookup_unique_string_id(db, &table, "reference", reference)? {
            return Ok(Some(id));
        }
    }

    for attr in &child.attributs {
        if !is_compteur_attr(attr) {
            continue;
        }
        let libelle_col = compteur::column_libelle(attr);
        let date_col = compteur::column_jjmmaaaa(attr);
        let num_col = compteur::column_numero(attr);
        let libelle = embed_data
            .get(&libelle_col)
            .or_else(|| embed_data.get(&attr.nom))
            .and_then(|v| v.as_str().map(str::to_string));
        let jjmmaaaa = embed_data
            .get(&date_col)
            .or_else(|| embed_data.get(&format!("{}_jjmmaaaa", attr.nom)))
            .and_then(|v| v.as_str().map(str::to_string));
        let numero = embed_data
            .get(&num_col)
            .or_else(|| embed_data.get(&format!("{}_numero", attr.nom)))
            .and_then(json_to_f64)
            .map(|n| n as i64);
        if let (Some(l), Some(d), Some(n)) = (libelle, jjmmaaaa, numero) {
            let sql = format!(
                "SELECT id FROM {table} WHERE {libelle_col} = ?1 AND {date_col} = ?2 AND {num_col} = ?3 LIMIT 1"
            );
            let id: Option<String> = db
                .conn
                .query_row(&sql, params![l, d, n], |row| row.get(0))
                .ok();
            if id.is_some() {
                return Ok(id);
            }
        }
    }

    if let Some(nom) = embed_data
        .get("nom")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if let Some(id) = lookup_unique_string_id(db, &table, "nom", nom)? {
            return Ok(Some(id));
        }
    }

    Ok(None)
}

fn read_target_value(
    db: &Database,
    child: &EntityDef,
    child_record_id: &str,
    target_field: &str,
) -> Result<Option<f64>, String> {
    let table = table_name(&child.nom);
    let target_attr = child
        .attributs
        .iter()
        .find(|a| a.nom == target_field)
        .ok_or_else(|| format!("Champ cible « {target_field} » introuvable sur {}.", child.nom))?;
    let col = if is_compteur_attr(target_attr) {
        compteur::column_numero(target_attr)
    } else {
        attr_column(target_attr)
    };
    if !table_has_column(db, &table, &col)? {
        return Ok(None);
    }
    let sql = format!("SELECT {col} FROM {table} WHERE id = ?1");
    let val: rusqlite::types::Value = db
        .conn
        .query_row(&sql, params![child_record_id], |row| row.get(0))
        .map_err(|_| format!("Enregistrement cible introuvable ({child_record_id})."))?;
    Ok(match val {
        rusqlite::types::Value::Integer(i) => Some(i as f64),
        rusqlite::types::Value::Real(f) => Some(f),
        rusqlite::types::Value::Text(s) => s.trim().replace(',', ".").parse().ok(),
        _ => None,
    })
}

fn write_target_value(
    db: &Database,
    child: &EntityDef,
    child_record_id: &str,
    target_field: &str,
    new_value: f64,
) -> Result<(), String> {
    let table = table_name(&child.nom);
    let target_attr = child
        .attributs
        .iter()
        .find(|a| a.nom == target_field)
        .ok_or_else(|| format!("Champ cible « {target_field} » introuvable."))?;
    let col = if is_compteur_attr(target_attr) {
        compteur::column_numero(target_attr)
    } else {
        attr_column(target_attr)
    };
    let sql = format!("UPDATE {table} SET {col} = ?1 WHERE id = ?2");
    let n = db
        .conn
        .execute(&sql, params![new_value, child_record_id])
        .map_err(|e| e.to_string())?;
    if n == 0 {
        return Err("Mise à jour du champ cible impossible (enregistrement introuvable).".into());
    }
    Ok(())
}

fn impact_already_applied(
    db: &Database,
    trigger_entity: &str,
    trigger_record_id: &str,
    owner_entity: &str,
    attr_nom: &str,
    line_index: i64,
) -> Result<bool, String> {
    let n: i64 = db
        .conn
        .query_row(
            "SELECT COUNT(*) FROM entity_relation_impact_log
             WHERE trigger_entity = ?1 AND trigger_record_id = ?2
               AND owner_entity = ?3 AND attr_nom = ?4 AND line_index = ?5",
            params![trigger_entity, trigger_record_id, owner_entity, attr_nom, line_index],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(n > 0)
}

fn log_impact(
    db: &Database,
    trigger_entity: &str,
    trigger_record_id: &str,
    owner_entity: &str,
    attr_nom: &str,
    line_index: i64,
    child_entity: &str,
    child_record_id: &str,
    target_field: &str,
    action: ImpactAction,
    delta: f64,
) -> Result<(), String> {
    let action_s = match action {
        ImpactAction::Increment => "increment",
        ImpactAction::Decrement => "decrement",
    };
    db.conn
        .execute(
            "INSERT INTO entity_relation_impact_log
             (id, trigger_entity, trigger_record_id, owner_entity, attr_nom, line_index,
              child_entity, child_record_id, target_field, action, delta, applied_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                Uuid::new_v4().to_string(),
                trigger_entity,
                trigger_record_id,
                owner_entity,
                attr_nom,
                line_index,
                child_entity,
                child_record_id,
                target_field,
                action_s,
                delta,
                Utc::now().to_rfc3339(),
            ],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn apply_single_impact(
    db: &Database,
    data_dir: &Path,
    registry: &EntityRegistry,
    owner_ent: &EntityDef,
    attr: &EntityAttribute,
    parent_row: &Map<String, Value>,
    line_data: Option<&Map<String, Value>>,
    line_index: i64,
    trigger_entity: &str,
    trigger_record_id: &str,
) -> Result<(), String> {
    let Some(child) = embed::resolve_child(registry, attr) else {
        return Ok(());
    };
    let source = attr.relation_impact_source.as_deref().unwrap_or("").trim();
    let target = attr.relation_impact_target.as_deref().unwrap_or("").trim();
    let action = attr
        .relation_impact_action
        .as_deref()
        .and_then(ImpactAction::parse)
        .ok_or_else(|| "Action d'impact invalide.".to_string())?;
    if source.is_empty() || target.is_empty() {
        return Ok(());
    }

    if impact_already_applied(
        db,
        trigger_entity,
        trigger_record_id,
        &owner_ent.nom,
        &attr.nom,
        line_index,
    )? {
        return Ok(());
    }

    let delta = read_source_delta(source, parent_row, line_data, attr)
        .filter(|d| *d != 0.0)
        .ok_or_else(|| format!("Valeur source « {source} » absente ou non numérique."))?;

    let (target_ent, target_field, embed_data) = resolve_impact_target(
        registry,
        child,
        target,
        attr,
        parent_row,
        line_data,
    )?;

    let Some(child_record_id) = find_child_record_id(db, &target_ent, &embed_data)? else {
        return Err(format!(
            "Impossible d'identifier l'enregistrement {} à impacter (cible « {target} »).",
            target_ent.nom
        ));
    };

    let current = read_target_value(db, &target_ent, &child_record_id, &target_field)?.unwrap_or(0.0);
    if action == ImpactAction::Decrement && delta > current {
        let attr_label = owner_ent
            .label
            .clone()
            .unwrap_or_else(|| owner_ent.nom.clone());
        let line_no = if line_index >= 0 {
            line_index as usize + 1
        } else {
            1
        };
        let item_label = line_data
            .map(embed_item_label)
            .unwrap_or_else(|| embed_item_label(&embed_data));
        return Err(stock_insufficient_message(
            &item_label,
            delta,
            current,
            &attr_label,
            line_no,
        ));
    }
    let new_value = match action {
        ImpactAction::Increment => current + delta,
        ImpactAction::Decrement => (current - delta).max(0.0),
    };
    write_target_value(db, &target_ent, &child_record_id, &target_field, new_value)?;
    if action == ImpactAction::Decrement {
        let _ = super::validation::spawn_stock_low_alert_task(
            db,
            data_dir,
            &target_ent,
            &child_record_id,
            &target_field,
            new_value,
        );
    }
    log_impact(
        db,
        trigger_entity,
        trigger_record_id,
        &owner_ent.nom,
        &attr.nom,
        line_index,
        &target_ent.nom,
        &child_record_id,
        &target_field,
        action,
        delta,
    )
}

fn enrich_context_with_embed_lists(
    db: &Database,
    registry: &EntityRegistry,
    owner_ent: &EntityDef,
    parent_entity: &str,
    parent_id: &str,
    context_row: &mut Map<String, Value>,
) -> Result<(), String> {
    for (list_attr, list_child) in child_table::embed_list_attrs(owner_ent, registry) {
        if context_row.contains_key(&list_attr.nom) {
            continue;
        }
        let items = child_table::load_embed_list(db, parent_entity, parent_id, list_child)?;
        context_row.insert(
            list_attr.nom.clone(),
            Value::Array(items.into_iter().map(Value::Object).collect()),
        );
    }
    Ok(())
}

fn process_entity_row(
    db: &Database,
    data_dir: &Path,
    registry: &EntityRegistry,
    owner_ent: &EntityDef,
    parent_row: &Map<String, Value>,
    trigger_entity: &str,
    trigger_record_id: &str,
    from_parent: bool,
    list_parent_entity: &str,
    list_parent_id: &str,
) -> Result<(), String> {
    for attr in owner_ent
        .attributs
        .iter()
        .filter(|a| !is_reserved_attribute(a))
    {
        if !has_impact_config(attr) {
            continue;
        }
        let defer = attr.relation_impact_defer;
        if from_parent {
            if !defer {
                continue;
            }
        } else if defer {
            continue;
        }

        if attr.relation_multiple {
            let items = child_table::parse_embed_list_items(parent_row.get(&attr.nom));
            for (idx, item) in items.iter().enumerate() {
                apply_single_impact(
                    db,
                    data_dir,
                    registry,
                    owner_ent,
                    attr,
                    parent_row,
                    Some(item),
                    idx as i64,
                    trigger_entity,
                    trigger_record_id,
                )?;
            }
        } else {
            apply_single_impact(
                db,
                data_dir,
                registry,
                owner_ent,
                attr,
                parent_row,
                None,
                -1,
                trigger_entity,
                trigger_record_id,
            )?;
        }
    }

    for attr in owner_ent
        .attributs
        .iter()
        .filter(|a| !is_reserved_attribute(a))
        .filter(|a| a.attr_type == "entity" && !a.relation_multiple)
    {
        let Some(child_ent) = embed::resolve_child(registry, attr) else {
            continue;
        };
        let mut child_row = extract_embed_one_to_one(attr, child_ent, parent_row);
        if child_row.is_empty() {
            continue;
        }
        enrich_context_with_embed_lists(
            db,
            registry,
            child_ent,
            list_parent_entity,
            list_parent_id,
            &mut child_row,
        )?;
        process_entity_row(
            db,
            data_dir,
            registry,
            child_ent,
            &child_row,
            trigger_entity,
            trigger_record_id,
            true,
            list_parent_entity,
            list_parent_id,
        )?;
    }

    Ok(())
}

fn validate_single_impact(
    db: &Database,
    registry: &EntityRegistry,
    owner_ent: &EntityDef,
    attr: &EntityAttribute,
    parent_row: &Map<String, Value>,
    line_data: Option<&Map<String, Value>>,
    line_index: i64,
) -> Result<(), String> {
    let Some(child) = embed::resolve_child(registry, attr) else {
        return Ok(());
    };
    let action = attr
        .relation_impact_action
        .as_deref()
        .and_then(ImpactAction::parse);
    if action != Some(ImpactAction::Decrement) {
        return Ok(());
    }
    let source = attr.relation_impact_source.as_deref().unwrap_or("").trim();
    let target = attr.relation_impact_target.as_deref().unwrap_or("").trim();
    if source.is_empty() || target.is_empty() {
        return Ok(());
    }
    let Some(qty) = read_source_delta(source, parent_row, line_data, attr) else {
        return Ok(());
    };
    if qty <= 0.0 {
        return Ok(());
    }
    let (target_ent, target_field, embed_data) =
        resolve_impact_target(registry, child, target, attr, parent_row, line_data)?;
    let Some(child_record_id) = find_child_record_id(db, &target_ent, &embed_data)? else {
        let line_no = if line_index >= 0 {
            line_index as usize + 1
        } else {
            1
        };
        return Err(format!(
            "« {} » ligne {line_no} : article introuvable.",
            attr.label.clone().unwrap_or_else(|| attr.nom.clone())
        ));
    };
    let cap = read_target_value(db, &target_ent, &child_record_id, &target_field)?.unwrap_or(0.0);
    if qty > cap {
        let attr_label = attr.label.clone().unwrap_or_else(|| attr.nom.clone());
        let line_no = if line_index >= 0 {
            line_index as usize + 1
        } else {
            1
        };
        let item_label = line_data
            .map(embed_item_label)
            .unwrap_or_else(|| embed_item_label(&embed_data));
        return Err(stock_insufficient_message(
            &item_label,
            qty,
            cap,
            &attr_label,
            line_no,
        ));
    }
    Ok(())
}

fn validate_process_entity_row(
    db: &Database,
    registry: &EntityRegistry,
    owner_ent: &EntityDef,
    parent_row: &Map<String, Value>,
    from_parent: bool,
    list_parent_entity: &str,
    list_parent_id: &str,
) -> Result<(), String> {
    for attr in owner_ent
        .attributs
        .iter()
        .filter(|a| !is_reserved_attribute(a))
    {
        if !has_impact_config(attr) {
            continue;
        }
        let defer = attr.relation_impact_defer;
        if from_parent {
            if !defer {
                continue;
            }
        } else if defer {
            continue;
        }

        if attr.relation_multiple {
            let items = child_table::parse_embed_list_items(parent_row.get(&attr.nom));
            for (idx, item) in items.iter().enumerate() {
                validate_single_impact(
                    db,
                    registry,
                    owner_ent,
                    attr,
                    parent_row,
                    Some(item),
                    idx as i64,
                )?;
            }
        } else {
            validate_single_impact(db, registry, owner_ent, attr, parent_row, None, -1)?;
        }
    }

    for attr in owner_ent
        .attributs
        .iter()
        .filter(|a| !is_reserved_attribute(a))
        .filter(|a| a.attr_type == "entity" && !a.relation_multiple)
    {
        let Some(child_ent) = embed::resolve_child(registry, attr) else {
            continue;
        };
        let mut child_row = extract_embed_one_to_one(attr, child_ent, parent_row);
        if child_row.is_empty() {
            continue;
        }
        enrich_context_with_embed_lists(
            db,
            registry,
            child_ent,
            list_parent_entity,
            list_parent_id,
            &mut child_row,
        )?;
        validate_process_entity_row(
            db,
            registry,
            child_ent,
            &child_row,
            true,
            list_parent_entity,
            list_parent_id,
        )?;
    }

    Ok(())
}

/// Valide les impacts (stock) avant validation finale (signature ou création directe).
pub fn validate_impacts_before_record_validated(
    db: &Database,
    data_dir: &Path,
    trigger_entity: &str,
    trigger_record_id: &str,
) -> Result<(), String> {
    let registry = super::registry::load(data_dir)?;
    let cfg = super::load_screen_config(data_dir, trigger_entity)?;
    let mut row = crate::dda::crud::get_row(db, &cfg, trigger_record_id)?;
    if let Some(owner_ent) = registry.find(trigger_entity) {
        for (list_attr, list_child) in child_table::embed_list_attrs(owner_ent, &registry) {
            if row.contains_key(&list_attr.nom) {
                continue;
            }
            let items = child_table::load_embed_list(
                db,
                trigger_entity,
                trigger_record_id,
                list_child,
            )?;
            row.insert(
                list_attr.nom.clone(),
                Value::Array(items.into_iter().map(Value::Object).collect()),
            );
        }
    }
    let Some(owner_ent) = registry.find(trigger_entity) else {
        return Ok(());
    };
    validate_process_entity_row(
        db,
        &registry,
        owner_ent,
        &row,
        false,
        trigger_entity,
        trigger_record_id,
    )
}

/// Valide que les quantités d'impact ne dépassent pas le stock fille.
pub fn validate_embed_impact_quantities(
    db: &Database,
    data_dir: &Path,
    entity_key: &str,
    embed_lists: &[(String, EntityDef, Vec<Map<String, Value>>)],
) -> Result<(), String> {
    let registry = super::registry::load(data_dir)?;
    let Some(owner_ent) = registry.find(entity_key) else {
        return Ok(());
    };
    for (attr_nom, _child_def, items) in embed_lists {
        let Some(attr) = owner_ent.attributs.iter().find(|a| a.nom == *attr_nom) else {
            continue;
        };
        if !has_impact_config(attr) {
            continue;
        }
        let action = attr
            .relation_impact_action
            .as_deref()
            .and_then(ImpactAction::parse);
        if action != Some(ImpactAction::Decrement) {
            continue;
        }
        let source = attr
            .relation_impact_source
            .as_deref()
            .map(str::trim)
            .unwrap_or("");
        let cap_field = attr
            .relation_impact_target
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(source);
        if source.is_empty() {
            continue;
        }
        let Some(child) = embed::resolve_child(&registry, attr) else {
            continue;
        };
        for (idx, item) in items.iter().enumerate() {
            let Some(qty) = read_source_delta(source, &Map::new(), Some(item), attr) else {
                continue;
            };
            if qty <= 0.0 {
                continue;
            }
            let Some(child_id) = find_child_record_id(db, child, item)? else {
                return Err(format!(
                    "« {} » ligne {} : article introuvable.",
                    attr.label.clone().unwrap_or_else(|| attr.nom.clone()),
                    idx + 1
                ));
            };
            let cap = read_target_value(db, child, &child_id, cap_field)?.unwrap_or(0.0);
            if qty > cap {
                let label = attr
                    .label
                    .clone()
                    .unwrap_or_else(|| attr.nom.clone());
                let item_label = embed_item_label(item);
                return Err(stock_insufficient_message(
                    &item_label,
                    qty,
                    cap,
                    &label,
                    idx + 1,
                ));
            }
        }
    }
    Ok(())
}

/// Applique les impacts configurés lorsqu'un enregistrement est validé (signature ou création directe).
pub fn apply_on_record_validated(
    db: &Database,
    data_dir: &Path,
    trigger_entity: &str,
    trigger_record_id: &str,
) -> Result<(), String> {
    let registry = super::registry::load(data_dir)?;
    let cfg = super::load_screen_config(data_dir, trigger_entity)?;
    let row = crate::dda::crud::get_row(db, &cfg, trigger_record_id)?;
    let Some(owner_ent) = registry.find(trigger_entity) else {
        return Ok(());
    };
    process_entity_row(
        db,
        data_dir,
        &registry,
        owner_ent,
        &row,
        trigger_entity,
        trigger_record_id,
        false,
        trigger_entity,
        trigger_record_id,
    )
}

/// Après création : applique si l'entité ne requiert pas de signature.
pub fn apply_after_create_if_ready(
    db: &Database,
    data_dir: &Path,
    entity_key: &str,
    record_id: &str,
) {
    let registry = match super::registry::load(data_dir) {
        Ok(r) => r,
        Err(_) => return,
    };
    if record_signature::entity_requires_signature(&registry, entity_key) {
        return;
    }
    if let Err(e) = apply_on_record_validated(db, data_dir, entity_key, record_id) {
        eprintln!("relation_impact après création {entity_key}/{record_id} : {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_impactable_types() {
        assert!(is_numeric_impactable("stock"));
        assert!(is_numeric_impactable("compteur"));
        assert!(!is_numeric_impactable("string"));
    }

    #[test]
    fn parse_impact_action() {
        assert_eq!(ImpactAction::parse("increment"), Some(ImpactAction::Increment));
        assert_eq!(ImpactAction::parse("décrémenter"), Some(ImpactAction::Decrement));
    }

    #[test]
    fn split_dotted_path_segments() {
        assert_eq!(
            split_dotted_path("article.qte_initial"),
            (vec!["article".into()], "qte_initial".into())
        );
        assert_eq!(
            split_dotted_path("commande.da.article.stock"),
            (
                vec!["commande".into(), "da".into(), "article".into()],
                "stock".into()
            )
        );
        assert_eq!(split_dotted_path("matricule"), (vec![], "matricule".into()));
    }
}

fn table_exists(db: &Database, table: &str) -> Result<bool, String> {
    let n: i64 = db
        .conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name = ?1",
            params![table],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(n > 0)
}
