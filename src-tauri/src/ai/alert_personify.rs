use crate::ai::llama_server::{ChatMessage, LlamaServer};
use crate::db::Database;

fn variant_tone(variant: &str) -> &'static str {
    match variant {
        "success" => "enthousiaste et rassurant — une bonne nouvelle à partager",
        "danger" => "direct mais bienveillant — un problème à signaler clairement",
        "warning" => "prudent sans alarmisme — une mise en garde amicale",
        _ => "naturel et utile — une info à transmettre avec clarté",
    }
}

/// Conserve plusieurs phrases (pas seulement la première ligne).
fn sanitize_message(raw: &str, max_len: usize) -> String {
    let mut text = raw.trim().to_string();
    if text.starts_with('«') && text.ends_with('»') && text.chars().count() > 2 {
        text = text
            .trim_start_matches('«')
            .trim_end_matches('»')
            .trim()
            .to_string();
    }
    text = text.trim_matches('"').trim().to_string();
    let text: String = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if text.chars().count() > max_len {
        let mut out: String = text.chars().take(max_len).collect();
        out.push_str("…");
        out
    } else {
        text
    }
}

/// Réécrit une notification système — Loggy parle à son collègue (phrases naturelles).
pub fn personify_alert(db: &Database, raw_message: &str, variant: &str) -> String {
    let trimmed = raw_message.trim();
    if trimmed.is_empty() || trimmed.len() > 800 {
        return trimmed.to_string();
    }
    if !LlamaServer::model_ready() {
        return trimmed.to_string();
    }
    if let Ok((profiled, _)) = crate::ai::hardware_profile::profile_summary(db) {
        if !profiled {
            let _ = LlamaServer::prepare(db, false);
        }
    }

    let app_name = crate::entity::branding::ecosystem_name(&db.data_dir);
    let tone = variant_tone(variant);
    let system = format!(
        "Tu es Loggy, l'assistant IA de {app_name}. Tu t'adresses à ton collègue utilisateur.\n\
         Règles strictes :\n\
         - Écris en français PARLÉ, en 2 ou 3 phrases COMPLÈTES (pas une seule ligne télégraphique).\n\
         - Première personne (je, j'ai, je viens de…). Tutoiement naturel, ton collègue de travail.\n\
         - Sois expressif et vivant : on doit entendre une vraie voix, pas une notification système.\n\
         - Intègre noms, chiffres et entités DANS les phrases (évite le style « titre : détail » ou listes sèches).\n\
         - Ton {tone}.\n\
         - Interdit : JSON, puces, guillemets englobants, te nommer à la 3e personne, plus de 3 phrases."
    );
    let variation = chrono::Utc::now().timestamp_subsec_nanos();
    let user = format!(
        "Notification à réécrire ({variant}) — variation {variation} :\n« {trimmed} »\n\
         Transforme ce message sec en paroles naturelles que tu dirais à ton collègue.\n\
         Exemples :\n\
         « PDF généré et téléchargé. » → « C'est bon, j'ai généré le PDF pour toi. Tu peux le récupérer dans tes téléchargements. »\n\
         « Import réussi pour « Véhicule » : 300 créé(s), 0 mis à jour. » → « L'import des véhicules est terminé. J'ai ajouté 300 nouvelles fiches et aucune mise à jour n'était nécessaire. »\n\
         « Tâche créée avec succès. » → « Je viens de créer la tâche, elle est prête dans ta liste. Tu peux la consulter quand tu veux. »\n\
         « Email — obligatoire » → « Il manque l'adresse e-mail sur ce champ. Tu peux la compléter avant qu'on enregistre ? »"
    );

    match LlamaServer::chat_with_options(
        Some(db),
        vec![
            ChatMessage {
                role: "system".into(),
                content: system,
            },
            ChatMessage {
                role: "user".into(),
                content: user,
            },
        ],
        0.78,
        220,
    ) {
        Ok(raw) => {
            let out = sanitize_message(&raw, 480);
            if out.len() >= 16 {
                out
            } else {
                trimmed.to_string()
            }
        }
        Err(_) => trimmed.to_string(),
    }
}

/// Réécrit un rappel de tâche — Loggy parle à son collègue (1re personne).
pub fn personify_task_reminder(db: &Database, raw_message: &str) -> String {
    let trimmed = raw_message.trim();
    if trimmed.is_empty() || trimmed.len() > 800 {
        return trimmed.to_string();
    }
    if !LlamaServer::model_ready() {
        return trimmed.to_string();
    }
    if let Ok((profiled, _)) = crate::ai::hardware_profile::profile_summary(db) {
        if !profiled {
            let _ = LlamaServer::prepare(db, false);
        }
    }

    let app_name = crate::entity::branding::ecosystem_name(&db.data_dir);
    let system = format!(
        "Tu es Loggy, l'assistant IA de {app_name}. Tu rappelles à ton collègue une tâche planifiée.\n\
         Règles strictes :\n\
         - Deux à trois phrases complètes en français parlé, première personne (je te rappelle…).\n\
         - Expressif et naturel — comme si tu passais la tête dans son bureau.\n\
         - Conserve l'intitulé, la date, l'heure, la description et la priorité si présents.\n\
         - Interdit : JSON, puces, guillemets englobants, te nommer à la 3e personne, plus de 3 phrases."
    );
    let variation = chrono::Utc::now().timestamp_subsec_nanos();
    let user = format!(
        "Rappel de tâche à réécrire — variation {variation} :\n« {trimmed} »\n\
         Réécris-le en paroles naturelles, comme si tu venais de le dire à ton collègue."
    );

    match LlamaServer::chat_with_options(
        Some(db),
        vec![
            ChatMessage {
                role: "system".into(),
                content: system,
            },
            ChatMessage {
                role: "user".into(),
                content: user,
            },
        ],
        0.78,
        260,
    ) {
        Ok(raw) => {
            let out = sanitize_message(&raw, 520);
            if out.len() >= 20 {
                out
            } else {
                trimmed.to_string()
            }
        }
        Err(_) => trimmed.to_string(),
    }
}
