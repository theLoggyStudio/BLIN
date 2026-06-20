pub mod parent_lignes;
pub mod child_table;
pub mod list_preview;
pub mod apply;
pub mod attr_types;
pub mod bootstrap;
pub mod branding;
pub mod compteur;
pub mod config;
pub mod create_draft;
pub mod registry_create_draft;
pub mod csv_io;
pub mod embed;
pub mod intent;
pub mod io_log;
pub mod logo;
pub mod knowledge;
pub mod registry;
pub mod record_signature;
pub mod session_scope;
pub mod relations;
pub mod relation_impact;
pub mod schema;
pub mod stats;
pub mod stock;
pub mod tache_visibility;
pub mod suggestions;
pub mod validation;

use std::path::{Path, PathBuf};

use crate::db::Database;
use crate::dda;

pub use apply::apply_registry;

pub fn registry_path(data_dir: &Path) -> PathBuf {
    data_dir.join("entities").join("registry.json")
}

pub fn generated_config_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("dda").join("generated")
}

pub fn load_screen_config(
    data_dir: &Path,
    entity_key: &str,
) -> Result<dda::config::ScreenConfigFile, String> {
    let path = generated_config_dir(data_dir).join(format!("{entity_key}.json"));
    if path.is_file() {
        return dda::registry::load_screen_config_file(&path);
    }
    let registry = registry::load(data_dir)?;
    let ent = registry
        .find(entity_key)
        .ok_or_else(|| format!("Entité « {entity_key} » introuvable."))?;
    Ok(config::build_screen_config(ent, &registry))
}

pub fn match_intent(message: &str, data_dir: &Path) -> Option<String> {
    let registry = registry::load(data_dir).ok()?;
    registry::match_intent(message, &registry)
}

pub fn live_summary(db: &Database, data_dir: &Path) -> Result<String, String> {
    let registry = registry::load(data_dir)?;
    let mut parts = vec![format!(
        "Entités métier déclarées ({}) :",
        registry.entities.len()
    )];
    for ent in &registry.entities {
        let table = schema::table_name(&ent.nom);
        let count: i64 = db
            .conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap_or(0);
        parts.push(format!(
            "- {} (table {}, {} attributs, {} enregistrements)",
            ent.nom,
            table,
            ent.attributs.len(),
            count
        ));
    }
    Ok(parts.join("\n"))
}
