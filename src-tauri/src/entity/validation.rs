//! Tâches de signature — trigger système à chaque `create_row` (entité `requires_signature`).

use chrono::Utc;
use serde_json::{json, Map, Value};
use uuid::Uuid;

use super::record_signature;
use super::registry::EntityDef;
use super::schema::{attr_column, table_has_column, table_name};
use crate::db::Database;
use crate::dda::crud;

const TACHE_ENTITY_KEY: &str = "tache";
pub const TACHE_TYPE_VALIDATION: &str = "validation";
pub const COL_UTILISATEUR_CIBLE: &str = "utilisateur_cible";

/// Hook post-insert : tâches de signature si l'objet n'est pas auto-signé ; annule la création si échec.
pub fn after_entity_row_created(
    db: &Database,
    entity_key: &str,
    created_row: &Map<String, Value>,
) -> Result<(), String> {
    let registry = super::registry::load(&db.data_dir)?;
    if !record_signature::entity_requires_signature(&registry, entity_key) {
        return Ok(());
    }
    if !record_signature::is_signature_pending(
        db,
        entity_key,
        created_row.get("id").and_then(|v| v.as_str()).unwrap_or(""),
        &registry,
    )? {
        return Ok(());
    }
    match spawn_signature_tasks(db, &db.data_dir, entity_key, created_row) {
        Ok(_) => Ok(()),
        Err(e) => {
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
                "Création annulée : les tâches de signature n'ont pas pu être générées ({e}). \
                 Vérifiez que l'entité « tache » existe dans le registre."
            ))
        }
    }
}

/// Une tâche privée par rôle signataire si l'objet créé n'est pas encore signé.
pub fn spawn_signature_tasks(
    db: &Database,
    data_dir: &std::path::Path,
    source_entity_key: &str,
    created_row: &Map<String, Value>,
) -> Result<Vec<String>, String> {
    if source_entity_key == TACHE_ENTITY_KEY {
        return Ok(Vec::new());
    }
    if is_signature_task_row(created_row) {
        return Ok(Vec::new());
    }

    let registry = super::registry::load(data_dir)?;
    let Some(source) = registry.find(source_entity_key) else {
        return Ok(Vec::new());
    };
    if !source.requires_signature || source.signatory_role_ids.is_empty() {
        return Ok(Vec::new());
    }

    let Some(tache_ent) = registry.find(TACHE_ENTITY_KEY) else {
        return Err(
            "Entité « tache » absente du registre — impossible de créer les tâches de signature."
                .into(),
        );
    };

    let record_id = created_row
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("Identifiant de l'enregistrement créé introuvable.")?;
    let summary = record_summary(source, created_row);
    let source_label = source.label.as_deref().unwrap_or(&source.nom);
    let roles = db.list_roles().map_err(|e| e.to_string())?;
    let users = db.list_users().map_err(|e| e.to_string())?;

    let mut created_task_ids = Vec::new();
    for role_id in &source.signatory_role_ids {
        let role_nom = roles
            .iter()
            .find(|r| &r.id == role_id)
            .map(|r| r.nom.as_str())
            .unwrap_or(role_id.as_str());
        let role_users: Vec<_> = users
            .iter()
            .filter(|u| u.actif && u.role_id == *role_id)
            .collect();
        let targets: Vec<Option<&str>> = if role_users.is_empty() {
            vec![None]
        } else {
            role_users.iter().map(|u| Some(u.id.as_str())).collect()
        };
        for user_id in targets {
            let libelle = format!("Signer {source_label} — {summary}");
            let required_labels = required_attribute_labels(source);
            let champs_obligatoires = if required_labels.is_empty() {
                "—".to_string()
            } else {
                required_labels.join(", ")
            };
            let user_line = user_id
                .and_then(|uid| role_users.iter().find(|u| u.id == uid).map(|u| u.nom.as_str()))
                .map(|nom| format!("\nUtilisateur cible : {nom}"))
                .unwrap_or_default();
            let description = format!(
                "Signature requise pour l'objet « {source_label} » ({source_entity_key}).\n\
                 Enregistrement : {record_id}\n\
                 Rôle signataire : {role_nom}{user_line}\n\
                 Champs obligatoires à contrôler : {champs_obligatoires}"
            );
            let task_id = insert_system_task(
                db,
                tache_ent,
                &libelle,
                &description,
                "signature",
                Some(source_entity_key),
                None,
                Some(role_id),
                record_id,
                None,
                user_id,
            )?;
            created_task_ids.push(task_id);
        }
    }
    Ok(created_task_ids)
}

