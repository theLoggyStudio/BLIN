use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::HangarRow;
use crate::AppState;

#[derive(Deserialize)]
pub struct CreateHangarRequest {
    pub reference: String,
    pub zone: String,
    pub capacite_m3: f64,
}

#[derive(Deserialize)]
pub struct UpdateHangarRequest {
    pub id: String,
    pub reference: String,
    pub zone: String,
    pub capacite_m3: f64,
    pub statut: String,
}

#[derive(Deserialize)]
pub struct HangarIdRequest {
    pub hangar_id: String,
}

#[derive(Serialize)]
pub struct HangarPhotoWithPath {
    pub id: String,
    pub hangar_id: String,
    pub filename: String,
    pub uploaded_at: String,
    pub source: String,
    pub absolute_path: String,
}

#[tauri::command]
pub fn hangars_list(state: State<'_, AppState>) -> Result<Vec<HangarRow>, String> {
    state.desktop_sessions.require_privilege("hangars:voir")?;
    let db = state.db.lock();
    db.list_hangars().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hangars_create(
    state: State<'_, AppState>,
    payload: CreateHangarRequest,
) -> Result<HangarRow, String> {
    state.desktop_sessions.require_privilege("hangars:modifier")?;
    let db = state.db.lock();
    db.create_hangar(&payload.reference, &payload.zone, payload.capacite_m3)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hangars_update(
    state: State<'_, AppState>,
    payload: UpdateHangarRequest,
) -> Result<HangarRow, String> {
    state.desktop_sessions.require_privilege("hangars:modifier")?;
    let db = state.db.lock();
    db.update_hangar(
        &payload.id,
        &payload.reference,
        &payload.zone,
        payload.capacite_m3,
        &payload.statut,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hangars_list_photos(
    state: State<'_, AppState>,
    payload: HangarIdRequest,
) -> Result<Vec<HangarPhotoWithPath>, String> {
    state.desktop_sessions.require_privilege("hangars:voir")?;
    let db = state.db.lock();
    let docs = db
        .list_documents(Some("bien"), Some(&payload.hangar_id))
        .map_err(|e| e.to_string())?;
    Ok(docs
        .into_iter()
        .map(|d| {
            let path = db.document_absolute_path(&d.id, &d.kind);
            HangarPhotoWithPath {
                id: d.id.clone(),
                hangar_id: payload.hangar_id.clone(),
                filename: d.original_name.clone(),
                uploaded_at: d.uploaded_at,
                source: d.source,
                absolute_path: path.to_string_lossy().to_string(),
            }
        })
        .collect())
}
