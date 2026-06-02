use rusqlite::params;

use super::config::{FieldDef, ScreenConfigFile};
use crate::db::{Database, DbError};

fn sqlite_type(field: &FieldDef) -> &'static str {
    match field.field_type.as_str() {
        "number" => "REAL",
        "boolean" => "INTEGER",
        _ => "TEXT",
    }
}

fn table_has_column(db: &Database, table: &str, column: &str) -> Result<bool, DbError> {
    let sql = format!("PRAGMA table_info({table})");
    let mut stmt = db.conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for name in rows.flatten() {
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn sync_table_from_config(db: &Database, cfg: &ScreenConfigFile) -> Result<(), String> {
    let table = &cfg.screen.table;
    db.conn
        .execute(
            &format!(
                "CREATE TABLE IF NOT EXISTS {table} (
                    id TEXT PRIMARY KEY NOT NULL,
                    created_at TEXT NOT NULL DEFAULT ''
                )"
            ),
            [],
        )
        .map_err(|e| e.to_string())?;

    for field in &cfg.fields {
        if field.column == "id" {
            continue;
        }
        let exists = table_has_column(db, table, &field.column).map_err(|e| e.to_string())?;
        if exists {
            continue;
        }
        let col_type = sqlite_type(field);
        let default_clause = if field.column == "created_at" {
            " DEFAULT ''"
        } else if field.required && col_type == "TEXT" {
            " NOT NULL DEFAULT ''"
        } else {
            ""
        };
        db.conn
            .execute(
                &format!(
                    "ALTER TABLE {table} ADD COLUMN {} {col_type}{default_clause}",
                    field.column
                ),
                [],
            )
            .map_err(|e| format!("ALTER {table}.{} : {e}", field.column))?;
    }

    db.conn
        .execute(
            "INSERT OR REPLACE INTO dda_screen_registry (screen_key, table_name, route, label, updated_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            params![
                cfg.screen.key,
                cfg.screen.table,
                cfg.screen.route,
                cfg.screen.label
            ],
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn ensure_dda_registry_table(db: &Database) -> Result<(), DbError> {
    db.conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS dda_screen_registry (
            screen_key TEXT PRIMARY KEY NOT NULL,
            table_name TEXT NOT NULL,
            route TEXT NOT NULL,
            label TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS dda_validation_rules (
            screen_key TEXT PRIMARY KEY NOT NULL,
            rules_json TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )?;
    Ok(())
}