/// Tâche « validation » privée pour l'auteur de l'objet signé.
pub fn spawn_creator_validation_task(
    db: &Database,
    data_dir: &std::path::Path,
    source_entity_key: &str,
    record_id: &str,
    creator_user_id: &str,
    signer_label: &str,
    source_row: &Map<String, Value>,
) -> Result<String, String> {
    let registry = super::registry::load(data_dir)?;
    let Some(source) = registry.find(source_entity_key) else {
        return Ok(String::new());
    };
    let Some(tache_ent) = registry.find(TACHE_ENTITY_KEY) else {
        return Err(
            "Entité « tache » absente du registre — impossible de notifier le créateur.".into(),
        );
    };

    let creator_role_id = super::record_signature::user_role_id(db, creator_user_id)?;
    let tache_table = table_name(TACHE_ENTITY_KEY);
    if table_has_column(db, &tache_table, "type_tache")? {
        let pending: i64 = db
            .conn
            .query_row(
                &format!(
                    "SELECT COUNT(*) FROM {tache_table}
                     WHERE type_tache = ?1
                       AND entite_a_valider = ?2
                       AND enregistrement_id = ?3
                       AND utilisateur_cible = ?4
                       AND statut != 'terminee'"
                ),
                rusqlite::params![
                    TACHE_TYPE_VALIDATION,
                    source_entity_key,
                    record_id,
                    creator_user_id
                ],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if pending > 0 {
            return Ok(String::new());
        }
    }

    let summary = record_summary(source, source_row);
    let source_label = source.label.as_deref().unwrap_or(&source.nom);
    let libelle = format!("Valider {source_label} — {summary}");
    let description = format!(
        "L'objet « {source_label} » ({source_entity_key}) que vous avez créé a été signé par {signer_label}.\n\
         Enregistrement : {record_id}\n\
         Veuillez valider la prise en compte de cette signature."
    );

    insert_system_task(
        db,
        tache_ent,
        &libelle,
        &description,
        TACHE_TYPE_VALIDATION,
        None,
        Some(source_entity_key),
        None,
        record_id,
        Some(&creator_role_id),
        Some(creator_user_id),
    )
}

/// Info post-signature pour les autres utilisateurs des rôles signataires (même rôle ou rôles restants).
pub fn spawn_other_signatory_roles_signed_notices(
    db: &Database,
    data_dir: &std::path::Path,
    source_entity_key: &str,
    record_id: &str,
    signer_user_id: &str,
    signer_label: &str,
    source_row: &Map<String, Value>,
    creator_user_id: Option<&str>,
) -> Result<(), String> {
    let registry = super::registry::load(data_dir)?;
    let Some(source) = registry.find(source_entity_key) else {
        return Ok(());
    };
    let Some(tache_ent) = registry.find(TACHE_ENTITY_KEY) else {
        return Ok(());
    };

    let summary = record_summary(source, source_row);
    let source_label = source.label.as_deref().unwrap_or(&source.nom);
    let tache_table = table_name(TACHE_ENTITY_KEY);
    let contacts = super::record_signature::signatory_contacts_for_entity(
        db,
        &registry,
        source_entity_key,
    )?;

    for contact in contacts {
        if contact.user_id == signer_user_id {
            continue;
        }
        if creator_user_id.is_some_and(|c| c == contact.user_id) {
            continue;
        }
        if table_has_column(db, &tache_table, "type_tache")? {
            let pending: i64 = db
                .conn
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM {tache_table}
                         WHERE type_tache = ?1
                           AND entite_a_valider = ?2
                           AND enregistrement_id = ?3
                           AND utilisateur_cible = ?4
                           AND statut != 'terminee'"
                    ),
                    rusqlite::params![
                        TACHE_TYPE_VALIDATION,
                        source_entity_key,
                        record_id,
                        contact.user_id
                    ],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            if pending > 0 {
                continue;
            }
        }

        let libelle = format!("{source_label} signé — {summary}");
        let description = format!(
            "L'objet « {source_label} » ({source_entity_key}) a été signé par {signer_label}.\n\
             Enregistrement : {record_id}\n\
             Rôle signataire concerné : {}\n\
             Votre signature n'était plus requise — consultez la fiche signée.",
            contact.role_nom
        );
        let _ = insert_system_task(
            db,
            tache_ent,
            &libelle,
            &description,
            TACHE_TYPE_VALIDATION,
            None,
            Some(source_entity_key),
            None,
            record_id,
            Some(contact.role_id.as_str()),
            Some(contact.user_id.as_str()),
        )?;
    }
    Ok(())
}

