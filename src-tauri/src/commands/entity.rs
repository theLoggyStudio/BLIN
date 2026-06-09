use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use crate::dda::config::ScreenConfigFile;
use crate::entity::{
    self,
    create_draft::EntityCreateDraft,
    record_signature::{RecordSignatureDetail, RelationSelectOptionExt, RowUserContext},
    registry::EntityRegistry,
    registry_create_draft::{RegistryCreateMatchResult, RegistryEntityCreateDraft},
    relations::RelationDetailResponse,
};
use crate::privileges::{can_create_registry_entity, has_any_entity_privilege, has_privilege};
use crate::sync_progress::{count_apply_registry_steps, SyncReporter};
use crate::AppState;

#[derive(Serialize)]
pub struct EntityRegistryResponse {
    #[serde(default)]
    pub ecosysteme: Option<String>,
    #[serde(default)]
    pub slogan: Option<String>,
    #[serde(default)]
    pub logo_url: Option<String>,
    #[serde(default)]
    pub logo: Option<String>,
    pub entities: Vec<crate::entity::registry::EntityDef>,
    pub count: usize,
    pub json: String,
}

#[derive(Deserialize)]
pub struct EntityRegistrySavePayload {
    pub registry: EntityRegistry,
}

#[derive(Deserialize)]
pub struct EntityKeyPayload {
    pub entity_key: String,
}

#[derive(Deserialize)]
pub struct EntityRelationFieldPayload {
    pub screen_key: String,
    pub field_key: String,
    #[serde(default)]
    pub exclude_record_id: Option<String>,
}

#[derive(Deserialize)]
pub struct EntityEmbedCopyPayload {
    pub screen_key: String,
    pub field_key: String,
    pub record_id: String,
}

#[derive(Deserialize)]
pub struct EntityRelationDetailPayload {
    pub screen_key: String,
    pub record_id: String,
}

#[derive(Deserialize)]
pub struct EntityRecordSignaturePayload {
    pub entity_key: String,
    pub record_id: String,
}

#[derive(Deserialize)]
pub struct EntityRecordRejectPayload {
    pub entity_key: String,
    pub record_id: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Deserialize)]
pub struct EntityIntentPayload {
    pub message: String,
}

#[derive(Deserialize)]
pub struct EntityStatsPayload {
    pub entity_key: String,
    pub group_by: String,
    /// count | sum | avg | max | min
    #[serde(default)]
    pub aggregate: Option<String>,
    /// Champ numérique pour sum / avg / max / min
    #[serde(default)]
    pub value_field: Option<String>,
    /// Legacy : « count » ou nom de champ → somme
    #[serde(default)]
    pub metric: Option<String>,
}

#[derive(Serialize)]
pub struct EntityStatRow {
    pub label: String,
    pub value: f64,
}

#[derive(Serialize)]
pub struct EntityStockStatus {
    pub enabled: bool,
    pub line_count: u32,
}

#[derive(Deserialize)]
pub struct EntityStockDestockPayload {
    pub stock_id: String,
    /// Quantité à retirer ; si absent, déstocke tout le disponible.
    #[serde(default)]
    pub quantity: Option<f64>,
}

#[tauri::command]
pub fn entity_registry_get(state: State<'_, AppState>) -> Result<EntityRegistryResponse, String> {
    state
        .desktop_sessions
        .require_privilege("parametres:entites")?;
    let db = state.db.lock();
    let registry = entity::registry::load(&db.data_dir)?;
    let json = serde_json::to_string_pretty(&registry).map_err(|e| e.to_string())?;
    let count = registry.entities.len();
    Ok(EntityRegistryResponse {
        ecosysteme: registry.ecosysteme.clone(),
        slogan: registry.slogan.clone(),
        logo_url: registry.logo_url.clone(),
        logo: registry.logo.clone(),
        entities: registry.entities,
        count,
        json,
    })
}

#[derive(Deserialize)]
pub struct EntityLogoUrlPayload {
    pub url: String,
}

#[tauri::command]
pub fn entity_logo_from_url(
    state: State<'_, AppState>,
    payload: EntityLogoUrlPayload,
) -> Result<String, String> {
    state
        .desktop_sessions
        .require_privilege("parametres:entites")?;
    entity::logo::fetch_from_url(&payload.url)
}

