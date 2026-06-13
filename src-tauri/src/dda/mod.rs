pub mod config;
pub mod crud;
pub mod filters;
pub mod knowledge;
pub mod media;
pub mod registry;
pub mod schema;
pub mod success_alerts;
pub mod triggers;
pub mod validation;

use std::path::Path;

use crate::db::Database;

pub const SYSTEM_SCREEN_KEYS: &[&str] = &["dashboard", "admin", "utilisateurs", "parametres"];

pub fn json_config_dir() -> std::path::PathBuf {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .map(|p| p.join("src").join("constante").join("json"))
        .unwrap_or_else(|| manifest.join("constante").join("json"))
}

/// Charge tous les JSON métier et exécute la chaîne de triggers (schéma, privilèges, IA, dossiers, impression).
pub fn sync_all_screens(db: &Database, data_dir: &Path) -> Result<Vec<String>, String> {
    let mut all_configs = Vec::new();
    let dir = json_config_dir();
    if dir.is_dir() {
        all_configs.extend(registry::load_all_screen_configs(&dir)?);
    }
    let gen = data_dir.join("dda").join("generated");
    if gen.is_dir() {
        all_configs.extend(registry::load_all_screen_configs(&gen)?);
    }
    if all_configs.is_empty() {
        return Ok(Vec::new());
    }
    let mut synced = Vec::new();
    let mut active_configs = Vec::new();
    for cfg in &all_configs {
        if cfg.screen.system {
            continue;
        }
        if SYSTEM_SCREEN_KEYS.contains(&cfg.screen.key.as_str()) {
            continue;
        }
        schema::sync_table_from_config(db, cfg)?;
        triggers::run_all(db, data_dir, cfg)?;
        active_configs.push(cfg.clone());
        synced.push(cfg.screen.key.clone());
    }
    knowledge::finalize_master_knowledge(data_dir, &active_configs)?;
    Ok(synced)
}

/// Réindexe la base de connaissances Loggy (FTS) après sync DDA.
pub fn reindex_ai_knowledge(db: &Database) -> Result<usize, String> {
    crate::ai::agent::Agent::new(db).reindex()
}

pub fn load_screen_config(screen_key: &str) -> Result<config::ScreenConfigFile, String> {
    let path = json_config_dir().join(format!("{screen_key}.json"));
    if path.is_file() {
        return registry::load_screen_config_file(&path);
    }
    Err(format!(
        "Écran « {screen_key} » introuvable. Les entités sont chargées via entity_get_screen_config ou {}/dda/generated/",
        "app_data"
    ))
}

pub fn load_screen_config_with_data_dir(
    screen_key: &str,
    data_dir: &Path,
) -> Result<config::ScreenConfigFile, String> {
    let generated = data_dir.join("dda").join("generated").join(format!("{screen_key}.json"));
    if generated.is_file() {
        return registry::load_screen_config_file(&generated);
    }
    let path = json_config_dir().join(format!("{screen_key}.json"));
    registry::load_screen_config_file(&path)
}
