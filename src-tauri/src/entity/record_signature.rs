//! Statut de signature des objets d'entité (`requires_signature` sur l'entité).

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

pub const SIGNATURE_STATUS_COLUMN: &str = "statut_signature";
pub const STATUS_NON_SIGNE: &str = "non_signe";
pub const STATUS_SIGNE: &str = "signe";

#[derive(Clone, Copy, Default)]
pub struct RowUserContext<'a> {
    pub role_id: Option<&'a str>,
    pub privileges: &'a [String],
}

pub fn entity_requires_signature(registry: &EntityRegistry, entity_key: &str) -> bool {
    registry
        .find(entity_key)
        .map(|e| e.requires_signature && !e.signatory_role_ids.is_empty())
        .unwrap_or(false)
}

pub fn ensure_signature_status_column(db: &Database, ent: &EntityDef) -> Result<(), String> {
    if !ent.requires_signature {
        return Ok(());
    }
    let table = table_name(&ent.nom);
    if table_has_column(db, &table, SIGNATURE_STATUS_COLUMN)? {
        return Ok(());
    }
    db.conn
        .execute(
            &format!(
                "ALTER TABLE {table} ADD COLUMN {SIGNATURE_STATUS_COLUMN} TEXT NOT NULL DEFAULT '{STATUS_SIGNE}'"
            ),
            [],
        )
        .map_err(|e| format!("ALTER {table}.{SIGNATURE_STATUS_COLUMN} : {e}"))?;
    Ok(())
}

/// Retire `statut_signature` si l'entité n'est plus soumise à signature.
pub fn prune_signature_status_column(db: &Database, ent: &EntityDef) -> Result<(), String> {
    if ent.requires_signature {
        return Ok(());
    }
    let table = table_name(&ent.nom);
    if !table_has_column(db, &table, SIGNATURE_STATUS_COLUMN)? {
        return Ok(());
    }
    let sql = format!("ALTER TABLE {table} DROP COLUMN {SIGNATURE_STATUS_COLUMN}");
    if let Err(e) = db.conn.execute(&sql, []) {
        eprintln!("DROP COLUMN {table}.{SIGNATURE_STATUS_COLUMN} : {e}");
    }
    Ok(())
}

/// À la création : non signé sauf si l'utilisateur est signataire agréé.
pub fn apply_signature_status_on_create(
    data: &mut Map<String, Value>,
    entity_key: &str,
    registry: &EntityRegistry,
    ctx: Option<RowUserContext<'_>>,
) {
    if !entity_requires_signature(registry, entity_key) {
        return;
    }
    let status = match ctx {
        Some(c) => c
            .role_id
            .filter(|role_id| can_sign_entity(registry, entity_key, role_id, c.privileges))
            .map(|_| STATUS_SIGNE)
            .unwrap_or(STATUS_NON_SIGNE),
        None => STATUS_NON_SIGNE,
    };
    data.insert(
        SIGNATURE_STATUS_COLUMN.into(),
        Value::String(status.into()),
    );
}

pub fn is_record_signed(
    db: &Database,
    entity_key: &str,
    record_id: &str,
    registry: &EntityRegistry,
) -> Result<bool, String> {
    if !entity_requires_signature(registry, entity_key) {
        return Ok(true);
    }
    let table = table_name(entity_key);
    if !table_has_column(db, &table, SIGNATURE_STATUS_COLUMN)? {
        return Ok(true);
    }
    let sql = format!("SELECT {SIGNATURE_STATUS_COLUMN} FROM {table} WHERE id = ?1");
    let status: Option<String> = db
        .conn
        .query_row(&sql, rusqlite::params![record_id], |row| row.get(0))
        .ok();
    Ok(status.as_deref() != Some(STATUS_NON_SIGNE))
}

