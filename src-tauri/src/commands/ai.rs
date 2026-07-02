use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};

use crate::ai::agent::Agent;
use crate::ai::store::{AiConversationSummary, AiMessageRow};
use crate::ai::config::{default_model_path, MODEL_DISPLAY_NAME};
use crate::ai::hardware_profile;
use crate::ai::runtime_config;
use crate::ai::runtime_install::{self, AiInstallProgress};
use crate::ai::tools::{execute_pending, ToolResult};
use crate::ai::web_search::{self, WebSearchConfig};
use crate::ai::{ChatReply, LlamaServer};
use crate::privileges::{has_privilege, require_privilege};
use crate::AppState;

#[derive(Serialize)]
pub struct AiStatus {
    pub llama_bin: bool,
    pub model_present: bool,
    pub model_name: String,
    pub model_path: String,
    pub install_dir: Option<String>,
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
    pub db_dir: String,
    pub db_path: String,
    pub db_paths: Vec<String>,
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
pub struct AiEntityAccessDeniedRequest {
    pub user_message: String,
    pub entity_key: String,
    #[serde(default)]
    pub entity_label: Option<String>,
    #[serde(default)]
    pub contact_role_names: Vec<String>,
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
    state
        .desktop_sessions
        .require_privilege("parametres:assistant")?;
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
        install_dir: runtime_config::configured_install_dir(&db.data_dir)
            .map(|p| p.to_string_lossy().to_string()),
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
        db_dir: db.data_dir.to_string_lossy().to_string(),
        db_path: db.main_db_path().to_string_lossy().to_string(),
        db_paths: db.list_sqlite_file_paths(),
    })
}

#[derive(Serialize)]
pub struct AiRuntimeStatus {
    pub ready: bool,
    pub configured: bool,
    pub install_dir: Option<String>,
    pub default_install_dir: String,
    pub model_path: String,
    pub llama_bin: bool,
    pub model_present: bool,
}

#[derive(serde::Deserialize)]
pub struct AiRuntimeInstallPayload {
    pub install_dir: String,
}

/// Statut installation Loggy (accessible avant connexion).
#[tauri::command]
pub fn ai_runtime_status(state: State<'_, AppState>) -> Result<AiRuntimeStatus, String> {
    let db = state.db.lock();
    runtime_config::refresh_from_data_dir(&db.data_dir);
    Ok(AiRuntimeStatus {
        ready: runtime_config::runtime_ready(),
        configured: runtime_config::configured_install_dir(&db.data_dir).is_some(),
        install_dir: runtime_config::configured_install_dir(&db.data_dir)
            .map(|p| p.to_string_lossy().to_string()),
        default_install_dir: runtime_config::default_install_dir().to_string_lossy().to_string(),
        model_path: default_model_path().to_string_lossy().to_string(),
        llama_bin: LlamaServer::bin_ready(),
        model_present: LlamaServer::model_ready(),
    })
}

/// Télécharge llama-server + modèle GGUF dans le dossier choisi (événements Tauri).
#[tauri::command]
pub fn ai_runtime_install(
    app: AppHandle,
    state: State<'_, AppState>,
    payload: AiRuntimeInstallPayload,
) -> Result<(), String> {
    let install_dir = PathBuf::from(payload.install_dir.trim());
    if install_dir.as_os_str().is_empty() {
        return Err("Choisissez un dossier d'installation.".into());
    }
    let db_arc = state.db.clone();
    std::thread::spawn(move || {
        let db = db_arc.lock();
        let app_handle = app.clone();
        let progress = Box::new(move |p: AiInstallProgress| {
            let _ = app_handle.emit("ai-install-progress", p);
        });
        match runtime_install::install_to(&db, &install_dir, progress) {
            Ok(()) => {
                let _ = app.emit("ai-install-done", ());
            }
            Err(e) => {
                let _ = app.emit("ai-install-error", e);
            }
        }
    });
    Ok(())
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
    state
        .desktop_sessions
        .require_privilege("parametres:assistant")?;
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
    state
        .desktop_sessions
        .require_privilege("parametres:assistant")?;
    let db = state.db.lock();
    let cfg = WebSearchConfig {
        enabled: payload.enabled,
    };
    web_search::save_config(&db.data_dir, &cfg)?;
    Ok(AiWebSearchConfigResponse {
        enabled: cfg.enabled,
    })
}

