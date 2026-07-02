use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

use regex::Regex;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ai::intent_filters::{extract_web_search_query, wants_internet_research_intent};
use crate::ai::llama_server::{ChatMessage, LlamaServer};
use crate::db::Database;

const USER_AGENT: &str =
    "Mozilla/5.0 (compatible; Blin/1.0; +https://blin.local)";
const SEARCH_TIMEOUT_SECS: u64 = 18;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

impl Default for WebSearchConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchHit {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchResult {
    pub query: String,
    pub abstract_text: Option<String>,
    pub abstract_url: Option<String>,
    pub hits: Vec<WebSearchHit>,
    /// Canal utilisé en premier (reqwest, curl, powershell).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

pub fn config_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("ai").join("internet.json")
}

pub fn load_config(data_dir: &Path) -> WebSearchConfig {
    let path = config_path(data_dir);
    if !path.is_file() {
        return WebSearchConfig::default();
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save_config(data_dir: &Path, cfg: &WebSearchConfig) -> Result<(), String> {
    let dir = data_dir.join("ai");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    fs::write(config_path(data_dir), json).map_err(|e| e.to_string())
}

pub fn is_enabled(data_dir: &Path) -> bool {
    load_config(data_dir).enabled
}

fn encode_query(q: &str) -> String {
    q.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "+".into(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}

fn ddg_instant_url(query: &str) -> String {
    format!(
        "https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1",
        encode_query(query)
    )
}

fn ddg_html_url(query: &str) -> String {
    format!(
        "https://html.duckduckgo.com/html/?q={}",
        encode_query(query)
    )
}

fn parse_ddg_instant_json(body: &str) -> (Option<String>, Option<String>) {
    let v: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return (None, None),
    };
    let abstract_text = v
        .get("AbstractText")
        .and_then(|t| t.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string());
    let abstract_url = v
        .get("AbstractURL")
        .and_then(|t| t.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string());
    (abstract_text, abstract_url)
}

/// URL cliquable (https) ; décode les redirections DuckDuckGo `uddg=`.
fn normalize_web_url(raw: &str) -> Option<String> {
    let mut s = raw.trim().to_string();
    if s.is_empty() {
        return None;
    }
    if let Some(idx) = s.find("uddg=") {
        let rest = &s[idx + 5..];
        let end = rest.find('&').unwrap_or(rest.len());
        s = percent_decode(&rest[..end]);
    }
    if s.starts_with("//") {
        s = format!("https:{}", s);
    }
    if s.starts_with("http://") || s.starts_with("https://") {
        Some(s)
    } else {
        None
    }
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    out.push(byte);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Sources uniques (titre, URL https) pour affichage cliquable.
pub fn collect_source_links(result: &WebSearchResult) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();

    if let Some(u) = result
        .abstract_url
        .as_ref()
        .and_then(|u| normalize_web_url(u))
    {
        if seen.insert(u.clone()) {
            let title = result
                .abstract_text
                .as_deref()
                .filter(|t| !t.trim().is_empty())
                .unwrap_or("Source principale")
                .to_string();
            out.push((title, u));
        }
    }

    for h in &result.hits {
        if let Some(u) = normalize_web_url(&h.url) {
            if seen.insert(u.clone()) {
                let title = if h.title.trim().is_empty() {
                    u.clone()
                } else {
                    h.title.clone()
                };
                out.push((title, u));
            }
        }
    }
    out
}

/// Bloc « Sources » avec URLs complètes (le front les rend cliquables).
pub fn format_sources_section(result: &WebSearchResult) -> String {
    let links = collect_source_links(result);
    if links.is_empty() {
        return String::new();
    }
    let mut lines = vec!["Sources (cliquez pour ouvrir) :".to_string()];
    for (title, url) in links {
        lines.push(format!("• {title} — {url}"));
    }
    lines.join("\n")
}

/// Ajoute les sources à la fin de la réponse si pas déjà présentes.
pub fn finalize_web_answer(body: &str, result: &WebSearchResult) -> String {
    let body = body.trim();
    let footer = format_sources_section(result);
    if footer.is_empty() {
        return body.to_string();
    }
    // On n'évite l'ajout que si le bloc cliquable exact est déjà présent
    // (ne pas se fier au simple mot « Sources » que le LLM peut écrire en prose).
    if body.contains("Sources (cliquez") {
        return body.to_string();
    }
    format!("{body}\n\n{footer}")
}

fn parse_ddg_html_hits(html: &str) -> Vec<WebSearchHit> {
    let link_re = match Regex::new(
        r#"(?is)class="result__a"[^>]*href="([^"]+)"[^>]*>([^<]+)</a>"#,
    ) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let snippet_re = match Regex::new(r#"(?is)class="result__snippet"[^>]*>([^<]+)</"#) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let links: Vec<_> = link_re
        .captures_iter(html)
        .map(|c| {
            (
                c.get(1).map(|m| m.as_str()).unwrap_or("").to_string(),
                c.get(2).map(|m| m.as_str()).unwrap_or("").trim().to_string(),
            )
        })
        .collect();
    let snippets: Vec<String> = snippet_re
        .captures_iter(html)
        .map(|c| c.get(1).map(|m| m.as_str().trim()).unwrap_or("").to_string())
        .collect();

    let mut hits = Vec::new();
    for (i, (url, title)) in links.into_iter().take(6).enumerate() {
        if title.is_empty() || url.is_empty() {
            continue;
        }
        let url = normalize_web_url(&url).unwrap_or(url);
        hits.push(WebSearchHit {
            title,
            url,
            snippet: snippets.get(i).cloned().unwrap_or_default(),
        });
    }
    hits
}

fn result_has_content(abstract_text: &Option<String>, hits: &[WebSearchHit]) -> bool {
    abstract_text.as_ref().is_some_and(|s| !s.trim().is_empty()) || !hits.is_empty()
}

fn merge_ddg_parts(
    query: &str,
    source: &str,
    instant: (Option<String>, Option<String>),
    hits: Vec<WebSearchHit>,
) -> Option<WebSearchResult> {
    let (abstract_text, abstract_url) = instant;
    if !result_has_content(&abstract_text, &hits) {
        return None;
    }
    Some(WebSearchResult {
        query: query.to_string(),
        abstract_text,
        abstract_url,
        hits,
        source: Some(source.to_string()),
    })
}

// --- Reqwest (HTTP direct) ---

fn fetch_text_reqwest(url: &str) -> Result<String, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(SEARCH_TIMEOUT_SECS))
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| e.to_string())?;
    client
        .get(url)
        .send()
        .map_err(|e| e.to_string())?
        .text()
        .map_err(|e| e.to_string())
}

fn search_ddg_reqwest(query: &str) -> Option<WebSearchResult> {
    let instant = fetch_text_reqwest(&ddg_instant_url(query))
        .map(|b| parse_ddg_instant_json(&b))
        .unwrap_or((None, None));
    let hits = fetch_text_reqwest(&ddg_html_url(query))
        .map(|h| parse_ddg_html_hits(&h))
        .unwrap_or_default();
    merge_ddg_parts(query, "duckduckgo-http", instant, hits)
}

// --- Terminal : curl.exe ---

fn fetch_text_curl(url: &str) -> Result<String, String> {
    let timeout = SEARCH_TIMEOUT_SECS.to_string();
    let output = Command::new("curl.exe")
        .args([
            "-s",
            "-L",
            "--max-time",
            &timeout,
            "-A",
            USER_AGENT,
            url,
        ])
        .output()
        .map_err(|e| format!("curl : {e}"))?;
    if !output.status.success() && output.stdout.is_empty() {
        return Err(format!(
            "curl code {}",
            output.status.code().unwrap_or(-1)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn search_ddg_curl(query: &str) -> Option<WebSearchResult> {
    let instant = fetch_text_curl(&ddg_instant_url(query))
        .ok()
        .map(|b| parse_ddg_instant_json(&b))
        .unwrap_or((None, None));
    let hits = fetch_text_curl(&ddg_html_url(query))
        .ok()
        .map(|h| parse_ddg_html_hits(&h))
        .unwrap_or_default();
    merge_ddg_parts(query, "duckduckgo-curl", instant, hits)
}

// --- Terminal : PowerShell ---

fn ps_escape(s: &str) -> String {
    s.replace('\'', "''")
}

fn fetch_text_powershell(url: &str) -> Result<String, String> {
    let u = ps_escape(url);
    let ua = ps_escape(USER_AGENT);
    let script = format!(
        "$ProgressPreference='SilentlyContinue'; \
         try {{ \
           $r = Invoke-WebRequest -Uri '{u}' -UseBasicParsing -TimeoutSec {SEARCH_TIMEOUT_SECS} \
             -UserAgent '{ua}'; \
           [Console]::Out.Write($r.Content) \
         }} catch {{ exit 1 }}"
    );
    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ])
        .output()
        .map_err(|e| format!("powershell : {e}"))?;
    if !output.status.success() && output.stdout.is_empty() {
        return Err("powershell Invoke-WebRequest a échoué".into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn search_ddg_powershell(query: &str) -> Option<WebSearchResult> {
    let instant = fetch_text_powershell(&ddg_instant_url(query))
        .ok()
        .map(|b| parse_ddg_instant_json(&b))
        .unwrap_or((None, None));
    let hits = fetch_text_powershell(&ddg_html_url(query))
        .ok()
        .map(|h| parse_ddg_html_hits(&h))
        .unwrap_or_default();
    merge_ddg_parts(query, "duckduckgo-powershell", instant, hits)
}

/// Essaie les canaux l'un APRÈS l'autre (jamais en parallèle).
///
/// DuckDuckGo bloque les rafales de requêtes simultanées (page « anomaly »
/// vide). En envoyant une seule requête à la fois, chaque canal sert aussi de
/// nouvel essai espacé : reqwest → curl → PowerShell.
fn search_ddg_sequential(query: &str) -> Option<WebSearchResult> {
    type Channel = fn(&str) -> Option<WebSearchResult>;
    let channels: [Channel; 3] = [
        search_ddg_reqwest,
        search_ddg_curl,
        search_ddg_powershell,
    ];

    for (i, channel) in channels.iter().enumerate() {
        if let Some(result) = channel(query) {
            return Some(result);
        }
        // Petite pause entre deux canaux pour éviter d'être pris pour un bot.
        if i + 1 < channels.len() {
            thread::sleep(Duration::from_millis(350));
        }
    }
    None
}

/// Recherche web DuckDuckGo — canaux séquentiels (HTTP direct, curl, PowerShell).
pub fn search(data_dir: &Path, query: &str) -> Result<WebSearchResult, String> {
    if !is_enabled(data_dir) {
        return Err(
            "La recherche Internet est désactivée. Activez-la dans Paramètres > Assistant."
                .into(),
        );
    }
    let q = query.trim();
    if q.len() < 2 {
        return Err("Requête de recherche trop courte.".into());
    }

    if let Some(result) = search_ddg_sequential(q) {
        return Ok(result);
    }

    Err(
        "Aucun résultat web (vérifiez votre connexion Internet, curl ou PowerShell, ou reformulez la question)."
            .into(),
    )
}

pub fn format_results_context(result: &WebSearchResult) -> String {
    let mut parts = vec![format!("Requête : {}", result.query)];
    if let Some(src) = &result.source {
        parts.push(format!("Canal : {src}"));
    }
    if let Some(abs) = &result.abstract_text {
        parts.push(format!("Résumé : {abs}"));
        if let Some(u) = &result.abstract_url {
            parts.push(format!("Source : {u}"));
        }
    }
    for (i, h) in result.hits.iter().enumerate().take(5) {
        parts.push(format!(
            "{}. {} — {}\n   {}",
            i + 1,
            h.title,
            h.url,
            h.snippet
        ));
    }
    parts.join("\n")
}

pub fn format_results_message(result: &WebSearchResult) -> String {
    let mut lines = vec![format!("Résultats web pour « {} » :", result.query)];
    if let Some(abs) = &result.abstract_text {
        lines.push(abs.clone());
    }
    for h in result.hits.iter().take(5) {
        let snip = if h.snippet.is_empty() {
            String::new()
        } else {
            format!(" — {}", h.snippet)
        };
        lines.push(format!("• {}{snip}", h.title));
    }
    finalize_web_answer(&lines.join("\n"), result)
}

/// Réponse Loggy en première personne à partir des résultats web.
pub fn synthesize_answer(
    db: &Database,
    user_message: &str,
    result: &WebSearchResult,
) -> Result<String, String> {
    if !LlamaServer::model_ready() {
        return Ok(format_results_message(result));
    }
    let context = format_results_context(result);
    let system = "Tu es Loggy, assistant de l'application. Tu réponds en français à la PREMIÈRE personne (je). \
         Tu t'appuies UNIQUEMENT sur les extraits de recherche web fournis. \
         Si les sources sont insuffisantes, dis-le honnêtement. \
         Ne rédige PAS toi-même de liste de sources ni d'URL et n'écris pas de section « Sources » : \
         les liens cliquables vers les sources seront ajoutés automatiquement après ta réponse. \
         Pas de JSON, pas de LaTeX.";
    let user = format!(
        "Question utilisateur : {user_message}\n\nExtraits web :\n{context}\n\n\
         Rédige une réponse claire et utile."
    );
    let _ = LlamaServer::prepare(db, false);
    match LlamaServer::chat_with_options(
        Some(db),
        vec![
            ChatMessage {
                role: "system".into(),
                content: system.into(),
            },
            ChatMessage {
                role: "user".into(),
                content: user,
            },
        ],
        0.35,
        500,
    ) {
        Ok(t) if !t.trim().is_empty() => Ok(finalize_web_answer(&t, result)),
        _ => Ok(format_results_message(result)),
    }
}

pub fn should_use_web_for_message(data_dir: &Path, message: &str) -> bool {
    is_enabled(data_dir) && wants_internet_research_intent(message)
}

pub fn query_from_message(message: &str) -> Option<String> {
    extract_web_search_query(message)
}

/// Demande au modèle local une requête de recherche concise à partir des
/// derniers messages utilisateur (résout les pronoms « il / lui / elle… »).
fn llm_query_from_context(prior_user_messages: &[&str]) -> Option<String> {
    if !LlamaServer::model_ready() {
        return None;
    }
    let convo = prior_user_messages
        .iter()
        .rev()
        .take(4)
        .rev()
        .map(|m| format!("- {}", m.trim()))
        .collect::<Vec<_>>()
        .join("\n");
    let system = "Tu génères UNIQUEMENT une requête de recherche web courte (2 à 6 mots-clés), en français. \
         À partir des derniers messages de l'utilisateur, déduis le SUJET principal et résous les pronoms \
         (il, lui, elle, le, la, ça). Réponds par la requête seule : pas de phrase, pas de guillemets, \
         pas de ponctuation finale, pas d'explication.";
    let user = format!("Messages récents de l'utilisateur :\n{convo}\n\nRequête de recherche :");
    let out = LlamaServer::chat_with_options(
        None,
        vec![
            ChatMessage {
                role: "system".into(),
                content: system.into(),
            },
            ChatMessage {
                role: "user".into(),
                content: user,
            },
        ],
        0.2,
        32,
    )
    .ok()?;
    let q = out
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .trim_matches(|c| c == '"' || c == '\'' || c == '`' || c == '.' || c == '?' || c == '!')
        .trim()
        .to_string();
    if q.len() < 2 || q.split_whitespace().count() > 10 {
        None
    } else {
        Some(q)
    }
}

/// Repli sans modèle : premier message de contexte porteur de contenu.
fn heuristic_subject(prior_user_messages: &[&str]) -> Option<String> {
    for m in prior_user_messages {
        if m.split_whitespace().count() >= 2 {
            return Some(m.trim().to_string());
        }
    }
    prior_user_messages.last().map(|s| s.trim().to_string())
}

/// Construit la requête web. Si le message courant contient un sujet explicite
/// (« cherche X sur internet »), on l'utilise. Sinon (« cherche sur internet »
/// seul), on déduit le sujet des messages précédents de la conversation.
///
/// `history` = (rôle, contenu) en ordre chronologique ; le message courant peut
/// y figurer en dernier (il est alors ignoré pour le contexte).
pub fn build_contextual_query(
    history: &[(String, String)],
    current_core: &str,
) -> Option<String> {
    if let Some(q) = extract_web_search_query(current_core) {
        return Some(q);
    }

    let mut prior: Vec<&str> = history
        .iter()
        .filter(|(role, _)| role == "user")
        .map(|(_, content)| content.trim())
        .filter(|content| !content.is_empty())
        .collect();
    // Retire le message courant s'il clôt l'historique.
    if prior
        .last()
        .map(|m| m.eq_ignore_ascii_case(current_core.trim()))
        .unwrap_or(false)
    {
        prior.pop();
    }
    if prior.is_empty() {
        return None;
    }

    llm_query_from_context(&prior).or_else(|| heuristic_subject(&prior))
}