pub fn assert_record_mutable(
    db: &Database,
    entity_key: &str,
    record_id: &str,
    registry: &EntityRegistry,
) -> Result<(), String> {
    if entity_requires_signature(registry, entity_key)
        && is_record_signed(db, entity_key, record_id, registry)?
    {
        return Err(
            "Impossible de modifier un objet signé. La signature verrouille l'enregistrement."
                .into(),
        );
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationSelectOptionExt {
    pub value: String,
    pub label: String,
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

pub fn can_sign_entity(
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
    if !ent.requires_signature {
        return false;
    }
    ent.signatory_role_ids.iter().any(|id| id == role_id)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatoryContact {
    pub user_id: String,
    pub nom: String,
    pub email: String,
    pub role_id: String,
    pub role_nom: String,
}

pub fn signatory_contacts_for_entity(
    db: &Database,
    registry: &EntityRegistry,
    entity_key: &str,
) -> Result<Vec<SignatoryContact>, String> {
    let Some(ent) = registry.find(entity_key) else {
        return Ok(Vec::new());
    };
    if !entity_requires_signature(registry, entity_key) {
        return Ok(Vec::new());
    }
    let roles = db.list_roles().map_err(|e| e.to_string())?;
    let users = db.list_users().map_err(|e| e.to_string())?;
    let signatory_ids: std::collections::HashSet<&str> =
        ent.signatory_role_ids.iter().map(String::as_str).collect();
    let mut contacts: Vec<SignatoryContact> = users
        .into_iter()
        .filter(|u| u.actif && signatory_ids.contains(u.role_id.as_str()))
        .map(|u| {
            let role_nom = roles
                .iter()
                .find(|r| r.id == u.role_id)
                .map(|r| r.nom.clone())
                .unwrap_or_else(|| u.role.clone());
            SignatoryContact {
                user_id: u.id,
                nom: u.nom,
                email: u.email,
                role_id: u.role_id,
                role_nom,
            }
        })
        .collect();
    contacts.sort_by(|a, b| {
        a.role_nom
            .cmp(&b.role_nom)
            .then_with(|| a.nom.cmp(&b.nom))
    });
    Ok(contacts)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordSignatureDetail {
    pub entity_key: String,
    pub entity_label: String,
    pub record_id: String,
    pub signed: bool,
    pub can_view: bool,
    pub can_sign: bool,
    pub fields: Vec<super::relations::RelationPanelField>,
    pub signatory_contacts: Vec<SignatoryContact>,
}

pub fn record_signature_detail(
    db: &Database,
    data_dir: &Path,
    entity_key: &str,
    record_id: &str,
    user_id: &str,
    privileges: &[String],
) -> Result<RecordSignatureDetail, String> {
    let registry = super::registry::load(data_dir)?;
    let role_id = user_role_id(db, user_id)?;
    let can_view = can_view_entity(privileges, entity_key);
    let can_sign = can_sign_entity(&registry, entity_key, &role_id, privileges);
    let signed = is_record_signed(db, entity_key, record_id, &registry)?;
    let signatory_contacts = signatory_contacts_for_entity(db, &registry, entity_key)?;
    let cfg = load_screen_config(data_dir, entity_key)?;
    let entity_label = cfg.screen.label.clone();
    let fields = if can_view {
        let row = crate::dda::crud::get_row(db, &cfg, record_id)?;
        row_to_panel_fields(&cfg, &row)
    } else {
        Vec::new()
    };
    Ok(RecordSignatureDetail {
        entity_key: entity_key.to_string(),
        entity_label,
        record_id: record_id.to_string(),
        signed,
        can_view,
        can_sign,
        fields,
        signatory_contacts,
    })
}

/// Signe l'objet une seule fois (n'importe quel signataire agréé) et clôture les tâches associées.
pub fn sign_record(
    db: &Database,
    data_dir: &Path,
    entity_key: &str,
    record_id: &str,
    user_id: &str,
    privileges: &[String],
) -> Result<(), String> {
    let registry = super::registry::load(data_dir)?;
    if !entity_requires_signature(&registry, entity_key) {
        return Err("Cette entité ne requiert pas de signature.".into());
    }
    if is_record_signed(db, entity_key, record_id, &registry)? {
        return Err("Cet objet est déjà signé.".into());
    }
    let role_id = user_role_id(db, user_id)?;
    if !can_sign_entity(&registry, entity_key, &role_id, privileges) {
        return Err("Vous n'êtes pas autorisé à signer cet objet.".into());
    }
    if !can_view_entity(privileges, entity_key) {
        return Err("Droit insuffisant pour consulter cette entité.".into());
    }

    let table = table_name(entity_key);
    if table_has_column(db, &table, SIGNATURE_STATUS_COLUMN)? {
        let n = db
            .conn
            .execute(
                &format!("UPDATE {table} SET {SIGNATURE_STATUS_COLUMN} = ?1 WHERE id = ?2"),
                rusqlite::params![STATUS_SIGNE, record_id],
            )
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("Enregistrement introuvable.".into());
        }
    }

    close_signature_tasks(db, &registry, entity_key, record_id)?;
    Ok(())
}

pub fn close_signature_tasks(
    db: &Database,
    registry: &EntityRegistry,
    entity_key: &str,
    record_id: &str,
) -> Result<(), String> {
    if registry.find(TACHE_ENTITY_KEY).is_none() {
        return Ok(());
    }
    let tache_table = table_name(TACHE_ENTITY_KEY);
    let sql = format!(
        "UPDATE {tache_table} SET statut = 'terminee'
         WHERE type_tache = 'signature'
           AND entite_a_signer = ?1
           AND enregistrement_id = ?2
           AND statut != 'terminee'"
    );
    db.conn
        .execute(&sql, rusqlite::params![entity_key, record_id])
        .map_err(|e| format!("Clôture des tâches de signature : {e}"))?;
    Ok(())
}
