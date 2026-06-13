//! Alertes succès Loggy — catalogues générés par trigger (personnalisation IA côté front).

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::config::ScreenConfigFile;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuccessAlertsCatalog {
    pub entity_key: String,
    pub entity_label: String,
    pub messages: HashMap<String, String>,
}

fn default_messages(label: &str) -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert(
        "create".into(),
        format!("Enregistrement créé pour « {label} »."),
    );
    m.insert(
        "create_named".into(),
        format!("Fiche « {{record_label}} » créée pour « {label} »."),
    );
    m.insert(
        "create_lines".into(),
        format!("Enregistrement créé pour « {label} » avec {{line_count}} lignes (même matricule)."),
    );
    m.insert(
        "update".into(),
        format!("Fiche « {label} » mise à jour avec succès."),
    );
    m.insert(
        "update_named".into(),
        format!("Fiche « {{record_label}} » mise à jour pour « {label} »."),
    );
    m.insert(
        "update_lines".into(),
        format!("Fiche « {label} » mise à jour ({{line_count}} lignes)."),
    );
    m.insert(
        "delete".into(),
        format!("Enregistrement supprimé pour « {label} »."),
    );
    m.insert(
        "delete_named".into(),
        format!("Fiche « {{record_label}} » supprimée pour « {label} »."),
    );
    m.insert(
        "import_ok".into(),
        format!(
            "Import CSV réussi pour « {label} » : {{inserted}} créé(s), {{updated}} mis à jour."
        ),
    );
    m.insert(
        "import_partial".into(),
        format!(
            "Import CSV partiel pour « {label} » : {{inserted}} créé(s), {{updated}} mis à jour, {{error_count}} erreur(s)."
        ),
    );
    m.insert(
        "export_csv".into(),
        format!("Export CSV terminé pour « {label} » ({{file_name}})."),
    );
    m.insert(
        "export_pdf_row".into(),
        format!("PDF fiche généré pour « {label} »."),
    );
    m.insert(
        "export_pdf_list".into(),
        format!("PDF liste généré pour « {label} »."),
    );
    m.insert(
        "signature_ok".into(),
        format!("Signature enregistrée pour « {label} »."),
    );
    m.insert(
        "signature_refuse".into(),
        format!("Refus de signature enregistré pour « {label} »."),
    );
    m
}

pub fn build_success_catalog(cfg: &ScreenConfigFile) -> SuccessAlertsCatalog {
    let label = cfg.screen.label.clone();
    SuccessAlertsCatalog {
        entity_key: cfg.screen.key.clone(),
        entity_label: label.clone(),
        messages: default_messages(&label),
    }
}

pub fn format_success_knowledge(cfg: &ScreenConfigFile, catalog: &SuccessAlertsCatalog) -> String {
    let mut s = format!(
        "=== ALERTES SUCCÈS — {} ({}) ===\n\
         Généré par trigger_success_alerts. Loggy personnalise ces phrases à l'affichage.\n",
        cfg.screen.key, cfg.screen.label
    );
    for (action, template) in &catalog.messages {
        s.push_str(&format!("  {action}: {template}\n"));
    }
    s
}

pub fn write_success_alerts(data_dir: &Path, cfg: &ScreenConfigFile) -> Result<(), String> {
    let dir = data_dir.join("dda").join("success");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let catalog = build_success_catalog(cfg);
    let json = serde_json::to_string_pretty(&catalog).map_err(|e| e.to_string())?;
    fs::write(dir.join(format!("{}.json", cfg.screen.key)), &json).map_err(|e| e.to_string())?;

    let knowledge = format_success_knowledge(cfg, &catalog);
    fs::write(
        dir.join(format!("{}_success_alerts.txt", cfg.screen.key)),
        &knowledge,
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn load_catalog(data_dir: &Path, entity_key: &str) -> Result<SuccessAlertsCatalog, String> {
    let path = data_dir
        .join("dda")
        .join("success")
        .join(format!("{entity_key}.json"));
    if !path.is_file() {
        let label = entity_key.to_string();
        return Ok(SuccessAlertsCatalog {
            entity_key: entity_key.to_string(),
            entity_label: label.clone(),
            messages: default_messages(&label),
        });
    }
    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

pub fn resolve_message(
    catalog: &SuccessAlertsCatalog,
    action: &str,
    params: &HashMap<String, String>,
) -> Option<String> {
    let template = catalog.messages.get(action)?;
    let mut out = template.clone();
    out = out.replace("{entity_label}", &catalog.entity_label);
    out = out.replace("{entity_key}", &catalog.entity_key);
    for (k, v) in params {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    Some(out)
}

pub fn format_master_success_alerts(data_dir: &Path) -> Result<String, String> {
    let dir = data_dir.join("dda").join("success");
    if !dir.is_dir() {
        return Ok(String::new());
    }
    let mut s = String::from(
        "=== BLIN — ALERTES SUCCÈS ENTITÉS (trigger auto) ===\n\
         Toast Loggy après CRUD, import/export CSV, PDF. Personnalisation : ai_alert_personify.\n\
         Placeholders : {entity_label}, {record_label}, {line_count}, {inserted}, {updated}, {error_count}, {file_name}.\n\n",
    );
    let mut files: Vec<_> = fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|x| x == "txt").unwrap_or(false)
                && p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.ends_with("_success_alerts.txt"))
        })
        .collect();
    files.sort();
    for path in files {
        if let Ok(chunk) = fs::read_to_string(&path) {
            s.push_str(&chunk);
            s.push('\n');
        }
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_substitutes_placeholders() {
        let catalog = SuccessAlertsCatalog {
            entity_key: "client".into(),
            entity_label: "Client".into(),
            messages: default_messages("Client"),
        };
        let mut params = HashMap::new();
        params.insert("inserted".into(), "5".into());
        params.insert("updated".into(), "2".into());
        let msg = resolve_message(&catalog, "import_ok", &params).unwrap();
        assert!(msg.contains("5"));
        assert!(msg.contains("Client"));
    }
}
