//! Traduction via Google Traduction (API web publique translate.googleapis.com).

use std::time::Duration;

use regex::Regex;
use reqwest::blocking::Client;
use serde_json::Value;

use super::intent_filters::normalize_message;

/// Langues cibles reconnues (mot-clé normalisé → code ISO, libellé affiché).
const LANGUAGES: &[(&str, &str, &str)] = &[
    ("anglais", "en", "anglais"),
    ("english", "en", "anglais"),
    ("francais", "fr", "français"),
    ("français", "fr", "français"),
    ("espagnol", "es", "espagnol"),
    ("spanish", "es", "espagnol"),
    ("allemand", "de", "allemand"),
    ("german", "de", "allemand"),
    ("italien", "it", "italien"),
    ("italian", "it", "italien"),
    ("portugais", "pt", "portugais"),
    ("portuguese", "pt", "portugais"),
    ("neerlandais", "nl", "néerlandais"),
    ("dutch", "nl", "néerlandais"),
    ("russe", "ru", "russe"),
    ("russian", "ru", "russe"),
    ("arabe", "ar", "arabe"),
    ("arabic", "ar", "arabe"),
    ("chinois", "zh-CN", "chinois"),
    ("mandarin", "zh-CN", "chinois"),
    ("japonais", "ja", "japonais"),
    ("japanese", "ja", "japonais"),
    ("coreen", "ko", "coréen"),
    ("korean", "ko", "coréen"),
    ("polonais", "pl", "polonais"),
    ("polish", "pl", "polonais"),
    ("turc", "tr", "turc"),
    ("turkish", "tr", "turc"),
    ("ukrainien", "uk", "ukrainien"),
    ("ukrainian", "uk", "ukrainien"),
    ("roumain", "ro", "roumain"),
    ("grec", "el", "grec"),
    ("greek", "el", "grec"),
    ("suedois", "sv", "suédois"),
    ("swedish", "sv", "suédois"),
    ("norvegien", "no", "norvégien"),
    ("danois", "da", "danois"),
    ("finnois", "fi", "finnois"),
    ("hongrois", "hu", "hongrois"),
    ("tcheque", "cs", "tchèque"),
    ("vietnamien", "vi", "vietnamien"),
    ("hindi", "hi", "hindi"),
    ("hebreu", "iw", "hébreu"),
    ("hebrew", "iw", "hébreu"),
];

#[derive(Debug, Clone)]
pub struct TranslationDirective {
    pub target_lang: String,
    pub target_label: String,
    pub core_message: String,
    /// Traduire le dernier message assistant (pas de nouvel appel LLM).
    pub translate_previous: bool,
}

/// Message utilisateur sans la consigne « traduit / réponds en … ».
pub fn core_message(message: &str) -> String {
    parse_translation_directive(message)
        .map(|d| d.core_message)
        .unwrap_or_else(|| message.trim().to_string())
}

/// Demande de traduire la dernière réponse de Loggy (sans nouvelle question).
pub fn wants_translate_previous_reply(message: &str) -> bool {
    let n = normalize_message(message);
    n.contains("ta reponse")
        || n.contains("ta reponse precedente")
        || n.contains("derniere reponse")
        || n.contains("message precedent")
        || n.contains("ce que tu viens de dire")
        || n.contains("reponse precedente")
}

fn is_meta_core_only(core: &str) -> bool {
    let n = normalize_message(core);
    n.is_empty()
        || n == "ta reponse"
        || n == "reponse"
        || n == "ca"
        || n == "cela"
}

/// Détecte « traduit », « traduis », « réponds en … » et extrait la langue + la question.
pub fn parse_translation_directive(message: &str) -> Option<TranslationDirective> {
    let n = normalize_message(message);
    let triggered = n.contains("traduit")
        || n.contains("traduis")
        || n.contains("traduire")
        || n.contains("traduction")
        || n.contains("repond en")
        || n.contains("reponds en")
        || n.contains("repondre en");
    if !triggered {
        return None;
    }

    let (target_lang, target_label) = detect_target_language(&n);
    let translate_prev = wants_translate_previous_reply(message);
    let core = strip_directive_prefix(message);
    let translate_previous = translate_prev || is_meta_core_only(&core);
    if !translate_previous && core.trim().len() < 2 {
        return None;
    }

    Some(TranslationDirective {
        target_lang,
        target_label,
        core_message: core,
        translate_previous,
    })
}

fn detect_target_language(normalized: &str) -> (String, String) {
    for (keyword, iso, label) in LANGUAGES {
        let needle = format!(" en {keyword}");
        if normalized.contains(&needle) || normalized.contains(&format!("{keyword} ")) {
            return (iso.to_string(), (*label).to_string());
        }
    }
    // « traduit » / « réponds en » sans langue explicite → français (réponse locale uniquement)
    ("fr".into(), "français".into())
}

