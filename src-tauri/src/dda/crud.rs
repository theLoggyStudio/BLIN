use std::collections::HashMap;

use chrono::Utc;
use rusqlite::{params, types::Value as SqlValue, Row};
use serde_json::{Map, Value};
use uuid::Uuid;

use super::config::{FieldDef, ScreenConfigFile};
use super::media::{relocate_draft_media, rewrite_path_after_relocate};
use super::validation::{validate_screen_data, validation_error_json};
use crate::entity::child_table;
use crate::entity::record_signature::{self, RowUserContext};
use crate::db::Database;

#[derive(Clone, Copy)]
pub struct ListRowsOptions<'a> {
    pub viewer_role_id: Option<&'a str>,
    pub viewer_user_id: Option<&'a str>,
    pub viewer_privileges: &'a [String],
}

impl Default for ListRowsOptions<'_> {
    fn default() -> Self {
        Self {
            viewer_role_id: None,
            viewer_user_id: None,
            viewer_privileges: &[],
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ListRowsPagination {
    pub offset: u32,
    pub limit: u32,
}

struct PreparedListQuery {
    select_prefix: String,
    count_sql: String,
    order_sql: String,
    params: Vec<SqlValue>,
}

fn prepare_list_query(
    cfg: &ScreenConfigFile,
    filters: &HashMap<String, String>,
    options: ListRowsOptions<'_>,
    registry: &crate::entity::registry::EntityRegistry,
) -> Result<PreparedListQuery, String> {
    let table = &cfg.screen.table;
    let mut cols: Vec<String> = cfg
        .persisted_fields()
        .iter()
        .map(|f| f.column.clone())
        .collect();
    if let Some(ent) = registry.find(&cfg.screen.key) {
        if crate::entity::parent_lignes::entity_has_embed_children(ent, registry) {
            let lignes_col = crate::entity::parent_lignes::LIGNES_COLUMN;
            if !cols.iter().any(|c| c == lignes_col) {
                cols.push(lignes_col.to_string());
            }
        }
    }
    let col_list = if cols.iter().any(|c| c == "id") {
        cols.join(", ")
    } else {
        format!("id, {}", cols.join(", "))
    };

    let mut data_sql = format!("SELECT {col_list} FROM {table} WHERE 1=1");
    let mut sql_params: Vec<SqlValue> = Vec::new();

    for field in cfg.filter_fields() {
        let Some(val) = filters.get(&field.key) else {
            continue;
        };
        if val.trim().is_empty() {
            continue;
        }
        crate::dda::filters::append_field_filter_sql(&mut data_sql, &mut sql_params, field, val, false);
    }

    let filter_field_keys: std::collections::HashSet<String> =
        cfg.filter_fields().into_iter().map(|f| f.key.to_string()).collect();
    for field in cfg.fields.iter().filter(|f| {
        f.filter.as_ref().is_some_and(|x| x.enabled) && crate::dda::config::is_persisted_field(f)
    }) {
        if filter_field_keys.contains(&field.key) {
            continue;
        }
        let Some(val) = filters.get(&field.key) else {
            continue;
        };
        if val.trim().is_empty() {
            continue;
        }
        crate::dda::filters::append_field_filter_sql(&mut data_sql, &mut sql_params, field, val, true);
    }

    if cfg.screen.key == crate::entity::tache_visibility::TACHE_ENTITY_KEY {
        if let Some(role_id) = options.viewer_role_id {
            if !crate::entity::tache_visibility::can_user_see_all_tasks(options.viewer_privileges) {
                data_sql.push_str(&crate::entity::tache_visibility::sql_visibility_filter(
                    role_id,
                    options.viewer_user_id,
                ));
            }
        }
    }

    let count_sql = data_sql.replacen(
        &format!("SELECT {col_list}"),
        "SELECT COUNT(*)",
        1,
    );

    let order = cfg
        .screen
        .default_order_by
        .as_deref()
        .unwrap_or("datetime(created_at) DESC");
    let order_sql = if order.contains(" DESC")
        || order.contains(" ASC")
        || order.contains('(')
        || order.contains(',')
    {
        format!(" ORDER BY {order}")
    } else {
        format!(" ORDER BY {order} COLLATE NOCASE")
    };

    Ok(PreparedListQuery {
        select_prefix: data_sql,
        count_sql,
        order_sql,
        params: sql_params,
    })
}

pub fn count_rows_with_options(
    db: &Database,
    cfg: &ScreenConfigFile,
    filters: &HashMap<String, String>,
    options: ListRowsOptions<'_>,
) -> Result<u64, String> {
    let registry = crate::entity::registry::load_data(&db.data_dir).map_err(|e| e.to_string())?;
    let mut filters = filters.clone();
    crate::entity::session_scope::merge_active_session_filter(
        &db.data_dir,
        &registry,
        &cfg.screen.key,
        &mut filters,
    )?;
    let query = prepare_list_query(cfg, &filters, options, &registry)?;
    let count: i64 = db
        .conn
        .query_row(
            &query.count_sql,
            rusqlite::params_from_iter(query.params.iter()),
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(count.max(0) as u64)
}

pub fn list_rows(
    db: &Database,
    cfg: &ScreenConfigFile,
    filters: &HashMap<String, String>,
) -> Result<Vec<Map<String, Value>>, String> {
    list_rows_with_options(db, cfg, filters, ListRowsOptions::default(), None)
}

pub fn list_rows_with_options(
    db: &Database,
    cfg: &ScreenConfigFile,
    filters: &HashMap<String, String>,
    options: ListRowsOptions<'_>,
    pagination: Option<ListRowsPagination>,
) -> Result<Vec<Map<String, Value>>, String> {
    let registry = crate::entity::registry::load_data(&db.data_dir).map_err(|e| e.to_string())?;
    let mut filters = filters.clone();
    crate::entity::session_scope::merge_active_session_filter(
        &db.data_dir,
        &registry,
        &cfg.screen.key,
        &mut filters,
    )?;

    let query = prepare_list_query(cfg, &filters, options, &registry)?;
    let mut sql = format!("{}{}", query.select_prefix, query.order_sql);
    let mut sql_params = query.params;
    if let Some(p) = pagination {
        sql.push_str(" LIMIT ? OFFSET ?");
        sql_params.push(SqlValue::Integer(i64::from(p.limit)));
        sql_params.push(SqlValue::Integer(i64::from(p.offset)));
    }

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
    let mut row = db
        .conn
        .query_row(&sql, params![id], row_to_json_map)
        .map_err(|e| e.to_string())?;
    if cfg.screen.key == crate::entity::tache_visibility::TACHE_ENTITY_KEY {
        if let Some(role_id) = options.viewer_role_id {
            if !crate::entity::tache_visibility::can_user_see_all_tasks(options.viewer_privileges)
                && !crate::entity::tache_visibility::row_visible_to_role(
                    &row,
                    role_id,
                    options.viewer_user_id,
                    false,
                )
            {
                return Err("Tâche introuvable ou non visible pour votre rôle.".into());
            }
        }
    }
    hydrate_embed_lists(db, cfg, &mut row)?;
    crate::entity::parent_lignes::hydrate_form_lines(&mut row, cfg);
    Ok(row)
}

fn hydrate_embed_lists(
    db: &Database,
    cfg: &ScreenConfigFile,
    row: &mut Map<String, Value>,
) -> Result<(), String> {
    let registry = crate::entity::registry::load(&db.data_dir).map_err(|e| e.to_string())?;
    let Some(ent) = registry.find(&cfg.screen.key) else {
        return Ok(());
    };
    let record_id = row
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if record_id.is_empty() {
        return Ok(());
    }
    for (attr, child) in child_table::embed_list_attrs(ent, &registry) {
        let items = child_table::load_embed_list(db, &cfg.screen.key, &record_id, child)?;
        row.insert(
            attr.nom.clone(),
            Value::Array(items.into_iter().map(Value::Object).collect()),
        );
    }
    Ok(())
}

fn extract_embed_lists(
    data: &mut Map<String, Value>,
    cfg: &ScreenConfigFile,
    registry: &crate::entity::registry::EntityRegistry,
) -> Vec<(String, crate::entity::registry::EntityDef, Vec<Map<String, Value>>)> {
    let Some(ent) = registry.find(&cfg.screen.key) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (attr, child) in child_table::embed_list_attrs(ent, registry) {
        let raw = data.remove(&attr.nom);
        let items = child_table::parse_embed_list_items(raw.as_ref());
        out.push((attr.nom.clone(), (*child).clone(), items));
    }
    out
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CreateRowOptions {
    /// Import CSV en masse : évite les hooks lourds par ligne (tâches, impacts).
    pub skip_post_create_hooks: bool,
}

pub fn create_row(
    db: &Database,
    cfg: &ScreenConfigFile,
    data: &Map<String, Value>,
) -> Result<Map<String, Value>, String> {
    create_row_with_user(db, cfg, data, None)
}

pub fn create_row_with_user(
    db: &Database,
    cfg: &ScreenConfigFile,
    data: &Map<String, Value>,
    user_ctx: Option<RowUserContext<'_>>,
) -> Result<Map<String, Value>, String> {
    create_row_with_user_and_options(db, cfg, data, user_ctx, CreateRowOptions::default())
}

pub fn create_row_with_user_and_options(
    db: &Database,
    cfg: &ScreenConfigFile,
    data: &Map<String, Value>,
    user_ctx: Option<RowUserContext<'_>>,
    options: CreateRowOptions,
) -> Result<Map<String, Value>, String> {
    let mut data = data.clone();
    let registry = crate::entity::registry::load(&db.data_dir).map_err(|e| e.to_string())?;
    crate::entity::session_scope::apply_active_session_on_create(
        db,
        &registry,
        &cfg.screen.key,
        &mut data,
    )?;
    record_signature::apply_signature_status_on_create(
        &mut data,
        &cfg.screen.key,
        &registry,
    );
    record_signature::apply_creator_on_create(
        &mut data,
        &cfg.screen.key,
        &registry,
        user_ctx,
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

    crate::entity::parent_lignes::merge_create_lines_into_data(&mut data, cfg);

    let report = validate_screen_data(cfg, &data, false);
    if !report.valid {
        return Err(validation_error_json(&report));
    }
    let embed_lists = extract_embed_lists(&mut data, cfg, &registry);
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

    if let Some(lignes_val) = data.get(crate::entity::parent_lignes::LIGNES_COLUMN) {
        let col = crate::entity::parent_lignes::LIGNES_COLUMN;
        if !columns.iter().any(|c| c == col) {
            columns.push(col.to_string());
            placeholders.push(format!("?{idx}"));
            idx += 1;
            values.push(SqlValue::Text(
                lignes_val
                    .as_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| lignes_val.to_string()),
            ));
        }
    }

    if let Some(sig_val) = data.get(record_signature::SIGNATURE_STATUS_COLUMN) {
        let col = record_signature::SIGNATURE_STATUS_COLUMN;
        if !columns.iter().any(|c| c == col) {
            columns.push(col.to_string());
            placeholders.push(format!("?{idx}"));
            idx += 1;
            values.push(SqlValue::Text(
                sig_val
                    .as_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| sig_val.to_string()),
            ));
        }
    }

    if let Some(creator_val) = data.get(record_signature::CREATED_BY_COLUMN) {
        let col = record_signature::CREATED_BY_COLUMN;
        if !columns.iter().any(|c| c == col) {
            columns.push(col.to_string());
            placeholders.push(format!("?{idx}"));
            idx += 1;
            values.push(SqlValue::Text(
                creator_val
                    .as_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| creator_val.to_string()),
            ));
        }
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
    for (attr_key, child, items) in embed_lists {
        child_table::save_embed_list(db, &cfg.screen.key, &id, &child, &items)?;
        data.insert(
            attr_key,
            Value::Array(items.into_iter().map(Value::Object).collect()),
        );
    }
    let row = get_row(db, cfg, &id)?;
    if !options.skip_post_create_hooks {
        crate::entity::validation::after_entity_row_created(db, &cfg.screen.key, &row)?;
        crate::entity::relation_impact::apply_after_create_if_ready(
            db,
            &db.data_dir,
            &cfg.screen.key,
            &id,
        );
        crate::entity::session_scope::activate_if_session_entity(
            &db.data_dir,
            &registry,
            &cfg.screen.key,
            &row,
        )?;
    }
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
    let registry = crate::entity::registry::load(&db.data_dir).map_err(|e| e.to_string())?;
    record_signature::assert_record_editable_by_user(
        db,
        &cfg.screen.key,
        id,
        options.viewer_user_id,
        &registry,
    )?;
    get_row_with_options(db, cfg, id, options)?;
    let mut data = data.clone();
    if cfg.screen.key == crate::entity::tache_visibility::TACHE_ENTITY_KEY {
        crate::entity::tache_visibility::apply_create_defaults(&mut data);
    }
    crate::entity::parent_lignes::merge_create_lines_into_data(&mut data, cfg);
    let report = validate_screen_data(cfg, &data, false);
    if !report.valid {
        return Err(validation_error_json(&report));
    }
    let embed_lists = extract_embed_lists(&mut data, cfg, &registry);
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
    if let Some(lignes_val) = data.get(crate::entity::parent_lignes::LIGNES_COLUMN) {
        let col = crate::entity::parent_lignes::LIGNES_COLUMN;
        sets.push(format!("{col} = ?{idx}"));
        idx += 1;
        values.push(SqlValue::Text(
            lignes_val
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| lignes_val.to_string()),
        ));
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
    for (attr_key, child, items) in embed_lists {
        child_table::save_embed_list(db, &cfg.screen.key, id, &child, &items)?;
        data.insert(
            attr_key,
            Value::Array(items.into_iter().map(Value::Object).collect()),
        );
    }
    get_row_with_options(db, cfg, id, options)
}

pub fn delete_row(db: &Database, cfg: &ScreenConfigFile, id: &str) -> Result<(), String> {
    delete_row_with_options(db, cfg, id, ListRowsOptions::default())
}

pub fn delete_row_with_options(
    db: &Database,
    cfg: &ScreenConfigFile,
    id: &str,
    options: ListRowsOptions<'_>,
) -> Result<(), String> {
    let registry = crate::entity::registry::load(&db.data_dir).map_err(|e| e.to_string())?;
    record_signature::assert_record_editable_by_user(
        db,
        &cfg.screen.key,
        id,
        options.viewer_user_id,
        &registry,
    )?;
    child_table::delete_embed_lists_for_parent(db, &registry, &cfg.screen.key, id)?;
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
