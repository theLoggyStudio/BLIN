use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::session::{ActiveSession, SessionUser};
use crate::ai::login_messages;
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
    pub login_greeting: String,
    pub login_notices: Vec<String>,
}

#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    pub new_password: String,
    pub confirm_password: String,
}

#[derive(Deserialize)]
pub struct VerifyPasswordRequest {
    pub password: String,
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

#[derive(Serialize)]
pub struct LoginMessagesPayload {
    pub greeting: String,
    pub invalid_credentials: String,
    pub prepared: bool,
}

fn cached_login_messages(state: &AppState) -> LoginMessagesPayload {
    let cache = state.login_messages.lock();
    LoginMessagesPayload {
        greeting: cache.greeting.clone(),
        invalid_credentials: cache.invalid_credentials.clone(),
        prepared: cache.prepared,
    }
}

/// Prépare en arrière-plan la salutation et le message d'identifiants invalides (Loggy).
/// Réponse immédiate (fallbacks) — personnification IA en thread si llama disponible.
#[tauri::command]
pub fn auth_prepare_login_messages(state: State<'_, AppState>) -> Result<LoginMessagesPayload, String> {
    {
        let cache = state.login_messages.lock();
        if cache.prepared {
            return Ok(LoginMessagesPayload {
                greeting: cache.greeting.clone(),
                invalid_credentials: cache.invalid_credentials.clone(),
                prepared: true,
            });
        }
    }

    let fast = {
        let db = state.db.lock();
        login_messages::prepare_fast(&db)
    };

    if !fast.prepared {
        let db_arc = state.db.clone();
        let cache_arc = state.login_messages.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(2));
            loop {
                if let Some(db) = db_arc.try_lock() {
                    let prepared = login_messages::prepare(&db);
                    if prepared.prepared {
                        *cache_arc.lock() = prepared;
                    }
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(150));
            }
        });
    } else {
        *state.login_messages.lock() = fast.clone();
    }

    Ok(LoginMessagesPayload {
        greeting: fast.greeting,
        invalid_credentials: fast.invalid_credentials,
        prepared: fast.prepared,
    })
}

/// Retourne les messages préparés (sans relancer la préparation).
#[tauri::command]
pub fn auth_get_login_messages(state: State<'_, AppState>) -> Result<LoginMessagesPayload, String> {
    Ok(cached_login_messages(&state))
}

/// État de la synchronisation lourde au démarrage (DDA + registre + RAG).
#[tauri::command]
pub fn app_startup_sync_status(
    state: State<'_, AppState>,
) -> Result<crate::startup_sync::StartupSyncStatusPayload, String> {
    Ok(state.startup_sync.lock().to_payload())
}

#[tauri::command]
pub fn auth_login(
    state: State<'_, AppState>,
    payload: LoginRequest,
) -> Result<LoginResponse, String> {
    let db = state.db.lock();
    let auth_result = db.authenticate(&payload.email, &payload.password);
    if auth_result.is_err() {
        let err = auth_result.err().map(|e| e.to_string()).unwrap_or_default();
        let cache = state.login_messages.lock();
        if cache.prepared && !cache.invalid_credentials.is_empty() {
            return Err(cache.invalid_credentials.clone());
        }
        return Err(if err.contains("Identifiants invalides") {
            login_messages::fallback_invalid()
        } else {
            err
        });
    }
    let (id, nom, role_nom, role_id, privileges, must_change_password) = auth_result.unwrap();

    let login_notices = crate::entity::validation::login_workflow_notices(&db, &id, &role_id)
        .unwrap_or_default();
    let app_name = crate::entity::branding::ecosystem_name(&db.data_dir);
    drop(db);

    let login_greeting = {
        let cache = state.login_messages.lock();
        if cache.prepared && !cache.greeting.is_empty() {
            login_messages::inject_user_name_into_greeting(&cache.greeting, &nom)
        } else {
            crate::ai::greetings::format_login_greeting(&nom, &app_name)
        }
    };

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
        login_greeting,
        login_notices,
    })
}

/// Ré-authentification (dépliage d'un panneau Paramètres).
#[tauri::command]
pub fn auth_verify_password(
    state: State<'_, AppState>,
    payload: VerifyPasswordRequest,
) -> Result<(), String> {
    let session = state.desktop_sessions.require_session()?;
    let db = state.db.lock();
    db.verify_user_password(&session.user.id, &payload.password)
        .map_err(|e| e.to_string())
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
