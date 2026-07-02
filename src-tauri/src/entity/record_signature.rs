//! Statut de signature des objets d'entité (`requires_signature` sur l'entité).

use std::path::Path;

use serde::Serialize;
use serde_json::{Map, Value};

use chrono::Utc;
use uuid::Uuid;

use super::load_screen_config;
use super::registry::{EntityDef, EntityRegistry};
use super::relations::{build_relation_panels, row_to_panel_fields, RelationPanel};
use super::schema::{table_has_column, table_name};
const TACHE_ENTITY_KEY: &str = "tache";
use crate::db::Database;
use crate::privileges::has_privilege;

pub const SIGNATURE_STATUS_COLUMN: &str = "statut_signature";
pub const SIGNED_BY_COLUMN: &str = "signe_par";
pub const CREATED_BY_COLUMN: &str = "cree_par";
pub const STATUS_NON_SIGNE: &str = "non_signe";
pub const STATUS_SIGNE: &str = "signe";
pub const STATUS_REFUSE: &str = "refuse";
pub const REFUSED_BY_COLUMN: &str = "refuse_par";
pub const REFUSAL_REASON_COLUMN: &str = "motif_refus";

#[derive(Clone, Copy)]
pub struct RowUserContext<'a> {
    pub user_id: &'a str,
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
    if !table_has_column(db, &table, SIGNATURE_STATUS_COLUMN)? {
        db.conn
            .execute(
                &format!(
                    "ALTER TABLE {table} ADD COLUMN {SIGNATURE_STATUS_COLUMN} TEXT NOT NULL DEFAULT '{STATUS_NON_SIGNE}'"
                ),
                [],
            )
            .map_err(|e| format!("ALTER {table}.{SIGNATURE_STATUS_COLUMN} : {e}"))?;
    }
    for col in [
        SIGNED_BY_COLUMN,
        CREATED_BY_COLUMN,
        REFUSED_BY_COLUMN,
        REFUSAL_REASON_COLUMN,
    ] {
        if !table_has_column(db, &table, col)? {
            db.conn
                .execute(&format!("ALTER TABLE {table} ADD COLUMN {col} TEXT"), [])
                .map_err(|e| format!("ALTER {table}.{col} : {e}"))?;
        }
    }
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

/// À la création : toujours « non signé » — la signature est une action explicite des rôles signataires.
pub fn apply_signature_status_on_create(
    data: &mut Map<String, Value>,
    entity_key: &str,
    registry: &EntityRegistry,
) {
    if !entity_requires_signature(registry, entity_key) {
        return;
    }
    data.insert(
        SIGNATURE_STATUS_COLUMN.into(),
        Value::String(STATUS_NON_SIGNE.into()),
    );
}

/// Enregistre l'auteur de la création (pour notification post-signature).
pub fn apply_creator_on_create(
    data: &mut Map<String, Value>,
    entity_key: &str,
    registry: &EntityRegistry,
    user_ctx: Option<RowUserContext<'_>>,
) {
    if !entity_requires_signature(registry, entity_key) {
        return;
    }
    let Some(ctx) = user_ctx else {
        return;
    };
    data.insert(
        CREATED_BY_COLUMN.into(),
        Value::String(ctx.user_id.to_string()),
    );
}

pub fn user_display_name(db: &Database, user_id: &str) -> Result<String, String> {
    db.conn
        .query_row(
            "SELECT nom FROM users WHERE id = ?1 AND actif = 1",
            rusqlite::params![user_id],
            |row| row.get(0),
        )
        .map_err(|_| "Utilisateur introuvable.".to_string())
}

pub fn signer_label(db: &Database, user_id: &str, role_id: &str) -> Result<String, String> {
    let nom = user_display_name(db, user_id)?;
    let role_nom = db
        .list_roles()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|r| r.id == role_id)
        .map(|r| r.nom)
        .unwrap_or_else(|| role_id.to_string());
    Ok(format!("{nom} ({role_nom})"))
}

