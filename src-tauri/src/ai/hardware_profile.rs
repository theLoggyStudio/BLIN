use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use chrono::Utc;
use reqwest::blocking::Client;
use rusqlite::params;

use crate::ai::config::{
    backend_label, bundle_name_for_backend, default_model_path, runtime_bundle_dir, LlamaBackendKind,
    LlamaRuntime, LLAMA_CTX,
};
use crate::db::Database;

const PROFILE_PORT: u16 = 18082;
const PROFILE_KEY: &str = "default";

static CACHED_RUNTIME: Mutex<Option<LlamaRuntime>> = Mutex::new(None);

#[derive(Clone, Debug)]
pub struct StoredProfile {
    pub backend: LlamaBackendKind,
    pub gpu_layers: u32,
    pub ctx_size: u32,
    pub threads: u32,
    pub load_ms: u64,
    pub profiled_at: String,
    pub bin_fingerprint: String,
}

pub fn cached_runtime() -> Option<LlamaRuntime> {
    CACHED_RUNTIME.lock().ok().and_then(|g| g.clone())
}

pub fn invalidate_cache() {
    if let Ok(mut g) = CACHED_RUNTIME.lock() {
        *g = None;
    }
}

fn bin_dir_for(kind: LlamaBackendKind) -> PathBuf {
    runtime_bundle_dir(bundle_name_for_backend(kind))
}

fn backend_dir_ready(dir: &Path, kind: LlamaBackendKind) -> bool {
    if !dir.join("llama-server.exe").is_file() {
        return false;
    }
    match kind {
        LlamaBackendKind::Cuda => dir.join("ggml-cuda.dll").is_file(),
        LlamaBackendKind::Vulkan => dir.join("ggml-vulkan.dll").is_file(),
        LlamaBackendKind::Cpu => true,
    }
}

fn available_backends() -> Vec<(LlamaBackendKind, PathBuf)> {
    let mut out = Vec::new();
    for kind in [
        LlamaBackendKind::Cuda,
        LlamaBackendKind::Vulkan,
        LlamaBackendKind::Cpu,
    ] {
        let dir = bin_dir_for(kind);
        if backend_dir_ready(&dir, kind) {
            out.push((kind, dir));
        }
    }
    out
}

fn bin_fingerprint() -> String {
    let mut parts: Vec<String> = Vec::new();
    for (kind, dir) in available_backends() {
        let exe = dir.join("llama-server.exe");
        if let Ok(meta) = std::fs::metadata(&exe) {
            parts.push(format!(
                "{:?}:{}",
                kind,
                meta.modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0)
            ));
        }
    }
    parts.join("|")
}

fn default_threads() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4)
        .clamp(2, 8)
}

