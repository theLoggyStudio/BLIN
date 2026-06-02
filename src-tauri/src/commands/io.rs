use serde::{Deserialize, Serialize};
use tauri::State;

use crate::csv_util::strip_bom;
use crate::db_io::{CsvImportResponse, CsvTable};
use crate::AppState;

#[derive(Deserialize)]
pub struct CsvExportRequest {
    pub table: String,
}

#[derive(Serialize)]
pub struct CsvExportResponse {
    pub csv: String,
    pub file_name: String,
}

#[derive(Deserialize)]
pub struct CsvImportRequest {
    pub table: String,
    pub csv: String,
}

#[tauri::command]
pub fn io_export_csv(
    state: State<'_, AppState>,
    payload: CsvExportRequest,
) -> Result<CsvExportResponse, String> {
    let table = CsvTable::from_str(payload.table.trim())?;
    state
        .desktop_sessions
        .require_privilege(table.export_privilege())?;

    let db = state.db.lock();
    let csv = db.export_csv_table(table).map_err(|e| e.to_string())?;
    Ok(CsvExportResponse {
        csv,
        file_name: table.file_name().to_string(),
    })
}

#[tauri::command]
pub fn io_import_csv(
    state: State<'_, AppState>,
    payload: CsvImportRequest,
) -> Result<CsvImportResponse, String> {
    let table = CsvTable::from_str(payload.table.trim())?;
    state
        .desktop_sessions
        .require_privilege(table.import_privilege())?;

    let csv = strip_bom(&payload.csv);
    let db = state.db.lock();
    db.import_csv_table(table, csv)
}