#[derive(serde::Deserialize)]
pub struct AiVisionConfigPayload {
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    /// Rétrocompatibilité.
    #[serde(default)]
    pub gemini_api_key: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

#[tauri::command]
pub fn ai_vision_get_config(
    state: State<'_, AppState>,
) -> Result<crate::ai::vision::VisionConfigPublic, String> {
    state
        .desktop_sessions
        .require_privilege("parametres:assistant")?;
    let db = state.db.lock();
    Ok(crate::ai::vision::public_config(&db.data_dir))
}

#[tauri::command]
pub fn ai_vision_set_config(
    state: State<'_, AppState>,
    payload: AiVisionConfigPayload,
) -> Result<crate::ai::vision::VisionConfigPublic, String> {
    state
        .desktop_sessions
        .require_privilege("parametres:assistant")?;
    let db = state.db.lock();
    let mut cfg = crate::ai::vision::load_config(&db.data_dir);
    let key = payload
        .api_key
        .as_deref()
        .or(payload.gemini_api_key.as_deref());
    crate::ai::vision::apply_config_patch(
        &mut cfg,
        payload.provider.as_deref(),
        key,
        payload.model.as_deref(),
    );
    crate::ai::vision::save_config(&db.data_dir, &cfg)?;
    Ok(crate::ai::vision::public_config(&db.data_dir))
}

#[derive(serde::Deserialize, Default)]
pub struct AiVisionAttributeHint {
    pub nom: String,
    #[serde(default)]
    pub required: bool,
}

#[derive(serde::Deserialize, Default)]
pub struct AiVisionEntityOptions {
    #[serde(default)]
    pub requires_signature: bool,
    #[serde(default)]
    pub ai_suggestions: bool,
    #[serde(default)]
    pub signatory_role_ids: Vec<String>,
    #[serde(default)]
    pub attribute_hints: Vec<AiVisionAttributeHint>,
}

#[derive(serde::Deserialize)]
pub struct AiVisionAnalyzeRequest {
    pub message: String,
    pub image_base64: String,
    pub conversation_id: Option<String>,
    #[serde(default)]
    pub entity_options: Option<AiVisionEntityOptions>,
}

#[tauri::command]
pub fn ai_vision_analyze(
    state: State<'_, AppState>,
    payload: AiVisionAnalyzeRequest,
) -> Result<ChatReply, String> {
    use uuid::Uuid;

    let session = state.desktop_sessions.require_session()?;
    if payload.image_base64.trim().is_empty() {
        return Err("Image manquante.".into());
    }
    let db = state.db.lock();
    let conv_id = payload
        .conversation_id
        .as_deref()
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    if payload.conversation_id.is_none() {
        let title = "Analyse image".to_string();
        db.ai_create_conversation(&conv_id, &session.user.id, &title)
            .map_err(|e| e.to_string())?;
    }
    let user_line = if payload.message.trim().is_empty() {
        "[Image jointe — analyse vision]".to_string()
    } else {
        format!("{} [Image jointe]", payload.message.trim())
    };
    db.ai_add_message(&conv_id, "user", &user_line)
        .map_err(|e| e.to_string())?;

    let vision_opts = payload.entity_options.map(|o| crate::ai::vision::VisionEntityOptions {
        requires_signature: o.requires_signature,
        ai_suggestions: o.ai_suggestions,
        signatory_role_ids: o.signatory_role_ids,
        attribute_hints: o
            .attribute_hints
            .into_iter()
            .map(|h| crate::ai::vision::VisionAttributeHint {
                nom: h.nom,
                required: h.required,
            })
            .collect(),
    });

    let msg = crate::ai::vision::analyze_image(
        &db.data_dir,
        &payload.message,
        &payload.image_base64,
        vision_opts,
    )?;
    db.ai_add_message(&conv_id, "assistant", &msg)
        .map_err(|e| e.to_string())?;
    Ok(ChatReply {
        conversation_id: conv_id,
        message: msg,
        tool_results: vec![],
        display_blocks: vec![],
        cols_request: None,
        open_entity_create: None,
        open_registry_entity_create: None,
    })
}

#[tauri::command]
pub fn ai_profile_runtime(
    state: State<'_, AppState>,
    payload: Option<AiProfileRequest>,
) -> Result<String, String> {
    let session = state.desktop_sessions.require_session()?;
    let force = payload.and_then(|p| p.force).unwrap_or(false);
    if force {
        if !has_privilege(&session.user.privileges, "ai:utiliser")
            && !has_privilege(&session.user.privileges, "parametres:assistant")
        {
            return Err("Privilège requis : ai:utiliser ou parametres:assistant".into());
        }
    } else {
        require_privilege(&session.user.privileges, "parametres:assistant")?;
    }
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
    state
        .desktop_sessions
        .require_privilege("parametres:assistant")?;
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
pub fn ai_entity_access_denied(
    state: State<'_, AppState>,
    payload: AiEntityAccessDeniedRequest,
) -> Result<String, String> {
    state.desktop_sessions.require_session()?;
    let key = payload.entity_key.trim();
    if key.is_empty() {
        return Err("Entité cible manquante.".into());
    }
    let db = state.db.lock();
    let data_dir = db.data_dir.clone();
    let registry = crate::entity::registry::load(&data_dir)?;
    let ent = registry.find(key);
    let label = payload
        .entity_label
        .filter(|s| !s.trim().is_empty())
        .or_else(|| ent.and_then(|e| e.label.clone()))
        .unwrap_or_else(|| key.to_string());
    let contact_roles = if payload.contact_role_names.is_empty() {
        db.list_role_names_with_entity_access(key)
            .map_err(|e| e.to_string())?
    } else {
        payload.contact_role_names
    };
    crate::ai::access_denied::generate_access_denied_phrase(
        &db,
        &payload.user_message,
        key,
        &label,
        &contact_roles,
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

#[derive(serde::Deserialize)]
pub struct AiAlertPersonifyPayload {
    pub message: String,
    #[serde(default = "default_alert_variant")]
    pub variant: String,
}

fn default_alert_variant() -> String {
    "info".into()
}

/// Réécrit une notification à la première personne (Loggy).
///
/// Ne bloque plus sur le LLM : on sert une réponse pré-générée depuis la réserve
/// (`alert_pool`) si disponible, sinon on renvoie le message brut (l'UI applique un
/// repli local instantané) et Loggy regénère la réserve en arrière-plan.
#[tauri::command]
pub fn ai_alert_personify(
    state: State<'_, AppState>,
    payload: AiAlertPersonifyPayload,
) -> Result<String, String> {
    state.desktop_sessions.require_session()?;
    let raw = payload.message.trim().to_string();
    let variant = payload.variant.clone();
    if raw.is_empty() || raw.chars().count() > 800 {
        return Ok(raw);
    }

    if let Some(body) = state.alert_pool.take_personified(&raw, &variant) {
        crate::ai::alert_pool::spawn_refill(
            state.alert_pool.clone(),
            state.db.clone(),
            raw,
            variant,
        );
        return Ok(body);
    }

    crate::ai::alert_pool::spawn_refill(
        state.alert_pool.clone(),
        state.db.clone(),
        raw.clone(),
        variant,
    );
    Ok(raw)
}

#[derive(serde::Deserialize)]
pub struct AiTaskReminderPersonifyPayload {
    pub message: String,
}

/// Réécrit un rappel de tâche planifiée (Loggy, persistant).
#[tauri::command]
pub fn ai_task_reminder_personify(
    state: State<'_, AppState>,
    payload: AiTaskReminderPersonifyPayload,
) -> Result<String, String> {
    state.desktop_sessions.require_session()?;
    let db = state.db.lock();
    Ok(crate::ai::alert_personify::personify_task_reminder(
        &db,
        &payload.message,
    ))
}

/// Commentaire Loggy sur un graphique — thread dédié, sans bloquer entity_stats.
#[tauri::command]
pub async fn ai_stats_interpret(
    state: State<'_, AppState>,
    payload: crate::ai::stats_interpret::StatsInterpretPayload,
) -> Result<String, String> {
    state.desktop_sessions.require_session()?;

    if payload.series.is_empty()
        || payload.series.iter().all(|s| s.points.is_empty())
    {
        return Ok("Je n'ai pas encore de données à commenter sur ce graphique.".into());
    }

    let fallback = crate::ai::stats_interpret::fallback_interpretation(&payload);
    if !crate::ai::llama_server::LlamaServer::model_ready() {
        return Ok(fallback);
    }

    let db_arc = state.db.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let app_name = {
            let db = db_arc.lock();
            crate::entity::branding::ecosystem_name(&db.data_dir)
        };

        let needs_prepare = {
            let db = db_arc.lock();
            crate::ai::hardware_profile::profile_summary(&db)
                .map(|(profiled, _)| !profiled)
                .unwrap_or(false)
        };
        if needs_prepare {
            let db = db_arc.lock();
            let _ = crate::ai::llama_server::LlamaServer::prepare(&db, false);
        }

        Ok(crate::ai::stats_interpret::interpret_stats_with_llm(
            &payload,
            &fallback,
            &app_name,
        ))
    })
    .await
    .map_err(|e| format!("Analyse Loggy interrompue : {e}"))?
}

/// Questions de suivi sur un graphique — contexte courbe uniquement.
#[tauri::command]
pub async fn ai_stats_chat(
    state: State<'_, AppState>,
    payload: crate::ai::stats_interpret::StatsChatPayload,
) -> Result<String, String> {
    state.desktop_sessions.require_session()?;

    if payload.chart.series.is_empty()
        || payload.chart.series.iter().all(|s| s.points.is_empty())
    {
        return Err("Aucune donnée de courbe à discuter.".into());
    }

    let message = payload.message.trim().to_string();
    if message.is_empty() {
        return Err("Message vide.".into());
    }

    let db_arc = state.db.clone();
    let chart = payload.chart;
    let initial_analysis = payload.initial_analysis;
    let history = payload.history;

    tauri::async_runtime::spawn_blocking(move || {
        let app_name = {
            let db = db_arc.lock();
            crate::entity::branding::ecosystem_name(&db.data_dir)
        };

        if crate::ai::llama_server::LlamaServer::model_ready() {
            let needs_prepare = {
                let db = db_arc.lock();
                crate::ai::hardware_profile::profile_summary(&db)
                    .map(|(profiled, _)| !profiled)
                    .unwrap_or(false)
            };
            if needs_prepare {
                let db = db_arc.lock();
                let _ = crate::ai::llama_server::LlamaServer::prepare(&db, false);
            }
        }

        Ok(crate::ai::stats_interpret::stats_chat_answer(
            &chart,
            &initial_analysis,
            &message,
            &history,
            &app_name,
        ))
    })
    .await
    .map_err(|e| format!("Discussion Loggy interrompue : {e}"))?
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
    let session = state.desktop_sessions.require_privilege("ai:utiliser")?;
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
    let session = state.desktop_sessions.require_session()?;
    let db = state.db.lock();
    db.ai_list_conversations(&session.user.id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn ai_conversation_messages(
    state: State<'_, AppState>,
    payload: AiConversationIdPayload,
) -> Result<Vec<AiMessageRow>, String> {
    let session = state.desktop_sessions.require_session()?;
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

#[derive(serde::Deserialize)]
pub struct AiRenameConversationPayload {
    pub conversation_id: String,
    pub title: String,
}

#[tauri::command]
pub fn ai_rename_conversation(
    state: State<'_, AppState>,
    payload: AiRenameConversationPayload,
) -> Result<(), String> {
    let session = state.desktop_sessions.require_privilege("ai:utiliser")?;
    let title = payload.title.trim();
    if title.is_empty() {
        return Err("Le titre ne peut pas être vide.".into());
    }
    if title.len() > 200 {
        return Err("Le titre est trop long (200 caractères max).".into());
    }
    let db = state.db.lock();
    let ok = db
        .ai_rename_conversation(&session.user.id, &payload.conversation_id, title)
        .map_err(|e| e.to_string())?;
    if !ok {
        return Err("Conversation introuvable.".into());
    }
    Ok(())
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
            "Binaire llama-server introuvable. Installez Loggy via Parametres ou l'assistant au premier lancement."
                .into(),
        );
    }
    if !LlamaServer::model_ready() {
        return Err(format!(
            "Modele GGUF absent : {}. Lancez l'installation Loggy (choix du dossier IA).",
            default_model_path().display()
        ));
    }
    LlamaServer::ensure_started(Some(&db))?;
    Ok("Serveur IA démarré et prêt.".into())
}