fn strip_directive_prefix(message: &str) -> String {
    let mut s = message.trim().to_string();
    let patterns = [
        r"(?i)^traduis?\s+ta\s+reponse\s+en\s+[a-zàâäéèêëïîôùûüçœæ\-]+\s*[:,\-–—]?\s*",
        r"(?i)^traduit\s+ta\s+reponse\s+en\s+[a-zàâäéèêëïîôùûüçœæ\-]+\s*[:,\-–—]?\s*",
        r"(?i)^traduis?\s+ta\s+reponse\s*[:,\-–—]?\s*",
        r"(?i)^traduit\s+ta\s+reponse\s*[:,\-–—]?\s*",
        r"(?i)^traduis?\s+en\s+[a-zàâäéèêëïîôùûüçœæ\-]+\s*[:,\-–—]?\s*",
        r"(?i)^traduire\s+en\s+[a-zàâäéèêëïîôùûüçœæ\-]+\s*[:,\-–—]?\s*",
        r"(?i)^traduit\s+en\s+[a-zàâäéèêëïîôùûüçœæ\-]+\s*[:,\-–—]?\s*",
        r"(?i)^traduit\s*[:,\-–—]?\s*",
        r"(?i)^reponds?\s+en\s+[a-zàâäéèêëïîôùûüçœæ\-]+\s*[:,\-–—]?\s*",
        r"(?i)^repondre\s+en\s+[a-zàâäéèêëïîôùûüçœæ\-]+\s*[:,\-–—]?\s*",
        r"(?i)^traduis?\s*[:,\-–—]?\s*",
    ];
    for pat in patterns {
        if let Ok(re) = Regex::new(pat) {
            s = re.replace(&s, "").trim().to_string();
        }
    }
    s
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

/// Appel Google Traduction (même service que translate.google.com, client gtx).
pub fn google_translate_text(text: &str, source_lang: &str, target_lang: &str) -> Result<String, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    if source_lang == target_lang {
        return Ok(trimmed.to_string());
    }

    const MAX_CHUNK: usize = 1500;
    let chunks: Vec<String> = if trimmed.chars().count() <= MAX_CHUNK {
        vec![trimmed.to_string()]
    } else {
        let mut parts = Vec::new();
        let mut cur = String::new();
        for word in trimmed.split_whitespace() {
            if cur.chars().count() + word.chars().count() + 1 > MAX_CHUNK && !cur.is_empty() {
                parts.push(cur.trim().to_string());
                cur.clear();
            }
            if !cur.is_empty() {
                cur.push(' ');
            }
            cur.push_str(word);
        }
        if !cur.is_empty() {
            parts.push(cur);
        }
        parts
    };

    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?;

    let mut out = String::new();
    for chunk in chunks {
        let url = format!(
            "https://translate.googleapis.com/translate_a/single?client=gtx&sl={}&tl={}&dt=t&q={}",
            source_lang,
            target_lang,
            encode_query(&chunk)
        );
        let res = client.get(&url).send().map_err(|e| format!("Google Traduction : {e}"))?;
        let v: Value = res.json().map_err(|e| format!("Google Traduction (JSON) : {e}"))?;
        let part = extract_translation(&v);
        if part.is_empty() {
            return Err("Google Traduction n'a renvoyé aucun texte.".into());
        }
        out.push_str(&part);
    }
    Ok(out.trim().to_string())
}

fn extract_translation(v: &Value) -> String {
    v.get(0)
        .and_then(|a| a.as_array())
        .map(|segments| {
            segments
                .iter()
                .filter_map(|seg| seg.get(0).and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
}

fn french_original_from_assistant(text: &str) -> String {
    let t = text.trim();
    if let Some(idx) = t.find("\n\n--- Traduction") {
        t[..idx].trim().to_string()
    } else {
        t.to_string()
    }
}

/// Traduit le dernier message assistant via Google (sans LLM).
pub fn try_translate_previous_reply(
    db: &crate::db::Database,
    conv_id: &str,
    user_message: &str,
) -> Option<Result<String, String>> {
    let dir = parse_translation_directive(user_message)?;
    if !dir.translate_previous || dir.target_lang == "fr" {
        return None;
    }
    let previous = db
        .ai_last_assistant_message(conv_id)
        .ok()
        .flatten()
        .filter(|s| !s.trim().is_empty())?;
    let original = french_original_from_assistant(&previous);
    Some(translate_and_format(&original, &dir.target_lang, &dir.target_label))
}

fn translate_and_format(original: &str, target_lang: &str, target_label: &str) -> Result<String, String> {
    match google_translate_text(original, "fr", target_lang) {
        Ok(translated) => Ok(format!(
            "{}\n\n--- Traduction {} (Google Traduction) ---\n{}",
            original.trim(),
            target_label,
            translated.trim()
        )),
        Err(e) => Err(format!(
            "Je n'ai pas pu traduire via Google Traduction : {e}"
        )),
    }
}

/// Réponse en français, puis traduction Google si une langue cible ≠ fr est demandée.
pub fn finalize_with_translation(user_message: &str, answer_fr: &str) -> Result<String, String> {
    let Some(dir) = parse_translation_directive(user_message) else {
        return Ok(answer_fr.to_string());
    };
    if dir.translate_previous {
        return Ok(answer_fr.to_string());
    }
    if dir.target_lang == "fr" {
        return Ok(answer_fr.to_string());
    }
    translate_and_format(answer_fr, &dir.target_lang, &dir.target_label).or_else(|e| {
        Ok(format!(
            "{}\n\n(Je n'ai pas pu traduire via Google Traduction : {e})",
            answer_fr.trim()
        ))
    })
}
