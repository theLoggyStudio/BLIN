//! Entités « session » métier : contexte actif, filtrage des listes et préremplissage à la création.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use super::registry::{EntityDef, EntityRegistry};
use crate::db::Database;
use crate::dda::config::ScreenConfigFile;

const ACTIVE_SESSION_FILE: &str = "active_business_session.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActiveBusinessSession {
    pub entity_key: String,
    pub record_id: String,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEntityInfo {
    pub key: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionBinding {
    pub field_key: String,
    pub session_entity_key: String,
}

pub fn active_session_path(data_dir: &Path) -> std::path::PathBuf {
    super::registry::entities_dir(data_dir).join(ACTIVE_SESSION_FILE)
}

pub fn load_active(data_dir: &Path) -> Result<Option<ActiveBusinessSession>, String> {
    let path = active_session_path(data_dir);
    if !path.is_file() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let session: ActiveBusinessSession = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    if session.entity_key.trim().is_empty() || session.record_id.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(session))
}

pub fn save_active(data_dir: &Path, session: &ActiveBusinessSession) -> Result<(), String> {
    let dir = super::registry::entities_dir(data_dir);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(session).map_err(|e| e.to_string())?;
    fs::write(active_session_path(data_dir), json).map_err(|e| e.to_string())
}

pub fn clear_active(data_dir: &Path) -> Result<(), String> {
    let path = active_session_path(data_dir);
    if path.is_file() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn entity_is_session(registry: &EntityRegistry, entity_key: &str) -> bool {
    registry
        .find(entity_key)
        .map(|e| e.is_session)
        .unwrap_or(false)
}

pub fn list_session_entities(registry: &EntityRegistry) -> Vec<SessionEntityInfo> {
    registry
        .entities
        .iter()
        .filter(|e| e.is_session)
        .map(|e| SessionEntityInfo {
            key: e.nom.clone(),
            label: e
                .label
                .clone()
                .unwrap_or_else(|| e.nom.clone()),
        })
        .collect()
}

/// Premier attribut `entity` pointant vers une entité `is_session`.
pub fn session_ref_binding(
    registry: &EntityRegistry,
    entity_key: &str,
) -> Option<SessionBinding> {
    let ent = registry.find(entity_key)?;
    for attr in &ent.attributs {
        if attr.attr_type != "entity" {
            continue;
        }
        let ref_key = attr.r#ref.as_deref()?.trim();
        if ref_key.is_empty() {
            continue;
        }
        if entity_is_session(registry, ref_key) {
            return Some(SessionBinding {
                field_key: attr.nom.clone(),
                session_entity_key: ref_key.to_string(),
            });
        }
    }
    None
}

fn value_is_empty(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => true,
        Some(Value::String(s)) => s.trim().is_empty(),
        Some(Value::Array(a)) => a.is_empty(),
        _ => false,
    }
}

/// Ajoute un filtre égal sur la liaison session si une session métier est active.
pub fn merge_active_session_filter(
    data_dir: &Path,
    registry: &EntityRegistry,
    screen_key: &str,
    filters: &mut std::collections::HashMap<String, String>,
) -> Result<(), String> {
    if entity_is_session(registry, screen_key) {
        return Ok(());
    }
    let Some(binding) = session_ref_binding(registry, screen_key) else {
        return Ok(());
    };
    let Some(active) = load_active(data_dir)? else {
        return Ok(());
    };
    if active.entity_key != binding.session_entity_key {
        return Ok(());
    }
    if filters
        .get(&binding.field_key)
        .is_some_and(|v| !v.trim().is_empty())
    {
        return Ok(());
    }
    filters.insert(binding.field_key.clone(), active.record_id.clone());
    Ok(())
}

