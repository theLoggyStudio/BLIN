use std::fs;
use std::path::Path;

use rusqlite::params;

use super::config::ScreenConfigFile;
use super::knowledge;
use super::validation::{build_validation_catalog, format_validation_knowledge};
use crate::db::Database;
use crate::print_template::{auto_print_description, build_fiche_html_from_config, FICHE_CSS};

use crate::sync_progress::SyncReporter;

/// Chaîne complète exécutée à chaque sync d'écran JSON (séquentielle).
pub fn run_all(db: &Database, data_dir: &Path, cfg: &ScreenConfigFile) -> Result<(), String> {
    run_all_with_progress(db, data_dir, cfg, None, None)
}

pub fn run_all_with_progress(
    db: &Database,
    data_dir: &Path,
    cfg: &ScreenConfigFile,
    progress: Option<&SyncReporter>,
    entity_label: Option<&str>,
) -> Result<(), String> {
    let key = cfg.screen.key.as_str();
    let label = entity_label.unwrap_or(key);
    let steps = [
        ("privileges", "Privilèges"),
        ("validations", "Validations"),
        ("knowledge", "Mémoire IA"),
        ("folders", "Dossiers"),
        ("print", "Impression"),
    ];
    for (step, fr) in steps {
        if let Some(rep) = progress {
            rep.tick(format!("{label} — {fr}"), Some(key), step);
        }
        match step {
            "privileges" => trigger_privileges(db, cfg)?,
            "validations" => trigger_validations(db, data_dir, cfg)?,
            "knowledge" => knowledge::write_screen_knowledge(data_dir, cfg)?,
            "folders" => trigger_folders(data_dir, cfg)?,
            "print" => trigger_print_model(db, cfg)?,
            _ => {}
        }
    }
    Ok(())
}

/// Trigger alertes : catalogue JSON + base + mémoire IA (erreurs bloquantes, avertissements).
fn trigger_validations(
    db: &Database,
    data_dir: &Path,
    cfg: &ScreenConfigFile,
) -> Result<(), String> {
    let dir = data_dir.join("dda").join("validations");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let catalog = build_validation_catalog(cfg);
    let catalog_json =
        serde_json::to_string_pretty(&catalog).map_err(|e| e.to_string())?;
    fs::write(dir.join(format!("{}.json", cfg.screen.key)), &catalog_json)
        .map_err(|e| e.to_string())?;

    let knowledge = format_validation_knowledge(cfg);
    fs::write(
        dir.join(format!("{}_validations.txt", cfg.screen.key)),
        &knowledge,
    )
    .map_err(|e| e.to_string())?;

    db.conn
        .execute(
            "INSERT INTO dda_validation_rules (screen_key, rules_json, updated_at)
             VALUES (?1, ?2, datetime('now'))
             ON CONFLICT(screen_key) DO UPDATE SET
               rules_json = excluded.rules_json,
               updated_at = excluded.updated_at",
            params![cfg.screen.key, catalog_json],
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

fn trigger_privileges(db: &Database, cfg: &ScreenConfigFile) -> Result<(), String> {
    let privs = [
        &cfg.screen.privileges.view,
        &cfg.screen.privileges.create,
        &cfg.screen.privileges.update,
        &cfg.screen.privileges.delete,
    ];
    let roles = ["role-admin", "role-agent", "role-directeur", "role-tech", "role-compta"];
    for role_id in roles {
        for privilege in privs {
            let _ = db.conn.execute(
                "INSERT OR IGNORE INTO role_privileges (role_id, privilege) VALUES (?1, ?2)",
                params![role_id, privilege],
            );
        }
        if let Some(p) = &cfg.screen.privileges.import {
            let _ = db.conn.execute(
                "INSERT OR IGNORE INTO role_privileges (role_id, privilege) VALUES (?1, ?2)",
                params![role_id, p],
            );
        }
        if let Some(p) = &cfg.screen.privileges.export {
            let _ = db.conn.execute(
                "INSERT OR IGNORE INTO role_privileges (role_id, privilege) VALUES (?1, ?2)",
                params![role_id, p],
            );
        }
    }
    Ok(())
}

fn trigger_folders(data_dir: &Path, cfg: &ScreenConfigFile) -> Result<(), String> {
    let Some(storage) = &cfg.screen.storage else {
        return Ok(());
    };
    for folder in &storage.folders {
        let path = data_dir.join(folder);
        fs::create_dir_all(&path).map_err(|e| format!("Dossier {path:?} : {e}"))?;
    }
    Ok(())
}

fn trigger_print_model(db: &Database, cfg: &ScreenConfigFile) -> Result<(), String> {
    let Some(print) = &cfg.screen.print else {
        return Ok(());
    };
    if !print.enabled || !print.single_object {
        return Ok(());
    }
    let screen_key = &print.screen_key;
    let base_name = print
        .template_name
        .clone()
        .unwrap_or_else(|| format!("Fiche {}", cfg.screen.label));
    let html = build_fiche_html_from_config(cfg);
    let description = auto_print_description("fiche", screen_key);
    db.sync_auto_print_model(
        screen_key,
        &base_name,
        &description,
        &html,
        FICHE_CSS,
        "Fiche",
    )
    .map_err(|e| e.to_string())
}
