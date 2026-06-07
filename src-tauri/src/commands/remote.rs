use serde::Serialize;
use tauri::{AppHandle, State};

use crate::remote::{RemoteConnectionInfo, RemoteServer, RemoteState};
use crate::AppState;

#[derive(Serialize)]
pub struct RemoteConnectionResponse {
    pub ip: String,
    pub url: String,
    #[serde(rename = "frontUrl")]
    pub front_url: String,
    pub port: u16,
    pub success: bool,
}

fn ensure_remote_server(app: &AppHandle, state: &AppState) {
    static STARTED: std::sync::Once = std::sync::Once::new();
    STARTED.call_once(|| {
        RemoteServer::start(RemoteState {
            db: state.db.clone(),
            remote_sessions: state.remote_sessions.clone(),
            pairing_token: state.pairing_token.clone(),
            app_handle: app.clone(),
        });
    });
}

#[tauri::command]
pub fn remote_connection_get(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<RemoteConnectionResponse, String> {
    ensure_remote_server(&app, &state);

    let token = state
        .db
        .lock()
        .create_pairing_token()
        .map_err(|e| e.to_string())?;

    *state.pairing_token.lock() = token.clone();

    let RemoteConnectionInfo { url, port, .. } = RemoteServer::connection_info(&token);
    let ip = local_ip_address::local_ip()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    Ok(RemoteConnectionResponse {
        ip,
        url: url.clone(),
        front_url: url,
        port,
        success: true,
    })
}
