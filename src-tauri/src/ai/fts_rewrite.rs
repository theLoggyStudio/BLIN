//! Réécriture courte du message utilisateur en requête FTS5 (OR) via llama-server.

use crate::ai::llama_server::{ChatMessage, LlamaServer};
use crate::db::Database;

const SYSTEM_PROMPT_FTS5: &str = r#"Convertis le message utilisateur en 3 à 5 mots-clés ou tokens techniques séparés par l'opérateur "OR" pour une recherche SQLite FTS5.
Inclus le nom de l'entité au singulier, au pluriel, et les suffixes probables (_schema, _tools, MASTER_entities_schema, MASTER_entities_tools).
Renvoie UNIQUEMENT la chaîne de recherche, sans phrase d'introduction ni bonjour.

Exemple : "Je veux voir la liste des factures clients"
Sortie : facture OR factures OR client OR clients OR MASTER_entities_schema"#;

/// Appel LLM rapide ; retombe sur le message brut si échec ou réponse invalide.
pub fn rewrite_user_query_for_fts(db: &Database, user_message: &str) -> String {
    let trimmed = user_message.trim();
    if trimmed.len() < 8 {
        return trimmed.to_string();
    }
    if !LlamaServer::model_ready() || !LlamaServer::bin_ready() {
        return trimmed.to_string();
    }

    let messages = vec![
        ChatMessage {
            role: "system".into(),
            content: SYSTEM_PROMPT_FTS5.into(),
        },
        ChatMessage {
            role: "user".into(),
            content: trimmed.into(),
        },
    ];

    let Ok(raw) = LlamaServer::chat_with_options(Some(db), messages, 0.1, 96) else {
        return trimmed.to_string();
    };

    let cleaned = clean_llm_fts_output(&raw);
    if is_usable_fts_rewrite(&cleaned) {
        cleaned
    } else {
        trimmed.to_string()
    }
}

fn clean_llm_fts_output(raw: &str) -> String {
    let mut line = raw
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or(raw)
        .trim()
        .to_string();
    if line.starts_with("```") {
        line = line.trim_start_matches('`').to_string();
        if let Some(rest) = line.strip_prefix("text") {
            line = rest.trim().to_string();
        }
        if let Some(idx) = line.find('`') {
            line = line[..idx].trim().to_string();
        }
    }
    line.trim_matches(|c| c == '"' || c == '\'' || c == '.' || c == ',')
        .trim()
        .to_string()
}

fn is_usable_fts_rewrite(s: &str) -> bool {
    if s.len() < 3 {
        return false;
    }
    if s.to_lowercase().contains(" or ") {
        return true;
    }
    // Un seul token technique long (ex. MASTER_entities_tools)
    s.chars().filter(|c| c.is_alphanumeric() || *c == '_').count() >= 4
}
