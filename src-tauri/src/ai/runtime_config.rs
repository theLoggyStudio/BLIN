//! Emplacement d'installation Loggy (IA) — distinct des données applicatives.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::ai::config::{bundle_name_for_backend, default_model_path, LlamaBackendKind};

static INSTALL_ROOT: Mutex<Option<PathBuf>> = Mutex::new(None);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiInstallConfig {
    pub install_dir: String,
    #[serde(default)]
    pub installed_at: Option<String>,
}

pub fn config_path(data_dir: &Path) -> PathBuf {
    data_dir.join("ai").join("install.json")
}

pub fn load_config(data_dir: &Path) -> Option<AiInstallConfig> {
    let path = config_path(data_dir);
    let raw = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn save_config(data_dir: &Path, install_dir: &Path) -> Result<(), String> {
    let ai_dir = data_dir.join("ai");
    fs::create_dir_all(&ai_dir).map_err(|e| e.to_string())?;
    let cfg = AiInstallConfig {
        install_dir: install_dir.to_string_lossy().to_string(),
        installed_at: Some(chrono::Utc::now().to_rfc3339()),
    };
    let json = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
    fs::write(config_path(data_dir), json).map_err(|e| e.to_string())?;
    refresh_from_data_dir(data_dir);
    Ok(())
}

pub fn refresh_from_data_dir(data_dir: &Path) {
    let root = load_config(data_dir).map(|c| PathBuf::from(c.install_dir));
    if let Ok(mut guard) = INSTALL_ROOT.lock() {
        *guard = root;
    }
}

pub fn install_root() -> Option<PathBuf> {
    INSTALL_ROOT.lock().ok().and_then(|g| g.clone())
}

pub fn configured_install_dir(data_dir: &Path) -> Option<PathBuf> {
    load_config(data_dir).map(|c| PathBuf::from(c.install_dir))
}

/// Dossier suggéré : %LOCALAPPDATA%\Blin\Loggy
pub fn default_install_dir() -> PathBuf {
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        return PathBuf::from(local).join("Blin").join("Loggy");
    }
    if let Ok(home) = std::env::var("USERPROFILE") {
        return PathBuf::from(home).join("Blin").join("Loggy");
    }
    PathBuf::from(r"C:\Blin\Loggy")
}

pub fn cpu_bin_dir(install_root: &Path) -> PathBuf {
    install_root.join(bundle_name_for_backend(LlamaBackendKind::Cpu))
}

pub fn is_runtime_ready_at(install_root: &Path) -> bool {
    let bin = cpu_bin_dir(install_root).join("llama-server.exe");
    bin.is_file() && default_model_path().is_file()
}

pub fn runtime_ready() -> bool {
    default_model_path().is_file()
        && crate::ai::config::runtime_bundle_dir(bundle_name_for_backend(LlamaBackendKind::Cpu))
            .join("llama-server.exe")
            .is_file()
}
