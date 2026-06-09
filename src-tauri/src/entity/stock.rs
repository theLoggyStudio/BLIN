//! Gestion de stock : entité `stock` auto, sync depuis attributs type `stock`, tâches de déstockage.

use std::path::Path;

use chrono::{NaiveDate, Utc};
use rusqlite::params;
use serde_json::{json, Map, Value};
use uuid::Uuid;

use super::registry::{EntityAttribute, EntityDef, EntityRegistry};
use super::schema::{attr_column, table_has_column, table_name};
const TACHE_ENTITY_KEY: &str = "tache";
use crate::db::Database;

pub const STOCK_ENTITY_KEY: &str = "stock";
pub const STOCK_ATTR_TYPE: &str = "stock";

const DESTOCK_TYPE: &str = "destockage";
const PEREMPTION_WARNING_DAYS: i64 = 30;

pub fn registry_has_stock(registry: &EntityRegistry) -> bool {
    registry
        .entities
        .iter()
        .filter(|e| e.nom != STOCK_ENTITY_KEY)
        .flat_map(|e| &e.attributs)
        .any(|a| !super::attr_types::is_reserved_attribute(a) && a.attr_type == STOCK_ATTR_TYPE)
}

pub fn stock_entity_def() -> EntityDef {
    EntityDef {
        nom: STOCK_ENTITY_KEY.into(),
        label: Some("Stock".into()),
        description: Some(
            "Suivi des quantités et articles périssables — généré automatiquement.".into(),
        ),
        ai_suggestions: false,
        requires_signature: false,
        signatory_role_ids: vec![],
        is_session: false,
        attributs: vec![
            EntityAttribute {
                nom: "entite_source".into(),
                attr_type: "string".into(),
                label: Some("Entité source".into()),
                required: true,
                r#ref: None,
                relation_multiple: false,
                relation_exclusive_parent: true,
                default: None,
                enum_options: None,
                ..Default::default()
            },
            EntityAttribute {
                nom: "enregistrement_id".into(),
                attr_type: "string".into(),
                label: Some("ID enregistrement".into()),
                required: true,
                r#ref: None,
                relation_multiple: false,
                relation_exclusive_parent: true,
                default: None,
                enum_options: None,
                ..Default::default()
            },
            EntityAttribute {
                nom: "libelle".into(),
                attr_type: "string".into(),
                label: Some("Libellé".into()),
                required: true,
                r#ref: None,
                relation_multiple: false,
                relation_exclusive_parent: true,
                default: None,
                enum_options: None,
                ..Default::default()
            },
            EntityAttribute {
                nom: "quantite".into(),
                attr_type: "number".into(),
                label: Some("Quantité".into()),
                required: true,
                r#ref: None,
                relation_multiple: false,
                relation_exclusive_parent: true,
                default: None,
                enum_options: None,
                ..Default::default()
            },
            EntityAttribute {
                nom: "article_perissable".into(),
                attr_type: "boolean".into(),
                label: Some("Article périssable".into()),
                required: false,
                r#ref: None,
                relation_multiple: false,
                relation_exclusive_parent: true,
                default: Some(json!(false)),
                enum_options: None,
                ..Default::default()
            },
            EntityAttribute {
                nom: "date_peremption".into(),
                attr_type: "date".into(),
                label: Some("Date de péremption".into()),
                required: false,
                r#ref: None,
                relation_multiple: false,
                relation_exclusive_parent: true,
                default: None,
                enum_options: None,
                ..Default::default()
            },
        ],
    }
}

/// Ajoute ou retire l'entité `stock` du registre selon la présence d'attributs `stock`.
pub fn ensure_stock_module(registry: &mut EntityRegistry) -> bool {
    let needs = registry_has_stock(registry);
    let pos = registry.entities.iter().position(|e| e.nom == STOCK_ENTITY_KEY);

    match (needs, pos) {
        (true, None) => {
            registry.entities.push(stock_entity_def());
            true
        }
        (false, Some(idx)) => {
            registry.entities.remove(idx);
            true
        }
        (true, Some(idx)) => {
            registry.entities[idx] = stock_entity_def();
            true
        }
        (false, None) => false,
    }
}