fn record_string_column(
    db: &Database,
    entity_key: &str,
    record_id: &str,
    column: &str,
) -> Result<Option<String>, String> {
    let table = table_name(entity_key);
    if !table_has_column(db, &table, column)? {
        return Ok(None);
    }
    let sql = format!("SELECT {column} FROM {table} WHERE id = ?1");
    let val: Option<String> = db
        .conn
        .query_row(&sql, rusqlite::params![record_id], |row| row.get(0))
        .ok()
        .flatten();
    Ok(val.filter(|s| !s.trim().is_empty()))
}

pub fn record_signature_status(
    db: &Database,
    entity_key: &str,
    record_id: &str,
    registry: &EntityRegistry,
) -> Result<Option<String>, String> {
    if !entity_requires_signature(registry, entity_key) {
        return Ok(None);
    }
    let table = table_name(entity_key);
    if !table_has_column(db, &table, SIGNATURE_STATUS_COLUMN)? {
        return Ok(None);
    }
    let sql = format!("SELECT {SIGNATURE_STATUS_COLUMN} FROM {table} WHERE id = ?1");
    let status: Option<String> = db
        .conn
        .query_row(&sql, rusqlite::params![record_id], |row| row.get(0))
        .ok();
    Ok(status)
}

pub fn is_record_signed(
    db: &Database,
    entity_key: &str,
    record_id: &str,
    registry: &EntityRegistry,
) -> Result<bool, String> {
    Ok(record_signature_status(db, entity_key, record_id, registry)?
        .as_deref()
        == Some(STATUS_SIGNE))
}

pub fn is_record_refused(
    db: &Database,
    entity_key: &str,
    record_id: &str,
    registry: &EntityRegistry,
) -> Result<bool, String> {
    Ok(record_signature_status(db, entity_key, record_id, registry)?
        .as_deref()
        == Some(STATUS_REFUSE))
}

pub fn is_signature_pending(
    db: &Database,
    entity_key: &str,
    record_id: &str,
    registry: &EntityRegistry,
) -> Result<bool, String> {
    Ok(record_signature_status(db, entity_key, record_id, registry)?
        .as_deref()
        == Some(STATUS_NON_SIGNE))
}

pub fn is_signable(
    db: &Database,
    entity_key: &str,
    record_id: &str,
    registry: &EntityRegistry,
) -> Result<bool, String> {
    Ok(matches!(
        record_signature_status(db, entity_key, record_id, registry)?.as_deref(),
        Some(STATUS_NON_SIGNE) | Some(STATUS_REFUSE)
    ))
}

#[derive(Debug, Clone)]
struct RoleSignatureEntry {
    role_id: String,
    user_id: String,
    signer_label: String,
}

