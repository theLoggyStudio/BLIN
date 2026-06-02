use crate::ai::llama_server::{ChatMessage, LlamaServer};
use crate::db::Database;

/// Phrase d'accueil générée par Loggy avant l'ouverture d'un écran entité.
pub fn generate_transition_phrase(
    db: &Database,
    user_message: &str,
    entity_key: &str,
    entity_label: &str,
) -> Result<String, String> {
    if !LlamaServer::model_ready() {
        return Err(
            "Modèle IA local indisponible. Copiez le fichier GGUF dans le dossier d'installation."
                .into(),
        );
    }
    let (profiled, _) = crate::ai::hardware_profile::profile_summary(db)?;
    if !profiled {
        LlamaServer::prepare(db, false)?;
    }

    let system = "Tu es Loggy, l'assistant de l'application métier. Tu réponds uniquement en français, par une seule phrase courte. \
         Tu parles TOUJOURS à la première personne du singulier (je, me, mon, j'ai, je vous…). \
         Interdit : il/elle pour te désigner, « Loggy », « nous », « l'assistant ».";
    let user = format!(
        "L'utilisateur a écrit : « {} »\n\
         Tu vas lui ouvrir l'écran de gestion de l'entité « {} » (clé technique : {}).\n\
         Rédige UNE phrase (une ligne) à la première personne : tu expliques que TU prépares ou affiches l'interface pour lui.\n\
         Exemples de ton (ne pas recopier mot pour mot) : « Je vous prépare l'interface… », « Je m'occupe d'ouvrir… ».\n\
         Varie les formulations. Ton professionnel et chaleureux.\n\
         Interdit : JSON, listes, guillemets englobants, plus d'une phrase, troisième personne.",
        user_message.trim(),
        entity_label,
        entity_key
    );

    let raw = LlamaServer::chat_with_options(
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
        0.72,
        96,
    )?;

    Ok(sanitize_phrase(&raw))
}

fn sanitize_phrase(raw: &str) -> String {
    let mut line = raw
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or(raw)
        .trim()
        .to_string();
    if line.starts_with("«") && line.ends_with("»") && line.chars().count() > 2 {
        line = line[1..line.len() - 1].trim().to_string();
    }
    line = line.trim_matches('"').trim().to_string();
    if line.len() > 220 {
        line.truncate(217);
        line.push_str("…");
    }
    line
}
