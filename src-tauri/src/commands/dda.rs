use std::collections::HashMap;

use serde::Deserialize;
use serde_json::{Map, Value};
use tauri::State;

use crate::dda::{
    self, crud::{self, ListRowsOptions},
    load_screen_config,
    media::{absolute_path, decode_base64, delete_media, save_media},
    validation::{validate_screen_data, ValidationReport},
};
use crate::entity::record_validation;
use crate::AppState;

#[derive(Deserialize)]
pub struct DdaScreenKeyPayload {
    pub screen_key: String,
}

#[derive(Deserialize)]
pub struct DdaListPayload {
    pub screen_key: String,
    #[serde(default)]
    pub filters: HashMap<String, String>,
}

#[derive(Deserialize)]
pub struct DdaIdPayload {
    pub screen_key: String,
    pub id: String,
}

#[derive(Deserialize)]
pub struct DdaWritePayload {
    pub screen_key: String,
    pub id: Option<String>,
    pub data: Map<String, Value>,
}

#[derive(Deserialize)]
pub struct DdaMediaPathPayload {
    pub relative_path: String,
}

#[derive(Deserialize)]
pub struct DdaMediaUploadPayload {
    pub screen_key: String,
    pub entity_id: String,
    pub storage_folder: String,
    pub original_name: String,
    pub data_base64: String,
}

#[derive(Deserialize)]
pub struct DdaMediaDeletePayload {
    pub screen_key: String,
    pub relative_path: String,
}

fn load_cfg(state: &AppState, key: &str) -> Result<crate::dda::config::ScreenConfigFile, String> {
    if dda::SYSTEM_SCREEN_KEYS.contains(&key) {
        return Err(format!("Écran système protégé : {key}"));
    }
    let db = state.db.lock();
    dda::load_screen_config_with_data_dir(key, &db.data_dir)
}

fn require_view(state: &AppState, cfg: &crate::dda::config::ScreenConfigFile) -> Result<(), String> {
    state
        .desktop_sessions
        .require_privilege(&cfg.screen.privileges.view)?;
    Ok(())
}

fn require_create(state: &AppState, cfg: &crate::dda::config::ScreenConfigFile) -> Result<(), String> {
    state
        .desktop_sessions
        .require_privilege(&cfg.screen.privileges.create)?;
    Ok(())
}

fn require_update(state: &AppState, cfg: &crate::dda::config::ScreenConfigFile) -> Result<(), String> {
    state
        .desktop_sessions
        .require_privilege(&cfg.screen.privileges.update)?;
    Ok(())
}

fn require_delete(state: &AppState, cfg: &crate::dda::config::ScreenConfigFile) -> Result<(), String> {
    state
        .desktop_sessions
        .require_privilege(&cfg.screen.privileges.delete)?;
    Ok(())
}

#[tauri::command]
pub fn dda_sync_screens(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    state.desktop_sessions.require_privilege("ai:utiliser")?;
    let db = state.db.lock();
    let data_dir = db.data_dir.clone();
    let synced = dda::sync_all_screens(&db, &data_dir)?;
    dda::reindex_ai_knowledge(&db)?;
    Ok(synced)
}

#[tauri::command]
pub fn dda_list_screens(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    state.desktop_sessions.require_privilege("ai:utiliser")?;
    let dir = dda::json_config_dir();
    let configs = crate::dda::registry::load_all_screen_configs(&dir)?;
    Ok(configs
        .into_iter()
        .filter(|c| !c.screen.system)
        .map(|c| c.screen.key)
        .collect())
}

#[tauri::command]
pub fn dda_list(
    state: State<'_, AppState>,
    payload: DdaListPayload,
) -> Result<Vec<Map<String, Value>>, String> {
    let cfg = load_cfg(&state, &payload.screen_key)?;
    require_view(&state, &cfg)?;
    let session = state.desktop_sessions.require_session()?;
    let db = state.db.lock();
    let role_id = record_validation::user_role_id(&db, &session.user.id)?;
    let opts = ListRowsOptions {
        viewer_role_id: Some(role_id.as_str()),
        viewer_privileges: &session.user.privileges,
    };
    crud::list_rows_with_options(&db, &cfg, &payload.filters, opts)
}

#[tauri::command]
pub fn dda_get(state: State<'_, AppState>, payload: DdaIdPayload) -> Result<Map<String, Value>, String> {
    let cfg = load_cfg(&state, &payload.screen_key)?;
    require_view(&state, &cfg)?;
    let session = state.desktop_sessions.require_session()?;
    let db = state.db.lock();
    let role_id = record_validation::user_role_id(&db, &session.user.id)?;
    let opts = ListRowsOptions {
        viewer_role_id: Some(role_id.as_str()),
        viewer_privileges: &session.user.privileges,
    };
    crud::get_row_with_options(&db, &cfg, &payload.id, opts)
}

