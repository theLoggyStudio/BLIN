use axum::{
    body::Bytes,
    extract::{Multipart, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use std::collections::HashMap;
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
                let addr = format!("0.0.0.0:{port}");
                let listener = match tokio::net::TcpListener::bind(&addr).await {
                    Ok(l) => l,
                    Err(e) => {
                        eprintln!("Serveur distant mobile indisponible (port {port}) : {e}");
                        return;
                    }
                };
                if let Err(e) = axum::serve(listener, router).await {
                    eprintln!("Serveur distant mobile arrêté : {e}");
                }
            });
        });
    }

    pub fn connection_info(pairing_token: &str) -> RemoteConnectionInfo {
        let ip = crate::network_local::cached_local_ip();
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
        .route("/api/remote/me", get(remote_me))
        .route("/api/remote/entities", get(remote_entities))
        .route("/api/remote/match-intent", post(remote_match_intent))
        .route("/api/remote/entity/:entity_key/rows", get(remote_entity_rows))
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
                    message: "Connecté — ouverture du tableau de bord".to_string(),
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

async fn remote_me(
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
    match state.remote_sessions.require_session(&token) {
        Ok(session) => Json(serde_json::json!({
            "nom": session.user.nom,
            "email": session.user.email,
            "role": session.user.role,
        }))
        .into_response(),
        Err(e) => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": e })),
        )
            .into_response(),
    }
}

async fn remote_entities(
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
    let session = match state.remote_sessions.require_session(&token) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": e })),
            )
                .into_response();
        }
    };
    let db = state.db.lock();
    let registry = match crate::entity::registry::load(&db.data_dir) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e })),
            )
                .into_response();
        }
    };
    let items = crate::entity::suggestions::list_for_user(&db.data_dir, &registry, &session.user);
    Json(items).into_response()
}

#[derive(Deserialize)]
struct RemoteMatchBody {
    message: String,
}

async fn remote_match_intent(
    State(state): State<RemoteState>,
    headers: HeaderMap,
    Json(body): Json<RemoteMatchBody>,
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
    if state.remote_sessions.require_session(&token).is_err() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Session expirée" })),
        )
            .into_response();
    }
    let data_dir = state.db.lock().data_dir.clone();
    let entity_key = crate::entity::match_intent(&body.message, &data_dir);
    Json(serde_json::json!({ "entity_key": entity_key })).into_response()
}

#[derive(Serialize)]
struct RemoteEntityColumn {
    key: String,
    label: String,
}

#[derive(Serialize)]
struct RemoteEntityRowsResponse {
    entity_key: String,
    label: String,
    columns: Vec<RemoteEntityColumn>,
    rows: Vec<serde_json::Map<String, serde_json::Value>>,
}

