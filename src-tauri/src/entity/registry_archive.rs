//! Archives du registre entités (5 dernières versions avant synchronisation).

use chrono::Utc;
use rusqlite::params;
use serde::Serialize;
use uuid::Uuid;

use super::registry::EntityRegistry;
use crate::db::{Database, DbError};

pub const MAX_REGISTRY_ARCHIVES: usize = 5;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistryArchiveSummary {
    pub id: String,
    pub archived_at: String,
}

/// Enregistre la version précédente du registre (JSON complet) avant écrasement.
pub fn push_previous(db: &Database, previous: &EntityRegistry) -> Result<(), DbError> {
    let mut to_archive = previous.clone();
    to_archive.logo = None;
    let json = serde_json::to_string_pretty(&to_archive)
        .map_err(|e| DbError::Message(format!("Sérialisation registre archive : {e}")))?;
    let id = Uuid::new_v4().to_string();
    let archived_at = Utc::now().to_rfc3339();
    db.conn.execute(
        "INSERT INTO entity_registry_archive (id, archived_at, registry_json) VALUES (?1, ?2, ?3)",
        params![id, archived_at, json],
    )?;
    trim_to_max(db, MAX_REGISTRY_ARCHIVES)?;
    Ok(())
}

fn trim_to_max(db: &Database, max: usize) -> Result<(), DbError> {
    loop {
        let count: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM entity_registry_archive", [], |r| r.get(0))?;
        if count <= max as i64 {
            break;
        }
        db.conn.execute(
            "DELETE FROM entity_registry_archive WHERE id = (
                SELECT id FROM entity_registry_archive ORDER BY archived_at ASC LIMIT 1
            )",
            [],
        )?;
    }
    Ok(())
}

pub fn list_summaries(db: &Database, limit: usize) -> Result<Vec<RegistryArchiveSummary>, DbError> {
    let lim = limit.min(MAX_REGISTRY_ARCHIVES) as i64;
    let mut stmt = db.conn.prepare(
        "SELECT id, archived_at FROM entity_registry_archive
         ORDER BY archived_at DESC LIMIT ?1",
    )?;
    let rows = stmt
        .query_map(params![lim], |row| {
            Ok(RegistryArchiveSummary {
                id: row.get(0)?,
                archived_at: row.get(1)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn get_json(db: &Database, id: &str) -> Result<String, DbError> {
    let id = id.trim();
    if id.is_empty() {
        return Err(DbError::Message("Identifiant d'archive manquant.".into()));
    }
    db.conn
        .query_row(
            "SELECT registry_json FROM entity_registry_archive WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                DbError::Message("Archive introuvable.".into())
            }
            other => DbError::Sqlite(other),
        })
}
