use std::fs;
use std::path::Path;

use crate::dda;
use crate::db::Database;
use crate::entity::{config, knowledge, registry, relations, schema, suggestions};
use crate::print_model_sync;
use crate::sync_progress::SyncReporter;

use super::generated_config_dir;
use super::registry::EntityRegistry;

/// Applique le registre : tables SQLite, JSON DDA générés, triggers (séquentiels), mémoire IA.
pub fn apply_registry(
    db: &Database,
    data_dir: &Path,
    previous: &EntityRegistry,
    progress: Option<&SyncReporter>,
) -> Result<Vec<String>, String> {
    let mut registry = registry::load(data_dir)?;
    let matricules = super::matricule_registry::load(data_dir).unwrap_or_default();
    matricules.resolve_unlinked_attrs(&mut registry);
    let stock_changed = super::stock::ensure_stock_module(&mut registry);
    super::tache_visibility::ensure_tache_visibility_in_registry(&mut registry);
    super::validation::ensure_tache_workflow_attrs(&mut registry);
    let auto_entities = relations::ensure_referenced_entities(&mut registry);
    if !auto_entities.is_empty() || stock_changed {
        registry::save(data_dir, &registry)?;
    }
    let gen_dir = generated_config_dir(data_dir);
    fs::create_dir_all(&gen_dir).map_err(|e| e.to_string())?;

    let previous_keys: Vec<String> = previous.entities.iter().map(|e| e.nom.clone()).collect();
    let current_keys: Vec<String> = registry.entities.iter().map(|e| e.nom.clone()).collect();
    let removed: Vec<_> = previous_keys
        .iter()
        .filter(|k| !current_keys.contains(k))
        .collect();

    if let Some(rep) = progress {
        if !removed.is_empty() {
            rep.tick(
                format!("Nettoyage de {} entité(s) supprimée(s)", removed.len()),
                None,
                "cleanup",
            );
        }
    }

    for removed in &removed {
        let table = schema::table_name(removed);
        db.conn
            .execute(&format!("DROP TABLE IF EXISTS {table}"), [])
            .map_err(|e| e.to_string())?;
        let _ = db.conn.execute(
            "DELETE FROM document_print_models WHERE screen_key = ?1",
            rusqlite::params![removed.as_str()],
        );
        let _ = super::stock::purge_stock_for_entity(db, removed);
        let path = gen_dir.join(format!("{removed}.json"));
        if path.is_file() {
            let _ = fs::remove_file(&path);
        }
    }

    let mut synced = Vec::new();
    for ent in &registry.entities {
        let prev = previous.find(&ent.nom);
        let label = ent.label.as_deref().unwrap_or(&ent.nom);
        if let Some(rep) = progress {
            rep.tick(format!("{label} — schéma SQLite"), Some(&ent.nom), "schema");
        }
        schema::sync_entity_table(db, ent, prev, &registry)?;
        if ent.nom == super::tache_visibility::TACHE_ENTITY_KEY {
            super::tache_visibility::ensure_visibility_columns(db)?;
            super::validation::ensure_tache_workflow_columns(db)?;
        }
        let cfg = config::build_screen_config(ent, &registry, data_dir);
        let json = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
        if let Some(rep) = progress {
            rep.tick(format!("{label} — configuration DDA"), Some(&ent.nom), "config");
        }
        fs::write(gen_dir.join(format!("{}.json", ent.nom)), json).map_err(|e| e.to_string())?;
        dda::triggers::run_all_with_progress(db, data_dir, &cfg, progress, Some(label))?;
        synced.push(ent.nom.clone());
    }

    if let Some(rep) = progress {
        rep.tick("Mémoire IA — catalogue entités", None, "knowledge_master");
    }
    knowledge::finalize_entity_knowledge(data_dir, &registry)?;
    if let Some(rep) = progress {
        rep.tick("Suggestions tableau de bord", None, "suggestions");
    }
    suggestions::write_dashboard_suggestions_trigger(data_dir, &registry)?;
    for orphan in super::registry::ORPHAN_ENTITY_KEYS {
        let _ = db.conn.execute(
            "DELETE FROM document_print_models WHERE screen_key = ?1",
            rusqlite::params![orphan],
        );
        super::stock::purge_stock_for_entity(db, orphan)?;
    }
    let _ = db.dedupe_print_model_names();
    print_model_sync::resync_all_registry_print_models(&db, data_dir, &registry)?;
    let _ = super::validation::reconcile_signature_tasks(db, data_dir);
    Ok(synced)
}
