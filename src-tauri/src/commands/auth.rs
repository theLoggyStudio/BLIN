use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::session::{ActiveSession, SessionUser};
use crate::AppState;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: SessionUser,
    pub must_change_password: bool,
}

#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    pub new_password: String,
    pub confirm_password: String,
}

fn session_user_from_auth(
    id: String,
    nom: String,
    email: String,
    role_nom: String,
    privileges: Vec<String>,
    must_change_password: bool,
) -> SessionUser {
    SessionUser {
        id,
        nom,
        email,
        role: role_nom,
        privileges,
        must_change_password,
    }
}

#[tauri::command]
pub fn auth_login(
    state: State<'_, AppState>,
    payload: LoginRequest,
) -> Result<LoginResponse, String> {
    let db = state.db.lock();
    let (id, nom, role_nom, _role_id, privileges, must_change_password) = db
        .authenticate(&payload.email, &payload.password)
        .map_err(|e| e.to_string())?;
    drop(db);

    let token = Uuid::new_v4().to_string();
    let user = session_user_from_auth(
        id,
        nom,
        payload.email,
        role_nom,
        privileges,
        must_change_password,
    );

    state.desktop_sessions.set(ActiveSession {
        token: token.clone(),
        user: user.clone(),
    });

    Ok(LoginResponse {
        token,
        user,
        must_change_password,
    })
}

#[tauri::command]
pub fn auth_change_password(
    state: State<'_, AppState>,
    payload: ChangePasswordRequest,
) -> Result<SessionUser, String> {
    let session = state.desktop_sessions.require_session()?;
    if payload.new_password != payload.confirm_password {
        return Err("Les mots de passe ne correspondent pas.".into());
    }

    let db = state.db.lock();
    let must_change = db
        .user_must_change_password(&session.user.id)
        .map_err(|e| e.to_string())?;
    if !must_change {
        return Err("Aucun changement de mot de passe requis.".into());
    }

    db.change_password(
        &session.user.id,
        &payload.new_password,
        false,
    )
    .map_err(|e| e.to_string())?;

    let mut user = session.user.clone();
    user.must_change_password = false;
    state.desktop_sessions.set(ActiveSession {
        token: session.token,
        user: user.clone(),
    });
    Ok(user)
}

#[tauri::command]
pub fn auth_logout(state: State<'_, AppState>) -> Result<(), String> {
    state.desktop_sessions.clear();
    Ok(())
}

#[tauri::command]
pub fn auth_current_user(state: State<'_, AppState>) -> Result<SessionUser, String> {
    let session = state.desktop_sessions.require_session()?;
    let db = state.db.lock();
    let must_change = db
        .user_must_change_password(&session.user.id)
        .map_err(|e| e.to_string())?;
    drop(db);

    let mut user = session.user.clone();
    if user.must_change_password != must_change {
        user.must_change_password = must_change;
        state.desktop_sessions.set(ActiveSession {
            token: session.token,
            user: user.clone(),
        });
    }
    Ok(user)
}

/// Recharge les privilèges de la session desktop (ex. après `entity_registry_save`).
#[tauri::command]
pub fn auth_sync_session_privileges(state: State<'_, AppState>) -> Result<SessionUser, String> {
    let db = state.db.lock();
    let user = state
        .desktop_sessions
        .sync_privileges(&db)?
        .ok_or_else(|| "Aucune session active".to_string())?;
    Ok(user)
}