#[tauri::command]
pub fn entity_registry_save(
    app: AppHandle,
    state: State<'_, AppState>,
    payload: EntityRegistrySavePayload,
) -> Result<Vec<String>, String> {
    state
        .desktop_sessions
        .require_privilege("parametres:entites")?;
    let db = state.db.lock();
    let data_dir = db.data_dir.clone();
    let previous = entity::registry::load(&data_dir)?;
    let mut registry = entity::registry::normalize_registry(payload.registry);
    let auto_created = entity::relations::ensure_referenced_entities(&mut registry);
    registry = entity::registry::normalize_registry(registry);
    let entity_count = registry.entities.len();
    let removed_count = previous
        .entities
        .iter()
        .filter(|e| !registry.entities.iter().any(|c| c.nom == e.nom))
        .count();
    let total_steps = 3 + count_apply_registry_steps(entity_count, removed_count);
    let reporter = SyncReporter::new(&app, total_steps);
    reporter.prep("Enregistrement du registre", "save");
    entity::registry::save(&data_dir, &registry)?;
    drop(db);

    let db = state.db.lock();
    let mut synced = entity::apply_registry(&db, &data_dir, &previous, Some(&reporter))?;
    synced.extend(auto_created);
    drop(db);

    let db = state.db.lock();
    reporter.tick("Réindexation mémoire IA (Loggy)", None, "reindex");
    crate::dda::reindex_ai_knowledge(&db)?;
    drop(db);

    let db = state.db.lock();
    reporter.tick("Mise à jour des privilèges de session", None, "session");
    let _ = state.desktop_sessions.sync_privileges(&db)?;
    reporter.finish("Synchronisation terminée");
    if let Err(e) = entity::branding::apply_window_branding(&app, &data_dir) {
        eprintln!("Avertissement branding fenêtre : {e}");
    }
    Ok(synced)
}

#[derive(Serialize)]
pub struct BrandingApplyResponse {
    pub window_title: String,
}

