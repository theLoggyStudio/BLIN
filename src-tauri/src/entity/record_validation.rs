//! Statut de validation des enregistrements (`requires_validation` sur l'entité).

use std::path::Path;

use serde::Serialize;
use serde_json::{Map, Value};

use super::load_screen_config;
use super::registry::{EntityDef, EntityRegistry};
use super::relations::row_to_panel_fields;
use super::schema::{table_has_column, table_name};
const TACHE_ENTITY_KEY: &str = "tache";
use crate::db::Database;
use crate::privileges::has_privilege;

pub const VALIDATION_STATUS_COLUMN: &str = "statut_validation";
pub const STATUS_NON_VALIDE: &str = "non_valide";
pub const STATUS_VALIDE: &str = "valide";

pub fn entity_requires_validation(registry: &EntityRegistry, entity_key: &str) -> bool {
    registry
        .find(entity_key)
        .map(|e| e.requires_validation && !e.validator_role_ids.is_empty())
        .unwrap_or(false)
}

pub fn ensure_validation_status_column(db: &Database, ent: &EntityDef) -> Result<(), String> {
    if !ent.requires_validation {
        return Ok(());
    }
    let table = table_name(&ent.nom);
    if table_has_column(db, &table, VALIDATION_STATUS_COLUMN)? {
        return Ok(());
    }
    db.conn
        .execute(
            &format!(
                "ALTER TABLE {table} ADD COLUMN {VALIDATION_STATUS_COLUMN} TEXT NOT NULL DEFAULT '{STATUS_VALIDE}'"
            ),
            [],
        )
        .map_err(|e| format!("ALTER {table}.{VALIDATION_STATUS_COLUMN} : {e}"))?;
    Ok(())
}

pub fn set_non_valide_on_create(data: &mut Map<String, Value>, entity_key: &str, registry: &EntityRegistry) {
    if entity_requires_validation(registry, entity_key) {
        data.insert(
            VALIDATION_STATUS_COLUMN.into(),
            Value::String(STATUS_NON_VALIDE.into()),
        );
    }
}

pub fn is_record_validated(
    db: &Database,
    entity_key: &str,
    record_id: &str,
    registry: &EntityRegistry,
) -> Result<bool, String> {
    if !entity_requires_validation(registry, entity_key) {
        return Ok(true);
    }
    let table = table_name(entity_key);
    if !table_has_column(db, &table, VALIDATION_STATUS_COLUMN)? {
        return Ok(true);
    }
    let sql = format!(
        "SELECT {VALIDATION_STATUS_COLUMN} FROM {table} WHERE id = ?1"
    );
    let status: Option<String> = db
        .conn
        .query_row(&sql, rusqlite::params![record_id], |row| row.get(0))
        .ok();
    Ok(status.as_deref() != Some(STATUS_NON_VALIDE))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationSelectOptionExt {
    pub value: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_status: Option<String>,
}

pub fn user_role_id(db: &Database, user_id: &str) -> Result<String, String> {
    db.conn
        .query_row(
            "SELECT role_id FROM users WHERE id = ?1 AND actif = 1",
            rusqlite::params![user_id],
            |row| row.get(0),
        )
        .map_err(|_| "Utilisateur introuvable.".to_string())
}

pub fn can_view_entity(privileges: &[String], entity_key: &str) -> bool {
    has_privilege(privileges, &format!("{entity_key}:voir"))
}

pub fn can_validate_entity(
    registry: &EntityRegistry,
    entity_key: &str,
    role_id: &str,
    privileges: &[String],
) -> bool {
    if has_privilege(privileges, "*") {
        return true;
    }
    let Some(ent) = registry.find(entity_key) else {
        return false;
    };
    if !ent.requires_validation {
        return false;
    }
    ent.validator_role_ids.iter().any(|id| id == role_id)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordValidationDetail {
    pub entity_key: String,
    pub entity_label: String,
    pub record_id: String,
    pub validated: bool,
    pub can_view: bool,
    pub can_validate: bool,
    pub fields: Vec<super::relations::RelationPanelField>,
}

pub fn record_validation_detail(
    db: &Database,
    data_dir: &Path,
    entity_key: &str,
    record_id: &str,
    user_id: &str,
    privileges: &[String],
) -> Result<RecordValidationDetail, String> {
    let registry = super::registry::load(data_dir)?;
    let role_id = user_role_id(db, user_id)?;
    let can_view = can_view_entity(privileges, entity_key);
    if !can_view {
        return Err("Droit insuffisant pour consulter cette entité.".into());
    }
    let can_validate = can_validate_entity(&registry, entity_key, &role_id, privileges);
    let cfg = load_screen_config(data_dir, entity_key)?;
    let row = crate::dda::crud::get_row(db, &cfg, record_id)?;
    let validated = is_record_validated(db, entity_key, record_id, &registry)?;
    let entity_label = cfg.screen.label.clone();
    Ok(RecordValidationDetail {
        entity_key: entity_key.to_string(),
        entity_label,
        record_id: record_id.to_string(),
        validated,
        can_view,
        can_validate,
        fields: row_to_panel_fields(&cfg, &row),
    })
}

/// Marque l'enregistrement validé et clôture les tâches de validation associées.
pub fn validate_record(
    db: &Database,
    data_dir: &Path,
    entity_key: &str,
    record_id: &str,
    user_id: &str,
    privileges: &[String],
) -> Result<(), String> {
    let registry = super::registry::load(data_dir)?;
    if !entity_requires_validation(&registry, entity_key) {
        return Err("Cette entité ne requiert pas de validation.".into());
    }
    let role_id = user_role_id(db, user_id)?;
    if !can_validate_entity(&registry, entity_key, &role_id, privileges) {
        return Err("Vous n'êtes pas autorisé à valider cet enregistrement.".into());
    }
    if !can_view_entity(privileges, entity_key) {
        return Err("Droit insuffisant pour consulter cette entité.".into());
    }

    let table = table_name(entity_key);
    if table_has_column(db, &table, VALIDATION_STATUS_COLUMN)? {
        let n = db
            .conn
            .execute(
                &format!(
                    "UPDATE {table} SET {VALIDATION_STATUS_COLUMN} = ?1 WHERE id = ?2"
                ),
                rusqlite::params![STATUS_VALIDE, record_id],
            )
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("Enregistrement introuvable.".into());
        }
    }

    if registry.find(TACHE_ENTITY_KEY).is_some() {
        let tache_table = table_name(TACHE_ENTITY_KEY);
        let sql = format!(
            "UPDATE {tache_table} SET statut = 'terminee'
             WHERE type_tache = 'validation'
               AND entite_a_valider = ?1
               AND enregistrement_id = ?2
               AND statut != 'terminee'"
        );
        let _ = db
            .conn
            .execute(&sql, rusqlite::params![entity_key, record_id])
            .map_err(|e| format!("Clôture des tâches de validation : {e}"))?;
    }

    Ok(())
}