/// Préremplit la liaison session à la création si le champ est vide.
pub fn apply_active_session_on_create(
    db: &Database,
    registry: &EntityRegistry,
    screen_key: &str,
    data: &mut Map<String, Value>,
) -> Result<(), String> {
    if entity_is_session(registry, screen_key) {
        return Ok(());
    }
    let Some(binding) = session_ref_binding(registry, screen_key) else {
        return Ok(());
    };
    let Some(active) = load_active(&db.data_dir)? else {
        return Ok(());
    };
    if active.entity_key != binding.session_entity_key {
        return Ok(());
    }
    if !value_is_empty(data.get(&binding.field_key)) {
        return Ok(());
    }
    let parent_ent = registry
        .find(screen_key)
        .ok_or_else(|| format!("Entité « {screen_key} » introuvable."))?;
    let parent_attr = parent_ent
        .attributs
        .iter()
        .find(|a| a.nom == binding.field_key && a.attr_type == "entity")
        .ok_or_else(|| format!("Attribut session « {} » introuvable.", binding.field_key))?;

    if parent_attr.relation_multiple {
        let session_ent = registry
            .find(&active.entity_key)
            .ok_or_else(|| format!("Entité session « {} » introuvable.", active.entity_key))?;
        let child = super::embed::resolve_child(registry, parent_attr)
            .ok_or_else(|| format!("Entité fille « {} » introuvable.", binding.field_key))?;
        let session_row = crate::dda::crud::get_row(
            db,
            &super::config::build_screen_config(session_ent, registry, &db.data_dir),
            &active.record_id,
        )
        .map_err(|_| format!("Enregistrement session introuvable ({})", active.record_id))?;
        let item = super::embed::child_object_from_row_for_embed_list(child, &session_row);
        data.insert(
            binding.field_key.clone(),
            Value::String(serde_json::to_string(&vec![item]).map_err(|e| e.to_string())?),
        );
    } else {
        let copied = super::relations::embed_values_from_record(
            db,
            &db.data_dir,
            screen_key,
            &binding.field_key,
            &active.record_id,
        )?;
        for (k, v) in copied {
            data.insert(k, v);
        }
    }
    Ok(())
}

/// Libellé d'un enregistrement session pour l'affichage (sidebar).
pub fn record_display_label(ent: &EntityDef, row: &Map<String, Value>) -> String {
    const PRIORITY: &[&str] = &["libelle", "nom", "titre", "reference", "intitule"];
    for key in PRIORITY {
        if let Some(Value::String(s)) = row.get(*key) {
            let t = s.trim();
            if !t.is_empty() {
                return t.to_string();
            }
        }
    }
    for (_col, raw) in row {
        if let Some(label) = label_from_embed_value(raw) {
            return label;
        }
    }
    row.get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            ent.label
                .clone()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| ent.nom.clone())
        })
}

fn label_from_embed_value(raw: &Value) -> Option<String> {
    let parsed = match raw {
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                return None;
            }
            serde_json::from_str::<Value>(t).ok().or_else(|| Some(Value::String(s.clone())))
        }
        other => Some(other.clone()),
    }?;
    const KEYS: &[&str] = &["libelle", "nom", "titre", "reference", "intitule"];
    match &parsed {
        Value::Array(items) => {
            for item in items {
                let map = item.as_object()?;
                for key in KEYS {
                    if let Some(Value::String(s)) = map.get(*key) {
                        let t = s.trim();
                        if !t.is_empty() {
                            return Some(t.to_string());
                        }
                    }
                }
            }
            None
        }
        Value::Object(map) => {
            for key in KEYS {
                if let Some(Value::String(s)) = map.get(*key) {
                    let t = s.trim();
                    if !t.is_empty() {
                        return Some(t.to_string());
                    }
                }
            }
            None
        }
        _ => None,
    }
}

pub fn resolve_record_label(
    db: &Database,
    data_dir: &Path,
    registry: &EntityRegistry,
    entity_key: &str,
    record_id: &str,
) -> Result<String, String> {
    let ent = registry
        .find(entity_key)
        .ok_or_else(|| format!("Entité « {entity_key} » introuvable."))?;
    let cfg = super::config::build_screen_config(ent, registry, data_dir);
    let row = crate::dda::crud::get_row(db, &cfg, record_id)
        .map_err(|_| format!("Enregistrement session introuvable ({record_id})."))?;
    Ok(record_display_label(ent, &row))
}

/// Après création d'une entité session : l'activer comme contexte courant.
pub fn activate_if_session_entity(
    data_dir: &Path,
    registry: &EntityRegistry,
    entity_key: &str,
    created_row: &Map<String, Value>,
) -> Result<(), String> {
    if !entity_is_session(registry, entity_key) {
        return Ok(());
    }
    let record_id = created_row
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("Identifiant session introuvable après création.")?;
    let ent = registry
        .find(entity_key)
        .ok_or_else(|| format!("Entité « {entity_key} » introuvable."))?;
    let label = record_display_label(ent, created_row);
    save_active(
        data_dir,
        &ActiveBusinessSession {
            entity_key: entity_key.to_string(),
            record_id: record_id.to_string(),
            label: Some(label),
        },
    )
}

pub fn binding_for_screen(
    registry: &EntityRegistry,
    cfg: &ScreenConfigFile,
) -> Option<SessionBinding> {
    session_ref_binding(registry, &cfg.screen.key)
}