async fn remote_entity_rows(
    State(state): State<RemoteState>,
    Path(entity_key): Path<String>,
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
    let priv_key = format!("{entity_key}:voir");
    let session = match state.remote_sessions.require_privilege(&token, &priv_key) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({ "error": e })),
            )
                .into_response();
        }
    };

    let db = state.db.lock();
    let data_dir = db.data_dir.clone();
    let registry = match crate::entity::registry::load(&data_dir) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e })),
            )
                .into_response();
        }
    };
    let cfg = match crate::entity::load_screen_config(&data_dir, &entity_key) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": e })),
            )
                .into_response();
        }
    };
    let label = registry
        .find(&entity_key)
        .and_then(|e| e.label.clone())
        .unwrap_or_else(|| entity_key.clone());

    let options = crate::dda::crud::ListRowsOptions {
        viewer_role_id: None,
        viewer_user_id: Some(&session.user.id),
        viewer_privileges: &session.user.privileges,
    };
    let pagination = crate::dda::crud::ListRowsPagination {
        offset: 0,
        limit: 80,
    };
    let rows = match crate::dda::crud::list_rows_with_options(
        &db,
        &cfg,
        &HashMap::new(),
        options,
        Some(pagination),
    ) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e })),
            )
                .into_response();
        }
    };

    let columns: Vec<RemoteEntityColumn> = cfg
        .list_columns()
        .into_iter()
        .take(6)
        .map(|f| RemoteEntityColumn {
            key: f.key.to_string(),
            label: f.label.to_string(),
        })
        .collect();

    Json(RemoteEntityRowsResponse {
        entity_key,
        label,
        columns,
        rows,
    })
    .into_response()
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
    .card {{ background: #171717; border: 1px solid #262626; border-radius: 1rem; padding: 1.25rem; max-width: 480px; margin: 0 auto 1rem; }}
    h1 {{ font-size: 1.15rem; background: linear-gradient(135deg, #dc2626, #2563eb); -webkit-background-clip: text; -webkit-text-fill-color: transparent; margin: 0 0 0.35rem; }}
    h2 {{ font-size: 1rem; color: #4DB6AC; margin: 0 0 0.75rem; }}
    p, .muted {{ color: #a3a3a3; font-size: 0.875rem; margin: 0 0 0.75rem; }}
    label {{ display: block; font-size: 0.75rem; color: #a3a3a3; margin: 0.5rem 0 0.25rem; }}
    input {{ width: 100%; padding: 0.75rem; border-radius: 0.5rem; border: 1px solid #404040; background: #0a0a0a; color: #fff; margin-bottom: 0.5rem; }}
    button, .pill {{ border: none; border-radius: 0.5rem; background: linear-gradient(135deg, #dc2626, #2563eb); color: #fff; font-weight: 600; cursor: pointer; }}
    button {{ width: 100%; padding: 0.875rem; margin-top: 0.35rem; }}
    .pill {{ display: inline-block; padding: 0.55rem 0.75rem; font-size: 0.8rem; margin: 0.25rem; }}
    .ghost {{ background: #262626; width: auto; padding: 0.45rem 0.75rem; font-size: 0.75rem; }}
    #msg {{ margin-top: 0.75rem; font-size: 0.875rem; text-align: center; }}
    .ok {{ color: #22c55e; }} .err {{ color: #ef4444; }}
    #dashboardSection {{ display: none; }}
    .row-card {{ background: #0f0f0f; border: 1px solid #333; border-radius: 0.65rem; padding: 0.65rem 0.75rem; margin-bottom: 0.5rem; font-size: 0.8rem; }}
    .row-card div {{ margin: 0.15rem 0; }}
    .row-label {{ color: #737373; font-size: 0.7rem; }}
    #entityView {{ display: none; margin-top: 0.75rem; }}
    .topbar {{ display: flex; justify-content: space-between; align-items: center; gap: 0.5rem; margin-bottom: 0.75rem; }}
  </style>
</head>
<body>
  <div class="card" id="loginSection">
    <h1>{app_name}</h1>
    <p>Connectez-vous pour accéder au tableau de bord mobile.</p>
    <form id="loginForm">
      <label>E-mail</label>
      <input id="email" type="email" required autocomplete="username"/>
      <label>Mot de passe</label>
      <input id="password" type="password" required autocomplete="current-password"/>
      <button type="submit">Se connecter</button>
    </form>
    <div id="msg"></div>
  </div>

  <div id="dashboardSection">
    <div class="card">
      <div class="topbar">
        <div>
          <h1 style="margin:0">Tableau de bord</h1>
          <p id="userLine" class="muted" style="margin:0.25rem 0 0"></p>
        </div>
        <button type="button" class="ghost" id="logoutBtn">Quitter</button>
      </div>
      <label>Que souhaitez-vous faire ?</label>
      <input id="commandInput" type="text" placeholder="Ex. gérer les clients"/>
      <button type="button" id="commandBtn">Rechercher</button>
      <p class="muted" style="margin-top:0.75rem">Entités accessibles :</p>
      <div id="entityPills"></div>
    </div>
    <div class="card" id="entityView">
      <div class="topbar">
        <h2 id="entityTitle">Entité</h2>
        <button type="button" class="ghost" id="backBtn">Retour</button>
      </div>
      <div id="entityRows"></div>
    </div>
  </div>

  <script>
    const pairingToken = "{pairing_token}";
    let sessionToken = localStorage.getItem('blin_remote_token');

    function authHeaders() {{
      return {{ Authorization: 'Bearer ' + sessionToken }};
    }}

    if (sessionToken) {{
      showDashboard();
    }}

    document.getElementById('loginForm').addEventListener('submit', async (e) => {{
      e.preventDefault();
      const msg = document.getElementById('msg');
      msg.textContent = 'Connexion…';
      msg.className = '';
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
          showDashboard();
        }} else {{
          msg.className = 'err';
          msg.textContent = data.message || 'Connexion impossible';
        }}
      }} catch {{
        msg.className = 'err';
        msg.textContent = 'Erreur réseau';
      }}
    }});

    document.getElementById('logoutBtn').addEventListener('click', () => {{
      localStorage.removeItem('blin_remote_token');
      sessionToken = null;
      document.getElementById('dashboardSection').style.display = 'none';
      document.getElementById('loginSection').style.display = 'block';
      document.getElementById('entityView').style.display = 'none';
    }});

    document.getElementById('backBtn').addEventListener('click', () => {{
      document.getElementById('entityView').style.display = 'none';
    }});

    document.getElementById('commandBtn').addEventListener('click', () => void runCommand());
    document.getElementById('commandInput').addEventListener('keydown', (e) => {{
      if (e.key === 'Enter') void runCommand();
    }});

    async function runCommand() {{
      const message = document.getElementById('commandInput').value.trim();
      if (!message) return;
      try {{
        const res = await fetch('/api/remote/match-intent', {{
          method: 'POST',
          headers: {{ ...authHeaders(), 'Content-Type': 'application/json' }},
          body: JSON.stringify({{ message }})
        }});
        const data = await res.json();
        if (data.entity_key) {{
          await openEntity(data.entity_key);
        }} else {{
          alert('Aucune entité reconnue pour cette commande.');
        }}
      }} catch {{
        alert('Erreur réseau');
      }}
    }}

    function showDashboard() {{
      document.getElementById('loginSection').style.display = 'none';
      document.getElementById('dashboardSection').style.display = 'block';
      void loadMe();
      void loadEntities();
    }}

    async function loadMe() {{
      try {{
        const res = await fetch('/api/remote/me', {{ headers: authHeaders() }});
        if (!res.ok) throw new Error('session');
        const me = await res.json();
        document.getElementById('userLine').textContent = me.nom + ' — ' + me.role;
      }} catch {{
        localStorage.removeItem('blin_remote_token');
        location.reload();
      }}
    }}

    async function loadEntities() {{
      const res = await fetch('/api/remote/entities', {{ headers: authHeaders() }});
      const entities = await res.json();
      const box = document.getElementById('entityPills');
      if (!Array.isArray(entities) || entities.length === 0) {{
        box.innerHTML = '<p class="muted">Aucune entité accessible avec votre compte.</p>';
        return;
      }}
      box.innerHTML = entities.map(e =>
        `<button type="button" class="pill" data-key="${{e.key}}">${{e.label || e.key}}</button>`
      ).join('');
      box.querySelectorAll('[data-key]').forEach(btn => {{
        btn.addEventListener('click', () => void openEntity(btn.getAttribute('data-key')));
      }});
    }}

    async function openEntity(key) {{
      const res = await fetch('/api/remote/entity/' + encodeURIComponent(key) + '/rows', {{
        headers: authHeaders()
      }});
      const data = await res.json();
      if (!res.ok) {{
        alert(data.error || 'Impossible de charger cette entité');
        return;
      }}
      document.getElementById('entityTitle').textContent = data.label || data.entity_key;
      const rowsBox = document.getElementById('entityRows');
      if (!data.rows || data.rows.length === 0) {{
        rowsBox.innerHTML = '<p class="muted">Aucun enregistrement.</p>';
      }} else {{
        rowsBox.innerHTML = data.rows.map(row => {{
          const parts = (data.columns || []).map(col => {{
            const val = row[col.key];
            const text = val === null || val === undefined ? '—' : String(val);
            return `<div><span class="row-label">${{col.label}}</span> ${{text}}</div>`;
          }}).join('');
          return `<div class="row-card">${{parts}}</div>`;
        }}).join('');
      }}
      document.getElementById('entityView').style.display = 'block';
      document.getElementById('entityView').scrollIntoView({{ behavior: 'smooth' }});
    }}
  </script>
</body>
</html>"#,
        pairing_token = pairing_token,
        app_name = app_name,
    )
}