fn kill_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn health_on_port(port: u16) -> bool {
    let url = format!("http://127.0.0.1:{port}/health");
    let Ok(client) = Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    else {
        return false;
    };
    client
        .get(&url)
        .send()
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

fn benchmark_load(
    bin_dir: &Path,
    gpu_layers: u32,
    threads: u32,
    ctx_size: u32,
    port: u16,
    timeout_secs: u64,
) -> Option<u64> {
    if !default_model_path().is_file() {
        return None;
    }
    let server = bin_dir.join("llama-server.exe");
    if !server.is_file() {
        return None;
    }

    let model = default_model_path();
    let ngl = gpu_layers.to_string();
    let mut child = Command::new(&server)
        .current_dir(bin_dir)
        .args([
            "-m",
            &model.to_string_lossy(),
            "--host",
            "127.0.0.1",
            "--port",
            &port.to_string(),
            "-ngl",
            &ngl,
            "-t",
            &threads.to_string(),
            "--ctx-size",
            &ctx_size.to_string(),
            "--parallel",
            "1",
            "--no-warmup",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let started = Instant::now();
    let max_iters = timeout_secs * 2;
    let mut ok = false;
    for _ in 0..max_iters {
        if health_on_port(port) {
            ok = true;
            break;
        }
        if child.try_wait().ok().flatten().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    let elapsed = started.elapsed().as_millis() as u64;
    kill_child(&mut child);
    if ok {
        Some(elapsed)
    } else {
        None
    }
}

struct BenchCandidate {
    backend: LlamaBackendKind,
    gpu_layers: u32,
    load_ms: u64,
}

fn run_profiling() -> Result<StoredProfile, String> {
    if !default_model_path().is_file() {
        return Err(
            "Modèle GGUF absent. Copiez le fichier dans le dossier d'installation (mode hors ligne)."
                .into(),
        );
    }
    if available_backends().is_empty() {
        return Err("Aucun llama-server local trouvé dans le dossier d'installation.".into());
    }

    let threads = default_threads();
    let ctx_size = LLAMA_CTX;
    let mut candidates: Vec<BenchCandidate> = Vec::new();

    if let Some(ms) = benchmark_load(
        &bin_dir_for(LlamaBackendKind::Cpu),
        0,
        threads,
        ctx_size,
        PROFILE_PORT,
        180,
    ) {
        candidates.push(BenchCandidate {
            backend: LlamaBackendKind::Cpu,
            gpu_layers: 0,
            load_ms: ms,
        });
    }

    let gpu_dir = bin_dir_for(LlamaBackendKind::Cuda);
    if backend_dir_ready(&gpu_dir, LlamaBackendKind::Cuda) {
        for layers in [99u32, 28, 0] {
            if layers == 0 {
                break;
            }
            if let Some(ms) = benchmark_load(&gpu_dir, layers, threads, ctx_size, PROFILE_PORT, 120)
            {
                candidates.push(BenchCandidate {
                    backend: LlamaBackendKind::Cuda,
                    gpu_layers: layers,
                    load_ms: ms,
                });
                break;
            }
        }
    } else if let Some((kind, dir)) = available_backends()
        .into_iter()
        .find(|(k, _)| *k == LlamaBackendKind::Vulkan)
    {
        if let Some(ms) = benchmark_load(&dir, 99, threads, ctx_size, PROFILE_PORT, 120) {
            candidates.push(BenchCandidate {
                backend: kind,
                gpu_layers: 99,
                load_ms: ms,
            });
        }
    }

    let best = candidates
        .iter()
        .min_by_key(|c| c.load_ms)
        .ok_or_else(|| {
            "Impossible de charger le modèle (mémoire insuffisante ?).".to_string()
        })?;

    let cpu_ms = candidates
        .iter()
        .find(|c| c.backend == LlamaBackendKind::Cpu)
        .map(|c| c.load_ms);
    let chosen = if best.backend != LlamaBackendKind::Cpu {
        if let Some(cpu) = cpu_ms {
            if best.load_ms > cpu.saturating_mul(2) {
                candidates
                    .iter()
                    .find(|c| c.backend == LlamaBackendKind::Cpu)
                    .unwrap_or(best)
            } else {
                best
            }
        } else {
            best
        }
    } else {
        best
    };

    Ok(StoredProfile {
        backend: chosen.backend,
        gpu_layers: chosen.gpu_layers,
        ctx_size,
        threads,
        load_ms: chosen.load_ms,
        profiled_at: Utc::now().to_rfc3339(),
        bin_fingerprint: bin_fingerprint(),
    })
}

impl Database {
    pub fn ai_load_runtime_profile(&self) -> Result<Option<StoredProfile>, crate::db::DbError> {
        let mut stmt = self.conn.prepare(
            "SELECT backend, gpu_layers, ctx_size, threads, load_ms, profiled_at, bin_fingerprint
             FROM ai_runtime_profile WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![PROFILE_KEY])?;
        if let Some(row) = rows.next()? {
            let backend_s: String = row.get(0)?;
            let backend = match backend_s.as_str() {
                "cuda" => LlamaBackendKind::Cuda,
                "vulkan" => LlamaBackendKind::Vulkan,
                _ => LlamaBackendKind::Cpu,
            };
            return Ok(Some(StoredProfile {
                backend,
                gpu_layers: row.get(1)?,
                ctx_size: row.get(2)?,
                threads: row.get(3)?,
                load_ms: row.get(4)?,
                profiled_at: row.get(5)?,
                bin_fingerprint: row.get(6)?,
            }));
        }
        Ok(None)
    }

    pub fn ai_save_runtime_profile(&self, p: &StoredProfile) -> Result<(), crate::db::DbError> {
        let backend_s = match p.backend {
            LlamaBackendKind::Cuda => "cuda",
            LlamaBackendKind::Vulkan => "vulkan",
            LlamaBackendKind::Cpu => "cpu",
        };
        self.conn.execute(
            "INSERT INTO ai_runtime_profile (id, backend, gpu_layers, ctx_size, threads, load_ms, profiled_at, bin_fingerprint)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET
               backend = excluded.backend,
               gpu_layers = excluded.gpu_layers,
               ctx_size = excluded.ctx_size,
               threads = excluded.threads,
               load_ms = excluded.load_ms,
               profiled_at = excluded.profiled_at,
               bin_fingerprint = excluded.bin_fingerprint",
            params![
                PROFILE_KEY,
                backend_s,
                p.gpu_layers,
                p.ctx_size,
                p.threads,
                p.load_ms,
                p.profiled_at,
                p.bin_fingerprint,
            ],
        )?;
        Ok(())
    }
}

pub fn stored_to_runtime(p: &StoredProfile) -> LlamaRuntime {
    LlamaRuntime {
        bin_dir: bin_dir_for(p.backend),
        backend: p.backend,
        gpu_layers: p.gpu_layers,
        ctx_size: p.ctx_size,
        threads: p.threads,
    }
}

pub fn ensure_runtime_profile(db: &Database, force: bool) -> Result<LlamaRuntime, String> {
    let fp = bin_fingerprint();
    if !force {
        if let Ok(Some(stored)) = db.ai_load_runtime_profile() {
            if stored.bin_fingerprint == fp {
                let rt = stored_to_runtime(&stored);
                if let Ok(mut g) = CACHED_RUNTIME.lock() {
                    *g = Some(rt.clone());
                }
                return Ok(rt);
            }
        }
        if let Some(rt) = cached_runtime() {
            return Ok(rt);
        }
    }

    invalidate_cache();
    let stored = run_profiling()?;
    db.ai_save_runtime_profile(&stored)
        .map_err(|e| e.to_string())?;
    let rt = stored_to_runtime(&stored);
    if let Ok(mut g) = CACHED_RUNTIME.lock() {
        *g = Some(rt.clone());
    }
    Ok(rt)
}

pub fn resolve_active_runtime(db: Option<&Database>) -> LlamaRuntime {
    if let Some(rt) = cached_runtime() {
        return rt;
    }
    if let Some(db) = db {
        if let Ok(Some(stored)) = db.ai_load_runtime_profile() {
            if stored.bin_fingerprint == bin_fingerprint() {
                let rt = stored_to_runtime(&stored);
                if let Ok(mut g) = CACHED_RUNTIME.lock() {
                    *g = Some(rt.clone());
                }
                return rt;
            }
        }
    }
    // Fallback avant profilage : CUDA si présent, sinon CPU
    for kind in [
        LlamaBackendKind::Cuda,
        LlamaBackendKind::Vulkan,
        LlamaBackendKind::Cpu,
    ] {
        let dir = bin_dir_for(kind);
        if backend_dir_ready(&dir, kind) {
            let gpu_layers = match kind {
                LlamaBackendKind::Cuda | LlamaBackendKind::Vulkan => 99,
                LlamaBackendKind::Cpu => 0,
            };
            return LlamaRuntime {
                bin_dir: dir,
                backend: kind,
                gpu_layers,
                ctx_size: LLAMA_CTX,
                threads: default_threads(),
            };
        }
    }
    LlamaRuntime {
        bin_dir: bin_dir_for(LlamaBackendKind::Cpu),
        backend: LlamaBackendKind::Cpu,
        gpu_layers: 0,
        ctx_size: LLAMA_CTX,
        threads: default_threads(),
    }
}

pub fn profile_summary(db: &Database) -> Result<(bool, String), String> {
    match db.ai_load_runtime_profile() {
        Ok(Some(p)) if p.bin_fingerprint == bin_fingerprint() => {
            let label = backend_label(p.backend);
            Ok((
                true,
                format!(
                    "{label}, {} calques, chargement {} ms",
                    p.gpu_layers, p.load_ms
                ),
            ))
        }
        _ => Ok((false, String::new())),
    }
}
