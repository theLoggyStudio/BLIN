use std::cell::Cell;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

pub const EVENT_ENTITY_SYNC_PROGRESS: &str = "entity-sync-progress";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncProgressPayload {
    pub current: u32,
    pub total: u32,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_key: Option<String>,
    pub step: String,
    pub done: bool,
}

/// Compteur d'étapes séquentielles (triggers exécutés l'un après l'autre, pas en parallèle).
pub struct SyncReporter<'a> {
    app: &'a AppHandle,
    total: u32,
    current: Cell<u32>,
}

impl<'a> SyncReporter<'a> {
    pub fn new(app: &'a AppHandle, total: u32) -> Self {
        Self {
            app,
            total: total.max(1),
            current: Cell::new(0),
        }
    }

    pub fn tick(&self, label: impl Into<String>, entity_key: Option<&str>, step: &str) {
        let n = self.current.get() + 1;
        self.current.set(n);
        let _ = self.app.emit(
            EVENT_ENTITY_SYNC_PROGRESS,
            SyncProgressPayload {
                current: n,
                total: self.total,
                label: label.into(),
                entity_key: entity_key.map(str::to_string),
                step: step.to_string(),
                done: false,
            },
        );
    }

    /// Étapes pré-save (normalisation, écriture registre).
    pub fn prep(&self, label: impl Into<String>, step: &str) {
        self.tick(label, None, step);
    }

    pub fn finish(&self, label: impl Into<String>) {
        let _ = self.app.emit(
            EVENT_ENTITY_SYNC_PROGRESS,
            SyncProgressPayload {
                current: self.total,
                total: self.total,
                label: label.into(),
                entity_key: None,
                step: "done".to_string(),
                done: true,
            },
        );
    }
}

/// Nombre d'étapes pour `apply_registry` : nettoyage + (schéma + config + 5 triggers) × entités + mémoire + suggestions.
pub fn count_apply_registry_steps(entity_count: usize, removed_count: usize) -> u32 {
    let cleanup = if removed_count > 0 { 1 } else { 0 };
    let per_entity = 7u32; // schéma, JSON DDA, privilèges, validations, knowledge, dossiers, impression
    cleanup + entity_count as u32 * per_entity + 2 // finalize knowledge + suggestions dashboard
}
