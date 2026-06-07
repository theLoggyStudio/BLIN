use rusqlite::params;

use super::attr_types::is_reserved_attribute;
use super::compteur::{self, is_compteur_attr};
use super::registry::{EntityAttribute, EntityDef};
use crate::db::Database;

pub fn table_name(entity_nom: &str) -> String {
    let safe: String = entity_nom
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("ent_{safe}")
}

fn sqlite_type(attr_type: &str) -> &'static str {
    match attr_type {
        "number" | "integer" | "float" | "stock" => "REAL",
        "boolean" | "bool" => "INTEGER",
        "photo" | "image" => "TEXT",
        _ => "TEXT",
    }
}

pub fn table_has_column(db: &Database, table: &str, column: &str) -> Result<bool, String> {
    let sql = format!("PRAGMA table_info({table})");
    let mut stmt = db.conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| e.to_string())?;
    for name in rows.flatten() {
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn list_columns(db: &Database, table: &str) -> Result<Vec<String>, String> {
    let sql = format!("PRAGMA table_info({table})");
    let mut stmt = db.conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| e.to_string())?;
    Ok(rows.flatten().collect())
}

pub fn sync_entity_table(
    db: &Database,
    ent: &EntityDef,
    previous: Option<&EntityDef>,
    registry: &super::registry::EntityRegistry,
) -> Result<(), String> {
    let table = table_name(&ent.nom);
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

    if !table_has_column(db, &table, "created_at")? {
        db.conn
            .execute(
                &format!("ALTER TABLE {table} ADD COLUMN created_at TEXT NOT NULL DEFAULT ''"),
                [],
            )
            .map_err(|e| format!("ALTER {table}.created_at : {e}"))?;
    }

    let mut desired: Vec<String> = vec!["id".into(), "created_at".into()];
    for attr in ent.attributs.iter().filter(|a| !is_reserved_attribute(a)) {
        if is_compteur_attr(attr) {
            for col in compteur::all_sql_columns(attr) {
                if !desired.contains(&col) {
                    desired.push(col);
                }
            }
        } else if attr.attr_type == "entity" {
            for (col, _) in super::embed::sql_columns_for_entity_attr(attr, &registry) {
                if !desired.contains(&col) {
                    desired.push(col);
                }
            }
        } else {
            let col = attr_column(attr);
            if !desired.contains(&col) {
                desired.push(col);
            }
        }
    }

    for attr in ent.attributs.iter().filter(|a| !is_reserved_attribute(a)) {
        if is_compteur_attr(attr) {
            let cols = [
                (compteur::column_libelle(attr), "TEXT"),
                (compteur::column_jjmmaaaa(attr), "TEXT"),
                (compteur::column_numero(attr), "INTEGER"),
            ];
            for (col, col_type) in cols {
                if table_has_column(db, &table, &col)? {
                    continue;
                }
                db.conn
                    .execute(
                        &format!("ALTER TABLE {table} ADD COLUMN {col} {col_type}"),
                        [],
                    )
                    .map_err(|e| format!("ALTER {table}.{col} : {e}"))?;
            }
            continue;
        }
        if attr.attr_type == "entity" {
            for (col, col_type) in super::embed::sql_columns_for_entity_attr(attr, &registry) {
                if table_has_column(db, &table, &col)? {
                    continue;
                }
                let not_null = if attr.required && col_type == "TEXT" {
                    " NOT NULL DEFAULT ''"
                } else {
                    ""
                };
                db.conn
                    .execute(
                        &format!("ALTER TABLE {table} ADD COLUMN {col} {col_type}{not_null}"),
                        [],
                    )
                    .map_err(|e| format!("ALTER {table}.{col} : {e}"))?;
            }
            continue;
        }
        let col = attr_column(attr);
        if table_has_column(db, &table, &col)? {
            continue;
        }
        let col_type = sqlite_type(&attr.attr_type);
        let not_null = if attr.required && col_type == "TEXT" {
            " NOT NULL DEFAULT ''"
        } else {
            ""
        };
        db.conn
            .execute(
                &format!("ALTER TABLE {table} ADD COLUMN {col} {col_type}{not_null}"),
                [],
            )
            .map_err(|e| format!("ALTER {table}.{col} : {e}"))?;
    }

    if super::parent_lignes::entity_has_embed_children(ent, registry) {
        let lignes_col = super::parent_lignes::LIGNES_COLUMN;
        if !table_has_column(db, &table, lignes_col)? {
            db.conn
                .execute(
                    &format!("ALTER TABLE {table} ADD COLUMN {lignes_col} TEXT"),
                    [],
                )
                .map_err(|e| format!("ALTER {table}.{lignes_col} : {e}"))?;
        }
        if !desired.contains(&lignes_col.to_string()) {
            desired.push(lignes_col.to_string());
        }
    }

    if let Some(prev) = previous {
        let prev_cols: Vec<String> = prev
            .attributs
            .iter()
            .flat_map(|a| columns_for_attr_flat(a, &registry))
            .collect();
        let new_cols: Vec<String> = ent
            .attributs
            .iter()
            .flat_map(|a| columns_for_attr_flat(a, &registry))
            .collect();
        for col in prev_cols {
            if !new_cols.contains(&col) {
                let sql = format!("ALTER TABLE {table} DROP COLUMN {col}");
                if let Err(e) = db.conn.execute(&sql, []) {
                    eprintln!("DROP COLUMN {table}.{col} : {e}");
                }
            }
        }
    } else {
        let existing = list_columns(db, &table)?;
        for col in existing {
            if col == "id" || col == "created_at" {
                continue;
            }
            if !desired.contains(&col) {
                let sql = format!("ALTER TABLE {table} DROP COLUMN {col}");
                if let Err(e) = db.conn.execute(&sql, []) {
                    eprintln!("DROP COLUMN {table}.{col} : {e}");
                }
            }
        }
    }

    super::record_signature::ensure_signature_status_column(db, ent)?;
    super::record_signature::prune_signature_status_column(db, ent)?;
    super::child_table::sync_fille_tables_for_registry(db, registry)?;
    drop_legacy_embed_columns(db, &table, ent, registry)?;
    drop_parasite_columns(db, &table)?;

    db.conn
        .execute(
            "INSERT OR REPLACE INTO dda_screen_registry (screen_key, table_name, route, label, updated_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            params![
                ent.nom,
                table,
                format!("/entite/{}", ent.nom),
                ent.label.as_deref().unwrap_or(&ent.nom)
            ],
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Colonnes orphelines (modèle FK / UI) à retirer des tables entités.
const PARASITE_COLUMNS: &[&str] = &["_detail"];

fn drop_parasite_columns(db: &Database, table: &str) -> Result<(), String> {
    for col in PARASITE_COLUMNS {
        if !table_has_column(db, table, col)? {
            continue;
        }
        let sql = format!("ALTER TABLE {table} DROP COLUMN {col}");
        if let Err(e) = db.conn.execute(&sql, []) {
            eprintln!("DROP COLUMN {table}.{col} : {e}");
        }
    }
    Ok(())
}

/// Ancien modèle : colonne nue `client` / `article` (FK) en plus des colonnes embarquées.
fn drop_legacy_embed_columns(
    db: &Database,
    table: &str,
    ent: &EntityDef,
    registry: &super::registry::EntityRegistry,
) -> Result<(), String> {
    for attr in ent.attributs.iter().filter(|a| !is_reserved_attribute(a)) {
        if attr.attr_type != "entity" || attr.relation_multiple {
            continue;
        }
        if super::embed::resolve_child(registry, attr).is_none() {
            continue;
        }
        let bare = attr_column(attr);
        if !table_has_column(db, table, &bare)? {
            continue;
        }
        let sql = format!("ALTER TABLE {table} DROP COLUMN {bare}");
        if let Err(e) = db.conn.execute(&sql, []) {
            eprintln!("DROP COLUMN {table}.{bare} : {e}");
        }
    }
    Ok(())
}

fn columns_for_attr_flat(
    attr: &EntityAttribute,
    registry: &super::registry::EntityRegistry,
) -> Vec<String> {
    if is_compteur_attr(attr) {
        compteur::all_sql_columns(attr).into_iter().collect()
        } else if attr.attr_type == "entity" {
            if attr.relation_multiple {
                vec![]
            } else {
                super::embed::sql_columns_for_entity_attr(attr, registry)
                    .into_iter()
                    .map(|(c, _)| c)
                    .collect()
            }
    } else {
        vec![attr_column(attr)]
    }
}

pub fn attr_column(attr: &EntityAttribute) -> String {
    let safe: String = attr
        .nom
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    safe
}
