use std::collections::HashMap;

use chrono::Utc;
use rusqlite::{params, types::Value as SqlValue, Row};
use serde_json::{Map, Value};
use uuid::Uuid;

use super::config::{FieldDef, ScreenConfigFile};
use super::media::{relocate_draft_media, rewrite_path_after_relocate};
use super::validation::{validate_screen_data, validation_error_json};
use crate::db::Database;

#[derive(Clone, Copy)]
pub struct ListRowsOptions<'a> {
    pub viewer_role_id: Option<&'a str>,
    pub viewer_privileges: &'a [String],
}

impl Default for ListRowsOptions<'_> {
    fn default() -> Self {
        Self {
            viewer_role_id: None,
            viewer_privileges: &[],
        }
    }
}

pub fn list_rows(
    db: &Database,
    cfg: &ScreenConfigFile,
    filters: &HashMap<String, String>,
) -> Result<Vec<Map<String, Value>>, String> {
    list_rows_with_options(db, cfg, filters, ListRowsOptions::default())
}

pub fn list_rows_with_options(
    db: &Database,
    cfg: &ScreenConfigFile,
    filters: &HashMap<String, String>,
    options: ListRowsOptions<'_>,
) -> Result<Vec<Map<String, Value>>, String> {
    let registry = crate::entity::registry::load(&db.data_dir).map_err(|e| e.to_string())?;
    let mut filters = filters.clone();
    crate::entity::session_scope::merge_active_session_filter(
        &db.data_dir,
        &registry,
        &cfg.screen.key,
        &mut filters,
    )?;

    let table = &cfg.screen.table;
    let cols: Vec<String> = cfg
        .persisted_fields()
        .iter()
        .map(|f| f.column.clone())
        .collect();
    let col_list = if cols.iter().any(|c| c == "id") {
        cols.join(", ")
    } else {
        format!("id, {}", cols.join(", "))
    };

    let mut sql = format!("SELECT {col_list} FROM {table} WHERE 1=1");
    let mut sql_params: Vec<SqlValue> = Vec::new();

    for field in cfg.filter_fields() {
        let Some(val) = filters.get(&field.key) else {
            continue;
        };
        if val.trim().is_empty() {
            continue;
        }
        let op = field.filter.as_ref().and_then(|f| f.operator.as_deref()).unwrap_or("contains");
        match op {
            "equals" => {
                sql.push_str(&format!(" AND {} = ?", field.column));
                sql_params.push(SqlValue::Text(val.trim().to_string()));
            }
            _ => {
                sql.push_str(&format!(" AND {} LIKE ?", field.column));
                sql_params.push(SqlValue::Text(format!("%{}%", val.trim())));
            }
        }
    }

    if cfg.screen.key == crate::entity::tache_visibility::TACHE_ENTITY_KEY {
        if let Some(role_id) = options.viewer_role_id {
            if !crate::entity::tache_visibility::can_user_see_all_tasks(options.viewer_privileges) {
                sql.push_str(&crate::entity::tache_visibility::sql_visibility_filter(role_id));
            }
        }
    }

    let order = cfg
        .screen
        .default_order_by
        .as_deref()
        .unwrap_or(&cfg.screen.label_field);
    sql.push_str(&format!(" ORDER BY {order} COLLATE NOCASE"));

    let mut stmt = db.conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(sql_params.iter()), row_to_json_map)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

pub fn get_row(db: &Database, cfg: &ScreenConfigFile, id: &str) -> Result<Map<String, Value>, String> {
    get_row_with_options(db, cfg, id, ListRowsOptions::default())
}

pub fn get_row_with_options(
    db: &Database,
    cfg: &ScreenConfigFile,
    id: &str,
    options: ListRowsOptions<'_>,
) -> Result<Map<String, Value>, String> {
    let table = &cfg.screen.table;
    let pk = &cfg.screen.primary_key;
    let sql = format!("SELECT * FROM {table} WHERE {pk} = ?1");
    let row = db
        .conn
        .query_row(&sql, params![id], row_to_json_map)
        .map_err(|e| e.to_string())?;
    if cfg.screen.key == crate::entity::tache_visibility::TACHE_ENTITY_KEY {
        if let Some(role_id) = options.viewer_role_id {
            if !crate::entity::tache_visibility::can_user_see_all_tasks(options.viewer_privileges)
                && !crate::entity::tache_visibility::row_visible_to_role(&row, role_id, false)
            {
                return Err("Tâche introuvable ou non visible pour votre rôle.".into());
            }
        }
    }
    Ok(row)
}

