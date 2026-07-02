use std::fs;
use std::path::Path;

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};

use super::registry::entities_dir;

const MAX_LOGO_BYTES: usize = 5 * 1024 * 1024;

pub const ECOSYSTEM_ICON_FILENAME: &str = "ecosystem-icon.png";

/// Icône Blin embarquée (générée depuis `public/logo.png` via `npm run tauri:icons`).
pub fn default_icon_png_bytes() -> &'static [u8] {
    include_bytes!("../../icons/icon.png")
}

/// PNG carré (512×512) adapté barre des tâches / set_icon.
pub fn square_png_for_taskbar(bytes: &[u8], size: u32) -> Result<Vec<u8>, String> {
    use image::imageops::FilterType;
    use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};

    let img = image::load_from_memory(bytes).map_err(|e| format!("Image invalide : {e}"))?;
    let (iw, ih) = (img.width(), img.height());
    if iw == 0 || ih == 0 {
        return Err("Image vide.".into());
    }
    let scale = (size as f32 / iw.max(ih) as f32).min(1.0);
    let w = ((iw as f32 * scale).round() as u32).max(1);
    let h = ((ih as f32 * scale).round() as u32).max(1);
    let resized = img.resize(w, h, FilterType::Lanczos3);
    let mut canvas = RgbaImage::from_pixel(size, size, Rgba([0, 0, 0, 255]));
    image::imageops::overlay(
        &mut canvas,
        &resized.to_rgba8(),
        ((size - w) / 2).into(),
        ((size - h) / 2).into(),
    );
    let mut out = Vec::new();
    DynamicImage::ImageRgba8(canvas)
        .write_to(&mut std::io::Cursor::new(&mut out), ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(out)
}

/// Indique si un logo écosystème personnalisé est enregistré sur disque.
pub fn has_custom_ecosystem_icon(data_dir: &Path) -> bool {
    let logo_path = entities_dir(data_dir).join("logo.base64");
    logo_path.is_file()
        && fs::read_to_string(&logo_path)
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false)
}

/// Garantit `entities/ecosystem-icon.png` sans réécrire un fichier déjà présent.
pub fn ensure_ecosystem_icon_png(data_dir: &Path) -> Result<std::path::PathBuf, String> {
    let dir = entities_dir(data_dir);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(ECOSYSTEM_ICON_FILENAME);
    if path.is_file() {
        return Ok(path);
    }
    if has_custom_ecosystem_icon(data_dir) {
        let logo_path = dir.join(super::registry::LOGO_FILENAME);
        let logo = fs::read_to_string(&logo_path).map_err(|e| e.to_string())?;
        persist_ecosystem_icon_png(data_dir, Some(logo.trim()))?;
    } else {
        fs::write(&path, default_icon_png_bytes()).map_err(|e| e.to_string())?;
    }
    Ok(path)
}

/// Décode un data-URI image en octets bruts.
pub fn decode_data_uri(data_uri: &str) -> Result<Vec<u8>, String> {
    let uri = data_uri.trim();
    let Some((header, payload)) = uri.split_once(',') else {
        return Err("Data-URI invalide.".into());
    };
    if !header.starts_with("data:") {
        return Err("Data-URI invalide.".into());
    }
    B64.decode(payload.trim()).map_err(|e| format!("Décodage base64 : {e}"))
}

/// PNG binaire pour régénération des icônes Tauri (`npm run tauri:icons`).
pub fn persist_ecosystem_icon_png(data_dir: &Path, logo_data_uri: Option<&str>) -> Result<(), String> {
    let Some(uri) = logo_data_uri.filter(|s| s.starts_with("data:")) else {
        return Ok(());
    };
    let bytes = decode_data_uri(uri)?;
    if bytes.is_empty() {
        return Err("Image vide.".into());
    }
    if bytes.len() > MAX_LOGO_BYTES {
        return Err(format!(
            "Image trop volumineuse (max {} Mo).",
            MAX_LOGO_BYTES / 1024 / 1024
        ));
    }
    let squared = square_png_for_taskbar(&bytes, 512)?;
    let dir = entities_dir(data_dir);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    fs::write(dir.join(ECOSYSTEM_ICON_FILENAME), &squared).map_err(|e| e.to_string())
}

fn mime_from_content_type(header: Option<&str>, bytes: &[u8]) -> String {
    if let Some(ct) = header {
        let ct = ct.split(';').next().unwrap_or(ct).trim().to_lowercase();
        if ct.starts_with("image/") {
            return ct;
        }
    }
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        return "image/png".into();
    }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return "image/jpeg".into();
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return "image/webp".into();
    }
    if bytes.starts_with(b"GIF8") {
        return "image/gif".into();
    }
    "image/png".into()
}

/// Télécharge une image depuis une URL HTTP(S) et renvoie un data-URI base64.
pub fn fetch_from_url(url: &str) -> Result<String, String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("URL du logo vide.".into());
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("L'URL du logo doit commencer par http:// ou https://".into());
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(45))
        .user_agent("Blin/1.0")
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(url)
        .send()
        .map_err(|e| format!("Impossible de télécharger le logo : {e}"))?;

    if !resp.status().is_success() {
        return Err(format!(
            "Téléchargement du logo échoué (HTTP {}).",
            resp.status()
        ));
    }

    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    let bytes = resp
        .bytes()
        .map_err(|e| format!("Lecture du logo : {e}"))?;

    if bytes.is_empty() {
        return Err("Le fichier image est vide.".into());
    }
    if bytes.len() > MAX_LOGO_BYTES {
        return Err(format!(
            "Image trop volumineuse ({} o, max {} Mo).",
            bytes.len(),
            MAX_LOGO_BYTES / 1024 / 1024
        ));
    }

    let mime = mime_from_content_type(content_type.as_deref(), &bytes);
    let encoded = B64.encode(bytes.as_ref());
    Ok(format!("data:{mime};base64,{encoded}"))
}
