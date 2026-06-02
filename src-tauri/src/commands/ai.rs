use serde::Serialize;
use tauri::State;

use crate::ai::agent::Agent;
use crate::ai::store::{AiConversationSummary, AiMessageRow};
use crate::ai::config::{default_model_path, MODEL_DISPLAY_NAME};
use crate::ai::hardware_profile;
use crate::ai::tools::{execute_pending, ToolResult};
use crate::ai::web_search::{self, WebSearchConfig};
use crate::ai::{ChatReply, LlamaServer};
use crate::AppState;

#[derive(Serialize)]
pub struct AiStatus {
    pub llama_bin: bool,
    pub model_present: bool,
    pub model_name: String,
    pub model_path: String,
    pub server_healthy: bool,
    pub gpu_enabled: bool,
    pub backend: String,
    pub gpu_layers: u32,
    pub ctx_size: u32,
    pub threads: u32,
    pub profiled: bool,
    pub profile_summary: String,
    pub offline_only: bool,
    pub web_search_enabled: bool,
    pub experience_entries: i64,
}

#[derive(serde::Deserialize)]
pub struct AiChatRequest {
    pub message: String,
    pub conversation_id: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct AiDashboardTransitionRequest {
    pub user_message: String,
    pub entity_key: String,
}

#[derive(serde::Deserialize)]
pub struct AiConfirmRequest {
    pub pending_id: String,
}

#[derive(serde::Deserialize)]
pub struct AiDismissRequest {
    pub pending_id: String,
}

#[derive(serde::Deserialize)]
pub struct AiProfileRequest {
    pub force: Option<bool>,
}

#[tauri::command]
pub fn ai_status(state: State<'_, AppState>) -> Result<AiStatus, String> {
    let db = state.db.lock();
    let (profiled, profile_summary) = hardware_profile::profile_summary(&db)?;
    let (backend, gpu_layers, ctx_size, threads) = LlamaServer::runtime_info(Some(&db));
    let healthy =
        LlamaServer::model_ready() && LlamaServer::bin_ready() && LlamaServer::health_ok();
    let web_cfg = web_search::load_config(&db.data_dir);
    Ok(AiStatus {
        llama_bin: LlamaServer::bin_ready(),
        model_present: LlamaServer::model_ready(),
        model_name: MODEL_DISPLAY_NAME.to_string(),
        model_path: default_model_path().to_string_lossy().to_string(),
        server_healthy: healthy,
        gpu_enabled: LlamaServer::using_gpu(Some(&db)),
        backend,
        gpu_layers,
        ctx_size,
        threads,
        profiled,
        profile_summary,
        offline_only: !web_cfg.enabled,
        web_search_enabled: web_cfg.enabled,
        experience_entries: db.ai_experience_count().unwrap_or(0),
    })
}

#[derive(serde::Deserialize)]
pub struct AiWebSearchConfigPayload {
    pub enabled: bool,
}

#[derive(Serialize)]
pub struct AiWebSearchConfigResponse {
    pub enabled: bool,
}

#[tauri::command]
pub fn ai_web_search_get_config(state: State<'_, AppState>) -> Result<AiWebSearchConfigResponse, String> {
    state.desktop_sessions.require_privilege("ai:utiliser")?;
    let db = state.db.lock();
    let cfg = web_search::load_config(&db.data_dir);
    Ok(AiWebSearchConfigResponse {
        enabled: cfg.enabled,
    })
}

#[tauri::command]
pub fn ai_web_search_set_config(
    state: State<'_, AppState>,
    payload: AiWebSearchConfigPayload,
) -> Result<AiWebSearchConfigResponse, String> {
    state.desktop_sessions.require_privilege("ai:utiliser")?;
    let db = state.db.lock();
    let cfg = WebSearchConfig {
        enabled: payload.enabled,
    };
    web_search::save_config(&db.data_dir, &cfg)?;
    Ok(AiWebSearchConfigResponse {
        enabled: cfg.enabled,
    })
}

#[tauri::command]
pub fn ai_profile_runtime(
    state: State<'_, AppState>,
    payload: Option<AiProfileRequest>,
) -> Result<String, String> {
    state.desktop_sessions.require_privilege("ai:utiliser")?;
    let force = payload.and_then(|p| p.force).unwrap_or(false);
    let db = state.db.lock();
    if !LlamaServer::model_ready() {
        return Err(
            "Modele absent. Copiez le fichier GGUF dans le dossier d'installation (aucune connexion Internet requise)."
                .into(),
        );
    }
    LlamaServer::stop();
    hardware_profile::invalidate_cache();
    let rt = LlamaServer::prepare(&db, force)?;
    let label = crate::ai::config::backend_label(rt.backend);
    Ok(format!(
        "Loggy configure pour {label} ({} calques GPU, {} threads, contexte {}).",
        rt.gpu_layers, rt.threads, rt.ctx_size
    ))
}

#[tauri::command]
pub fn ai_reindex(state: State<'_, AppState>) -> Result<usize, String> {
    state.desktop_sessions.require_privilege("ai:utiliser")?;
    let db = state.db.lock();
    Agent::new(&db).reindex()
}

/// Questions pratiques depuis le tableau de bord (réponses rapides, Internet, LLM léger).
#[tauri::command]
pub fn ai_dashboard_answer(
    state: State<'_, AppState>,
    payload: AiChatRequest,
) -> Result<ChatReply, String> {
    let session = state.desktop_sessions.require_session()?;
    let msg = payload.message.trim();
    if msg.len() < 2 {
        return Err("Message trop court.".into());
    }
    let db = state.db.lock();
    crate::ai::dashboard_chat::answer_practical(
        &db,
        &session.user,
        payload.conversation_id.as_deref(),
        msg,
    )
}

#[tauri::command]
pub fn ai_chat(state: State<'_, AppState>, payload: AiChatRequest) -> Result<ChatReply, String> {
    let session = state.desktop_sessions.require_privilege("ai:utiliser")?;
    let msg = payload.message.trim();
    if msg.len() < 2 {
        return Err("Message trop court.".into());
    }
    let db = state.db.lock();
    if !LlamaServer::model_ready() {
        return Err(
            "Loggy necessite le modele local. Copiez-le dans le dossier indique (hors ligne)."
                .into(),
        );
    }
    let (profiled, _) = hardware_profile::profile_summary(&db)?;
    if !profiled {
        LlamaServer::prepare(&db, false)?;
    }
    Agent::new(&db).chat(
        &session.user,
        payload.conversation_id.as_deref(),
        msg,
    )
}

#[tauri::command]
pub fn ai_dashboard_transition(
    state: State<'_, AppState>,
    payload: AiDashboardTransitionRequest,
) -> Result<String, String> {
    state.desktop_sessions.require_session()?;
    let key = payload.entity_key.trim();
    if key.is_empty() {
        return Err("Entité cible manquante.".into());
    }
    let db = state.db.lock();
    let data_dir = db.data_dir.clone();
    let registry = crate::entity::registry::load(&data_dir)?;
    let ent = registry
        .find(key)
        .ok_or_else(|| format!("Entité « {key} » introuvable."))?;
    let label = ent.label.as_deref().unwrap_or(&ent.nom);
    crate::ai::dashboard_transition::generate_transition_phrase(
        &db,
        &payload.user_message,
        key,
        label,
    )
}

#[tauri::command]
pub fn ai_confirm_action(
    state: State<'_, AppState>,
    payload: AiConfirmRequest,
) -> Result<ToolResult, String> {
    let session = state.desktop_sessions.require_privilege("ai:utiliser")?;
    let db = state.db.lock();
    execute_pending(&db, &payload.pending_id, &session.user.privileges)
}

#[tauri::command]
pub fn ai_dismiss_action(
    state: State<'_, AppState>,
    payload: AiDismissRequest,
) -> Result<(), String> {
    state.desktop_sessions.require_privilege("ai:utiliser")?;
    let db = state.db.lock();
    db.ai_delete_pending(&payload.pending_id)
        .map_err(|e| e.to_string())
}

#[derive(serde::Deserialize)]
pub struct AiConversationIdPayload {
    pub conversation_id: String,
}

#[tauri::command]
pub fn ai_list_conversations(state: State<'_, AppState>) -> Result<Vec<AiConversationSummary>, String> {
    let session = state.desktop_sessions.require_privilege("ai:utiliser")?;
    let db = state.db.lock();
    db.ai_list_conversations(&session.user.id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn ai_conversation_messages(
    state: State<'_, AppState>,
    payload: AiConversationIdPayload,
) -> Result<Vec<AiMessageRow>, String> {
    let session = state.desktop_sessions.require_privilege("ai:utiliser")?;
    let db = state.db.lock();
    if !db
        .ai_conversation_owned_by(&payload.conversation_id, &session.user.id)
        .map_err(|e| e.to_string())?
    {
        return Err("Conversation introuvable.".into());
    }
    db.ai_list_conversation_messages(&payload.conversation_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn ai_delete_conversation(
    state: State<'_, AppState>,
    payload: AiConversationIdPayload,
) -> Result<(), String> {
    let session = state.desktop_sessions.require_privilege("ai:utiliser")?;
    let db = state.db.lock();
    let ok = db
        .ai_delete_conversation(&session.user.id, &payload.conversation_id)
        .map_err(|e| e.to_string())?;
    if !ok {
        return Err("Conversation introuvable ou déjà supprimée.".into());
    }
    Ok(())
}

#[tauri::command]
pub fn ai_stop_server(_state: State<'_, AppState>) -> Result<(), String> {
    LlamaServer::stop();
    Ok(())
}

/// Démarre llama-server (chargement du modèle — peut prendre 30 s à 2 min).
#[tauri::command]
pub fn ai_start_server(state: State<'_, AppState>) -> Result<String, String> {
    state.desktop_sessions.require_privilege("ai:utiliser")?;
    let db = state.db.lock();
    if !LlamaServer::bin_ready() {
        return Err(
            "Binaire llama-server introuvable. Vérifiez le dossier llama-b8184-bin-win-cpu-x64 à la racine du projet."
                .into(),
        );
    }
    if !LlamaServer::model_ready() {
        return Err(format!(
            "Modèle GGUF absent : {}. Lancez npm run llm:install ou copiez le fichier manuellement.",
            default_model_path().display()
        ));
    }
    LlamaServer::ensure_started(Some(&db))?;
    Ok("Serveur IA démarré et prêt.".into())
}
