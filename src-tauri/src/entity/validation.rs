//! Tâches de validation — trigger système à chaque `create_row` (entité `requires_validation`).

use chrono::Utc;
use serde_json::{json, Map, Value};
use uuid::Uuid;

use super::record_validation;
use super::registry::EntityDef;
use super::schema::{attr_column, table_has_column, table_name};
use crate::db::Database;
use crate::dda::crud;

const TACHE_ENTITY_KEY: &str = "tache";

/// Hook post-insert : tâches de validation obligatoires ; annule la création si échec.
pub fn after_entity_row_created(
    db: &Database,
    entity_key: &str,
    created_row: &Map<String, Value>,
) -> Result<(), String> {
    match spawn_validation_tasks(db, &db.data_dir, entity_key, created_row) {
        Ok(_) => Ok(()),
        Err(e) => {
            let registry = super::registry::load(&db.data_dir)?;
            if !record_validation::entity_requires_validation(&registry, entity_key) {
                eprintln!("Tâches de validation (non bloquant) : {e}");
                return Ok(());
            }
            let id = created_row
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !id.is_empty() {
                if let Ok(cfg) = super::load_screen_config(&db.data_dir, entity_key) {
                    let _ = crud::delete_row(db, &cfg, id);
                }
            }
            Err(format!(
                "Création annulée : les tâches de validation n'ont pas pu être générées ({e}). \
                 Vérifiez que l'entité « tache » existe dans le registre."
            ))
        }
    }
}

/// Une tâche privée par rôle valideur dès qu'un enregistrement est créé sur une entité « à valider ».
pub fn spawn_validation_tasks(
    db: &Database,
    data_dir: &std::path::Path,
    source_entity_key: &str,
    created_row: &Map<String, Value>,
) -> Result<Vec<String>, String> {
    if source_entity_key == TACHE_ENTITY_KEY {
        return Ok(Vec::new());
    }
    if is_validation_task_row(created_row) {
        return Ok(Vec::new());
    }

    let registry = super::registry::load(data_dir)?;
    let Some(source) = registry.find(source_entity_key) else {
        return Ok(Vec::new());
    };
    if !source.requires_validation || source.validator_role_ids.is_empty() {
        return Ok(Vec::new());
    }

    let Some(tache_ent) = registry.find(TACHE_ENTITY_KEY) else {
        return Err(
            "Entité « tache » absente du registre — impossible de créer les validations.".into(),
        );
    };

    let record_id = created_row
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("Identifiant de l'enregistrement créé introuvable.")?;
    let summary = record_summary(source, created_row);
    let source_label = source.label.as_deref().unwrap_or(&source.nom);
    let roles = db.list_roles().map_err(|e| e.to_string())?;

    let mut created_task_ids = Vec::new();
    for role_id in &source.validator_role_ids {
        let role_nom = roles
            .iter()
            .find(|r| &r.id == role_id)
            .map(|r| r.nom.as_str())
            .unwrap_or(role_id.as_str());
        let libelle = format!("Valider {source_label} — {summary}");
        let required_labels = required_attribute_labels(source);
        let champs_obligatoires = if required_labels.is_empty() {
            "—".to_string()
        } else {
            required_labels.join(", ")
        };
        let description = format!(
            "Validation requise pour l'entité « {source_label} » ({source_entity_key}).\n\
             Enregistrement : {record_id}\n\
             Rôle valideur : {role_nom}\n\
             Champs obligatoires à contrôler : {champs_obligatoires}"
        );
        let task_id = insert_validation_task(
            db,
            tache_ent,
            &libelle,
            &description,
            source_entity_key,
            record_id,
            role_id,
        )?;
        created_task_ids.push(task_id);
    }
    Ok(created_task_ids)
}

fn required_attribute_labels(ent: &EntityDef) -> Vec<String> {
    ent.attributs
        .iter()
        .filter(|a| a.required)
        .map(|a| {
            a.label
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(a.nom.as_str())
                .to_string()
        })
        .collect()
}

