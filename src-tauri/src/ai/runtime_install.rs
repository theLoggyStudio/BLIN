//! Téléchargement et déploiement de llama-server + modèle GGUF dans un dossier choisi.

use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::Command;

use reqwest::blocking::Client;
use serde::Serialize;

use crate::ai::config::{
    bundle_name_for_backend, LlamaBackendKind, MODEL_BUNDLE_DIR, MODEL_DOWNLOAD_URL,
    MODEL_FILENAME,
};
use crate::ai::hardware_profile::invalidate_cache;
use crate::ai::llama_server::LlamaServer;
use crate::ai::runtime_config::{cpu_bin_dir, save_config};
use crate::db::Database;

const LLAMA_RELEASE_BASE: &str = "https://github.com/ggml-org/llama.cpp/releases/download/b8184";
const CPU_ZIP_NAME: &str = "llama-b8184-bin-win-cpu-x64.zip";

#[derive(Clone, Serialize)]
pub struct AiInstallProgress {
    pub phase: String,
    pub percent: u8,
    pub message: String,
}

pub type ProgressFn = Box<dyn Fn(AiInstallProgress) + Send>;

fn emit(progress: &ProgressFn, phase: &str, percent: u8, message: &str) {
    progress(AiInstallProgress {
        phase: phase.to_string(),
        percent,
        message: message.to_string(),
    });
}

fn http_client() -> Result<Client, String> {
    Client::builder()
        .timeout(std::time::Duration::from_secs(3600))
        .build()
        .map_err(|e| e.to_string())
}

fn download_file(
    url: &str,
    dest: &Path,
    progress: &ProgressFn,
    phase: &str,
    pct_start: u8,
    pct_end: u8,
) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = dest.with_extension("part");
    let client = http_client()?;

    let mut resume_from = 0u64;
    if tmp.is_file() {
        resume_from = fs::metadata(&tmp).map(|m| m.len()).unwrap_or(0);
    }

    let mut req = client.get(url);
    if resume_from > 0 {
        req = req.header("Range", format!("bytes={resume_from}-"));
    }
    let mut resp = req.send().map_err(|e| format!("Telechargement : {e}"))?;
    if !resp.status().is_success() && resp.status().as_u16() != 206 {
        return Err(format!("HTTP {} pour {url}", resp.status()));
    }

    let total = resp
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .map(|n| n + resume_from);

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(resume_from > 0)
        .truncate(resume_from == 0)
        .write(true)
        .open(&tmp)
        .map_err(|e| e.to_string())?;

    let mut buf = [0u8; 256 * 1024];
    let mut downloaded = resume_from;
    loop {
        let n = resp.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        std::io::Write::write_all(&mut file, &buf[..n]).map_err(|e| e.to_string())?;
        downloaded += n as u64;
        if let Some(total) = total {
            if total > 0 {
                let ratio = downloaded as f32 / total as f32;
                let pct = pct_start as f32 + ratio * (pct_end - pct_start) as f32;
                emit(
                    progress,
                    phase,
                    pct.clamp(0.0, 100.0) as u8,
                    &format!(
                        "Telechargement… {} / {} Mo",
                        downloaded / 1_048_576,
                        total / 1_048_576
                    ),
                );
            }
        }
    }
    drop(file);
    fs::rename(&tmp, dest).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(windows)]
fn expand_zip(zip_path: &Path, dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    let zip = zip_path.to_string_lossy().replace('\'', "''");
    let out = dest.to_string_lossy().replace('\'', "''");
    let status = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &format!("Expand-Archive -LiteralPath '{zip}' -DestinationPath '{out}' -Force"),
        ])
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("Extraction ZIP echouee (Expand-Archive).".into());
    }
    Ok(())
}

#[cfg(not(windows))]
fn expand_zip(_zip_path: &Path, _dest: &Path) -> Result<(), String> {
    Err("Installation IA supportee uniquement sous Windows.".into())
}

fn flatten_nested_cpu_dir(dest: &Path) {
    let nested = dest.join(bundle_name_for_backend(LlamaBackendKind::Cpu));
    if nested.is_dir() && nested.join("llama-server.exe").is_file() {
        if let Ok(entries) = fs::read_dir(&nested) {
            for entry in entries.flatten() {
                let target = dest.join(entry.file_name());
                let _ = fs::rename(entry.path(), target);
            }
        }
        let _ = fs::remove_dir_all(&nested);
    }
}

pub fn install_to(
    db: &Database,
    install_root: &Path,
    progress: ProgressFn,
) -> Result<(), String> {
    if install_root.as_os_str().is_empty() {
        return Err("Chemin d'installation vide.".into());
    }
    fs::create_dir_all(install_root).map_err(|e| e.to_string())?;

    emit(&progress, "prepare", 0, "Preparation du dossier Loggy…");

    let cpu_dir = cpu_bin_dir(install_root);
    let server_exe = cpu_dir.join("llama-server.exe");
    if !server_exe.is_file() {
        emit(&progress, "bin", 2, "Telechargement de llama-server (CPU)…");
        let zip_path = install_root.join(CPU_ZIP_NAME);
        let url = format!("{LLAMA_RELEASE_BASE}/{CPU_ZIP_NAME}");
        download_file(&url, &zip_path, &progress, "bin", 2, 18)?;
        emit(&progress, "bin", 20, "Extraction de llama-server…");
        expand_zip(&zip_path, install_root)?;
        flatten_nested_cpu_dir(install_root);
        let _ = fs::remove_file(&zip_path);
        if !server_exe.is_file() {
            return Err(format!(
                "llama-server.exe introuvable apres extraction ({})",
                server_exe.display()
            ));
        }
    } else {
        emit(&progress, "bin", 20, "llama-server deja present.");
    }

    save_config(&db.data_dir, install_root)?;
    invalidate_cache();
    LlamaServer::stop();

    let model_dir = cpu_dir.join(MODEL_BUNDLE_DIR);
    let model_path = model_dir.join(MODEL_FILENAME);
    if !model_path.is_file() {
        emit(
            &progress,
            "model",
            22,
            "Telechargement du modele Ministral 8B (~5 Go)…",
        );
        fs::create_dir_all(&model_dir).map_err(|e| e.to_string())?;
        download_file(
            MODEL_DOWNLOAD_URL,
            &model_path,
            &progress,
            "model",
            22,
            98,
        )?;
    } else {
        emit(&progress, "model", 98, "Modele deja present.");
    }

    if !model_path.is_file() {
        return Err("Modele GGUF absent apres telechargement.".into());
    }

    save_config(&db.data_dir, install_root)?;
    emit(&progress, "done", 100, "Installation Loggy terminee.");
    Ok(())
}
