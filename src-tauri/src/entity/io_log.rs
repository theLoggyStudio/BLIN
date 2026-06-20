//! Journal des imports / exports CSV : par utilisateur (importateur) et par écran (entité).

use rusqlite::params;
use serde::Serialize;
use uuid::Uuid;

use crate::db::Database;

/// Enregistre un évènement d'import ou d'export (best-effort : ne bloque jamais l'opération).
pub fn record(
    db: &Database,
    kind: &str,
    entity_key: &str,
    entity_label: &str,
    user_name: &str,
    object_count: i64,
) {
    let _ = db.conn.execute(
        "INSERT INTO import_export_log
            (id, kind, entity_key, entity_label, user_name, object_count, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))",
        params![
            Uuid::new_v4().to_string(),
            kind,
            entity_key,
            entity_label,
            user_name,
            object_count
        ],
    );
}

/// Ligne du tableau récapitulatif (un importateur).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IoLogSummary {
    pub user_name: String,
    pub import_count: i64,
    pub export_count: i64,
    pub import_objects: i64,
    pub export_objects: i64,
}

pub fn summary(db: &Database) -> Result<Vec<IoLogSummary>, String> {
    let mut stmt = db
        .conn
        .prepare(
            "SELECT user_name,
                    SUM(CASE WHEN kind = 'import' THEN 1 ELSE 0 END) AS import_count,
                    SUM(CASE WHEN kind = 'export' THEN 1 ELSE 0 END) AS export_count,
                    SUM(CASE WHEN kind = 'import' THEN object_count ELSE 0 END) AS import_objects,
                    SUM(CASE WHEN kind = 'export' THEN object_count ELSE 0 END) AS export_objects
             FROM import_export_log
             GROUP BY user_name
             ORDER BY import_count DESC, user_name ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(IoLogSummary {
                user_name: r.get(0)?,
                import_count: r.get(1)?,
                export_count: r.get(2)?,
                import_objects: r.get(3)?,
                export_objects: r.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Détail d'un importateur : un évènement par ligne (écran, type, nb d'objets, date).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IoLogEntry {
    pub kind: String,
    pub entity_key: String,
    pub entity_label: String,
    pub user_name: String,
    pub object_count: i64,
    pub created_at: String,
}

pub fn detail(db: &Database, user_name: &str) -> Result<Vec<IoLogEntry>, String> {
    let mut stmt = db
        .conn
        .prepare(
            "SELECT kind, entity_key, entity_label, user_name, object_count, created_at
             FROM import_export_log
             WHERE user_name = ?1
             ORDER BY created_at DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![user_name], |r| {
            Ok(IoLogEntry {
                kind: r.get(0)?,
                entity_key: r.get(1)?,
                entity_label: r.get(2)?,
                user_name: r.get(3)?,
                object_count: r.get(4)?,
                created_at: r.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}
