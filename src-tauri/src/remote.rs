use axum::{
    body::Bytes,
    extract::{Multipart, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tower_http::{cors::CorsLayer, limit::RequestBodyLimitLayer};
use uuid::Uuid;

use crate::db::Database;
use crate::session::{ActiveSession, RemoteSessionStore, SessionUser};

#[derive(Clone)]
pub struct RemoteState {
    pub db: Arc<Mutex<Database>>,
    pub remote_sessions: Arc<RemoteSessionStore>,
    pub pairing_token: Arc<Mutex<String>>,
    pub app_handle: AppHandle,
}

#[derive(Debug, Serialize, Clone)]
pub struct RemoteConnectionInfo {
    pub url: String,
    pub pairing_token: String,
    pub port: u16,
}

pub struct RemoteServer;

impl RemoteServer {
    pub const PORT: u16 = 38472;

    pub fn start(state: RemoteState) {
        let port = Self::PORT;
        let router = build_router(state);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("runtime tokio");
            rt.block_on(async {
                let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
                    .await
                    .expect("bind serveur distant");
                axum::serve(listener, router)
                    .await
                    .expect("serveur distant");
            });
        });
    }

    pub fn connection_info(pairing_token: &str) -> RemoteConnectionInfo {
        let ip = local_ip_address::local_ip()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| "127.0.0.1".to_string());
        let url = format!("http://{ip}:{}/pair?token={pairing_token}", Self::PORT);
        RemoteConnectionInfo {
            url,
            pairing_token: pairing_token.to_string(),
            port: Self::PORT,
        }
    }
}

fn build_router(state: RemoteState) -> Router {
    Router::new()
        .route("/pair", get(pair_page))
        .route("/api/remote/login", post(remote_login))
        .route("/api/remote/hangars", get(remote_hangars))
        .route("/api/remote/upload", post(remote_upload))
        .route("/api/health", get(health))
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn health(State(state): State<RemoteState>) -> impl IntoResponse {
    let data_dir = state.db.lock().data_dir.clone();
    let app = crate::entity::branding::ecosystem_name(&data_dir);
    Json(serde_json::json!({ "status": "ok", "app": app }))
}

#[derive(Deserialize)]
struct PairQuery {
    token: String,
}

async fn pair_page(
    State(state): State<RemoteState>,
    Query(query): Query<PairQuery>,
) -> impl IntoResponse {
    let valid = state
        .db
        .lock()
        .validate_pairing_token(&query.token)
        .unwrap_or(false);

    if !valid {
        let app = crate::entity::branding::ecosystem_name(&state.db.lock().data_dir);
        return Html(format!(
            r#"<!DOCTYPE html><html lang="fr"><body style="font-family:sans-serif;background:#0a0a0a;color:#fff;padding:2rem"><h1>Jeton invalide ou expiré</h1><p>Regénérez le QR code depuis {app} (validité 15 min).</p></body></html>"#
        ))
        .into_response();
    }

    let app = crate::entity::branding::ecosystem_name(&state.db.lock().data_dir);
    Html(mobile_portal_html(&query.token, &app)).into_response()
}

#[derive(Deserialize)]
struct RemoteLoginBody {
    email: String,
    password: String,
    pairing_token: String,
}

#[derive(Serialize)]
struct RemoteLoginResponse {
    success: bool,
    message: String,
    token: Option<String>,
}

async fn remote_login(
    State(state): State<RemoteState>,
    Json(body): Json<RemoteLoginBody>,
) -> impl IntoResponse {
    let db = state.db.lock();

    if !db
        .validate_pairing_token(&body.pairing_token)
        .unwrap_or(false)
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(RemoteLoginResponse {
                success: false,
                message: "Jeton de couplage invalide ou expiré (15 min)".to_string(),
                token: None,
            }),
        );
    }

    match db.authenticate(&body.email, &body.password) {
        Ok((id, nom, role_nom, _role_id, privileges, _must_change)) => {
            if !crate::privileges::has_privilege(&privileges, "hangars:voir") {
                return (
                    StatusCode::FORBIDDEN,
                    Json(RemoteLoginResponse {
                        success: false,
                        message: "Privilège hangars:voir requis pour la connexion mobile".to_string(),
                        token: None,
                    }),
                );
            }
            drop(db);

            let token = Uuid::new_v4().to_string();
            let user = SessionUser {
                id,
                nom: nom.clone(),
                email: body.email.clone(),
                role: role_nom,
                privileges,
                must_change_password: false,
            };
            state.remote_sessions.insert(ActiveSession {
                token: token.clone(),
                user,
            });

            let _ = state.db.lock().mark_pairing_used(&body.pairing_token);

            let _ = state.app_handle.emit(
                "remote-connected",
                serde_json::json!({
                    "nom": nom,
                    "email": body.email,
                    "source": "mobile"
                }),
            );

            (
                StatusCode::OK,
                Json(RemoteLoginResponse {
                    success: true,
                    message: "Connecté — vous pouvez envoyer des photos de hangar".to_string(),
                    token: Some(token),
                }),
            )
        }
        Err(_) => (
            StatusCode::UNAUTHORIZED,
            Json(RemoteLoginResponse {
                success: false,
                message: "Identifiants invalides".to_string(),
                token: None,
            }),
        ),
    }
}

