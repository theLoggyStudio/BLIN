//! Tables `{entité_fille}_fille` pour liaisons one-to-many (seule jointure autorisée).

use chrono::Utc;
use rusqlite::params;
use serde_json::{Map, Value};
use uuid::Uuid;

use super::attr_types::is_reserved_attribute;
use super::compteur::{self, is_compteur_attr};
use super::embed;
use super::registry::{EntityAttribute, EntityDef, EntityRegistry};
use super::schema::{attr_column, table_has_column, table_name};
use crate::db::Database;

pub fn fille_table_name(child_entity_key: &str) -> String {
    format!("{}_fille", table_name(child_entity_key))
}

fn sqlite_type(attr_type: &str) -> &'static str {
    match attr_type {
        "number" | "integer" | "float" | "stock" => "REAL",
        "boolean" | "bool" => "INTEGER",
        _ => "TEXT",
    }
}

fn copyable_child_columns(child: &EntityDef) -> Vec<(String, &'static str)> {
    let mut cols = Vec::new();
    for attr in embed::copyable_child_attributes(child) {
        if is_compteur_attr(attr) {
            for col in compteur::all_sql_columns(attr) {
                let col_type = if col.ends_with("_numero") {
                    "INTEGER"
                } else {
                    "TEXT"
                };
                cols.push((col, col_type));
            }
        } else {
            cols.push((attr_column(attr), sqlite_type(&attr.attr_type)));
        }
    }
    cols
}

pub fn sync_fille_table(db: &Database, child: &EntityDef) -> Result<(), String> {
    let table = fille_table_name(&child.nom);
    db.conn
        .execute(
            &format!(
                "CREATE TABLE IF NOT EXISTS {table} (
                    id TEXT PRIMARY KEY NOT NULL,
                    parent_id TEXT NOT NULL,
                    parent_entity TEXT NOT NULL,
                    created_at TEXT NOT NULL DEFAULT ''
                )"
            ),
            [],
        )
        .map_err(|e| e.to_string())?;

    for (col, col_type) in copyable_child_columns(child) {
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

    let idx = format!("idx_{table}_parent");
    let _ = db.conn.execute(
        &format!(
            "CREATE INDEX IF NOT EXISTS {idx} ON {table}(parent_entity, parent_id)"
        ),
        [],
    );

    Ok(())
}

pub fn sync_fille_tables_for_registry(db: &Database, registry: &EntityRegistry) -> Result<(), String> {
    let mut synced = std::collections::HashSet::new();
    for ent in &registry.entities {
        for attr in ent.attributs.iter().filter(|a| !is_reserved_attribute(a)) {
            if attr.attr_type != "entity" || !attr.relation_multiple {
                continue;
            }
            let Some(child) = embed::resolve_child(registry, attr) else {
                continue;
            };
            if synced.insert(child.nom.clone()) {
                sync_fille_table(db, child)?;
            }
        }
    }
    Ok(())
}

fn json_to_sql(v: &Value, attr_type: &str) -> rusqlite::types::Value {
    match v {
        Value::Null => rusqlite::types::Value::Null,
        Value::Bool(b) => rusqlite::types::Value::Integer(if *b { 1 } else { 0 }),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                rusqlite::types::Value::Integer(i)
            } else {
                rusqlite::types::Value::Real(n.as_f64().unwrap_or(0.0))
            }
        }
        Value::String(s) => rusqlite::types::Value::Text(s.clone()),
        other => rusqlite::types::Value::Text(other.to_string()),
    }
}

pub fn parse_embed_list_items(raw: Option<&Value>) -> Vec<Map<String, Value>> {
    match raw {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|v| v.as_object().cloned())
            .collect(),
        Some(Value::String(s)) if !s.trim().is_empty() => {
            serde_json::from_str::<Value>(s)
                .ok()
                .and_then(|v| {
                    if let Value::Array(items) = v {
                        Some(
                            items
                                .iter()
                                .filter_map(|x| x.as_object().cloned())
                                .collect(),
                        )
                    } else {
                        None
                    }
                })
                .unwrap_or_default()
        }
        _ => Vec::new(),
    }
}