fn record_role_signatures(
    db: &Database,
    entity_key: &str,
    record_id: &str,
) -> Result<Vec<RoleSignatureEntry>, String> {
    let mut stmt = db
        .conn
        .prepare(
            "SELECT role_id, user_id, signer_label FROM entity_record_role_signatures
             WHERE entity_key = ?1 AND record_id = ?2
             ORDER BY signed_at ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![entity_key, record_id], |row| {
            Ok(RoleSignatureEntry {
                role_id: row.get(0)?,
                user_id: row.get(1)?,
                signer_label: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.flatten().collect())
}

fn record_signed_role_ids(
    db: &Database,
    entity_key: &str,
    record_id: &str,
) -> Result<Vec<String>, String> {
    Ok(record_role_signatures(db, entity_key, record_id)?
        .into_iter()
        .map(|e| e.role_id)
        .collect())
}

pub fn role_has_signed(
    db: &Database,
    entity_key: &str,
    record_id: &str,
    role_id: &str,
) -> Result<bool, String> {
    let n: i64 = db
        .conn
        .query_row(
            "SELECT COUNT(*) FROM entity_record_role_signatures
             WHERE entity_key = ?1 AND record_id = ?2 AND role_id = ?3",
            rusqlite::params![entity_key, record_id, role_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(n > 0)
}

fn insert_role_signature(
    db: &Database,
    entity_key: &str,
    record_id: &str,
    role_id: &str,
    user_id: &str,
    signer_label: &str,
) -> Result<(), String> {
    db.conn
        .execute(
            "INSERT INTO entity_record_role_signatures
             (id, entity_key, record_id, role_id, user_id, signer_label, signed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                Uuid::new_v4().to_string(),
                entity_key,
                record_id,
                role_id,
                user_id,
                signer_label,
                Utc::now().to_rfc3339(),
            ],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn clear_role_signatures(
    db: &Database,
    entity_key: &str,
    record_id: &str,
) -> Result<(), String> {
    db.conn
        .execute(
            "DELETE FROM entity_record_role_signatures
             WHERE entity_key = ?1 AND record_id = ?2",
            rusqlite::params![entity_key, record_id],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn all_required_roles_signed(
    db: &Database,
    registry: &EntityRegistry,
    entity_key: &str,
    record_id: &str,
) -> Result<bool, String> {
    let Some(ent) = registry.find(entity_key) else {
        return Ok(true);
    };
    let signed = record_signed_role_ids(db, entity_key, record_id)?;
    Ok(ent
        .signatory_role_ids
        .iter()
        .all(|role_id| signed.iter().any(|s| s == role_id)))
}

/// Vrai si, après la signature du rôle courant, tous les signataires requis auront signé.
fn signing_would_complete_all_roles(
    db: &Database,
    registry: &EntityRegistry,
    entity_key: &str,
    record_id: &str,
    signing_role_id: &str,
) -> Result<bool, String> {
    let Some(ent) = registry.find(entity_key) else {
        return Ok(false);
    };
    if ent.signatory_role_ids.is_empty() {
        return Ok(false);
    }
    let signed = record_signed_role_ids(db, entity_key, record_id)?;
    Ok(ent.signatory_role_ids.iter().all(|role_id| {
        role_id == signing_role_id || signed.iter().any(|s| s == role_id)
    }))
}

fn combined_signers_label(entries: &[RoleSignatureEntry]) -> String {
    entries
        .iter()
        .map(|e| e.signer_label.as_str())
        .collect::<Vec<_>>()
        .join(" ; ")
}

fn role_nom_for_id(db: &Database, role_id: &str) -> String {
    db.list_roles()
        .ok()
        .and_then(|roles| {
            roles
                .into_iter()
                .find(|r| r.id == role_id)
                .map(|r| r.nom)
        })
        .unwrap_or_else(|| role_id.to_string())
}

/// Seul le créateur peut modifier/supprimer tant que l'objet n'est pas signé ; jamais après signature.
pub fn assert_record_editable_by_user(
    db: &Database,
    entity_key: &str,
    record_id: &str,
    user_id: Option<&str>,
    registry: &EntityRegistry,
) -> Result<(), String> {
    if !entity_requires_signature(registry, entity_key) {
        return Ok(());
    }
    if is_record_signed(db, entity_key, record_id, registry)? {
        return Err(
            "Impossible de modifier un objet signé. La signature verrouille l'enregistrement."
                .into(),
        );
    }
    if is_record_refused(db, entity_key, record_id, registry)? {
        return Err(
            "Impossible de modifier un objet refusé. Un signataire peut le réaccepter par signature."
                .into(),
        );
    }
    if !is_signature_pending(db, entity_key, record_id, registry)? {
        return Ok(());
    }
    let Some(uid) = user_id else {
        return Err("Seul l'auteur de l'objet peut le modifier avant signature.".into());
    };
    if let Some(creator_id) = record_string_column(db, entity_key, record_id, CREATED_BY_COLUMN)?
        .as_deref()
    {
        if creator_id != uid {
            return Err("Seul l'auteur de l'objet peut le modifier avant signature.".into());
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationSelectOptionExt {
    pub value: String,
    pub label: String,
    /// Attributs formatés « Libellé : valeur · … » pour affichage en petit.
    pub detail: String,
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

/// Autorisation de signature : uniquement via `signatory_role_ids` de l'entité (pas de privilège DDA).
pub fn can_sign_entity(registry: &EntityRegistry, entity_key: &str, role_id: &str) -> bool {
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
pub struct RoleSignatureProgress {
    pub role_id: String,
    pub role_nom: String,
    pub signed: bool,
    pub signer_label: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordSignatureDetail {
    pub entity_key: String,
    pub entity_label: String,
    pub record_id: String,
    pub signed: bool,
    pub rejected: bool,
    pub can_view: bool,
    pub can_sign: bool,
    pub can_reject: bool,
    pub refused_by: Option<String>,
    pub refusal_reason: Option<String>,
    pub fields: Vec<super::relations::RelationPanelField>,
    pub panels: Vec<RelationPanel>,
    pub signatory_contacts: Vec<SignatoryContact>,
    pub signature_roles: Vec<RoleSignatureProgress>,
    pub signature_required_count: usize,
    pub signature_done_count: usize,
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
    let can_sign = can_sign_entity(&registry, entity_key, &role_id);
    let signed = is_record_signed(db, entity_key, record_id, &registry)?;
    let rejected = is_record_refused(db, entity_key, record_id, &registry)?;
    let signable = is_signable(db, entity_key, record_id, &registry)?;
    let user_role_signed = role_has_signed(db, entity_key, record_id, &role_id)?;
    let can_act_sign = signable && can_sign && !user_role_signed;
    let can_act_reject = signable && can_sign;
    let refused_by = if rejected {
        record_string_column(db, entity_key, record_id, REFUSED_BY_COLUMN)?
    } else {
        None
    };
    let refusal_reason = if rejected {
        record_string_column(db, entity_key, record_id, REFUSAL_REASON_COLUMN)?
    } else {
        None
    };
    let signatory_contacts = signatory_contacts_for_entity(db, &registry, entity_key)?;
    let cfg = load_screen_config(data_dir, entity_key)?;
    let entity_label = cfg.screen.label.clone();
    let (fields, panels) = if can_view {
        let row = crate::dda::crud::get_row(db, &cfg, record_id)?;
        let panels = build_relation_panels(db, data_dir, &cfg, &row)?;
        let fields = panels
            .iter()
            .find(|p| p.primary)
            .map(|p| p.fields.clone())
            .unwrap_or_else(|| row_to_panel_fields(&cfg, &row));
        (fields, panels)
    } else {
        (Vec::new(), Vec::new())
    };

    let ent = registry.find(entity_key);
    let signed_entries = record_role_signatures(db, entity_key, record_id)?;
    let signature_roles: Vec<RoleSignatureProgress> = ent
        .map(|e| {
            e.signatory_role_ids
                .iter()
                .map(|role_id| {
                    let entry = signed_entries.iter().find(|s| s.role_id == *role_id);
                    RoleSignatureProgress {
                        role_id: role_id.clone(),
                        role_nom: role_nom_for_id(db, role_id),
                        signed: entry.is_some(),
                        signer_label: entry.map(|e| e.signer_label.clone()),
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let signature_done_count = signature_roles.iter().filter(|r| r.signed).count();
    let signature_required_count = signature_roles.len();

    Ok(RecordSignatureDetail {
        entity_key: entity_key.to_string(),
        entity_label,
        record_id: record_id.to_string(),
        signed,
        rejected,
        can_view,
        can_sign: can_act_sign,
        can_reject: can_act_reject,
        refused_by,
        refusal_reason,
        fields,
        panels,
        signatory_contacts,
        signature_roles,
        signature_required_count,
        signature_done_count,
    })
}

/// Chaque rôle signataire doit signer ; l'impact stock/liaison n'a lieu qu'une fois tous signés.
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
    if !is_signable(db, entity_key, record_id, &registry)? {
        return Err("Cet objet n'est pas en attente de signature.".into());
    }
    let role_id = user_role_id(db, user_id)?;
    if !can_sign_entity(&registry, entity_key, &role_id) {
        return Err("Vous n'êtes pas autorisé à signer cet objet.".into());
    }
    if !can_view_entity(privileges, entity_key) {
        return Err("Droit insuffisant pour consulter cette entité.".into());
    }
    if role_has_signed(db, entity_key, record_id, &role_id)? {
        return Err("Votre rôle a déjà signé cet objet.".into());
    }

    if signing_would_complete_all_roles(db, &registry, entity_key, record_id, &role_id)? {
        super::relation_impact::validate_impacts_before_record_validated(
            db,
            data_dir,
            entity_key,
            record_id,
        )?;
    }

    let was_refused = is_record_refused(db, entity_key, record_id, &registry)?;
    let signer_label = signer_label(db, user_id, &role_id)?;
    let creator_id = record_string_column(db, entity_key, record_id, CREATED_BY_COLUMN)?;

    insert_role_signature(
        db,
        entity_key,
        record_id,
        &role_id,
        user_id,
        &signer_label,
    )?;

    close_signature_tasks_for_role(db, &registry, entity_key, record_id, &role_id)?;

    let table = table_name(entity_key);
    let all_signed = all_required_roles_signed(db, &registry, entity_key, record_id)?;

    if table_has_column(db, &table, SIGNATURE_STATUS_COLUMN)? {
        let new_status = if all_signed {
            STATUS_SIGNE
        } else {
            STATUS_NON_SIGNE
        };
        let mut sets = vec![format!("{SIGNATURE_STATUS_COLUMN} = ?1")];
        let mut params: Vec<rusqlite::types::Value> =
            vec![rusqlite::types::Value::Text(new_status.into())];
        let mut idx = 2usize;

        if all_signed && table_has_column(db, &table, SIGNED_BY_COLUMN)? {
            let entries = record_role_signatures(db, entity_key, record_id)?;
            sets.push(format!("{SIGNED_BY_COLUMN} = ?{idx}"));
            params.push(rusqlite::types::Value::Text(combined_signers_label(&entries)));
            idx += 1;
        } else if was_refused && table_has_column(db, &table, SIGNED_BY_COLUMN)? {
            sets.push(format!("{SIGNED_BY_COLUMN} = NULL"));
        }

        if was_refused {
            for col in [REFUSED_BY_COLUMN, REFUSAL_REASON_COLUMN] {
                if table_has_column(db, &table, col)? {
                    sets.push(format!("{col} = NULL"));
                }
            }
        }

        params.push(rusqlite::types::Value::Text(record_id.to_string()));
        let sql = format!("UPDATE {table} SET {} WHERE id = ?{idx}", sets.join(", "));
        let n = db
            .conn
            .execute(&sql, rusqlite::params_from_iter(params.iter()))
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("Enregistrement introuvable.".into());
        }
    }

    let cfg = load_screen_config(data_dir, entity_key)?;
    let row = crate::dda::crud::get_row(db, &cfg, record_id)?;

    if !all_signed {
        return Ok(());
    }

    close_signature_tasks(db, &registry, entity_key, record_id)?;

    super::validation::spawn_other_signatory_roles_signed_notices(
        db,
        data_dir,
        entity_key,
        record_id,
        user_id,
        &signer_label,
        &row,
        creator_id.as_deref(),
    )?;

    if let Some(creator) = creator_id.as_deref() {
        if creator != user_id {
            super::validation::spawn_creator_validation_task(
                db,
                data_dir,
                entity_key,
                record_id,
                creator,
                &signer_label,
                &row,
            )?;
        }
    }

    super::relation_impact::apply_on_record_validated(db, data_dir, entity_key, record_id)?;

    if let Some(ent) = registry.find(entity_key) {
        let _ = super::validation::apply_signed_object_title(db, ent, record_id, &row);
    }

    Ok(())
}

/// Refuse la signature (rôles signataires) et notifie le créateur + les autres signataires.
pub fn reject_record(
    db: &Database,
    data_dir: &Path,
    entity_key: &str,
    record_id: &str,
    user_id: &str,
    privileges: &[String],
    reason: Option<&str>,
) -> Result<(), String> {
    let registry = super::registry::load(data_dir)?;
    if !entity_requires_signature(&registry, entity_key) {
        return Err("Cette entité ne requiert pas de signature.".into());
    }
    if is_record_signed(db, entity_key, record_id, &registry)? {
        return Err("Cet objet est déjà signé.".into());
    }
    if is_record_refused(db, entity_key, record_id, &registry)? {
        return Err("La signature de cet objet a déjà été refusée.".into());
    }
    if !is_signature_pending(db, entity_key, record_id, &registry)? {
        return Err("Cet objet n'est pas en attente de signature.".into());
    }
    let role_id = user_role_id(db, user_id)?;
    if !can_sign_entity(&registry, entity_key, &role_id) {
        return Err("Vous n'êtes pas autorisé à refuser la signature de cet objet.".into());
    }
    if !can_view_entity(privileges, entity_key) {
        return Err("Droit insuffisant pour consulter cette entité.".into());
    }

    let refuser_label = signer_label(db, user_id, &role_id)?;
    let creator_id = record_string_column(db, entity_key, record_id, CREATED_BY_COLUMN)?;
    let reason_trimmed = reason.map(str::trim).filter(|s| !s.is_empty());

    let table = table_name(entity_key);
    if table_has_column(db, &table, SIGNATURE_STATUS_COLUMN)? {
        let mut sets = vec![format!("{SIGNATURE_STATUS_COLUMN} = ?1")];
        let mut params: Vec<rusqlite::types::Value> =
            vec![rusqlite::types::Value::Text(STATUS_REFUSE.into())];
        let mut idx = 2usize;
        if table_has_column(db, &table, REFUSED_BY_COLUMN)? {
            sets.push(format!("{REFUSED_BY_COLUMN} = ?{idx}"));
            params.push(rusqlite::types::Value::Text(refuser_label.clone()));
            idx += 1;
        }
        if table_has_column(db, &table, REFUSAL_REASON_COLUMN)? {
            sets.push(format!("{REFUSAL_REASON_COLUMN} = ?{idx}"));
            params.push(match reason_trimmed {
                Some(r) => rusqlite::types::Value::Text(r.to_string()),
                None => rusqlite::types::Value::Null,
            });
            idx += 1;
        }
        params.push(rusqlite::types::Value::Text(record_id.to_string()));
        let sql = format!("UPDATE {table} SET {} WHERE id = ?{idx}", sets.join(", "));
        let n = db
            .conn
            .execute(&sql, rusqlite::params_from_iter(params.iter()))
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("Enregistrement introuvable.".into());
        }
    }

    clear_role_signatures(db, entity_key, record_id)?;
    close_signature_tasks(db, &registry, entity_key, record_id)?;

    let cfg = load_screen_config(data_dir, entity_key)?;
    let row = crate::dda::crud::get_row(db, &cfg, record_id)?;

    super::validation::spawn_other_signatory_roles_rejection_notices(
        db,
        data_dir,
        entity_key,
        record_id,
        user_id,
        &refuser_label,
        reason_trimmed,
        &row,
        creator_id.as_deref(),
    )?;

    if let Some(creator) = creator_id.as_deref() {
        if creator != user_id {
            super::validation::spawn_creator_rejection_task(
                db,
                data_dir,
                entity_key,
                record_id,
                creator,
                &refuser_label,
                reason_trimmed,
                &row,
            )?;
        }
    }

    if let Err(e) = super::validation::spawn_signature_tasks(db, data_dir, entity_key, &row) {
        eprintln!("Recréation tâches signature après refus {entity_key}/{record_id} : {e}");
    }

    Ok(())
}

pub fn close_signature_tasks_for_role(
    db: &Database,
    registry: &EntityRegistry,
    entity_key: &str,
    record_id: &str,
    role_id: &str,
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
           AND role_signataire = ?3
           AND statut != 'terminee'"
    );
    db.conn
        .execute(&sql, rusqlite::params![entity_key, record_id, role_id])
        .map_err(|e| format!("Clôture des tâches de signature (rôle) : {e}"))?;
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