fn extract_bearer(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

async fn remote_hangars(
    State(state): State<RemoteState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = match extract_bearer(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Token requis" })),
            )
                .into_response();
        }
    };

    if state
        .remote_sessions
        .require_privilege(&token, "hangars:voir")
        .is_err()
    {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "Accès refusé" })),
        )
            .into_response();
    }

    let hangars = state.db.lock().list_hangars().unwrap_or_default();
    Json(hangars).into_response()
}

async fn remote_upload(
    State(state): State<RemoteState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let token = match extract_bearer(&headers) {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "success": false, "message": "Token requis" })),
            );
        }
    };

    if let Err(e) = state
        .remote_sessions
        .require_privilege(&token, "hangars:modifier")
    {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "success": false, "message": e })),
        );
    }

    let mut hangar_id: Option<String> = None;
    let mut file_bytes: Option<Bytes> = None;
    let mut content_type = "image/jpeg".to_string();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "hangar_id" {
            hangar_id = field.text().await.ok();
        } else if name == "photo" {
            content_type = field
                .content_type()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "image/jpeg".to_string());
            file_bytes = field.bytes().await.ok();
        }
    }

    let hangar_id = match hangar_id {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "success": false, "message": "hangar_id requis" })),
            );
        }
    };

    let bytes = match file_bytes {
        Some(b) if !b.is_empty() => b,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "success": false, "message": "photo requise" })),
            );
        }
    };

    let db = state.db.lock();
    if !db.hangar_exists(&hangar_id).unwrap_or(false) {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "success": false, "message": "Hangar introuvable" })),
        );
    }

    let original_name = if content_type.contains("png") {
        format!("mobile-{}.png", Uuid::new_v4())
    } else {
        format!("mobile-{}.jpg", Uuid::new_v4())
    };
    drop(db);

    let photo = match state.db.lock().import_document(
        &original_name,
        &bytes,
        "bien",
        Some(&hangar_id),
        "Photo terrain mobile",
        "mobile",
    ) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "success": false, "message": e })),
            );
        }
    };

    let hangar_ref: String = state
        .db
        .lock()
        .list_hangars()
        .ok()
        .and_then(|list| {
            list.into_iter()
                .find(|h| h.id == hangar_id)
                .map(|h| h.reference)
        })
        .unwrap_or_else(|| hangar_id.clone());

    let _ = state.app_handle.emit(
        "hangar-photo-uploaded",
        serde_json::json!({
            "hangar_id": hangar_id,
            "hangar_reference": hangar_ref,
            "photo_id": photo.id,
            "filename": photo.original_name,
            "source": "mobile"
        }),
    );

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "message": format!("Photo envoyée au desktop {}", crate::entity::branding::ecosystem_name(&state.db.lock().data_dir)),
            "photo_id": photo.id,
            "filename": photo.original_name
        })),
    )
}