pub fn create_row(
    db: &Database,
    cfg: &ScreenConfigFile,
    data: &Map<String, Value>,
) -> Result<Map<String, Value>, String> {
    let mut data = data.clone();
    let registry = crate::entity::registry::load(&db.data_dir).map_err(|e| e.to_string())?;
    crate::entity::session_scope::apply_active_session_on_create(
        &db.data_dir,
        &registry,
        &cfg.screen.key,
        &mut data,
    )?;
    crate::entity::record_validation::set_non_valide_on_create(
        &mut data,
        &cfg.screen.key,
        &registry,
    );
    if cfg.screen.key == crate::entity::tache_visibility::TACHE_ENTITY_KEY {
        crate::entity::tache_visibility::apply_create_defaults(&mut data);
    }
    crate::entity::compteur::apply_compteurs_on_create(
        db,
        &registry,
        &cfg.screen.key,
        &mut data,
    )?;

    let report = validate_screen_data(cfg, &data, false);
    if !report.valid {
        return Err(validation_error_json(&report));
    }
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let table = &cfg.screen.table;

    let mut columns = vec!["id".to_string(), "created_at".to_string()];
    let mut placeholders = vec!["?1".to_string(), "?2".to_string()];
    let mut values: Vec<SqlValue> = vec![SqlValue::Text(id.clone()), SqlValue::Text(now)];
    let mut idx = 3usize;

    for field in cfg.writable_columns() {
        if field.column == "created_at" {
            continue;
        }
        let val = json_to_sql(
            data.get(&field.key).or_else(|| data.get(&field.column)),
            field,
        );
        columns.push(field.column.clone());
        placeholders.push(format!("?{idx}"));
        idx += 1;
        values.push(val);
    }

    for field in cfg.persisted_fields() {
        if !field
            .form
            .as_ref()
            .and_then(|f| f.auto_generated)
            .unwrap_or(false)
        {
            continue;
        }
        if columns.iter().any(|c| c == &field.column) {
            continue;
        }
        let val = json_to_sql(
            data.get(&field.key).or_else(|| data.get(&field.column)),
            field,
        );
        columns.push(field.column.clone());
        placeholders.push(format!("?{idx}"));
        idx += 1;
        values.push(val);
    }

    let sql = format!(
        "INSERT INTO {table} ({}) VALUES ({})",
        columns.join(", "),
        placeholders.join(", ")
    );
    db.conn
        .execute(&sql, rusqlite::params_from_iter(values.iter()))
        .map_err(|e| e.to_string())?;

    finalize_media_paths_after_create(db, cfg, &id, &data)?;
    let row = get_row(db, cfg, &id)?;
    crate::entity::validation::after_entity_row_created(db, &cfg.screen.key, &row)?;
    crate::entity::session_scope::activate_if_session_entity(&db.data_dir, &registry, &cfg.screen.key, &row)?;
    Ok(row)
}

pub fn update_row(
    db: &Database,
    cfg: &ScreenConfigFile,
    id: &str,
    data: &Map<String, Value>,
) -> Result<Map<String, Value>, String> {
    update_row_with_options(db, cfg, id, data, ListRowsOptions::default())
}

pub fn update_row_with_options(
    db: &Database,
    cfg: &ScreenConfigFile,
    id: &str,
    data: &Map<String, Value>,
    options: ListRowsOptions<'_>,
) -> Result<Map<String, Value>, String> {
    get_row_with_options(db, cfg, id, options)?;
    let mut data = data.clone();
    if cfg.screen.key == crate::entity::tache_visibility::TACHE_ENTITY_KEY {
        crate::entity::tache_visibility::apply_create_defaults(&mut data);
    }
    let report = validate_screen_data(cfg, &data, false);
    if !report.valid {
        return Err(validation_error_json(&report));
    }
    let table = &cfg.screen.table;
    let pk = &cfg.screen.primary_key;
    let mut sets = Vec::new();
    let mut values: Vec<SqlValue> = Vec::new();
    let mut idx = 1usize;

    for field in cfg.writable_columns() {
        if field.column == "id" || field.column == "created_at" {
            continue;
        }
        let Some(v) = data.get(&field.key).or_else(|| data.get(&field.column)) else {
            continue;
        };
        sets.push(format!("{} = ?{idx}", field.column));
        idx += 1;
        values.push(json_to_sql(Some(v), field));
    }
    if sets.is_empty() {
        return get_row_with_options(db, cfg, id, options);
    }
    let sql = format!(
        "UPDATE {table} SET {} WHERE {pk} = ?{idx}",
        sets.join(", ")
    );
    values.push(SqlValue::Text(id.to_string()));
    db.conn
        .execute(&sql, rusqlite::params_from_iter(values.iter()))
        .map_err(|e| e.to_string())?;
    get_row_with_options(db, cfg, id, options)
}

