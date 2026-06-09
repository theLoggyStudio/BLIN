use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::attr_types::{is_reserved_attribute, normalize_attribute};
use super::logo;
use super::{generated_config_dir, registry_path};

pub const LOGO_FILENAME: &str = "logo.base64";

/// Entités obsolètes retirées du registre à la normalisation (ex. typo « atricles »).
pub const ORPHAN_ENTITY_KEYS: &[&str] = &["atricles"];

pub fn is_orphan_entity_key(key: &str) -> bool {
    ORPHAN_ENTITY_KEYS
        .iter()
        .any(|k| k.eq_ignore_ascii_case(key.trim()))
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntityAttribute {
    #[serde(default)]
    pub nom: String,
    #[serde(rename = "type")]
    #[serde(default = "default_attr_type_string")]
    pub attr_type: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub r#ref: Option<String>,
    #[serde(default)]
    pub relation_multiple: bool,
    #[serde(default = "default_relation_exclusive_parent")]
    pub relation_exclusive_parent: bool,
    #[serde(default)]
    pub default: Option<Value>,
    #[serde(default)]
    pub enum_options: Option<Vec<String>>,
    #[serde(default)]
    pub relation_impact_source: Option<String>,
    #[serde(default)]
    pub relation_impact_target: Option<String>,
    #[serde(default)]
    pub relation_impact_action: Option<String>,
    #[serde(default)]
    pub relation_impact_defer: bool,
}

fn default_attr_type_string() -> String {
    "string".into()
}

fn default_ai_suggestions() -> bool {
    true
}

fn default_relation_exclusive_parent() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDef {
    pub nom: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// Afficher dans la barre de suggestions du tableau de bord / Loggy.
    #[serde(default = "default_ai_suggestions")]
    pub ai_suggestions: bool,
    /// Trigger système : tâches de signature auto à chaque création (une par rôle signataire).
    #[serde(default, alias = "requires_validation")]
    pub requires_signature: bool,
    /// Identifiants de rôles SQLite autorisés à signer (ex. role-admin, role-directeur).
    #[serde(default, alias = "validator_role_ids")]
    pub signatory_role_ids: Vec<String>,
    /// Contexte métier : enregistrements = sessions actives (filtrage / préremplissage des liaisons).
    #[serde(default)]
    pub is_session: bool,
    pub attributs: Vec<EntityAttribute>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EntityRegistry {
    #[serde(default)]
    pub ecosysteme: Option<String>,
    /// Slogan affiché sous le nom de l'écosystème (sidebar).
    #[serde(default)]
    pub slogan: Option<String>,
    /// URL optionnelle (import JSON) — téléchargée en base64 à l'enregistrement si présente.
    #[serde(default)]
    pub logo_url: Option<String>,
    /// Aperçu data-URI (chargé depuis le disque, jamais écrit dans registry.json).
    #[serde(default, skip_serializing)]
    pub logo: Option<String>,
    pub entities: Vec<EntityDef>,
}

impl EntityRegistry {
    pub fn find(&self, key: &str) -> Option<&EntityDef> {
        self.entities.iter().find(|e| e.nom == key)
    }
}

pub fn empty_registry() -> EntityRegistry {
    EntityRegistry::default()
}

pub fn entities_dir(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("entities")
}

pub fn logo_path(data_dir: &Path) -> std::path::PathBuf {
    entities_dir(data_dir).join(LOGO_FILENAME)
}

/// `true` si le formulaire contient une liaison `entity` vers une entité avec `ai_suggestions: false`.
pub fn qualifies_for_ai_suggestions(registry: &EntityRegistry, ent: &EntityDef) -> bool {
    if ent.nom == super::stock::STOCK_ENTITY_KEY || ent.nom == "tache" {
        return false;
    }
    if is_orphan_entity_key(&ent.nom) {
        return false;
    }
    let has_entity_link = ent.attributs.iter().any(|a| a.attr_type == "entity");
    if !has_entity_link {
        return true;
    }
    ent.attributs.iter().any(|a| {
        if a.attr_type != "entity" {
            return false;
        }
        let Some(ref_name) = a.r#ref.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
            return false;
        };
        registry
            .find(ref_name)
            .map(|t| !t.ai_suggestions)
            .unwrap_or(false)
    })
}

/// Barre de commande : uniquement les entités qui référencent une fiche « technique » (non suggérée).
pub fn apply_ai_suggestions_visibility(registry: &mut EntityRegistry) {
    let names: Vec<String> = registry.entities.iter().map(|e| e.nom.clone()).collect();
    for nom in names {
        let is_session = registry
            .find(&nom)
            .map(|e| e.is_session)
            .unwrap_or(false);
        let visible = if is_session {
            true
        } else {
            registry
                .find(&nom)
                .map(|e| qualifies_for_ai_suggestions(registry, e))
                .unwrap_or(false)
        };
        if let Some(ent) = registry.entities.iter_mut().find(|e| e.nom == nom) {
            ent.ai_suggestions = visible;
        }
    }
}

pub fn normalize_registry(mut registry: EntityRegistry) -> EntityRegistry {
    registry
        .entities
        .retain(|e| !is_orphan_entity_key(&e.nom));
    for ent in &mut registry.entities {
        ent.attributs.retain(|a| !is_reserved_attribute(a));
        for attr in &mut ent.attributs {
            normalize_attribute(attr);
        }
        if ent.nom != super::stock::STOCK_ENTITY_KEY && ent.nom != "tache" {
            ent.is_session = true;
        }
        if ent.nom == super::stock::STOCK_ENTITY_KEY || ent.nom == "tache" {
            ent.ai_suggestions = false;
        }
        if !ent.requires_signature {
            ent.signatory_role_ids.clear();
        } else {
            ent.signatory_role_ids.retain(|id| !id.trim().is_empty());
            ent.signatory_role_ids.sort();
            ent.signatory_role_ids.dedup();
        }
    }
    registry
}

pub fn load_logo_from_disk(data_dir: &Path) -> Option<String> {
    let path = logo_path(data_dir);
    if path.is_file() {
        fs::read_to_string(&path).ok().filter(|s| !s.trim().is_empty())
    } else {
        None
    }
}

pub fn persist_logo(data_dir: &Path, logo_data_uri: Option<&str>) -> Result<(), String> {
    let dir = entities_dir(data_dir);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = logo_path(data_dir);
    match logo_data_uri.filter(|s| !s.trim().is_empty()) {
        Some(content) => fs::write(&path, content.trim()).map_err(|e| e.to_string()),
        None => {
            if path.is_file() {
                let _ = fs::remove_file(&path);
            }
            Ok(())
        }
    }
}

/// Télécharge le logo depuis `logo_url` et l'enregistre sur disque.
pub fn sync_logo_from_url(data_dir: &Path, logo_url: Option<&str>) -> Result<Option<String>, String> {
    let Some(url) = logo_url.map(str::trim).filter(|s| !s.is_empty()) else {
        persist_logo(data_dir, None)?;
        return Ok(None);
    };
    let data_uri = logo::fetch_from_url(url)?;
    persist_logo(data_dir, Some(&data_uri))?;
    Ok(Some(data_uri))
}

pub fn load(data_dir: &Path) -> Result<EntityRegistry, String> {
    let path = registry_path(data_dir);
    if !path.is_file() {
        return Ok(empty_registry());
    }
    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    if raw.trim().is_empty() {
        return Ok(empty_registry());
    }
    let mut registry: EntityRegistry =
        serde_json::from_str(&raw).map_err(|e| format!("registry.json invalide : {e}"))?;
    registry.logo = load_logo_from_disk(data_dir);
    Ok(normalize_registry(registry))
}

pub fn save(data_dir: &Path, registry: &EntityRegistry) -> Result<(), String> {
    let path = registry_path(data_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    if let Some(logo) = registry
        .logo
        .as_deref()
        .filter(|s| s.starts_with("data:"))
    {
        persist_logo(data_dir, Some(logo))?;
        logo::persist_ecosystem_icon_png(data_dir, Some(logo))?;
    } else if let Some(url) = registry.logo_url.as_deref().filter(|s| !s.is_empty()) {
        if let Ok(Some(uri)) = sync_logo_from_url(data_dir, Some(url)) {
            logo::persist_ecosystem_icon_png(data_dir, Some(&uri))?;
        }
    }

    let mut to_write = registry.clone();
    to_write.logo = None;
    let json = serde_json::to_string_pretty(&to_write).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

pub fn list_keys_in_generated_dir(data_dir: &Path) -> Result<Vec<String>, String> {
    let dir = generated_config_dir(data_dir);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut keys = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                keys.push(stem.to_string());
            }
        }
    }
    Ok(keys)
}

/// Correspondance locale (sans appel LLM) pour le tableau de bord.
pub fn match_intent(message: &str, registry: &EntityRegistry) -> Option<String> {
    super::intent::match_intent(message, registry)
}