fn mobile_portal_html(pairing_token: &str, app_name: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="fr">
<head>
  <meta charset="UTF-8"/>
  <meta name="viewport" content="width=device-width, initial-scale=1"/>
  <title>{app_name} Mobile</title>
  <style>
    * {{ box-sizing: border-box; }}
    body {{ font-family: system-ui, sans-serif; background: #0a0a0a; color: #fafafa; margin: 0; padding: 1rem; }}
    .card {{ background: #171717; border: 1px solid #262626; border-radius: 1rem; padding: 1.5rem; max-width: 420px; margin: 0 auto 1rem; }}
    h1 {{ font-size: 1.2rem; color: #06b6d4; margin: 0 0 0.5rem; }}
    p {{ color: #a3a3a3; font-size: 0.875rem; }}
    label {{ display: block; font-size: 0.75rem; color: #a3a3a3; margin: 0.5rem 0 0.25rem; }}
    input, select {{ width: 100%; padding: 0.75rem; border-radius: 0.5rem; border: 1px solid #404040; background: #0a0a0a; color: #fff; margin-bottom: 0.5rem; }}
    button {{ width: 100%; padding: 0.875rem; border: none; border-radius: 0.5rem; background: linear-gradient(135deg, #dc2626, #2563eb); color: #fff; font-weight: 600; margin-top: 0.5rem; }}
    #msg {{ margin-top: 0.75rem; font-size: 0.875rem; text-align: center; }}
    .ok {{ color: #22c55e; }} .err {{ color: #ef4444; }}
    #uploadSection {{ display: none; }}
    .preview {{ max-width: 100%; border-radius: 0.5rem; margin-top: 0.5rem; }}
  </style>
</head>
<body>
  <div class="card" id="loginSection">
    <h1>{app_name} Mobile</h1>
    <p>Connectez-vous pour envoyer des photos de hangar au desktop.</p>
    <form id="loginForm">
      <label>E-mail</label>
      <input id="email" type="email" required/>
      <label>Mot de passe</label>
      <input id="password" type="password" required/>
      <button type="submit">Se connecter</button>
    </form>
    <div id="msg"></div>
  </div>
  <div class="card" id="uploadSection">
    <h1>Envoyer une photo</h1>
    <label>Hangar</label>
    <select id="hangarSelect"></select>
    <label>Photo (appareil photo)</label>
    <input id="photoInput" type="file" accept="image/*" capture="environment"/>
    <img id="preview" class="preview" style="display:none"/>
    <button type="button" id="uploadBtn">Envoyer au desktop</button>
    <div id="uploadMsg"></div>
  </div>
  <script>
    const pairingToken = "{pairing_token}";
    let sessionToken = localStorage.getItem('blin_remote_token');

    if (sessionToken) {{
      showUpload();
      loadHangars();
    }}

    document.getElementById('loginForm').addEventListener('submit', async (e) => {{
      e.preventDefault();
      const msg = document.getElementById('msg');
      msg.textContent = 'Connexion…';
      try {{
        const res = await fetch('/api/remote/login', {{
          method: 'POST',
          headers: {{ 'Content-Type': 'application/json' }},
          body: JSON.stringify({{
            email: document.getElementById('email').value,
            password: document.getElementById('password').value,
            pairing_token: pairingToken
          }})
        }});
        const data = await res.json();
        if (data.success && data.token) {{
          sessionToken = data.token;
          localStorage.setItem('blin_remote_token', sessionToken);
          msg.className = 'ok';
          msg.textContent = data.message;
          showUpload();
          loadHangars();
        }} else {{
          msg.className = 'err';
          msg.textContent = data.message;
        }}
      }} catch {{
        msg.className = 'err';
        msg.textContent = 'Erreur réseau';
      }}
    }});

    function showUpload() {{
      document.getElementById('loginSection').style.display = 'none';
      document.getElementById('uploadSection').style.display = 'block';
    }}

    async function loadHangars() {{
      const res = await fetch('/api/remote/hangars', {{
        headers: {{ Authorization: 'Bearer ' + sessionToken }}
      }});
      const hangars = await res.json();
      const sel = document.getElementById('hangarSelect');
      sel.innerHTML = hangars.map(h => `<option value="${{h.id}}">${{h.reference}} (${{h.zone}})</option>`).join('');
    }}

    document.getElementById('photoInput').addEventListener('change', (e) => {{
      const file = e.target.files[0];
      if (!file) return;
      const img = document.getElementById('preview');
      img.src = URL.createObjectURL(file);
      img.style.display = 'block';
    }});

    document.getElementById('uploadBtn').addEventListener('click', async () => {{
      const file = document.getElementById('photoInput').files[0];
      const hangarId = document.getElementById('hangarSelect').value;
      const uploadMsg = document.getElementById('uploadMsg');
      if (!file) {{ uploadMsg.textContent = 'Choisissez une photo'; uploadMsg.className = 'err'; return; }}
      const fd = new FormData();
      fd.append('hangar_id', hangarId);
      fd.append('photo', file);
      uploadMsg.textContent = 'Envoi…';
      try {{
        const res = await fetch('/api/remote/upload', {{
          method: 'POST',
          headers: {{ Authorization: 'Bearer ' + sessionToken }},
          body: fd
        }});
        const data = await res.json();
        uploadMsg.textContent = data.message || (data.success ? 'OK' : 'Erreur');
        uploadMsg.className = data.success ? 'ok' : 'err';
      }} catch {{
        uploadMsg.className = 'err';
        uploadMsg.textContent = 'Erreur réseau';
      }}
    }});
  </script>
</body>
</html>"#,
        pairing_token = pairing_token,
        app_name = app_name,
    )
}
