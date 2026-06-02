use serde::Deserialize;
use tauri::State;

use crate::db::{FinanceRow, GenerateMonthlyFinancesResult};
use crate::AppState;

#[derive(Deserialize)]
pub struct FinanceIdRequest {
    pub id: String,
}

#[derive(Deserialize)]
pub struct GenerateMonthlyRequest {
    pub annee: i32,
    pub mois: u32,
}

#[tauri::command]
pub fn finances_list(state: State<'_, AppState>) -> Result<Vec<FinanceRow>, String> {
    state
        .desktop_sessions
        .require_privilege("finances:valider")?;
    let db = state.db.lock();
    db.list_finances().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn finances_validate(
    state: State<'_, AppState>,
    payload: FinanceIdRequest,
) -> Result<FinanceRow, String> {
    state
        .desktop_sessions
        .require_privilege("finances:valider")?;
    let db = state.db.lock();
    db.validate_finance(&payload.id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn finances_generate_month(
    state: State<'_, AppState>,
    payload: GenerateMonthlyRequest,
) -> Result<GenerateMonthlyFinancesResult, String> {
    state
        .desktop_sessions
        .require_privilege("finances:valider")?;
    let db = state.db.lock();
    db.generate_monthly_finances(payload.annee, payload.mois)
        .map_err(|e| e.to_string())
}