pub fn delete_row(db: &Database, cfg: &ScreenConfigFile, id: &str) -> Result<(), String> {
    let table = &cfg.screen.table;
    let pk = &cfg.screen.primary_key;
    let sql = format!("DELETE FROM {table} WHERE {pk} = ?1");
    db.conn
        .execute(&sql, params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn json_to_sql(v: Option<&Value>, field: &FieldDef) -> SqlValue {
    match v {
        None | Some(Value::Null) => {
            if field.field_type == "number" {
                SqlValue::Null
            } else {
                SqlValue::Text(String::new())
            }
        }
        Some(Value::String(s)) => SqlValue::Text(s.clone()),
        Some(Value::Number(n)) => {
            if let Some(i) = n.as_i64() {
                SqlValue::Integer(i)
            } else {
                SqlValue::Real(n.as_f64().unwrap_or(0.0))
            }
        }
        Some(Value::Bool(b)) => SqlValue::Integer(if *b { 1 } else { 0 }),
        Some(Value::Array(items)) if field.field_type == "images" => {
            let paths: Vec<String> = items
                .iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect();
            SqlValue::Text(serde_json::to_string(&paths).unwrap_or_else(|_| "[]".to_string()))
        }
        Some(other) => SqlValue::Text(other.to_string()),
    }
}

fn finalize_media_paths_after_create(
    db: &Database,
    cfg: &ScreenConfigFile,
    entity_id: &str,
    data: &Map<String, Value>,
) -> Result<(), String> {
    let draft_id = data
        .get("_uploadDraftId")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let Some(draft_id) = draft_id else {
        return Ok(());
    };

    let mut updates: Vec<(&str, String)> = Vec::new();
    for field in cfg.media_fields() {
        let folder = field
            .form
            .as_ref()
            .and_then(|f| f.storage_folder.clone())
            .or_else(|| default_storage_folder(cfg, field));
        let Some(folder) = folder else {
            continue;
        };
        relocate_draft_media(&db.data_dir, &folder, &draft_id, entity_id)?;

        let key = field.key.as_str();
        let col = field.column.as_str();
        let raw = data.get(key).or_else(|| data.get(col));
        let Some(raw) = raw else {
            continue;
        };

        match field.field_type.as_str() {
            "image" => {
                if let Some(s) = raw.as_str() {
                    if super::media::is_draft_path(s, &draft_id) {
                        updates.push((col, rewrite_path_after_relocate(s, &draft_id, entity_id)));
                    }
                }
            }
            "images" => {
                let paths = parse_image_paths_value(raw);
                let rewritten: Vec<String> = paths
                    .into_iter()
                    .map(|p| rewrite_path_after_relocate(&p, &draft_id, entity_id))
                    .collect();
                if !rewritten.is_empty() {
                    updates.push((
                        col,
                        serde_json::to_string(&rewritten).unwrap_or_else(|_| "[]".to_string()),
                    ));
                }
            }
            _ => {}
        }
    }

    if updates.is_empty() {
        return Ok(());
    }
    let table = &cfg.screen.table;
    for (col, val) in updates {
        let sql = format!("UPDATE {table} SET {col} = ?1 WHERE id = ?2");
        db.conn
            .execute(&sql, params![val, entity_id])
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn parse_image_paths_value(v: &Value) -> Vec<String> {
    match v {
        Value::Array(items) => items
            .iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .collect(),
        Value::String(s) if !s.trim().is_empty() => serde_json::from_str(s).unwrap_or_else(|_| {
            if s.starts_with('[') {
                Vec::new()
            } else {
                vec![s.clone()]
            }
        }),
        _ => Vec::new(),
    }
}

fn default_storage_folder(cfg: &ScreenConfigFile, _field: &FieldDef) -> Option<String> {
    cfg.screen.storage.as_ref().and_then(|s| {
        s.folders
            .iter()
            .find(|f| f.contains("photo"))
            .cloned()
            .or_else(|| s.folders.first().cloned())
    })
}

fn row_to_json_map(row: &Row<'_>) -> Result<Map<String, Value>, rusqlite::Error> {
    let mut m = Map::new();
    let count = row.as_ref().column_count();
    for i in 0..count {
        let name = row.as_ref().column_name(i)?.to_string();
        let val: rusqlite::types::Value = row.get(i)?;
        m.insert(name, sql_value_to_json(val));
    }
    Ok(m)
}

fn sql_value_to_json(v: rusqlite::types::Value) -> Value {
    match v {
        rusqlite::types::Value::Null => Value::Null,
        rusqlite::types::Value::Integer(i) => Value::Number(i.into()),
        rusqlite::types::Value::Real(f) => {
            serde_json::Number::from_f64(f).map(Value::Number).unwrap_or(Value::Null)
        }
        rusqlite::types::Value::Text(s) => Value::String(s),
        rusqlite::types::Value::Blob(b) => {
            Value::String(String::from_utf8_lossy(&b).to_string())
        }
    }
}