pub fn validate_stock_row(data: &Map<String, Value>) -> Result<(), String> {
    let perishable = data
        .get("article_perissable")
        .map(|v| v == true || v.as_i64() == Some(1) || v.as_str() == Some("1") || v.as_str() == Some("true"))
        .unwrap_or(false);
    if !perishable {
        return Ok(());
    }
    let date = data
        .get("date_peremption")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("");
    if date.is_empty() {
        return Err(
            "Pour un article périssable, la date de péremption est obligatoire.".into(),
        );
    }
    Ok(())
}

/// Supprime la ligne stock liée à un enregistrement source supprimé.
pub fn remove_stock_line_for_source(
    db: &Database,
    data_dir: &Path,
    source_key: &str,
    record_id: &str,
) -> Result<(), String> {
    if source_key == STOCK_ENTITY_KEY || source_key == TACHE_ENTITY_KEY {
        return Ok(());
    }
    let registry = super::registry::load(data_dir)?;
    if !registry_has_stock(&registry) {
        return Ok(());
    }
    let table = table_name(STOCK_ENTITY_KEY);
    if !table_exists(db, &table)? {
        return Ok(());
    }
    db.conn
        .execute(
            &format!(
                "DELETE FROM {table} WHERE entite_source = ?1 AND enregistrement_id = ?2"
            ),
            params![source_key, record_id],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Après création/mise à jour d'une entité métier : synchronise les lignes de stock.
pub fn sync_lines_from_source(
    db: &Database,
    data_dir: &Path,
    source_key: &str,
    row: &Map<String, Value>,
) -> Result<(), String> {
    if source_key == STOCK_ENTITY_KEY || source_key == TACHE_ENTITY_KEY {
        return Ok(());
    }
    let registry = super::registry::load(data_dir)?;
    if !registry_has_stock(&registry) {
        return Ok(());
    }
    let Some(source) = registry.find(source_key) else {
        return Ok(());
    };

    let record_id = row
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("ID enregistrement manquant pour sync stock.")?;

    let libelle = record_label(source, row);
    let stock_attrs: Vec<_> = source
        .attributs
        .iter()
        .filter(|a| a.attr_type == STOCK_ATTR_TYPE && !super::attr_types::is_reserved_attribute(a))
        .collect();

    if stock_attrs.is_empty() {
        return Ok(());
    }

    let table = table_name(STOCK_ENTITY_KEY);
    if !table_exists(db, &table)? {
        return Ok(());
    }

    let mut total_qty = 0.0f64;
    for attr in &stock_attrs {
        let col = attr_column(attr);
        let q = row.get(&attr.nom).or_else(|| row.get(&col));
        let qty = value_as_f64(q).unwrap_or(0.0);
        total_qty += qty;
    }
    let qty = if stock_attrs.len() == 1 {
        value_as_f64(
            row.get(&stock_attrs[0].nom)
                .or_else(|| row.get(&attr_column(stock_attrs[0]))),
        )
        .unwrap_or(total_qty)
    } else {
        total_qty
    };

    let existing_id: Option<String> = db
        .conn
        .query_row(
            &format!(
                "SELECT id FROM {table} WHERE entite_source = ?1 AND enregistrement_id = ?2 LIMIT 1"
            ),
            params![source_key, record_id],
            |r| r.get(0),
        )
        .ok();

    if qty <= 0.0 {
        if let Some(id) = existing_id {
            let _ = db
                .conn
                .execute(&format!("DELETE FROM {table} WHERE id = ?1"), params![id]);
        }
        return Ok(());
    }

    if let Some(id) = existing_id {
        db.conn
            .execute(
                &format!(
                    "UPDATE {table} SET libelle = ?1, quantite = ?2 WHERE id = ?3"
                ),
                params![libelle, qty, id],
            )
            .map_err(|e| e.to_string())?;
        if let Ok(row_stock) = fetch_stock_row(db, &id) {
            let _ = maybe_spawn_destock_task(db, data_dir, &row_stock);
        }
    } else {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        db.conn
            .execute(
                &format!(
                    "INSERT INTO {table} (id, created_at, entite_source, enregistrement_id, libelle, quantite, article_perissable, date_peremption)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, NULL)"
                ),
                params![id, now, source_key, record_id, libelle, qty],
            )
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Met à jour le(s) champ(s) `stock` de l'enregistrement source depuis la ligne inventaire.
pub fn sync_stock_to_source(
    db: &Database,
    data_dir: &Path,
    stock_row: &Map<String, Value>,
) -> Result<(), String> {
    let source_key = stock_row
        .get("entite_source")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or("Entité source manquante sur la ligne stock.")?;
    let record_id = stock_row
        .get("enregistrement_id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or("ID enregistrement source manquant sur la ligne stock.")?;
    let qty = value_as_f64(stock_row.get("quantite")).unwrap_or(0.0);

    let registry = super::registry::load(data_dir)?;
    let Some(source) = registry.find(source_key) else {
        return Ok(());
    };
    let stock_attrs: Vec<_> = source
        .attributs
        .iter()
        .filter(|a| a.attr_type == STOCK_ATTR_TYPE && !super::attr_types::is_reserved_attribute(a))
        .collect();
    if stock_attrs.is_empty() {
        return Ok(());
    }

    let table = table_name(source_key);
    if !table_exists(db, &table)? {
        return Ok(());
    }

    if stock_attrs.len() == 1 {
        let col = attr_column(stock_attrs[0]);
        if table_has_column(db, &table, &col)? {
            db.conn
                .execute(
                    &format!("UPDATE {table} SET {col} = ?1 WHERE id = ?2"),
                    params![qty, record_id],
                )
                .map_err(|e| e.to_string())?;
        }
    } else {
        let per = qty / stock_attrs.len() as f64;
        for attr in &stock_attrs {
            let col = attr_column(attr);
            if table_has_column(db, &table, &col)? {
                db.conn
                    .execute(
                        &format!("UPDATE {table} SET {col} = ?1 WHERE id = ?2"),
                        params![per, record_id],
                    )
                    .map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(())
}

/// Réduit la quantité en stock et synchronise l'enregistrement source.
pub fn destock_line(
    db: &Database,
    data_dir: &Path,
    stock_id: &str,
    quantity_to_remove: Option<f64>,
) -> Result<Map<String, Value>, String> {
    let row = fetch_stock_row(db, stock_id)?;
    let current = value_as_f64(row.get("quantite")).unwrap_or(0.0);
    if current <= 0.0 {
        return Err("Le stock est déjà à zéro.".into());
    }
    let remove = quantity_to_remove.unwrap_or(current);
    if remove <= 0.0 {
        return Err("Indiquez une quantité positive à déstocker.".into());
    }
    let remove = remove.min(current);
    let new_qty = current - remove;

    let table = table_name(STOCK_ENTITY_KEY);
    if new_qty <= 0.0 {
        db.conn
            .execute(&format!("DELETE FROM {table} WHERE id = ?1"), params![stock_id])
            .map_err(|e| e.to_string())?;
        let mut cleared = row.clone();
        cleared.insert("quantite".into(), json!(0.0));
        sync_stock_to_source(db, data_dir, &cleared)?;
    } else {
        db.conn
            .execute(
                &format!("UPDATE {table} SET quantite = ?1 WHERE id = ?2"),
                params![new_qty, stock_id],
            )
            .map_err(|e| e.to_string())?;
        let updated = fetch_stock_row(db, stock_id)?;
        sync_stock_to_source(db, data_dir, &updated)?;
        complete_open_destock_tasks(db, stock_id)?;
        return Ok(updated);
    }

    complete_open_destock_tasks(db, stock_id)?;
    let mut out = row;
    out.insert("quantite".into(), json!(0.0));
    Ok(out)
}

fn complete_open_destock_tasks(db: &Database, stock_id: &str) -> Result<(), String> {
    let tache_table = table_name(TACHE_ENTITY_KEY);
    if !table_exists(db, &tache_table)? {
        return Ok(());
    }
    db.conn
        .execute(
            &format!(
                "UPDATE {tache_table} SET statut = 'terminee' WHERE type_tache = ?1 AND (entite_a_signer = ?2 OR entite_a_valider = ?2) AND enregistrement_id = ?3 AND statut != 'terminee'"
            ),
            params![DESTOCK_TYPE, STOCK_ENTITY_KEY, stock_id],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Après enregistrement d'une ligne stock : validation, sync source, tâche de déstockage si péremption proche.
pub fn after_stock_row_saved(db: &Database, data_dir: &Path, row: &Map<String, Value>) -> Result<(), String> {
    validate_stock_row(row)?;
    sync_stock_to_source(db, data_dir, row)?;
    maybe_spawn_destock_task(db, data_dir, row)?;
    Ok(())
}

/// Vérifie toutes les lignes stock (ex. à l'ouverture de l'écran).
pub fn scan_all_destock_tasks(db: &Database, data_dir: &Path) -> Result<u32, String> {
    let registry = super::registry::load(data_dir)?;
    if !registry.find(STOCK_ENTITY_KEY).is_some() {
        return Ok(0);
    }
    let table = table_name(STOCK_ENTITY_KEY);
    if !table_exists(db, &table)? {
        return Ok(0);
    }
    let mut stmt = db
        .conn
        .prepare(&format!("SELECT * FROM {table}"))
        .map_err(|e| e.to_string())?;
    let mut count = 0u32;
    let rows = stmt
        .query_map([], |row| row_to_map(row))
        .map_err(|e| e.to_string())?;
    for row in rows.flatten() {
        if maybe_spawn_destock_task(db, data_dir, &row).unwrap_or(false) {
            count += 1;
        }
    }
    Ok(count)
}

fn maybe_spawn_destock_task(db: &Database, data_dir: &Path, row: &Map<String, Value>) -> Result<bool, String> {
    let perishable = row
        .get("article_perissable")
        .map(|v| v == true || v.as_i64() == Some(1) || v.as_str() == Some("1") || v.as_str() == Some("true"))
        .unwrap_or(false);
    if !perishable {
        return Ok(false);
    }
    let date_str = row
        .get("date_peremption")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let Some(date_str) = date_str else {
        return Ok(false);
    };
    let exp = NaiveDate::parse_from_str(&date_str[..10.min(date_str.len())], "%Y-%m-%d")
        .map_err(|_| "Date de péremption invalide.")?;
    let today = Utc::now().date_naive();
    let days = (exp - today).num_days();
    if days > PEREMPTION_WARNING_DAYS {
        return Ok(false);
    }

    let stock_id = row
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("ID ligne stock manquant.")?;

    let tache_table = table_name(TACHE_ENTITY_KEY);
    if !table_exists(db, &tache_table)? {
        return Ok(false);
    }

    let exists: i64 = db.conn.query_row(
        &format!(
            "SELECT COUNT(*) FROM {tache_table} WHERE type_tache = ?1 AND (entite_a_signer = ?2 OR entite_a_valider = ?2) AND enregistrement_id = ?3 AND statut != 'terminee'"
        ),
        params![DESTOCK_TYPE, STOCK_ENTITY_KEY, stock_id],
        |r| r.get(0),
    ).unwrap_or(0);
    if exists > 0 {
        return Ok(false);
    }

    let registry = super::registry::load(data_dir)?;
    let Some(tache_ent) = registry.find(TACHE_ENTITY_KEY) else {
        return Ok(false);
    };

    let libelle_src = row
        .get("libelle")
        .and_then(|v| v.as_str())
        .unwrap_or("article");
    let libelle = format!("Déstockage — {libelle_src} (péremption {date_str})");
    let description = format!(
        "Article périssable : péremption dans {days} jour(s) ({date_str}).\n\
         Ligne stock : {stock_id}"
    );

    insert_destock_task(db, tache_ent, &libelle, &description, stock_id)?;
    Ok(true)
}

fn insert_destock_task(
    db: &Database,
    tache_ent: &EntityDef,
    libelle: &str,
    description: &str,
    stock_id: &str,
) -> Result<(), String> {
    let table = table_name(TACHE_ENTITY_KEY);
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let mut data = Map::new();
    data.insert("libelle".into(), json!(libelle));
    data.insert("description".into(), json!(description));
    data.insert("heure_debut".into(), json!("09:00"));
    data.insert("statut".into(), json!("a_faire"));
    data.insert("priorite".into(), json!("haute"));
    data.insert("type_tache".into(), json!(DESTOCK_TYPE));
    data.insert(
        super::tache_visibility::COL_VISIBILITE.into(),
        json!(super::tache_visibility::VIS_PUBLIQUE),
    );
    data.insert("entite_a_signer".into(), json!(STOCK_ENTITY_KEY));
    data.insert("enregistrement_id".into(), json!(stock_id));

    let mut columns = vec!["id".to_string(), "created_at".to_string()];
    let mut placeholders = vec!["?1".to_string(), "?2".to_string()];
    let mut values: Vec<rusqlite::types::Value> = vec![
        rusqlite::types::Value::Text(id),
        rusqlite::types::Value::Text(now),
    ];
    let mut idx = 3usize;

    for attr in &tache_ent.attributs {
        let col = attr_column(attr);
        if col == "id" || col == "created_at" {
            continue;
        }
        if !table_has_column(db, &table, &col)? {
            continue;
        }
        let key = attr.nom.as_str();
        let val = data.get(key).cloned().unwrap_or(Value::Null);
        let sql_val = super::validation::json_value_to_sql(&val, &attr.attr_type);
        columns.push(col);
        placeholders.push(format!("?{idx}"));
        idx += 1;
        values.push(sql_val);
    }

    let sql = format!(
        "INSERT INTO {table} ({}) VALUES ({})",
        columns.join(", "),
        placeholders.join(", ")
    );
    db.conn
        .execute(&sql, rusqlite::params_from_iter(values.iter()))
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn fetch_stock_row(db: &Database, id: &str) -> Result<Map<String, Value>, String> {
    let table = table_name(STOCK_ENTITY_KEY);
    db.conn
        .query_row(
            &format!("SELECT * FROM {table} WHERE id = ?1"),
            params![id],
            row_to_map,
        )
        .map_err(|e| e.to_string())
}

/// Supprime les lignes d'inventaire liées à une entité retirée du registre (ex. « atricles »).
pub fn purge_stock_for_entity(db: &Database, source_key: &str) -> Result<(), String> {
    let table = table_name(STOCK_ENTITY_KEY);
    if !table_exists(db, &table)? {
        return Ok(());
    }
    let _ = db.conn.execute(
        &format!("DELETE FROM {table} WHERE entite_source = ?1"),
        rusqlite::params![source_key],
    );
    Ok(())
}

fn table_exists(db: &Database, table: &str) -> Result<bool, String> {
    let n: i64 = db
        .conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name = ?1",
            params![table],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(n > 0)
}

fn record_label(ent: &EntityDef, row: &Map<String, Value>) -> String {
    let label_field = ent
        .attributs
        .iter()
        .find(|a| matches!(a.attr_type.as_str(), "string") && !super::attr_types::is_reserved_attribute(a))
        .map(|a| a.nom.as_str())
        .unwrap_or("libelle");
    row.get(label_field)
        .or_else(|| row.get("libelle"))
        .or_else(|| row.get("nom"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Article".into())
}

fn value_as_f64(v: Option<&Value>) -> Option<f64> {
    let v = v?;
    if let Some(n) = v.as_f64() {
        return Some(n);
    }
    if let Some(i) = v.as_i64() {
        return Some(i as f64);
    }
    if let Some(s) = v.as_str() {
        return s.parse().ok();
    }
    None
}

fn row_to_map(row: &rusqlite::Row<'_>) -> Result<Map<String, Value>, rusqlite::Error> {
    let mut m = Map::new();
    let count = row.as_ref().column_count();
    for i in 0..count {
        let name = row.as_ref().column_name(i)?.to_string();
        let val: rusqlite::types::Value = row.get(i)?;
        m.insert(name, rusqlite_value_to_json(val));
    }
    Ok(m)
}

fn rusqlite_value_to_json(v: rusqlite::types::Value) -> Value {
    match v {
        rusqlite::types::Value::Null => Value::Null,
        rusqlite::types::Value::Integer(i) => json!(i),
        rusqlite::types::Value::Real(f) => json!(f),
        rusqlite::types::Value::Text(s) => json!(s),
        rusqlite::types::Value::Blob(b) => json!(b),
    }
}