fn is_validation_task_row(row: &Map<String, Value>) -> bool {
    row.get("type_tache")
        .and_then(|v| v.as_str())
        .map(|s| s == "validation")
        .unwrap_or(false)
}

fn record_summary(ent: &EntityDef, row: &Map<String, Value>) -> String {
    const PRIORITY: &[&str] = &[
        "libelle",
        "nom",
        "titre",
        "reference",
        "nom_classe",
        "intitule",
    ];
    for key in PRIORITY {
        if let Some(Value::String(s)) = row.get(*key) {
            let t = s.trim();
            if !t.is_empty() {
                return t.to_string();
            }
        }
    }
    for attr in &ent.attributs {
        if !matches!(attr.attr_type.as_str(), "string" | "email") {
            continue;
        }
        if let Some(Value::String(s)) = row.get(&attr.nom) {
            let t = s.trim();
            if !t.is_empty() {
                return t.to_string();
            }
        }
    }
    row.get("id")
        .map(|v| match v {
            Value::String(s) => s.clone(),
            _ => v.to_string(),
        })
        .unwrap_or_else(|| "nouvel enregistrement".into())
}

fn insert_validation_task(
    db: &Database,
    tache_ent: &EntityDef,
    libelle: &str,
    description: &str,
    source_entity_key: &str,
    record_id: &str,
    role_id: &str,
) -> Result<String, String> {
    let table = table_name(TACHE_ENTITY_KEY);
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let mut data = Map::new();
    data.insert("libelle".into(), json!(libelle));
    data.insert("description".into(), json!(description));
    data.insert("heure_debut".into(), json!("09:00"));
    data.insert("statut".into(), json!("a_faire"));
    data.insert("priorite".into(), json!("normale"));
    data.insert("type_tache".into(), json!("validation"));
    data.insert(
        super::tache_visibility::COL_VISIBILITE.into(),
        json!(super::tache_visibility::VIS_PRIVEE),
    );
    data.insert("entite_a_valider".into(), json!(source_entity_key));
    data.insert("enregistrement_id".into(), json!(record_id));
    data.insert("role_validateur".into(), json!(role_id));

    let mut columns = vec!["id".to_string(), "created_at".to_string()];
    let mut placeholders = vec!["?1".to_string(), "?2".to_string()];
    let mut values: Vec<rusqlite::types::Value> = vec![
        rusqlite::types::Value::Text(id.clone()),
        rusqlite::types::Value::Text(now),
    ];
    let mut idx = 3usize;

    for attr in tache_ent.attributs.iter() {
        let col = attr_column(attr);
        if col == "id" || col == "created_at" {
            continue;
        }
        if !table_has_column(db, &table, &col)? {
            continue;
        }
        let key = attr.nom.as_str();
        let val = data.get(key).cloned().unwrap_or(Value::Null);
        let sql_val = json_value_to_sql(&val, &attr.attr_type);
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
        .map_err(|e| format!("Création tâche de validation : {e}"))?;

    Ok(id)
}

pub fn json_value_to_sql(v: &Value, attr_type: &str) -> rusqlite::types::Value {
    match v {
        Value::Null => rusqlite::types::Value::Null,
        Value::Bool(b) => rusqlite::types::Value::Integer(if *b { 1 } else { 0 }),
        Value::Number(n) => rusqlite::types::Value::Real(n.as_f64().unwrap_or(0.0)),
        Value::String(s) => rusqlite::types::Value::Text(s.clone()),
        other => rusqlite::types::Value::Text(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn record_summary_prefers_libelle() {
        let ent = EntityDef {
            nom: "x".into(),
            label: None,
            description: None,
            ai_suggestions: true,
            requires_validation: false,
            validator_role_ids: vec![],
            attributs: vec![],
        };
        let mut row = Map::new();
        row.insert("libelle".into(), json!("Mon titre"));
        row.insert("id".into(), json!("abc"));
        assert_eq!(record_summary(&ent, &row), "Mon titre");
    }
}
