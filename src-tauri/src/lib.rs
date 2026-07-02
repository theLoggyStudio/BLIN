mod date_format;
mod ai;
mod entity;
mod dda;
mod bien_detail;
mod monnaie_domaine;
mod commands;
mod csv_util;
mod db;
mod db_io;
mod print_model_sync;
mod print_seed;
mod print_template;
mod privileges;
mod network_local;
mod remote;
mod session;
mod startup_sync;
mod sync_progress;

use parking_lot::Mutex;
use session::{RemoteSessionStore, SessionManager, SharedRemoteSessions, SharedSession};
use std::sync::Arc;
use ai::login_messages::PreparedLoginMessages;
use tauri::Manager;

pub use db::Database;
pub use remote::{RemoteConnectionInfo, RemoteServer, RemoteState};
pub use session::SessionUser;

pub struct AppState {
    pub db: Arc<Mutex<Database>>,
    pub desktop_sessions: SharedSession,
    pub remote_sessions: SharedRemoteSessions,
    pub pairing_token: Arc<Mutex<String>>,
    pub login_messages: Arc<Mutex<PreparedLoginMessages>>,
    /// Réserve de réponses Loggy pré-générées (alertes instantanées).
    pub alert_pool: Arc<ai::alert_pool::AlertPool>,
    /// État de la synchronisation lourde au démarrage (arrière-plan).
    pub startup_sync: startup_sync::SharedStartupSync,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("répertoire données application");

            let database = Database::open(app_data_dir).expect("initialisation base de données SQLite");
            ai::runtime_config::refresh_from_data_dir(&database.data_dir);
            dda::schema::ensure_dda_registry_table(&database).expect("dda registry");

            let pool_dir = database.data_dir.clone();
            let alert_pool = Arc::new(
                ai::alert_pool::AlertPool::open(&pool_dir).unwrap_or_else(|e| {
                    eprintln!("Avertissement pool d'alertes (réserve volatile) : {e}");
                    ai::alert_pool::AlertPool::open_memory()
                }),
            );

            let db = Arc::new(Mutex::new(database));
            let startup_sync = startup_sync::new_shared_state();
            let desktop_sessions: SharedSession = Arc::new(SessionManager::new());
            let remote_sessions: SharedRemoteSessions = Arc::new(RemoteSessionStore::new());

            let pairing_arc = Arc::new(Mutex::new(String::new()));

            app.manage(AppState {
                db: db.clone(),
                desktop_sessions,
                remote_sessions,
                pairing_token: pairing_arc,
                login_messages: Arc::new(Mutex::new(PreparedLoginMessages::default())),
                alert_pool,
                startup_sync: startup_sync.clone(),
            });

            let data_dir = app.state::<AppState>().db.lock().data_dir.clone();
            if let Err(e) = entity::branding::apply_window_title(app.handle(), &data_dir) {
                eprintln!("Avertissement titre fenêtre au démarrage : {e}");
            }
            if let Err(e) =
                entity::branding::apply_window_icon_if_changed(app.handle(), &data_dir, true)
            {
                eprintln!("Avertissement icône barre des tâches au démarrage : {e}");
            }

            network_local::warm_local_ip_cache();

