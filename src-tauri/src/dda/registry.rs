use std::fs;
use std::path::Path;

use super::config::ScreenConfigFile;

pub fn load_all_screen_configs(dir: &Path) -> Result<Vec<ScreenConfigFile>, String> {
    let mut configs = Vec::new();
    let entries = fs::read_dir(dir).map_err(|e| format!("Lecture {dir:?} : {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
            if name.starts_with('_') {
                continue;
            }
            if name.contains(".registry") {
                continue;
            }
        }
        let raw = fs::read_to_string(&path).map_err(|e| format!("JSON {path:?} : {e}"))?;
        if !raw.contains("\"screen\"") {
            continue;
        }
        configs.push(load_screen_config_file(&path)?);
    }
    configs.sort_by(|a, b| a.screen.label.cmp(&b.screen.label));
    Ok(configs)
}

pub fn load_screen_config_file(path: &Path) -> Result<ScreenConfigFile, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("JSON {path:?} : {e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("Parse {path:?} : {e}"))
}
