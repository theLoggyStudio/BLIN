//! Messages de connexion pré-personnifiés au démarrage (salutation + identifiants invalides).

use crate::ai::alert_personify::personify_alert;
use crate::ai::greetings;
use crate::ai::llama_server::LlamaServer;
use crate::db::Database;

pub const LOGIN_INVALID_RAW: &str =
    "Identifiants invalides — e-mail ou mot de passe incorrect.";

#[derive(Clone, Default, Debug)]
pub struct PreparedLoginMessages {
    pub greeting: String,
    pub invalid_credentials: String,
    pub prepared: bool,
}

/// Retour immédiat sans attendre llama-server (évite de bloquer l'écran de connexion).
pub fn prepare_fast(db: &Database) -> PreparedLoginMessages {
    let app_name = crate::entity::branding::ecosystem_name(&db.data_dir);
    if !LlamaServer::model_ready() {
        return PreparedLoginMessages {
            greeting: fallback_greeting(&app_name),
            invalid_credentials: fallback_invalid(),
            prepared: false,
        };
    }
    prepare_with_llama(db)
}

/// Prépare les messages Loggy pour la page de connexion (salutation sans nom + erreur identifiants).
pub fn prepare(db: &Database) -> PreparedLoginMessages {
    if !LlamaServer::model_ready() {
        return prepare_fast(db);
    }
    prepare_with_llama(db)
}

fn prepare_with_llama(db: &Database) -> PreparedLoginMessages {
    let app_name = crate::entity::branding::ecosystem_name(&db.data_dir);
    let raw_greeting = greetings::format_login_greeting("", &app_name);
    let greeting = personify_alert(db, &raw_greeting, "success");
    let invalid_credentials = personify_alert(db, LOGIN_INVALID_RAW, "danger");
    let greeting = if greeting.trim().len() >= 12 {
        greeting
    } else {
        fallback_greeting(&app_name)
    };
    let invalid_credentials = if invalid_credentials.trim().len() >= 12 {
        invalid_credentials
    } else {
        fallback_invalid()
    };
    PreparedLoginMessages {
        greeting,
        invalid_credentials,
        prepared: true,
    }
}

/// Insère le prénom/nom dans une salutation déjà personnifiée (sans nom).
pub fn inject_user_name_into_greeting(prepared: &str, user_name: &str) -> String {
    let name = user_name.trim();
    if name.is_empty() {
        return prepared.to_string();
    }
    let patterns = [
        ("Bonjour !", format!("Bonjour {name} !")),
        ("Bonsoir !", format!("Bonsoir {name} !")),
        ("Bonjour,", format!("Bonjour {name},")),
        ("Bonsoir,", format!("Bonsoir {name},")),
        ("bonjour !", format!("Bonjour {name} !")),
        ("bonsoir !", format!("Bonsoir {name} !")),
    ];
    for (from, to) in &patterns {
        if prepared.contains(from) {
            return prepared.replacen(from, to, 1);
        }
    }
    format!("{name}, {prepared}")
}

pub fn fallback_greeting(app_name: &str) -> String {
    let t = greetings::format_login_greeting("", app_name);
    t.strip_prefix("Connexion — ")
        .unwrap_or(&t)
        .trim()
        .to_string()
}

pub fn fallback_invalid() -> String {
    "Je n'ai pas reconnu cet e-mail ou ce mot de passe. Vérifie les identifiants et réessaie.".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inject_name_after_bonjour() {
        let s = inject_user_name_into_greeting("Bonjour ! Bienvenue sur Blin.", "Marie");
        assert!(s.contains("Bonjour Marie !"));
    }
}
