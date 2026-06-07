use std::collections::HashSet;

use serde::Serialize;
use serde_json::{Map, Value};

use super::load_screen_config;
use super::record_signature::{self, RelationSelectOptionExt, SIGNATURE_STATUS_COLUMN, STATUS_NON_SIGNE, STATUS_SIGNE};
use super::schema::table_has_column;
use super::registry::{EntityAttribute, EntityDef, EntityRegistry};
use crate::dda::config::FieldDef;
use crate::db::Database;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationPanelField {
    pub key: String,
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationPanel {
    pub entity_key: String,
    pub label: String,
    pub primary: bool,
    pub via_field: Option<String>,
    pub fields: Vec<RelationPanelField>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationDetailResponse {
    pub panels: Vec<RelationPanel>,
}

fn humanize_entity_name(nom: &str) -> String {
    let s = nom.replace('_', " ");
    let mut chars = s.chars();
    match chars.next() {
        None => nom.to_string(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn stub_entity(nom: &str) -> EntityDef {
    EntityDef {
        nom: nom.to_string(),
        label: Some(humanize_entity_name(nom)),
        description: None,
        ai_suggestions: true,
        requires_signature: false,
        signatory_role_ids: vec![],
        is_session: false,
        attributs: vec![EntityAttribute {
            nom: "nom".into(),
            attr_type: "string".into(),
            label: Some("Nom".into()),
            required: false,
            r#ref: None,
            relation_multiple: false,
            relation_exclusive_parent: true,
            default: None,
            enum_options: None,
        }],
    }
}

/// Trigger : crée automatiquement les entités référencées par des attributs `entity`.
pub fn ensure_referenced_entities(registry: &mut EntityRegistry) -> Vec<String> {
    let mut existing: HashSet<String> = registry.entities.iter().map(|e| e.nom.clone()).collect();
    let mut pending: Vec<String> = Vec::new();

    for ent in &registry.entities {
        for attr in &ent.attributs {
            if attr.attr_type != "entity" {
                continue;
            }
            let Some(ref_key) = attr.r#ref.as_deref().map(str::trim).filter(|s| !s.is_empty())
            else {
                continue;
            };
            let key = ref_key.to_lowercase().replace(' ', "_");
            if !existing.contains(&key) && !pending.contains(&key) {
                pending.push(key);
            }
        }
    }

    let mut created = Vec::new();
    for key in pending {
        registry.entities.push(stub_entity(&key));
        existing.insert(key.clone());
        created.push(key);
    }
    created
}

pub fn field_ref_entity(field: &FieldDef) -> Option<&str> {
    field
        .form
        .as_ref()
        .and_then(|f| f.ref_entity.as_deref())
        .filter(|s| !s.is_empty())
}

fn display_value(v: &Value) -> String {
    match v {
        Value::Null => "—".into(),
        Value::Bool(b) => if *b { "Oui" } else { "Non" }.into(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

pub fn row_to_panel_fields(
    cfg: &crate::dda::config::ScreenConfigFile,
    row: &Map<String, Value>,
) -> Vec<RelationPanelField> {
    cfg.fields
        .iter()
        .filter(|f| {
            f.field_type != "hidden"
                && f.field_type != "detail_link"
                && f.field_type != "entity_embed"
                && f.field_type != "entity_embed_list"
                && f.form.as_ref().and_then(|m| m.embed_parent.as_ref()).is_none()
        })
        .map(|f| {
            let raw = row.get(&f.key).or_else(|| row.get(&f.column));
            RelationPanelField {
                key: f.key.clone(),
                label: f.label.clone(),
                value: raw.map(display_value).unwrap_or_else(|| "—".into()),
            }
        })
        .collect()
}

fn embed_panel_fields(
    cfg: &crate::dda::config::ScreenConfigFile,
    parent: &Map<String, Value>,
    field_key: &str,
) -> Vec<RelationPanelField> {
    cfg.fields
        .iter()
        .filter(|f| {
            f.form
                .as_ref()
                .and_then(|m| m.embed_parent.as_deref())
                == Some(field_key)
        })
        .map(|f| {
            let raw = parent.get(&f.key).or_else(|| parent.get(&f.column));
            RelationPanelField {
                key: f.key.clone(),
                label: f.label.clone(),
                value: raw.map(display_value).unwrap_or_else(|| "—".into()),
            }
        })
        .collect()
}

fn parse_embed_list_objects(raw: Option<&Value>) -> Vec<Map<String, Value>> {
    let Some(v) = raw else {
        return Vec::new();
    };
    if let Value::Array(items) = v {
        return items
            .iter()
            .filter_map(|item| item.as_object().cloned())
            .collect();
    }
    if let Value::String(s) = v {
        let t = s.trim();
        if t.is_empty() {
            return Vec::new();
        }
        if let Ok(Value::Array(items)) = serde_json::from_str::<Value>(t) {
            return items
                .iter()
                .filter_map(|item| item.as_object().cloned())
                .collect();
        }
    }
    Vec::new()
}

pub fn relation_detail(
    db: &Database,
    data_dir: &std::path::Path,
    screen_key: &str,
    record_id: &str,
) -> Result<RelationDetailResponse, String> {
    let cfg = load_screen_config(data_dir, screen_key)?;
    let parent = crate::dda::crud::get_row(db, &cfg, record_id)?;

    let parent_label = cfg.screen.label.clone();
    let mut panels = vec![RelationPanel {
        entity_key: screen_key.to_string(),
        label: parent_label,
        primary: true,
        via_field: None,
        fields: row_to_panel_fields(&cfg, &parent),
    }];

    for field in &cfg.fields {
        if field.field_type == "entity_ref" {
            let Some(ref_key) = field_ref_entity(field) else {
                continue;
            };
            let fk = parent
                .get(&field.key)
                .or_else(|| parent.get(&field.column))
                .and_then(|v| {
                    if v.is_null() {
                        None
                    } else {
                        Some(v.to_string().trim_matches('"').to_string())
                    }
                })
                .filter(|s| !s.is_empty());
            let Some(fk_id) = fk else {
                continue;
            };

            let ref_cfg = load_screen_config(data_dir, ref_key)?;
            let ref_row = crate::dda::crud::get_row(db, &ref_cfg, &fk_id)?;
            let ref_label = ref_cfg.screen.label.clone();
            panels.push(RelationPanel {
                entity_key: ref_key.to_string(),
                label: ref_label,
                primary: false,
                via_field: Some(field.key.clone()),
                fields: row_to_panel_fields(&ref_cfg, &ref_row),
            });
            continue;
        }

        if field.field_type == "entity_embed" {
            let Some(ref_key) = field_ref_entity(field) else {
                continue;
            };
            let embed_fields = embed_panel_fields(&cfg, &parent, &field.key);
            if embed_fields.is_empty() {
                continue;
            }
            let ref_cfg = load_screen_config(data_dir, ref_key)?;
            panels.push(RelationPanel {
                entity_key: ref_key.to_string(),
                label: ref_cfg.screen.label.clone(),
                primary: false,
                via_field: Some(field.key.clone()),
                fields: embed_fields,
            });
            continue;
        }

        if field.field_type == "entity_embed_list" {
            let Some(ref_key) = field_ref_entity(field) else {
                continue;
            };
            let ref_cfg = load_screen_config(data_dir, ref_key)?;
            let raw = parent.get(&field.key).or_else(|| parent.get(&field.column));
            let items = parse_embed_list_objects(raw);
            if items.is_empty() {
                continue;
            }
            for (idx, obj) in items.into_iter().enumerate() {
                let fields: Vec<RelationPanelField> = obj
                    .iter()
                    .map(|(k, v)| RelationPanelField {
                        key: k.clone(),
                        label: k.clone(),
                        value: display_value(v),
                    })
                    .collect();
                panels.push(RelationPanel {
                    entity_key: ref_key.to_string(),
                    label: format!(
                        "{} — élément {}",
                        ref_cfg.screen.label,
                        idx + 1
                    ),
                    primary: false,
                    via_field: Some(field.key.clone()),
                    fields,
                });
            }
        }
    }

    Ok(RelationDetailResponse { panels })
}

/// Options pour un champ entity_ref : enregistrements libres (non déjà liés sur cette entité parente).
pub fn relation_select_options(
    db: &Database,
    data_dir: &std::path::Path,
    screen_key: &str,
    field_key: &str,
    exclude_record_id: Option<&str>,
) -> Result<Vec<RelationSelectOptionExt>, String> {
    fn parse_relation_ids(raw: &str) -> Vec<String> {
        let t = raw.trim();
        if t.is_empty() {
            return Vec::new();
        }
        if let Ok(Value::Array(items)) = serde_json::from_str::<Value>(t) {
            return items
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
                .filter(|s| !s.is_empty())
                .collect();
        }
        vec![t.to_string()]
    }

    let parent_cfg = load_screen_config(data_dir, screen_key)?;
    let field = parent_cfg
        .fields
        .iter()
        .find(|f| f.key == field_key)
        .ok_or_else(|| format!("Champ « {field_key} » introuvable."))?;
    if field.field_type != "entity_ref"
        && field.field_type != "entity_ref_list"
        && field.field_type != "entity_embed"
        && field.field_type != "entity_embed_list"
    {
        return Err(format!("Le champ « {field_key} » n'est pas une liaison entité."));
    }
    let is_embed = field.field_type == "entity_embed" || field.field_type == "entity_embed_list";
    let ref_key = field_ref_entity(field)
        .ok_or_else(|| format!("Liaison « {field_key} » sans entité cible (refEntity)."))?;
    let ref_cfg = load_screen_config(data_dir, ref_key)?;
    let registry = super::registry::load(data_dir)?;
    let ref_requires_signature = record_signature::entity_requires_signature(&registry, ref_key);
    let parent_table = &parent_cfg.screen.table;
    let fk_column = &field.column;
    let pk = &parent_cfg.screen.primary_key;

    let exclusive_parent = field
        .form
        .as_ref()
        .and_then(|f| f.relation_exclusive_parent)
        .unwrap_or(true);
    let mut used: HashSet<String> = HashSet::new();
    if exclusive_parent && !is_embed {
        let sql = format!(
            "SELECT {pk}, {fk_column} FROM {parent_table} WHERE {fk_column} IS NOT NULL AND TRIM({fk_column}) != ''"
        );
        let mut stmt = db.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?;
        for (parent_id, fk_raw) in rows.flatten() {
            if exclude_record_id.map(|x| x == parent_id).unwrap_or(false) {
                continue;
            }
            for id in parse_relation_ids(&fk_raw) {
                used.insert(id);
            }
        }
    }

    let ref_table = &ref_cfg.screen.table;
    let label_col = &ref_cfg.screen.label_field;
    let has_status_col =
        ref_requires_signature && table_has_column(db, ref_table, SIGNATURE_STATUS_COLUMN)?;

    let list_sql = if has_status_col {
        format!(
            "SELECT {pk}, {label_col}, {SIGNATURE_STATUS_COLUMN} FROM {ref_table} ORDER BY {label_col}"
        )
    } else {
        format!("SELECT {pk}, {label_col} FROM {ref_table} ORDER BY {label_col}")
    };
    let mut stmt = db.conn.prepare(&list_sql).map_err(|e| e.to_string())?;
    let rows: Vec<(String, String, Option<String>)> = if has_status_col {
        stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|e| e.to_string())?
        .flatten()
        .collect()
    } else {
        stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let label: String = row.get(1)?;
            Ok((id, label, None))
        })
        .map_err(|e| e.to_string())?
        .flatten()
        .collect()
    };

    let current_fks: HashSet<String> = if is_embed {
        HashSet::new()
    } else {
        exclude_record_id
            .and_then(|parent_id| {
                let sql = format!(
                    "SELECT {fk_column} FROM {parent_table} WHERE {pk} = ?1",
                    fk_column = fk_column,
                    parent_table = parent_table,
                    pk = pk,
                );
                db.conn
                    .query_row(&sql, rusqlite::params![parent_id], |row| {
                        row.get::<_, String>(0)
                    })
                    .ok()
            })
            .map(|raw| parse_relation_ids(&raw).into_iter().collect())
            .unwrap_or_default()
    };

    let mut options = Vec::new();
    options.push(RelationSelectOptionExt {
        value: "".into(),
        label: "— Aucun —".into(),
    });
    for (id, label, statut) in rows {
        let non_signe = !is_embed && statut.as_deref() == Some(STATUS_NON_SIGNE);
        if non_signe {
            continue;
        }
        let is_current = current_fks.contains(id.as_str());
        if used.contains(&id) && !is_current {
            continue;
        }
        let display = if label.trim().is_empty() {
            id.clone()
        } else {
            label
        };
        options.push(RelationSelectOptionExt {
            value: id,
            label: display,
        });
    }
    Ok(options)
}

/// Copie les valeurs d'un enregistrement fille vers les clés embarquées du parent (1-1).
pub fn embed_values_from_record(
    db: &Database,
    data_dir: &std::path::Path,
    screen_key: &str,
    field_key: &str,
    record_id: &str,
) -> Result<Map<String, Value>, String> {
    let registry = super::registry::load(data_dir)?;
    let parent_ent = registry
        .find(screen_key)
        .ok_or_else(|| format!("Entité « {screen_key} » introuvable."))?;
    let parent_attr = parent_ent
        .attributs
        .iter()
        .find(|a| a.nom == field_key)
        .ok_or_else(|| format!("Attribut « {field_key} » introuvable."))?;
    if parent_attr.attr_type != "entity" {
        return Err(format!("« {field_key} » n'est pas une liaison entité."));
    }
    let child = super::embed::resolve_child(&registry, parent_attr)
        .ok_or_else(|| format!("Entité fille introuvable pour « {field_key} »."))?;
    let ref_cfg = load_screen_config(data_dir, &child.nom)?;
    let row = crate::dda::crud::get_row(db, &ref_cfg, record_id)?;
    Ok(super::embed::values_from_child_row(parent_attr, child, &row))
}

/// Objet fille (clés sans préfixe) pour liste embarquée.
pub fn embed_child_object_from_record(
    db: &Database,
    data_dir: &std::path::Path,
    screen_key: &str,
    field_key: &str,
    record_id: &str,
) -> Result<Map<String, Value>, String> {
    let registry = super::registry::load(data_dir)?;
    let parent_ent = registry
        .find(screen_key)
        .ok_or_else(|| format!("Entité « {screen_key} » introuvable."))?;
    let parent_attr = parent_ent
        .attributs
        .iter()
        .find(|a| a.nom == field_key)
        .ok_or_else(|| format!("Attribut « {field_key} » introuvable."))?;
    let child = super::embed::resolve_child(&registry, parent_attr)
        .ok_or_else(|| format!("Entité fille introuvable pour « {field_key} »."))?;
    let ref_cfg = load_screen_config(data_dir, &child.nom)?;
    let row = crate::dda::crud::get_row(db, &ref_cfg, record_id)?;
    Ok(super::embed::child_object_from_row_for_embed_list(child, &row))
}

pub fn registry_entity_ref_targets(registry: &EntityRegistry) -> Vec<(String, String)> {
    registry
        .entities
        .iter()
        .map(|e| {
            let label = e.label.clone().unwrap_or_else(|| e.nom.clone());
            (e.nom.clone(), label)
        })
        .collect()
}
