use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db_io::DocumentRow;
use crate::AppState;

#[derive(Deserialize)]
pub struct DocumentListRequest {
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
}

#[derive(Deserialize)]
pub struct DocumentImportRequest {
    pub original_name: String,
    pub data_base64: String,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub caption: Option<String>,
}

#[derive(Deserialize)]
pub struct DocumentIdRequest {
    pub id: String,
}

#[derive(Serialize)]
pub struct DocumentExportResponse {
    pub base64: String,
    pub mime: String,
    pub file_name: String,
    pub kind: String,
}

#[derive(Serialize)]
pub struct DocumentWithPath {
    pub id: String,
    pub original_name: String,
    pub kind: String,
    pub bytes: i64,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub caption: String,
    pub uploaded_at: String,
    pub source: String,
    pub absolute_path: String,
}

#[tauri::command]
pub fn documents_list(
    state: State<'_, AppState>,
    payload: DocumentListRequest,
) -> Result<Vec<DocumentWithPath>, String> {
    state.desktop_sessions.require_privilege("documents:voir")?;
    let db = state.db.lock();
    let docs = db
        .list_documents(payload.entity_type.as_deref(), payload.entity_id.as_deref())
        .map_err(|e| e.to_string())?;
    Ok(docs
        .into_iter()
        .map(|d| document_with_path(&db, d))
        .collect())
}

#[tauri::command]
pub fn documents_import(
    state: State<'_, AppState>,
    payload: DocumentImportRequest,
) -> Result<DocumentWithPath, String> {
    state
        .desktop_sessions
        .require_privilege("documents:importer")?;

    let raw = B64
        .decode(payload.data_base64.trim())
        .map_err(|e| format!("Base64 invalide : {e}"))?;

    let db = state.db.lock();
    let row = db.import_document(
        &payload.original_name,
        &raw,
        payload.entity_type.as_deref().unwrap_or("general"),
        payload.entity_id.as_deref(),
        payload.caption.as_deref().unwrap_or(""),
        "desktop",
    )?;
    Ok(document_with_path(&db, row))
}

#[tauri::command]
pub fn documents_export(
    state: State<'_, AppState>,
    payload: DocumentIdRequest,
) -> Result<DocumentExportResponse, String> {
    state
        .desktop_sessions
        .require_privilege("documents:exporter")?;
    let db = state.db.lock();
    let ex = db.export_document(&payload.id)?;
    Ok(DocumentExportResponse {
        base64: ex.base64,
        mime: ex.mime,
        file_name: ex.file_name,
        kind: ex.kind,
    })
}

#[tauri::command]
pub fn documents_delete(
    state: State<'_, AppState>,
    payload: DocumentIdRequest,
) -> Result<(), String> {
    state
        .desktop_sessions
        .require_privilege("documents:supprimer")?;
    let db = state.db.lock();
    db.delete_document(&payload.id)
}

fn document_with_path(db: &crate::db::Database, d: DocumentRow) -> DocumentWithPath {
    let path = db.document_absolute_path(&d.id, &d.kind);
    DocumentWithPath {
        id: d.id,
        original_name: d.original_name,
        kind: d.kind,
        bytes: d.bytes,
        entity_type: d.entity_type,
        entity_id: d.entity_id,
        caption: d.caption,
        uploaded_at: d.uploaded_at,
        source: d.source,
        absolute_path: path.to_string_lossy().to_string(),
    }
}
