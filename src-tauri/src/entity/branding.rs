use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Mutex;
use std::time::UNIX_EPOCH;

use tauri::{AppHandle, Manager};

use super::registry;

pub const DEFAULT_APP_NAME: &str = "Blin";
pub const DEFAULT_SLOGAN: &str = "Gestion par entités";

static LAST_TASKBAR_ICON_FINGERPRINT: Mutex<Option<String>> = Mutex::new(None);

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

/// Met à jour uniquement le titre de la fenêtre principale.
pub fn apply_window_title(app: &AppHandle, data_dir: &Path) -> Result<String, String> {
    let (name, slogan) = load_branding(data_dir);
    let title = window_title(&name, &slogan);
    let window = main_window(app)?;
    window.set_title(&title).map_err(|e| e.to_string())?;
    Ok(title)
}

/// Empreinte de l'icône barre des tâches (fichier custom ou PNG embarqué).
pub fn taskbar_icon_fingerprint(data_dir: &Path) -> String {
    if super::logo::has_custom_ecosystem_icon(data_dir) {
        let path = registry::entities_dir(data_dir).join(super::logo::ECOSYSTEM_ICON_FILENAME);
        if let Ok(meta) = std::fs::metadata(&path) {
            let modified = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            return format!("custom:{}:{}", meta.len(), modified);
        }
        return "custom:missing".into();
    }
    let mut hasher = DefaultHasher::new();
    super::logo::default_icon_png_bytes().hash(&mut hasher);
    format!("default:{}", hasher.finish())
}

/// Applique l'icône uniquement si la source a changé (ou `force`).
pub fn apply_window_icon_if_changed(
    app: &AppHandle,
    data_dir: &Path,
    force: bool,
) -> Result<bool, String> {
    let fingerprint = taskbar_icon_fingerprint(data_dir);
    {
        let guard = LAST_TASKBAR_ICON_FINGERPRINT
            .lock()
            .map_err(|e| e.to_string())?;
        if !force && guard.as_deref() == Some(fingerprint.as_str()) {
            return Ok(false);
        }
    }

    let window = main_window(app)?;
    apply_window_icon_on(&window, data_dir)?;
    if let Ok(mut guard) = LAST_TASKBAR_ICON_FINGERPRINT.lock() {
        *guard = Some(fingerprint);
    }
    Ok(true)
}

/// Restaure l'icône barre des tâches (embarquée ou logo écosystème carré).
pub fn apply_window_icon(app: &AppHandle, data_dir: &Path) -> Result<(), String> {
    apply_window_icon_if_changed(app, data_dir, false)?;
    Ok(())
}

fn main_window(app: &AppHandle) -> Result<tauri::WebviewWindow, String> {
    app.get_webview_window("main")
        .ok_or_else(|| "Fenêtre principale introuvable.".to_string())
}

fn apply_window_icon_on(window: &tauri::WebviewWindow, data_dir: &Path) -> Result<(), String> {
    let mut icon = tauri::image::Image::from_bytes(super::logo::default_icon_png_bytes())
        .map_err(|e| e.to_string())?;
    if super::logo::has_custom_ecosystem_icon(data_dir) {
        let icon_path = super::logo::ensure_ecosystem_icon_png(data_dir)?;
        if let Ok(from_disk) = tauri::image::Image::from_path(&icon_path) {
            icon = from_disk;
        }
    }
    window.set_icon(icon).map_err(|e| e.to_string())?;
    Ok(())
}

/// Met à jour le titre ; l'icône barre des tâches seulement si le logo a changé.
pub fn apply_window_branding(app: &AppHandle, data_dir: &Path) -> Result<(), String> {
    apply_window_title(app, data_dir)?;
    apply_window_icon_if_changed(app, data_dir, false)?;
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