#[tauri::command]
pub fn dda_create(
    state: State<'_, AppState>,
    payload: DdaWritePayload,
) -> Result<Map<String, Value>, String> {
    let cfg = load_cfg(&state, &payload.screen_key)?;
    require_create(&state, &cfg)?;
    if payload.screen_key == crate::entity::stock::STOCK_ENTITY_KEY {
        crate::entity::stock::validate_stock_row(&payload.data)?;
    }
    let db = state.db.lock();
    let data_dir = db.data_dir.clone();
    let row = crud::create_row(&db, &cfg, &payload.data)?;
    if payload.screen_key == crate::entity::stock::STOCK_ENTITY_KEY {
        crate::entity::stock::after_stock_row_saved(&db, &data_dir, &row)?;
    } else if let Err(e) = crate::entity::stock::sync_lines_from_source(
        &db,
        &data_dir,
        &payload.screen_key,
        &row,
    ) {
        eprintln!("Sync stock : {e}");
    }
    Ok(row)
}

#[tauri::command]
pub fn dda_update(
    state: State<'_, AppState>,
    payload: DdaWritePayload,
) -> Result<Map<String, Value>, String> {
    let id = payload.id.ok_or("id requis pour la mise à jour")?;
    let cfg = load_cfg(&state, &payload.screen_key)?;
    require_update(&state, &cfg)?;
    if payload.screen_key == crate::entity::stock::STOCK_ENTITY_KEY {
        crate::entity::stock::validate_stock_row(&payload.data)?;
    }
    let session = state.desktop_sessions.require_session()?;
    let db = state.db.lock();
    let data_dir = db.data_dir.clone();
    let role_id = record_validation::user_role_id(&db, &session.user.id)?;
    let opts = ListRowsOptions {
        viewer_role_id: Some(role_id.as_str()),
        viewer_privileges: &session.user.privileges,
    };
    let row = crud::update_row_with_options(&db, &cfg, &id, &payload.data, opts)?;
    if payload.screen_key == crate::entity::stock::STOCK_ENTITY_KEY {
        crate::entity::stock::after_stock_row_saved(&db, &data_dir, &row)?;
    } else if let Err(e) = crate::entity::stock::sync_lines_from_source(
        &db,
        &data_dir,
        &payload.screen_key,
        &row,
    ) {
        eprintln!("Sync stock : {e}");
    }
    Ok(row)
}

#[tauri::command]
pub fn dda_validate(
    state: State<'_, AppState>,
    payload: DdaWritePayload,
) -> Result<ValidationReport, String> {
    let cfg = load_cfg(&state, &payload.screen_key)?;
    require_view(&state, &cfg)?;
    let for_filter = false;
    Ok(validate_screen_data(&cfg, &payload.data, for_filter))
}

#[tauri::command]
pub fn dda_validate_filters(
    state: State<'_, AppState>,
    payload: DdaListPayload,
) -> Result<ValidationReport, String> {
    let cfg = load_cfg(&state, &payload.screen_key)?;
    require_view(&state, &cfg)?;
    let mut map = serde_json::Map::new();
    for (k, v) in &payload.filters {
        map.insert(k.clone(), serde_json::Value::String(v.clone()));
    }
    Ok(validate_screen_data(&cfg, &map, true))
}

#[tauri::command]
pub fn dda_media_absolute_path(
    state: State<'_, AppState>,
    payload: DdaMediaPathPayload,
) -> Result<String, String> {
    state.desktop_sessions.require_session()?;
    let db = state.db.lock();
    let path = absolute_path(&db.data_dir, &payload.relative_path)?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn dda_media_upload(
    state: State<'_, AppState>,
    payload: DdaMediaUploadPayload,
) -> Result<String, String> {
    let cfg = load_cfg(&state, &payload.screen_key)?;
    if payload.entity_id.contains("_draft/") {
        require_create(&state, &cfg)?;
    } else {
        require_update(&state, &cfg)?;
    }
    let bytes = decode_base64(&payload.data_base64)?;
    let db = state.db.lock();
    save_media(
        &db.data_dir,
        &payload.storage_folder,
        &payload.entity_id,
        &payload.original_name,
        &bytes,
    )
}

#[tauri::command]
pub fn dda_media_delete(
    state: State<'_, AppState>,
    payload: DdaMediaDeletePayload,
) -> Result<(), String> {
    let cfg = load_cfg(&state, &payload.screen_key)?;
    require_update(&state, &cfg)?;
    let db = state.db.lock();
    delete_media(&db.data_dir, &payload.relative_path)
}

#[tauri::command]
pub fn dda_delete(state: State<'_, AppState>, payload: DdaIdPayload) -> Result<(), String> {
    let cfg = load_cfg(&state, &payload.screen_key)?;
    require_delete(&state, &cfg)?;
    let db = state.db.lock();
    let data_dir = db.data_dir.clone();
    if payload.screen_key != crate::entity::stock::STOCK_ENTITY_KEY {
        let _ = crate::entity::stock::remove_stock_line_for_source(
            &db,
            &data_dir,
            &payload.screen_key,
            &payload.id,
        );
    }
    crud::delete_row(&db, &cfg, &payload.id)
}
