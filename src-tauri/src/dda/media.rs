use std::fs;
use std::path::{Path, PathBuf};

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use uuid::Uuid;

pub fn absolute_path(data_dir: &Path, relative: &str) -> Result<PathBuf, String> {
    if relative.trim().is_empty() {
        return Err("Chemin média vide".to_string());
    }
    if relative.contains("..") {
        return Err("Chemin média invalide".to_string());
    }
    let full = data_dir.join(relative.replace('\\', "/"));
    if !full.starts_with(data_dir) {
        return Err("Accès média refusé".to_string());
    }
    if !full.is_file() {
        return Err(format!("Fichier introuvable : {relative}"));
    }
    Ok(full)
}

pub fn save_media(
    data_dir: &Path,
    folder: &str,
    entity_id: &str,
    original_name: &str,
    bytes: &[u8],
) -> Result<String, String> {
    if entity_id.trim().is_empty() {
        return Err("Identifiant entité requis pour l'upload".to_string());
    }
    if folder.contains("..") || entity_id.contains("..") {
        return Err("Chemin de stockage invalide".to_string());
    }
    let ext = extension_from_name(original_name);
    let filename = format!("{}.{}", Uuid::new_v4(), ext);
    let relative = format!(
        "{}/{}/{}",
        folder.trim_end_matches('/'),
        entity_id,
        filename
    );
    let dest = data_dir.join(&relative);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&dest, bytes).map_err(|e| e.to_string())?;
    Ok(relative.replace('\\', "/"))
}

pub fn delete_media(data_dir: &Path, relative: &str) -> Result<(), String> {
    if relative.trim().is_empty() {
        return Ok(());
    }
    let path = absolute_path(data_dir, relative)?;
    fs::remove_file(&path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn decode_base64(data_base64: &str) -> Result<Vec<u8>, String> {
    B64
        .decode(data_base64.trim())
        .map_err(|e| format!("Base64 invalide : {e}"))
}

/// Déplace les fichiers d'un brouillon (`_draft/{draft_id}`) vers le dossier définitif de l'entité.
pub fn relocate_draft_media(
    data_dir: &Path,
    folder: &str,
    draft_id: &str,
    entity_id: &str,
) -> Result<(), String> {
    let draft_dir = data_dir.join(folder).join("_draft").join(draft_id);
    if !draft_dir.is_dir() {
        return Ok(());
    }
    let target_dir = data_dir.join(folder).join(entity_id);
    fs::create_dir_all(&target_dir).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(&draft_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        let dest = target_dir.join(&name);
        if dest.exists() {
            fs::remove_file(&dest).ok();
        }
        fs::rename(entry.path(), dest).map_err(|e| e.to_string())?;
    }
    fs::remove_dir_all(&draft_dir).ok();
    Ok(())
}

pub fn rewrite_path_after_relocate(relative: &str, draft_id: &str, entity_id: &str) -> String {
    relative.replace(&format!("_draft/{draft_id}"), entity_id)
}

pub fn is_draft_path(relative: &str, draft_id: &str) -> bool {
    relative.contains(&format!("_draft/{draft_id}"))
}

fn extension_from_name(name: &str) -> String {
    Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .filter(|e| !e.is_empty())
        .unwrap_or("jpg")
        .to_ascii_lowercase()
}
