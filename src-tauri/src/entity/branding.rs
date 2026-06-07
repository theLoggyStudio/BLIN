use std::path::Path;

use tauri::{AppHandle, Manager};

use super::registry;

pub const DEFAULT_APP_NAME: &str = "Blin";
pub const DEFAULT_SLOGAN: &str = "Gestion par entités";

/// Titre barre de fenêtre / onglet : « Écosystème — Slogan ».
pub fn window_title(name: &str, slogan: &str) -> String {
    let t = name.trim();
    let s = slogan.trim();
    if !t.is_empty() && !s.is_empty() {
        format!("{t} — {s}")
    } else if !t.is_empty() {
        t.to_string()
    } else if !s.is_empty() {
        s.to_string()
    } else {
        DEFAULT_APP_NAME.to_string()
    }
}

/// Met à jour le titre et l'icône de la fenêtre principale (barre + barre des tâches).
pub fn apply_window_branding(app: &AppHandle, data_dir: &Path) -> Result<(), String> {
    let (name, slogan) = load_branding(data_dir);
    let title = window_title(&name, &slogan);
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Fenêtre principale introuvable.".to_string())?;
    window.set_title(&title).map_err(|e| e.to_string())?;

    let icon_path = super::logo::ensure_ecosystem_icon_png(data_dir)?;
    let image = tauri::image::Image::from_path(&icon_path).map_err(|e| e.to_string())?;
    window.set_icon(image).map_err(|e| e.to_string())?;
    Ok(())
}

/// Nom et slogan affichés (sidebar, exports, mobile distant).
pub fn load_branding(data_dir: &Path) -> (String, String) {
    let reg = registry::load(data_dir).unwrap_or_default();
    let name = reg
        .ecosysteme
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_APP_NAME.to_string());
    let slogan = reg
        .slogan
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_SLOGAN.to_string());
    (name, slogan)
}

pub fn ecosystem_name(data_dir: &Path) -> String {
    load_branding(data_dir).0
}

pub fn ecosystem_slogan(data_dir: &Path) -> String {
    load_branding(data_dir).1
}
