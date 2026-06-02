use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::ai::config::{backend_label, default_model_path, LlamaRuntime, LLAMA_PORT};
use crate::ai::hardware_profile::{self, resolve_active_runtime};
use crate::db::Database;

static SERVER_CHILD: Mutex<Option<Child>> = Mutex::new(None);
static ACTIVE_RT: Mutex<Option<LlamaRuntime>> = Mutex::new(None);

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessageOut,
}

#[derive(Deserialize)]
struct ChatMessageOut {
    content: String,
}

pub struct LlamaServer;

impl LlamaServer {
    fn runtime(db: Option<&Database>) -> LlamaRuntime {
        if let Ok(guard) = ACTIVE_RT.lock() {
            if let Some(rt) = guard.clone() {
                return rt;
            }
        }
        let rt = resolve_active_runtime(db);
        if let Ok(mut guard) = ACTIVE_RT.lock() {
            *guard = Some(rt.clone());
        }
        rt
    }

    pub fn prepare(db: &Database, force_profile: bool) -> Result<LlamaRuntime, String> {
        let rt = hardware_profile::ensure_runtime_profile(db, force_profile)?;
        if let Ok(mut guard) = ACTIVE_RT.lock() {
            *guard = Some(rt.clone());
        }
        Ok(rt)
    }

    pub fn model_ready() -> bool {
        default_model_path().is_file()
    }

    pub fn bin_ready() -> bool {
        Self::runtime(None).bin_dir.join("llama-server.exe").is_file()
    }

    pub fn runtime_info(db: Option<&Database>) -> (String, u32, u32, u32) {
        let rt = Self::runtime(db);
        (
            backend_label(rt.backend).to_string(),
            rt.gpu_layers,
            rt.ctx_size,
            rt.threads,
        )
    }

    pub fn using_gpu(db: Option<&Database>) -> bool {
        Self::runtime(db).gpu_layers > 0
    }

    fn process_alive() -> bool {
        let Ok(mut guard) = SERVER_CHILD.lock() else {
            return false;
        };
        let Some(child) = guard.as_mut() else {
            return false;
        };
        match child.try_wait() {
            Ok(None) => true,
            _ => false,
        }
    }

    pub fn health_ok() -> bool {
        if !Self::process_alive() {
            return false;
        }
        let url = format!("http://127.0.0.1:{LLAMA_PORT}/health");
        let Ok(client) = Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
        else {
            return false;
        };
        match client.get(&url).send() {
            Ok(r) => r.status().is_success(),
            Err(_) => false,
        }
    }

    pub fn ensure_started(db: Option<&Database>) -> Result<(), String> {
        if Self::health_ok() {
            return Ok(());
        }
        Self::stop();
        let rt = Self::runtime(db);
        let server_exe = rt.bin_dir.join("llama-server.exe");
        if !server_exe.is_file() {
            return Err(format!(
                "llama-server introuvable ({})",
                server_exe.display()
            ));
        }
        if !Self::model_ready() {
            return Err(format!(
                "Modele GGUF absent : {}. Installez le modele localement (hors ligne).",
                default_model_path().display()
            ));
        }

        let model = default_model_path();
        let ngl = rt.gpu_layers.to_string();
        let child = Command::new(&server_exe)
            .current_dir(&rt.bin_dir)
            .args([
                "-m",
                &model.to_string_lossy(),
                "--host",
                "127.0.0.1",
                "--port",
                &LLAMA_PORT.to_string(),
                "-ngl",
                &ngl,
                "-t",
                &rt.threads.to_string(),
                "--ctx-size",
                &rt.ctx_size.to_string(),
                "--parallel",
                "1",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Demarrage llama-server : {e}"))?;

        if let Ok(mut guard) = SERVER_CHILD.lock() {
            *guard = Some(child);
        }

        for _ in 0..180 {
            if Self::health_ok() {
                return Ok(());
            }
            if !Self::process_alive() {
                let hint = if rt.gpu_layers > 0 {
                    " (VRAM insuffisante ? Relancez le profilage materiel.)"
                } else {
                    ""
                };
                return Err(format!(
                    "llama-server s'est arrete pendant le chargement du modele{hint}"
                ));
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        Self::stop();
        Err("Le serveur llama n'a pas repondu a temps.".into())
    }

    pub fn stop() {
        if let Ok(mut guard) = SERVER_CHILD.lock() {
            if let Some(mut child) = guard.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    fn is_connection_error(msg: &str) -> bool {
        let m = msg.to_lowercase();
        m.contains("error sending request")
            || m.contains("connection refused")
            || m.contains("connexion")
            || m.contains("timed out")
            || m.contains("broken pipe")
    }

    fn chat_once(
        messages: Vec<ChatMessage>,
        temperature: f32,
        max_tokens: u32,
    ) -> Result<String, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .map_err(|e| e.to_string())?;

        let req = ChatRequest {
            model: "local".into(),
            messages,
            temperature,
            max_tokens,
        };

        let url = format!("http://127.0.0.1:{LLAMA_PORT}/v1/chat/completions");
        let res = client
            .post(&url)
            .json(&req)
            .send()
            .map_err(|e| format!("Appel llama : {e}"))?;

        let status = res.status();
        let body_text = res.text().map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("Llama HTTP {status} : {body_text}"));
        }

        let parsed: ChatResponse =
            serde_json::from_str(&body_text).map_err(|e| format!("Reponse llama invalide : {e}"))?;
        parsed
            .choices
            .first()
            .map(|c| c.message.content.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "Reponse vide du modele.".into())
    }

    pub fn chat(db: Option<&Database>, messages: Vec<ChatMessage>) -> Result<String, String> {
        Self::chat_with_options(db, messages, 0.15, 600)
    }

    pub fn chat_with_options(
        db: Option<&Database>,
        messages: Vec<ChatMessage>,
        temperature: f32,
        max_tokens: u32,
    ) -> Result<String, String> {
        Self::ensure_started(db)?;
        let messages = normalize_chat_messages(messages);
        match Self::chat_once(messages.clone(), temperature, max_tokens) {
            Ok(text) => Ok(text),
            Err(e) if Self::is_connection_error(&e) => {
                Self::stop();
                Self::ensure_started(db)?;
                Self::chat_once(messages, temperature, max_tokens)
            }
            Err(e) => Err(e),
        }
    }
}

/// Évite les rôles consécutifs identiques (exigence du template Jinja llama-server).
pub fn normalize_chat_messages(messages: Vec<ChatMessage>) -> Vec<ChatMessage> {
    let mut system: Option<ChatMessage> = None;
    let mut dialog: Vec<ChatMessage> = Vec::new();

    for m in messages {
        if m.role == "system" {
            system = Some(m);
            continue;
        }
        let role = if m.role == "assistant" {
            "assistant".to_string()
        } else {
            "user".to_string()
        };
        if let Some(last) = dialog.last_mut() {
            if last.role == role {
                if !last.content.is_empty() && !m.content.is_empty() {
                    last.content.push('\n');
                }
                last.content.push_str(&m.content);
                continue;
            }
        }
        dialog.push(ChatMessage {
            role,
            content: m.content,
        });
    }

    while dialog.first().map(|m| m.role.as_str()) == Some("assistant") {
        dialog.remove(0);
    }

    let mut out = Vec::new();
    if let Some(s) = system {
        out.push(s);
    }
    out.extend(dialog);
    out
}