            startup_sync::spawn_startup_sync(app.handle().clone(), db, startup_sync);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::auth::auth_login,
            commands::auth::auth_logout,
            commands::auth::auth_current_user,
            commands::auth::auth_change_password,
            commands::auth::auth_verify_password,
            commands::auth::auth_sync_session_privileges,
            commands::auth::auth_prepare_login_messages,
            commands::auth::auth_get_login_messages,
            commands::auth::app_startup_sync_status,
            commands::ai::ai_status,
            commands::ai::ai_runtime_status,
            commands::ai::ai_runtime_install,
            commands::ai::ai_profile_runtime,
            commands::ai::ai_reindex,
            commands::ai::ai_chat,
            commands::ai::ai_dashboard_transition,
            commands::ai::ai_entity_access_denied,
            commands::ai::ai_alert_personify,
            commands::ai::ai_task_reminder_personify,
            commands::ai::ai_stats_interpret,
            commands::ai::ai_stats_chat,
            commands::ai::ai_dashboard_answer,
            commands::ai::ai_list_conversations,
            commands::ai::ai_conversation_messages,
            commands::ai::ai_delete_conversation,
            commands::ai::ai_rename_conversation,
            commands::ai::ai_web_search_get_config,
            commands::ai::ai_web_search_set_config,
            commands::ai::ai_vision_get_config,
            commands::ai::ai_vision_set_config,
            commands::ai::ai_vision_analyze,
            commands::ai::ai_confirm_action,
            commands::ai::ai_dismiss_action,
            commands::ai::ai_stop_server,
            commands::ai::ai_start_server,
            commands::dda::dda_sync_screens,
            commands::dda::dda_list_screens,
            commands::dda::dda_list,
            commands::dda::dda_get,
            commands::dda::dda_create,
            commands::dda::dda_update,
            commands::dda::dda_delete,
            commands::dda::dda_validate,
            commands::dda::dda_validate_filters,
            commands::dda::dda_media_absolute_path,
            commands::dda::dda_media_upload,
            commands::dda::dda_media_delete,
            commands::entity::entity_registry_get,
            commands::entity::entity_registry_archive_list,
            commands::entity::entity_registry_archive_get,
            commands::entity::entity_registry_save,
            commands::entity::entity_branding_get,
            commands::entity::entity_branding_apply_title,
            commands::entity::entity_branding_restore_icon,
            commands::entity::entity_branding_apply_window,
            commands::entity::entity_logo_from_url,
            commands::entity::entity_check_access,
            commands::entity::entity_list_manageable,
            commands::entity::entity_match_intent,
            commands::entity::entity_match_create_draft,
            commands::entity::entity_registry_create_access,
            commands::entity::entity_registry_list_brief,
            commands::entity::entity_match_registry_create_draft,
            commands::entity::entity_registry_append_entity,
            commands::entity::entity_get_screen_config,
            commands::entity::entity_stats_config,
            commands::entity::entity_compteur_preview,
            commands::entity::entity_matricule_registry_list,
            commands::entity::entity_matricule_registry_create,
            commands::entity::entity_inline_create_allowed,
            commands::entity::entity_embed_impact_meta,
            commands::entity::entity_child_numeric_field,
            commands::entity::entity_relation_options,
            commands::entity::entity_embed_values_from_record,
            commands::entity::entity_embed_child_from_record,
            commands::entity::entity_list_roles,
            commands::entity::entity_record_signature_detail,
            commands::entity::entity_record_sign,
            commands::entity::entity_record_reject,
            commands::entity::entity_relation_detail,
            commands::entity::entity_stats,
            commands::entity::entity_active_session_get,
            commands::entity::entity_active_session_set,
            commands::entity::entity_active_session_clear,
            commands::entity::entity_export_csv,
            commands::entity::entity_import_csv,
            commands::entity::io_log_summary,
            commands::entity::io_log_detail,
            commands::entity::entity_success_message,
            commands::entity::entity_stock_status,
            commands::entity::entity_stock_scan_destock,
            commands::entity::entity_stock_destock,
            commands::print_models::print_models_list,
            commands::print_models::print_models_get,
            commands::print_models::print_models_upsert,
            commands::print_models::print_models_delete,
            commands::print_models::print_models_defaults,
            commands::print_models::print_row_render,
            commands::print_models::print_list_render,
            commands::users::users_list,
            commands::users::users_list_roles,
            commands::users::users_create,
            commands::users::users_update,
            commands::users::users_reset_password,
            commands::users::privileges_list_catalog,
            commands::users::roles_create,
            commands::users::roles_update,
            commands::users::roles_delete,
            commands::users::roles_list_with_privileges,
            commands::users::roles_update_privileges,
            commands::remote::remote_connection_get,
        ])
        .run(tauri::generate_context!())
        .expect("erreur au démarrage de l'application Tauri");
}