/// Applique écosystème + slogan + icône sur la fenêtre Tauri (titre barre, barre des tâches).
#[tauri::command]
pub fn entity_branding_apply_window(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<BrandingApplyResponse, String> {
    let db = state.db.lock();
    let data_dir = db.data_dir.clone();
    drop(db);
    entity::branding::apply_window_branding(&app, &data_dir)?;
    let (name, slogan) = entity::branding::load_branding(&data_dir);
    Ok(BrandingApplyResponse {
        window_title: entity::branding::window_title(&name, &slogan),
    })
}

#[tauri::command]
pub fn entity_relation_options(
    state: State<'_, AppState>,
    payload: EntityRelationFieldPayload,
) -> Result<Vec<RelationSelectOptionExt>, String> {
    let cfg = {
        let db = state.db.lock();
        entity::load_screen_config(&db.data_dir, &payload.screen_key)?
    };
    state
        .desktop_sessions
        .require_privilege(&cfg.screen.privileges.view)?;
    let db = state.db.lock();
    entity::relations::relation_select_options(
        &db,
        &db.data_dir,
        &payload.screen_key,
        &payload.field_key,
        payload.exclude_record_id.as_deref(),
    )
}

#[tauri::command]
pub fn entity_embed_values_from_record(
    state: State<'_, AppState>,
    payload: EntityEmbedCopyPayload,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    let cfg = {
        let db = state.db.lock();
        entity::load_screen_config(&db.data_dir, &payload.screen_key)?
    };
    state
        .desktop_sessions
        .require_privilege(&cfg.screen.privileges.view)?;
    let db = state.db.lock();
    entity::relations::embed_values_from_record(
        &db,
        &db.data_dir,
        &payload.screen_key,
        &payload.field_key,
        &payload.record_id,
    )
}

#[tauri::command]
pub fn entity_embed_child_from_record(
    state: State<'_, AppState>,
    payload: EntityEmbedCopyPayload,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    let cfg = {
        let db = state.db.lock();
        entity::load_screen_config(&db.data_dir, &payload.screen_key)?
    };
    state
        .desktop_sessions
        .require_privilege(&cfg.screen.privileges.view)?;
    let db = state.db.lock();
    entity::relations::embed_child_object_from_record(
        &db,
        &db.data_dir,
        &payload.screen_key,
        &payload.field_key,
        &payload.record_id,
    )
}

#[tauri::command]
pub fn entity_list_roles(state: State<'_, AppState>) -> Result<Vec<crate::db::RoleRow>, String> {
    state.desktop_sessions.require_privilege("tache:voir")?;
    let db = state.db.lock();
    db.list_roles().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn entity_record_signature_detail(
    state: State<'_, AppState>,
    payload: EntityRecordSignaturePayload,
) -> Result<RecordSignatureDetail, String> {
    state.desktop_sessions.require_session()?;
    let session = state.desktop_sessions.require_session()?;
    let db = state.db.lock();
    entity::record_signature::record_signature_detail(
        &db,
        &db.data_dir,
        &payload.entity_key,
        &payload.record_id,
        &session.user.id,
        &session.user.privileges,
    )
}

#[tauri::command]
pub fn entity_record_sign(
    state: State<'_, AppState>,
    payload: EntityRecordSignaturePayload,
) -> Result<(), String> {
    state.desktop_sessions.require_session()?;
    let session = state.desktop_sessions.require_session()?;
    let db = state.db.lock();
    let data_dir = db.data_dir.clone();
    entity::record_signature::sign_record(
        &db,
        &data_dir,
        &payload.entity_key,
        &payload.record_id,
        &session.user.id,
        &session.user.privileges,
    )
}

#[tauri::command]
pub fn entity_record_reject(
    state: State<'_, AppState>,
    payload: EntityRecordRejectPayload,
) -> Result<(), String> {
    state.desktop_sessions.require_session()?;
    let session = state.desktop_sessions.require_session()?;
    let db = state.db.lock();
    let data_dir = db.data_dir.clone();
    entity::record_signature::reject_record(
        &db,
        &data_dir,
        &payload.entity_key,
        &payload.record_id,
        &session.user.id,
        &session.user.privileges,
        payload.reason.as_deref(),
    )
}

#[tauri::command]
pub fn entity_relation_detail(
    state: State<'_, AppState>,
    payload: EntityRelationDetailPayload,
) -> Result<RelationDetailResponse, String> {
    let cfg = {
        let db = state.db.lock();
        entity::load_screen_config(&db.data_dir, &payload.screen_key)?
    };
    state
        .desktop_sessions
        .require_privilege(&cfg.screen.privileges.view)?;
    let db = state.db.lock();
    entity::relations::relation_detail(
        &db,
        &db.data_dir,
        &payload.screen_key,
        &payload.record_id,
    )
}

#[derive(Serialize)]
pub struct EntityAccessCheckResult {
    pub allowed: bool,
    pub entity_key: String,
    pub entity_label: String,
    pub contact_role_names: Vec<String>,
}

#[tauri::command]
pub fn entity_check_access(
    state: State<'_, AppState>,
    payload: EntityKeyPayload,
) -> Result<EntityAccessCheckResult, String> {
    state.desktop_sessions.require_session()?;
    let db = state.db.lock();
    let _ = state.desktop_sessions.sync_privileges(&db)?;
    let session = state.desktop_sessions.require_session()?;
    let key = payload.entity_key.trim();
    if key.is_empty() {
        return Err("Entité cible manquante.".into());
    }
    let registry = entity::registry::load(&db.data_dir)?;
    let ent = registry
        .find(key)
        .ok_or_else(|| format!("Entité « {key} » introuvable."))?;
    let label = ent
        .label
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| key.to_string());
    let allowed = has_any_entity_privilege(&session.user.privileges, key);
    let contact_role_names = if allowed {
        vec![]
    } else {
        db.list_role_names_with_entity_access(key)
            .map_err(|e| e.to_string())?
    };
    Ok(EntityAccessCheckResult {
        allowed,
        entity_key: key.to_string(),
        entity_label: label,
        contact_role_names,
    })
}

#[tauri::command]
pub fn entity_list_manageable(
    state: State<'_, AppState>,
) -> Result<Vec<entity::suggestions::EntitySuggestion>, String> {
    state.desktop_sessions.require_session()?;
    let db = state.db.lock();
    let _ = state.desktop_sessions.sync_privileges(&db)?;
    let session = state.desktop_sessions.require_session()?;
    let registry = entity::registry::load(&db.data_dir)?;
    Ok(entity::suggestions::list_for_user(
        &db.data_dir,
        &registry,
        &session.user,
    ))
}

#[tauri::command]
pub fn entity_match_intent(
    state: State<'_, AppState>,
    payload: EntityIntentPayload,
) -> Result<Option<String>, String> {
    let db = state.db.lock();
    Ok(entity::match_intent(&payload.message, &db.data_dir))
}

#[tauri::command]
pub fn entity_match_create_draft(
    state: State<'_, AppState>,
    payload: EntityIntentPayload,
) -> Result<Option<EntityCreateDraft>, String> {
    state.desktop_sessions.require_session()?;
    let db = state.db.lock();
    let session = state.desktop_sessions.require_session()?;
    let registry = entity::registry::load(&db.data_dir)?;
    let Some(draft) = entity::create_draft::match_create_draft(&payload.message, &registry) else {
        return Ok(None);
    };
    let priv_key = format!("{}:creer", draft.entity_key);
    if !has_privilege(&session.user.privileges, &priv_key) {
        return Ok(None);
    }
    Ok(Some(draft))
}

#[derive(Serialize)]
pub struct RegistryCreateAccessResult {
    pub allowed: bool,
}

#[derive(Serialize)]
pub struct EntityRegistryBrief {
    pub nom: String,
    pub label: Option<String>,
}

#[tauri::command]
pub fn entity_registry_list_brief(
    state: State<'_, AppState>,
) -> Result<Vec<EntityRegistryBrief>, String> {
    let session = state.desktop_sessions.require_session()?;
    if !can_create_registry_entity(&session.user.privileges) {
        return Err("Privilège requis : parametres:entites:creer ou parametres:entites".into());
    }
    let db = state.db.lock();
    let registry = entity::registry::load(&db.data_dir)?;
    Ok(registry
        .entities
        .iter()
        .map(|e| EntityRegistryBrief {
            nom: e.nom.clone(),
            label: e.label.clone(),
        })
        .collect())
}

#[tauri::command]
pub fn entity_registry_create_access(
    state: State<'_, AppState>,
) -> Result<RegistryCreateAccessResult, String> {
    let session = state.desktop_sessions.require_session()?;
    Ok(RegistryCreateAccessResult {
        allowed: can_create_registry_entity(&session.user.privileges),
    })
}

#[tauri::command]
pub fn entity_match_registry_create_draft(
    state: State<'_, AppState>,
    payload: EntityIntentPayload,
) -> Result<RegistryCreateMatchResult, String> {
    state.desktop_sessions.require_session()?;
    let db = state.db.lock();
    let session = state.desktop_sessions.require_session()?;
    let registry = entity::registry::load(&db.data_dir)?;
    let allowed = can_create_registry_entity(&session.user.privileges);
    Ok(entity::registry_create_draft::match_registry_create_with_access(
        &payload.message,
        &registry,
        allowed,
    ))
}

#[derive(Deserialize)]
pub struct EntityRegistryAppendPayload {
    pub entity: crate::entity::registry::EntityDef,
}

#[tauri::command]
pub fn entity_registry_append_entity(
    app: AppHandle,
    state: State<'_, AppState>,
    payload: EntityRegistryAppendPayload,
) -> Result<Vec<String>, String> {
    let session = state.desktop_sessions.require_session()?;
    if !can_create_registry_entity(&session.user.privileges) {
        return Err("Privilège requis : parametres:entites:creer ou parametres:entites".into());
    }
    let db = state.db.lock();
    let data_dir = db.data_dir.clone();
    let previous = entity::registry::load(&data_dir)?;
    let mut registry = previous.clone();
    let nom = payload.entity.nom.trim().to_lowercase();
    if nom.is_empty() {
        return Err("Le nom de l'entité est obligatoire.".into());
    }
    if registry.entities.iter().any(|e| e.nom == nom) {
        return Err(format!("L'entité « {nom} » existe déjà dans le registre."));
    }
    let mut ent = payload.entity;
    ent.nom = nom;
    registry.entities.push(ent);
    registry = entity::registry::normalize_registry(registry);
    let auto_created = entity::relations::ensure_referenced_entities(&mut registry);
    registry = entity::registry::normalize_registry(registry);
    let entity_count = registry.entities.len();
    let removed_count = 0usize;
    let total_steps = 3 + count_apply_registry_steps(entity_count, removed_count);
    let reporter = SyncReporter::new(&app, total_steps);
    reporter.prep("Enregistrement du registre", "save");
    entity::registry::save(&data_dir, &registry)?;
    drop(db);

    let db = state.db.lock();
    let mut synced = entity::apply_registry(&db, &data_dir, &previous, Some(&reporter))?;
    synced.extend(auto_created);
    drop(db);

    let db = state.db.lock();
    reporter.tick("Réindexation mémoire IA (Loggy)", None, "reindex");
    crate::dda::reindex_ai_knowledge(&db)?;
    drop(db);

    let db = state.db.lock();
    reporter.tick("Mise à jour des privilèges de session", None, "session");
    let _ = state.desktop_sessions.sync_privileges(&db)?;
    reporter.finish("Synchronisation terminée");
    if let Err(e) = entity::branding::apply_window_branding(&app, &data_dir) {
        eprintln!("Avertissement branding fenêtre : {e}");
    }
    Ok(synced)
}

#[tauri::command]
pub fn entity_get_screen_config(
    state: State<'_, AppState>,
    payload: EntityKeyPayload,
) -> Result<ScreenConfigFile, String> {
    let db = state.db.lock();
    entity::load_screen_config(&db.data_dir, &payload.entity_key)
}

/// Prévisualise date (jjmmaaaa) et n° quotidien des attributs compteur / matricule à la création.
#[tauri::command]
pub fn entity_compteur_preview(
    state: State<'_, AppState>,
    payload: EntityKeyPayload,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    state.desktop_sessions.require_session()?;
    let db = state.db.lock();
    let registry = entity::registry::load(&db.data_dir)?;
    entity::compteur::preview_compteurs_on_create(&db, &registry, &payload.entity_key)
}

#[tauri::command]
pub fn entity_stock_status(state: State<'_, AppState>) -> Result<EntityStockStatus, String> {
    state.desktop_sessions.require_session()?;
    let db = state.db.lock();
    let registry = entity::registry::load(&db.data_dir)?;
    let has_module = registry.find(entity::stock::STOCK_ENTITY_KEY).is_some()
        && entity::stock::registry_has_stock(&registry);
    if !has_module {
        return Ok(EntityStockStatus {
            enabled: false,
            line_count: 0,
        });
    }
    let session = state.desktop_sessions.require_session()?;
    let priv_key = format!("{}:voir", entity::stock::STOCK_ENTITY_KEY);
    if !has_privilege(&session.user.privileges, &priv_key) {
        return Ok(EntityStockStatus {
            enabled: false,
            line_count: 0,
        });
    }
    let table = entity::schema::table_name(entity::stock::STOCK_ENTITY_KEY);
    let line_count: u32 = db
        .conn
        .query_row(
            &format!("SELECT COUNT(*) FROM {table}"),
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|n| n.max(0) as u32)
        .unwrap_or(0);
    Ok(EntityStockStatus {
        enabled: true,
        line_count,
    })
}

#[tauri::command]
pub fn entity_stock_destock(
    state: State<'_, AppState>,
    payload: EntityStockDestockPayload,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    state.desktop_sessions.require_session()?;
    let session = state.desktop_sessions.require_session()?;
    if !has_privilege(&session.user.privileges, "stock:modifier") {
        return Err("Privilège stock:modifier requis pour déstocker.".into());
    }
    let db = state.db.lock();
    entity::stock::destock_line(&db, &db.data_dir, &payload.stock_id, payload.quantity)
}

#[tauri::command]
pub fn entity_stock_scan_destock(state: State<'_, AppState>) -> Result<u32, String> {
    state.desktop_sessions.require_session()?;
    let db = state.db.lock();
    entity::stock::scan_all_destock_tasks(&db, &db.data_dir)
}

#[tauri::command]
pub fn entity_stats(
    state: State<'_, AppState>,
    payload: EntityStatsPayload,
) -> Result<Vec<EntityStatRow>, String> {
    let cfg = {
        let db = state.db.lock();
        entity::load_screen_config(&db.data_dir, &payload.entity_key)?
    };
    state
        .desktop_sessions
        .require_privilege(&cfg.screen.privileges.view)?;

    let (op, value_field) = entity::stats::resolve_aggregate(
        payload.aggregate.as_deref(),
        payload.value_field.as_deref(),
        payload.metric.as_deref(),
    )?;

    let db = state.db.lock();
    let registry = entity::registry::load(&db.data_dir)?;
    let ent = registry
        .find(&payload.entity_key)
        .ok_or_else(|| format!("Entité « {} » introuvable.", payload.entity_key))?;

    let rows = entity::stats::query_entity_stats(
        &db,
        &cfg,
        ent,
        &payload.group_by,
        op,
        value_field.as_deref(),
    )?;
    Ok(rows
        .into_iter()
        .map(|r| EntityStatRow {
            label: r.label,
            value: r.value,
        })
        .collect())
}

#[derive(Serialize)]
pub struct EntityActiveSessionResponse {
    pub active: Option<entity::session_scope::ActiveBusinessSession>,
    pub session_entities: Vec<entity::session_scope::SessionEntityInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding: Option<entity::session_scope::SessionBinding>,
}

#[derive(Deserialize)]
pub struct EntityActiveSessionSetPayload {
    pub entity_key: String,
    pub record_id: String,
}

#[derive(Deserialize)]
pub struct EntitySessionContextPayload {
    #[serde(default)]
    pub screen_key: Option<String>,
}

#[tauri::command]
pub fn entity_active_session_get(
    state: State<'_, AppState>,
    payload: EntitySessionContextPayload,
) -> Result<EntityActiveSessionResponse, String> {
    state.desktop_sessions.require_session()?;
    let db = state.db.lock();
    let registry = entity::registry::load(&db.data_dir)?;
    let active = entity::session_scope::load_active(&db.data_dir)?;
    let binding = payload
        .screen_key
        .as_deref()
        .and_then(|k| entity::session_scope::session_ref_binding(&registry, k));
    Ok(EntityActiveSessionResponse {
        active,
        session_entities: entity::session_scope::list_session_entities(&registry),
        binding,
    })
}

#[tauri::command]
pub fn entity_active_session_set(
    state: State<'_, AppState>,
    payload: EntityActiveSessionSetPayload,
) -> Result<entity::session_scope::ActiveBusinessSession, String> {
    state.desktop_sessions.require_session()?;
    let db = state.db.lock();
    let registry = entity::registry::load(&db.data_dir)?;
    if !entity::session_scope::entity_is_session(&registry, &payload.entity_key) {
        return Err(format!(
            "L'entité « {} » n'est pas déclarée comme session dans le registre.",
            payload.entity_key
        ));
    }
    let label = entity::session_scope::resolve_record_label(
        &db,
        &db.data_dir,
        &registry,
        &payload.entity_key,
        &payload.record_id,
    )?;
    let session = entity::session_scope::ActiveBusinessSession {
        entity_key: payload.entity_key,
        record_id: payload.record_id,
        label: Some(label),
    };
    entity::session_scope::save_active(&db.data_dir, &session)?;
    Ok(session)
}

#[tauri::command]
pub fn entity_active_session_clear(state: State<'_, AppState>) -> Result<(), String> {
    state.desktop_sessions.require_session()?;
    let db = state.db.lock();
    entity::session_scope::clear_active(&db.data_dir)
}

#[derive(Deserialize)]
pub struct EntityCsvPayload {
    pub entity_key: String,
}

#[derive(Deserialize)]
pub struct EntityCsvImportPayload {
    pub entity_key: String,
    pub csv: String,
}

#[derive(Serialize)]
pub struct EntityCsvExportResponse {
    pub csv: String,
    pub file_name: String,
}

#[tauri::command]
pub fn entity_export_csv(
    state: State<'_, AppState>,
    payload: EntityCsvPayload,
) -> Result<EntityCsvExportResponse, String> {
    let cfg = {
        let db = state.db.lock();
        entity::load_screen_config(&db.data_dir, &payload.entity_key)?
    };
    let export_priv = cfg
        .screen
        .privileges
        .export
        .clone()
        .unwrap_or_else(|| format!("{}:exporter", payload.entity_key));
    state.desktop_sessions.require_privilege(&export_priv)?;
    let db = state.db.lock();
    let (csv, file_name) =
        entity::csv_io::export_entity_csv(&db, &db.data_dir, &payload.entity_key)?;
    Ok(EntityCsvExportResponse { csv, file_name })
}

#[tauri::command]
pub fn entity_import_csv(
    state: State<'_, AppState>,
    payload: EntityCsvImportPayload,
) -> Result<entity::csv_io::EntityCsvImportResult, String> {
    let cfg = {
        let db = state.db.lock();
        entity::load_screen_config(&db.data_dir, &payload.entity_key)?
    };
    let import_priv = cfg
        .screen
        .privileges
        .import
        .clone()
        .unwrap_or_else(|| format!("{}:importer", payload.entity_key));
    state.desktop_sessions.require_privilege(&import_priv)?;
    let db = state.db.lock();
    entity::csv_io::import_entity_csv(&db, &db.data_dir, &payload.entity_key, &payload.csv)
}
