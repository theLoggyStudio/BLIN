use serde::Deserialize;
use tauri::State;

use crate::db::{RoleRow, UserRow};
use crate::AppState;

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub nom: String,
    pub email: String,
    pub password: String,
    pub role_id: String,
}

#[derive(Deserialize)]
pub struct UpdateUserRequest {
    pub id: String,
    pub nom: String,
    pub email: String,
    pub role_id: String,
    pub actif: bool,
}

#[tauri::command]
pub fn users_list(state: State<'_, AppState>) -> Result<Vec<UserRow>, String> {
    state.desktop_sessions.require_privilege("users:voir")?;
    let db = state.db.lock();
    db.list_users().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn users_list_roles(state: State<'_, AppState>) -> Result<Vec<RoleRow>, String> {
    state.desktop_sessions.require_privilege("users:modifier")?;
    let db = state.db.lock();
    db.list_roles().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn users_create(
    state: State<'_, AppState>,
    payload: CreateUserRequest,
) -> Result<UserRow, String> {
    state.desktop_sessions.require_privilege("users:modifier")?;
    let db = state.db.lock();
    db.create_user(&payload.nom, &payload.email, &payload.password, &payload.role_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn users_update(
    state: State<'_, AppState>,
    payload: UpdateUserRequest,
) -> Result<UserRow, String> {
    state.desktop_sessions.require_privilege("users:modifier")?;
    let db = state.db.lock();
    db.update_user(
        &payload.id,
        &payload.nom,
        &payload.email,
        &payload.role_id,
        payload.actif,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn privileges_list_catalog(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    state.desktop_sessions.require_privilege("users:modifier")?;
    let db = state.db.lock();
    db.list_all_privileges().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn roles_list_with_privileges(
    state: State<'_, AppState>,
) -> Result<Vec<crate::db::RoleWithPrivilegesRow>, String> {
    state.desktop_sessions.require_privilege("users:modifier")?;
    let db = state.db.lock();
    db.list_roles_with_privileges().map_err(|e| e.to_string())
}

#[derive(Deserialize)]
pub struct UpdateRolePrivilegesRequest {
    pub role_id: String,
    pub privileges: Vec<String>,
}

#[tauri::command]
pub fn roles_update_privileges(
    state: State<'_, AppState>,
    payload: UpdateRolePrivilegesRequest,
) -> Result<(), String> {
    state.desktop_sessions.require_privilege("users:modifier")?;
    let db = state.db.lock();
    db.update_role_privileges(&payload.role_id, &payload.privileges)
        .map_err(|e| e.to_string())
}

#[derive(Deserialize)]
pub struct CreateRoleRequest {
    pub nom: String,
}

#[derive(Deserialize)]
pub struct UpdateRoleRequest {
    pub id: String,
    pub nom: String,
}

#[derive(Deserialize)]
pub struct RoleIdRequest {
    pub id: String,
}

#[tauri::command]
pub fn roles_create(
    state: State<'_, AppState>,
    payload: CreateRoleRequest,
) -> Result<RoleRow, String> {
    state.desktop_sessions.require_privilege("users:modifier")?;
    let db = state.db.lock();
    db.create_role(&payload.nom).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn roles_update(
    state: State<'_, AppState>,
    payload: UpdateRoleRequest,
) -> Result<RoleRow, String> {
    state.desktop_sessions.require_privilege("users:modifier")?;
    let db = state.db.lock();
    db.update_role(&payload.id, &payload.nom)
        .map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
pub struct RoleDeleteResponse {
    pub success: bool,
}

#[tauri::command]
pub fn roles_delete(
    state: State<'_, AppState>,
    payload: RoleIdRequest,
) -> Result<RoleDeleteResponse, String> {
    state.desktop_sessions.require_privilege("users:modifier")?;
    let db = state.db.lock();
    db.delete_role(&payload.id).map_err(|e| e.to_string())?;
    Ok(RoleDeleteResponse { success: true })
}