pub fn save_embed_list(
    db: &Database,
    parent_entity: &str,
    parent_id: &str,
    child: &EntityDef,
    items: &[Map<String, Value>],
) -> Result<(), String> {
    let table = fille_table_name(&child.nom);
    db.conn
        .execute(
            &format!("DELETE FROM {table} WHERE parent_entity = ?1 AND parent_id = ?2"),
            params![parent_entity, parent_id],
        )
        .map_err(|e| e.to_string())?;

    let child_cols = copyable_child_columns(child);
    for item in items {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let mut columns = vec![
            "id".to_string(),
            "parent_id".to_string(),
            "parent_entity".to_string(),
            "created_at".to_string(),
        ];
        let mut placeholders: Vec<String> = (1..=4).map(|i| format!("?{i}")).collect();
        let mut values: Vec<rusqlite::types::Value> = vec![
            rusqlite::types::Value::Text(id),
            rusqlite::types::Value::Text(parent_id.to_string()),
            rusqlite::types::Value::Text(parent_entity.to_string()),
            rusqlite::types::Value::Text(now),
        ];
        let mut idx = 5usize;

        for (col, _) in &child_cols {
            let attr = child.attributs.iter().find(|a| {
                if is_compteur_attr(a) {
                    col.starts_with(&attr_column(a))
                } else {
                    attr_column(a) == *col
                }
            });
            let val = if let Some(a) = attr {
                if is_compteur_attr(a) {
                    let base = attr_column(a);
                    if col.ends_with("_libelle") {
                        item.get(&format!("{base}_libelle"))
                            .or_else(|| item.get("libelle"))
                    } else if col.ends_with("_jjmmaaaa") {
                        item.get(&format!("{base}_jjmmaaaa"))
                            .or_else(|| item.get("jjmmaaaa"))
                    } else if col.ends_with("_numero") {
                        item.get(&format!("{base}_numero"))
                            .or_else(|| item.get("numero"))
                    } else {
                        item.get(&a.nom)
                    }
                } else {
                    item.get(&a.nom).or_else(|| item.get(col))
                }
            } else {
                item.get(col)
            }
            .cloned()
            .unwrap_or(Value::Null);
            let attr_type = attr.map(|a| a.attr_type.as_str()).unwrap_or("string");
            columns.push(col.clone());
            placeholders.push(format!("?{idx}"));
            idx += 1;
            values.push(json_to_sql(&val, attr_type));
        }

        let sql = format!(
            "INSERT INTO {table} ({}) VALUES ({})",
            columns.join(", "),
            placeholders.join(", ")
        );
        db.conn
            .execute(&sql, rusqlite::params_from_iter(values.iter()))
            .map_err(|e| format!("INSERT {table} : {e}"))?;
    }
    Ok(())
}

pub fn load_embed_list(
    db: &Database,
    parent_entity: &str,
    parent_id: &str,
    child: &EntityDef,
) -> Result<Vec<Map<String, Value>>, String> {
    let table = fille_table_name(&child.nom);
    if !table_exists(db, &table)? {
        return Ok(Vec::new());
    }
    let sql = format!(
        "SELECT * FROM {table} WHERE parent_entity = ?1 AND parent_id = ?2 ORDER BY created_at, id"
    );
    let mut stmt = db.conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![parent_entity, parent_id], |row| {
            let count = row.as_ref().column_count();
            let mut m = Map::new();
            for i in 0..count {
                let name = row.as_ref().column_name(i)?.to_string();
                if matches!(name.as_str(), "id" | "parent_id" | "parent_entity" | "created_at") {
                    continue;
                }
                let val: rusqlite::types::Value = row.get(i)?;
                m.insert(name, sql_value_to_json(val));
            }
            Ok(m)
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

fn table_exists(db: &Database, table: &str) -> Result<bool, String> {
    let n: i64 = db
        .conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name = ?1",
            params![table],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(n > 0)
}

fn sql_value_to_json(v: rusqlite::types::Value) -> Value {
    match v {
        rusqlite::types::Value::Null => Value::Null,
        rusqlite::types::Value::Integer(i) => Value::Number(i.into()),
        rusqlite::types::Value::Real(f) => serde_json::Number::from_f64(f)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        rusqlite::types::Value::Text(s) => Value::String(s),
        rusqlite::types::Value::Blob(b) => Value::String(String::from_utf8_lossy(&b).to_string()),
    }
}

pub fn delete_embed_lists_for_parent(
    db: &Database,
    registry: &EntityRegistry,
    parent_entity: &str,
    parent_id: &str,
) -> Result<(), String> {
    let parent_ent = registry.find(parent_entity);
    let Some(parent_ent) = parent_ent else {
        return Ok(());
    };
    for attr in parent_ent
        .attributs
        .iter()
        .filter(|a| !is_reserved_attribute(a))
    {
        if attr.attr_type != "entity" || !attr.relation_multiple {
            continue;
        }
        let Some(child) = embed::resolve_child(registry, attr) else {
            continue;
        };
        let table = fille_table_name(&child.nom);
        if table_exists(db, &table)? {
            db.conn
                .execute(
                    &format!("DELETE FROM {table} WHERE parent_entity = ?1 AND parent_id = ?2"),
                    params![parent_entity, parent_id],
                )
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub fn embed_list_attrs<'a>(
    ent: &'a EntityDef,
    registry: &'a EntityRegistry,
) -> Vec<(&'a EntityAttribute, &'a EntityDef)> {
    ent.attributs
        .iter()
        .filter(|a| !is_reserved_attribute(a))
        .filter(|a| a.attr_type == "entity" && a.relation_multiple)
        .filter_map(|a| embed::resolve_child(registry, a).map(|c| (a, c)))
        .collect()
}
