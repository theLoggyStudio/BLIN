use crate::ai::llama_server::{ChatMessage, LlamaServer};
use crate::db::Database;

pub fn fallback_denial_phrase(entity_label: &str, contact_roles: &[String]) -> String {
    if contact_roles.is_empty() {
        return format!(
            "Je ne peux pas ouvrir « {entity_label} » : vous n'avez pas les droits nécessaires. \
             Aucun rôle n'est configuré pour cette entité — contactez votre administrateur."
        );
    }
    if contact_roles.len() == 1 {
        return format!(
            "Je ne peux pas ouvrir « {entity_label} » car vous n'avez pas les droits nécessaires. \
             Contactez une personne ayant le rôle « {} ».",
            contact_roles[0]
        );
    }
    format!(
        "Je ne peux pas ouvrir « {entity_label} » car vous n'avez pas les droits nécessaires. \
         Contactez une personne ayant l'un de ces rôles : {}.",
        contact_roles.join(", ")
    )
}

/// Message Loggy lorsque l'utilisateur tente d'ouvrir une entité sans privilège.
pub fn generate_access_denied_phrase(
    db: &Database,
    user_message: &str,
    entity_key: &str,
    entity_label: &str,
    contact_roles: &[String],
) -> Result<String, String> {
    if !LlamaServer::model_ready() {
        return Ok(fallback_denial_phrase(entity_label, contact_roles));
    }
    let (profiled, _) = crate::ai::hardware_profile::profile_summary(db)?;
    if !profiled {
        LlamaServer::prepare(db, false)?;
    }

    let roles_line = if contact_roles.is_empty() {
        "Aucun rôle configuré — suggère de contacter l'administrateur.".to_string()
    } else if contact_roles.len() == 1 {
        format!("Rôle à contacter : « {} ».", contact_roles[0])
    } else {
        format!("Rôles à contacter : {}.", contact_roles.join(", "))
    };

    let system = "Tu es Loggy, l'assistant de l'application métier. Tu réponds uniquement en français, par une ou deux phrases courtes. \
         Tu parles TOUJOURS à la première personne du singulier (je, me, mon, j'ai, je vous…). \
         Interdit : il/elle pour te désigner, « Loggy », « nous », « l'assistant ».";
    let user = format!(
        "L'utilisateur a écrit : « {} »\n\
         Il souhaite accéder à l'entité « {} » (clé : {}), mais il n'a AUCUN privilège sur cette entité.\n\
         {}\n\
         Rédige un message poli : tu expliques que TU ne peux pas ouvrir cet écran pour lui faute de droits, \
         et tu lui indiques clairement quel(s) rôle(s) contacter.\n\
         Ton professionnel et bienveillant. Interdit : JSON, listes à puces, guillemets englobants, plus de deux phrases.",
        user_message.trim(),
        entity_label,
        entity_key,
        roles_line
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
        0.65,
        128,
    )?;

    Ok(sanitize_phrase(&raw))
}

fn sanitize_phrase(raw: &str) -> String {
    let mut line = raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
    if line.starts_with("«") && line.ends_with("»") && line.chars().count() > 2 {
        line = line[1..line.len() - 1].trim().to_string();
    }
    line = line.trim_matches('"').trim().to_string();
    if line.len() > 320 {
        line.truncate(317);
        line.push_str("…");
    }
    line
}
