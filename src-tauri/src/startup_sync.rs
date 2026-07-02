//! Synchronisation lourde au démarrage (DDA, registre entités, RAG) en arrière-plan.

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use parking_lot::Mutex;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::db::Database;

pub const STARTUP_SYNC_DONE_EVENT: &str = "app-startup-sync-done";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupSyncStatusPayload {
    pub phase: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum StartupSyncPhase {
    Pending,
    Running,
    Done,
    Failed(String),
}

impl StartupSyncPhase {
    pub fn to_payload(&self) -> StartupSyncStatusPayload {
        match self {
            Self::Pending => StartupSyncStatusPayload {
                phase: "pending".into(),
                error: None,
            },
            Self::Running => StartupSyncStatusPayload {
                phase: "running".into(),
                error: None,
            },
            Self::Done => StartupSyncStatusPayload {
                phase: "done".into(),
                error: None,
            },
            Self::Failed(msg) => StartupSyncStatusPayload {
                phase: "failed".into(),
                error: Some(msg.clone()),
            },
        }
    }
}

pub type SharedStartupSync = Arc<Mutex<StartupSyncPhase>>;

pub fn new_shared_state() -> SharedStartupSync {
    Arc::new(Mutex::new(StartupSyncPhase::Pending))
}

/// Sync lourde — verrou DB relâché entre chaque étape pour laisser passer login / branding.
fn run_startup_sync(db: &Arc<Mutex<Database>>) {
    let data_dir = {
        let guard = db.lock();
        guard.data_dir.clone()
    };

    {
        let guard = db.lock();
        if let Err(e) = crate::dda::sync_all_screens(&guard, &data_dir) {
            eprintln!("Avertissement sync DDA (démarrage) : {e}");
        }
    }

    if let Err(e) = crate::entity::bootstrap::ensure_default_registry(&data_dir) {
        eprintln!("Avertissement registre par défaut : {e}");
    }

    let prev = crate::entity::registry::load(&data_dir).unwrap_or_default();
    {
        let guard = db.lock();
        if let Err(e) = crate::entity::apply_registry(&guard, &data_dir, &prev, None) {
            eprintln!("Avertissement sync entités (démarrage) : {e}");
        }
    }

    {
        let guard = db.lock();
        if let Err(e) = crate::dda::reindex_ai_knowledge(&guard) {
            eprintln!("Avertissement réindexation Loggy (démarrage) : {e}");
        }
    }
}

/// Lance la sync lourde dans un thread dédié (fenêtre + connexion disponibles tout de suite).
pub fn spawn_startup_sync(app: AppHandle, db: Arc<Mutex<Database>>, status: SharedStartupSync) {
    std::thread::spawn(move || {
        // Laisser le webview et les premiers appels IPC (auth, branding) passer.
        thread::sleep(Duration::from_millis(400));

        {
            let mut guard = status.lock();
            *guard = StartupSyncPhase::Running;
        }

        run_startup_sync(&db);

        let phase = StartupSyncPhase::Done;
        {
            let mut guard = status.lock();
            *guard = phase.clone();
        }

        if let Err(e) = app.emit(STARTUP_SYNC_DONE_EVENT, phase.to_payload()) {
            eprintln!("Émission événement sync démarrage : {e}");
        }
    });
}
