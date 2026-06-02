use std::path::{Path, PathBuf};

// Port local du serveur llama (evite conflit avec remote 38472).
pub const LLAMA_PORT: u16 = 18080;

// Contexte reduit pour tenir dans 8 Go VRAM (7B Q5 + KV).
pub const LLAMA_CTX: u32 = 2048;

/// Ministral 8B — multilingue, très bon en français (Mistral AI, 2024).
pub const MODEL_BUNDLE_DIR: &str = "Ministral-8B-Instruct-2410-GGUF";
pub const MODEL_FILENAME: &str = "Ministral-8B-Instruct-2410.Q5_K_S.gguf";
pub const MODEL_DISPLAY_NAME: &str = "Ministral 8B Instruct (optimisé français)";
pub const MODEL_DOWNLOAD_URL: &str =
    "https://huggingface.co/mradermacher/Ministral-8B-Instruct-2410-GGUF/resolve/main/Ministral-8B-Instruct-2410.Q5_K_S.gguf";

const BIN_CPU: &str = "llama-b8184-bin-win-cpu-x64";
const BIN_CUDA: &str = "llama-b8184-bin-win-cuda-13.1-x64";
const BIN_VULKAN: &str = "llama-b8184-bin-win-vulkan-x64";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlamaBackendKind {
    Cuda,
    Vulkan,
    Cpu,
}

#[derive(Clone)]
pub struct LlamaRuntime {
    pub bin_dir: PathBuf,
    pub backend: LlamaBackendKind,
    pub gpu_layers: u32,
    pub ctx_size: u32,
    pub threads: u32,
}

pub fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn runtime_base_roots() -> Vec<PathBuf> {
    let mut roots = vec![project_root()];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            roots.push(exe_dir.to_path_buf());
            roots.push(exe_dir.join("resources"));
            if let Some(parent) = exe_dir.parent() {
                roots.push(parent.join("Resources"));
            }
        }
    }
    roots
}

pub fn runtime_bundle_dir(bundle_name: &str) -> PathBuf {
    let mut fallback = None::<PathBuf>;
    for root in runtime_base_roots() {
        let candidate = root.join(bundle_name);
        if fallback.is_none() {
            fallback = Some(candidate.clone());
        }
        if candidate.is_dir() {
            return candidate;
        }
    }
    fallback.unwrap_or_else(|| project_root().join(bundle_name))
}

pub fn bundle_name_for_backend(kind: LlamaBackendKind) -> &'static str {
    match kind {
        LlamaBackendKind::Cuda => BIN_CUDA,
        LlamaBackendKind::Vulkan => BIN_VULKAN,
        LlamaBackendKind::Cpu => BIN_CPU,
    }
}

pub fn model_bundle_dir() -> PathBuf {
    runtime_bundle_dir(BIN_CPU).join(MODEL_BUNDLE_DIR)
}

pub fn default_model_path() -> PathBuf {
    model_bundle_dir().join(MODEL_FILENAME)
}

pub fn resolve_llama_runtime() -> LlamaRuntime {
    crate::ai::hardware_profile::resolve_active_runtime(None)
}

pub fn backend_label(kind: LlamaBackendKind) -> &'static str {
    match kind {
        LlamaBackendKind::Cuda => "CUDA (GPU NVIDIA)",
        LlamaBackendKind::Vulkan => "Vulkan (GPU)",
        LlamaBackendKind::Cpu => "CPU",
    }
}