/// Tâche « validation » pour le créateur après refus de signature.
pub fn spawn_creator_rejection_task(
    db: &Database,
    data_dir: &std::path::Path,
    source_entity_key: &str,
    record_id: &str,
    creator_user_id: &str,
    refuser_label: &str,
    reason: Option<&str>,
    source_row: &Map<String, Value>,
) -> Result<String, String> {
    let registry = super::registry::load(data_dir)?;
    let Some(source) = registry.find(source_entity_key) else {
        return Ok(String::new());
    };
    let Some(tache_ent) = registry.find(TACHE_ENTITY_KEY) else {
        return Err(
            "Entité « tache » absente du registre — impossible de notifier le créateur.".into(),
        );
    };

    let creator_role_id = super::record_signature::user_role_id(db, creator_user_id)?;
    let tache_table = table_name(TACHE_ENTITY_KEY);
    if table_has_column(db, &tache_table, "type_tache")? {
        let pending: i64 = db
            .conn
            .query_row(
                &format!(
                    "SELECT COUNT(*) FROM {tache_table}
                     WHERE type_tache = ?1
                       AND entite_a_valider = ?2
                       AND enregistrement_id = ?3
                       AND utilisateur_cible = ?4
                       AND statut != 'terminee'"
                ),
                rusqlite::params![
                    TACHE_TYPE_VALIDATION,
                    source_entity_key,
                    record_id,
                    creator_user_id
                ],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if pending > 0 {
            return Ok(String::new());
        }
    }

    let summary = record_summary(source, source_row);
    let source_label = source.label.as_deref().unwrap_or(&source.nom);
    let libelle = format!("Refus signature — {source_label} — {summary}");
    let motif_line = reason
        .map(|r| format!("Motif : {r}\n"))
        .unwrap_or_default();
    let description = format!(
        "L'objet « {source_label} » ({source_entity_key}) que vous avez créé a été refusé par {refuser_label}.\n\
         Enregistrement : {record_id}\n\
         {motif_line}\
         Un signataire peut réaccepter l'objet par signature."
    );

    insert_system_task(
        db,
        tache_ent,
        &libelle,
        &description,
        TACHE_TYPE_VALIDATION,
        None,
        Some(source_entity_key),
        None,
        record_id,
        Some(&creator_role_id),
        Some(creator_user_id),
    )
}

/// Info post-refus pour les autres utilisateurs des rôles signataires.
pub fn spawn_other_signatory_roles_rejection_notices(
    db: &Database,
    data_dir: &std::path::Path,
    source_entity_key: &str,
    record_id: &str,
    refuser_user_id: &str,
    refuser_label: &str,
    reason: Option<&str>,
    source_row: &Map<String, Value>,
    creator_user_id: Option<&str>,
) -> Result<(), String> {
    let registry = super::registry::load(data_dir)?;
    let Some(source) = registry.find(source_entity_key) else {
        return Ok(());
    };
    let Some(tache_ent) = registry.find(TACHE_ENTITY_KEY) else {
        return Ok(());
    };

    let summary = record_summary(source, source_row);
    let source_label = source.label.as_deref().unwrap_or(&source.nom);
    let tache_table = table_name(TACHE_ENTITY_KEY);
    let motif_line = reason
        .map(|r| format!("Motif : {r}\n"))
        .unwrap_or_default();
    let contacts = super::record_signature::signatory_contacts_for_entity(
        db,
        &registry,
        source_entity_key,
    )?;

    for contact in contacts {
        if contact.user_id == refuser_user_id {
            continue;
        }
        if creator_user_id.is_some_and(|c| c == contact.user_id) {
            continue;
        }
        if table_has_column(db, &tache_table, "type_tache")? {
            let pending: i64 = db
                .conn
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM {tache_table}
                         WHERE type_tache = ?1
                           AND entite_a_valider = ?2
                           AND enregistrement_id = ?3
                           AND utilisateur_cible = ?4
                           AND statut != 'terminee'"
                    ),
                    rusqlite::params![
                        TACHE_TYPE_VALIDATION,
                        source_entity_key,
                        record_id,
                        contact.user_id
                    ],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            if pending > 0 {
                continue;
            }
        }

        let libelle = format!("{source_label} refusé — {summary}");
        let description = format!(
            "L'objet « {source_label} » ({source_entity_key}) a été refusé par {refuser_label}.\n\
             Enregistrement : {record_id}\n\
             Rôle signataire concerné : {}\n\
             {motif_line}\
             La signature n'est plus requise pour cet objet.",
            contact.role_nom
        );
        let _ = insert_system_task(
            db,
            tache_ent,
            &libelle,
            &description,
            TACHE_TYPE_VALIDATION,
            None,
            Some(source_entity_key),
            None,
            record_id,
            Some(contact.role_id.as_str()),
            Some(contact.user_id.as_str()),
        )?;
    }
    Ok(())
}

