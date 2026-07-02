//! Catalogue global des définitions de matricule (libellé + base).

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::registry::entities_dir;

const REGISTRY_FILENAME: &str = "matricules.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatriculeDef {
    pub id: String,
    pub libelle: String,
    pub base: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatriculeRegistry {
    pub matricules: Vec<MatriculeDef>,
}

pub fn registry_path(data_dir: &Path) -> std::path::PathBuf {
    entities_dir(data_dir).join(REGISTRY_FILENAME)
}

pub fn load(data_dir: &Path) -> Result<MatriculeRegistry, String> {
    let path = registry_path(data_dir);
    if !path.exists() {
        return Ok(MatriculeRegistry::default());
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("Lecture {path:?} : {e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("JSON matricules : {e}"))
}

pub fn save(data_dir: &Path, registry: &MatriculeRegistry) -> Result<(), String> {
    let path = registry_path(data_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let raw =
        serde_json::to_string_pretty(registry).map_err(|e| format!("Sérialisation matricules : {e}"))?;
    fs::write(&path, raw).map_err(|e| format!("Écriture {path:?} : {e}"))
}

impl MatriculeRegistry {
    pub fn find(&self, id: &str) -> Option<&MatriculeDef> {
        self.matricules.iter().find(|m| m.id == id)
    }

    /// Si l'attribut n'a pas de `matricule_ref`, utilise l'unique définition du catalogue.
    pub fn resolve_for_attr<'a>(&'a self, attr: &super::registry::EntityAttribute) -> Option<&'a MatriculeDef> {
        if let Some(id) = attr
            .matricule_ref
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            return self.find(id);
        }
        if self.matricules.len() == 1 {
            return self.matricules.first();
        }
        None
    }

    /// Lie automatiquement les attributs matricule sans ref quand une seule définition existe.
    pub fn resolve_unlinked_attrs(&self, registry: &mut super::registry::EntityRegistry) {
        if self.matricules.len() != 1 {
            return;
        }
        let id = self.matricules[0].id.clone();
        for ent in &mut registry.entities {
            for attr in &mut ent.attributs {
                if attr.attr_type != "matricule" {
                    continue;
                }
                let empty = attr
                    .matricule_ref
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or("")
                    .is_empty();
                if empty {
                    attr.matricule_ref = Some(id.clone());
                }
            }
        }
    }

    pub fn find_by_libelle(&self, libelle: &str) -> Option<&MatriculeDef> {
        let t = libelle.trim();
        self.matricules
            .iter()
            .find(|m| m.libelle.eq_ignore_ascii_case(t))
    }

    pub fn find_by_base(&self, base: &str) -> Option<&MatriculeDef> {
        let t = base.trim();
        self.matricules
            .iter()
            .find(|m| m.base.eq_ignore_ascii_case(t))
    }
}

fn normalize_base(raw: &str) -> Result<String, String> {
    let t = raw.trim().to_uppercase();
    if t.is_empty() {
        return Err("La base du matricule est obligatoire.".into());
    }
    if !t.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err("La base ne doit contenir que des lettres et chiffres.".into());
    }
    Ok(t)
}

fn normalize_libelle(raw: &str) -> Result<String, String> {
    let t = raw.trim();
    if t.is_empty() {
        return Err("Le libellé du matricule est obligatoire.".into());
    }
    Ok(t.to_string())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatriculeCreatePayload {
    pub libelle: String,
    pub base: String,
}

pub fn create_matricule(
    data_dir: &Path,
    payload: MatriculeCreatePayload,
) -> Result<MatriculeDef, String> {
    let libelle = normalize_libelle(&payload.libelle)?;
    let base = normalize_base(&payload.base)?;
    let mut registry = load(data_dir)?;
    if registry.find_by_libelle(&libelle).is_some() {
        return Err(format!(
            "Un matricule avec le libellé « {libelle} » existe déjà."
        ));
    }
    if registry.find_by_base(&base).is_some() {
        return Err(format!(
            "Un matricule avec la base « {base} » existe déjà."
        ));
    }
    let def = MatriculeDef {
        id: Uuid::new_v4().to_string(),
        libelle,
        base,
    };
    registry.matricules.push(def.clone());
    save(data_dir, &registry)?;
    Ok(def)
}