/// Messages affichés à la connexion : signatures en attente + validations / infos post-signature.
pub fn login_workflow_notices(
    db: &Database,
    user_id: &str,
    role_id: &str,
) -> Result<Vec<String>, String> {
    let registry = super::registry::load(&db.data_dir)?;
    if registry.find(TACHE_ENTITY_KEY).is_none() {
        return Ok(Vec::new());
    }
    let tache_table = table_name(TACHE_ENTITY_KEY);
    if !table_has_column(db, &tache_table, "type_tache")? {
        return Ok(Vec::new());
    }

    let mut notices: Vec<String> = Vec::new();

    if table_has_column(db, &tache_table, "role_signataire")? {
        let mut sig_notices: Vec<String> = Vec::new();
        if table_has_column(db, &tache_table, COL_UTILISATEUR_CIBLE)? {
            let sql = format!(
                "SELECT libelle FROM {tache_table}
                 WHERE type_tache = 'signature'
                   AND utilisateur_cible = ?1
                   AND statut != 'terminee'
                 ORDER BY datetime(created_at) DESC
                 LIMIT 5"
            );
            let mut stmt = db.conn.prepare(&sql).map_err(|e| e.to_string())?;
            sig_notices.extend(
                stmt.query_map(rusqlite::params![user_id], |row| row.get(0))
                    .map_err(|e| e.to_string())?
                    .flatten(),
            );
        }
        let sql = format!(
            "SELECT libelle FROM {tache_table}
             WHERE type_tache = 'signature'
               AND role_signataire = ?1
               AND COALESCE({COL_UTILISATEUR_CIBLE}, '') = ''
               AND statut != 'terminee'
             ORDER BY datetime(created_at) DESC
             LIMIT 5"
        );
        let mut stmt = db.conn.prepare(&sql).map_err(|e| e.to_string())?;
        sig_notices.extend(
            stmt.query_map(rusqlite::params![role_id], |row| row.get(0))
                .map_err(|e| e.to_string())?
                .flatten(),
        );
        sig_notices.sort();
        sig_notices.dedup();
        notices.extend(
            sig_notices
                .into_iter()
                .take(5)
                .map(|libelle| format!("Signature en attente — {libelle}")),
        );
    }

    if table_has_column(db, &tache_table, COL_UTILISATEUR_CIBLE)? {
        let sql = format!(
            "SELECT libelle FROM {tache_table}
             WHERE type_tache = ?1
               AND utilisateur_cible = ?2
               AND statut != 'terminee'
             ORDER BY datetime(created_at) DESC
             LIMIT 10"
        );
        let mut stmt = db.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let user_rows: Vec<String> = stmt
            .query_map(rusqlite::params![TACHE_TYPE_VALIDATION, user_id], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .flatten()
            .collect();
        for libelle in user_rows {
            let prefix = if libelle.starts_with("Refus signature") {
                "Signature refusée"
            } else if libelle.starts_with("Valider ") {
                "Objet signé"
            } else if libelle.contains(" refusé —") {
                "Info refus signature"
            } else {
                "Info signature"
            };
            notices.push(format!("{prefix} — {libelle}"));
        }
    }

    if table_has_column(db, &tache_table, "role_validateur")? {
        let sql = format!(
            "SELECT libelle FROM {tache_table}
             WHERE type_tache = ?1
               AND role_validateur = ?2
               AND COALESCE({COL_UTILISATEUR_CIBLE}, '') = ''
               AND statut != 'terminee'
             ORDER BY datetime(created_at) DESC
             LIMIT 5"
        );
        let mut stmt = db.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let role_rows: Vec<String> = stmt
            .query_map(rusqlite::params![TACHE_TYPE_VALIDATION, role_id], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .flatten()
            .collect();
        notices.extend(
            role_rows
                .into_iter()
                .map(|libelle| format!("Info signature — {libelle}")),
        );
    }

    Ok(notices)
}

/// Attributs tâche pour validation post-signature (enum, liens entité, utilisateur cible).
pub fn ensure_tache_workflow_attrs(registry: &mut super::registry::EntityRegistry) {
    let Some(ent) = registry.entities.iter_mut().find(|e| e.nom == TACHE_ENTITY_KEY) else {
        return;
    };

    if let Some(attr) = ent
        .attributs
        .iter_mut()
        .find(|a| a.nom == "type_tache")
    {
        if !attr.attr_type.contains("validation") {
            attr.attr_type = "enum[signature,validation,generale,destockage]".into();
        }
    }

    let needed = [
        (
            "entite_a_valider",
            "string",
            Some("Entité à valider"),
        ),
        (
            "role_validateur",
            "string",
            Some("Rôle validateur"),
        ),
        (
            COL_UTILISATEUR_CIBLE,
            "string",
            Some("Utilisateur cible"),
        ),
    ];
    for (nom, ty, label) in needed {
        if ent.attributs.iter().any(|a| a.nom == nom) {
            continue;
        }
        ent.attributs.push(super::registry::EntityAttribute {
            nom: nom.into(),
            attr_type: ty.into(),
            label: label.map(String::from),
            required: false,
            r#ref: None,
            relation_multiple: false,
            relation_exclusive_parent: true,
            default: None,
            enum_options: None,
        });
    }
}

pub fn ensure_tache_workflow_columns(db: &Database) -> Result<(), String> {
    let table = table_name(TACHE_ENTITY_KEY);
    for col in ["entite_a_valider", "role_validateur", COL_UTILISATEUR_CIBLE] {
        if !table_has_column(db, &table, col)? {
            db.conn
                .execute(&format!("ALTER TABLE {table} ADD COLUMN {col} TEXT"), [])
                .map_err(|e| format!("ALTER {table}.{col} : {e}"))?;
        }
    }
    Ok(())
}

pub fn reconcile_signature_tasks(db: &Database, data_dir: &std::path::Path) -> Result<usize, String> {
    let registry = super::registry::load(data_dir)?;
    if registry.find(TACHE_ENTITY_KEY).is_none() {
        return Ok(0);
    }
    let tache_table = table_name(TACHE_ENTITY_KEY);
    if !table_has_column(db, &tache_table, "type_tache")? {
        return Ok(0);
    }

    let mut created = 0usize;
    for ent in &registry.entities {
        if ent.nom == TACHE_ENTITY_KEY
            || !record_signature::entity_requires_signature(&registry, &ent.nom)
        {
            continue;
        }
        let source_table = table_name(&ent.nom);
        if !table_has_column(db, &source_table, record_signature::SIGNATURE_STATUS_COLUMN)? {
            continue;
        }
        let sql = format!(
            "SELECT id FROM {source_table} WHERE {} = ?1",
            record_signature::SIGNATURE_STATUS_COLUMN
        );
        let mut stmt = db.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let ids: Vec<String> = stmt
            .query_map(
                rusqlite::params![record_signature::STATUS_NON_SIGNE],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?
            .flatten()
            .collect();

        for record_id in ids {
            let pending: i64 = db
                .conn
                .query_row(
                    &format!(
                        "SELECT COUNT(*) FROM {tache_table}
                         WHERE type_tache = 'signature'
                           AND entite_a_signer = ?1
                           AND enregistrement_id = ?2
                           AND statut != 'terminee'"
                    ),
                    rusqlite::params![ent.nom, record_id],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            if pending > 0 {
                continue;
            }
            let cfg = super::load_screen_config(data_dir, &ent.nom)?;
            let row = crud::get_row(db, &cfg, &record_id)?;
            let n = spawn_signature_tasks(db, data_dir, &ent.nom, &row)?.len();
            created += n;
        }
    }
    Ok(created)
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

fn is_signature_task_row(row: &Map<String, Value>) -> bool {
    row.get("type_tache")
        .and_then(|v| v.as_str())
        .map(|s| s == "signature")
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

fn insert_system_task(
    db: &Database,
    tache_ent: &EntityDef,
    libelle: &str,
    description: &str,
    type_tache: &str,
    entite_a_signer: Option<&str>,
    entite_a_valider: Option<&str>,
    role_signataire: Option<&str>,
    record_id: &str,
    role_validateur: Option<&str>,
    utilisateur_cible: Option<&str>,
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
    data.insert("type_tache".into(), json!(type_tache));
    data.insert(
        super::tache_visibility::COL_VISIBILITE.into(),
        json!(super::tache_visibility::VIS_PRIVEE),
    );
    data.insert("enregistrement_id".into(), json!(record_id));
    if let Some(key) = entite_a_signer {
        data.insert("entite_a_signer".into(), json!(key));
    }
    if let Some(key) = entite_a_valider {
        data.insert("entite_a_valider".into(), json!(key));
    }
    if let Some(role) = role_signataire {
        data.insert("role_signataire".into(), json!(role));
    }
    if let Some(role) = role_validateur {
        data.insert("role_validateur".into(), json!(role));
    }
    if let Some(uid) = utilisateur_cible {
        data.insert(COL_UTILISATEUR_CIBLE.into(), json!(uid));
    }

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
        .map_err(|e| format!("Création tâche de signature : {e}"))?;

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
            requires_signature: false,
            signatory_role_ids: vec![],
            is_session: false,
            attributs: vec![],
        };
        let mut row = Map::new();
        row.insert("libelle".into(), json!("Mon titre"));
        row.insert("id".into(), json!("abc"));
        assert_eq!(record_summary(&ent, &row), "Mon titre");
    }
}
