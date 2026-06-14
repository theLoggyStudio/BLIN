//! Salutations et réponses sociales — hors outils / hors LLM.

use chrono::Timelike;
use crate::ai::intent_filters::normalize_message;
use crate::db::DashboardStats;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GreetingKind {
    Hello,
    Thanks,
    Goodbye,
    HowAreYou,
    WhoAreYou,
    Help,
    Unknown,
}

const BUSINESS_HINTS: &[&str] = &[
    "loyer",
    "liste",
    "cree",
    "creer",
    "supprime",
    "modifie",
    "pdf",
    "export",
    "generer",
    "reference",
    "ref ",
    "fin-",
    "loy-",
    "ctr",
];

const HELLO: &[&str] = &[
    "bonjour",
    "bonsoir",
    "salut",
    "coucou",
    "hello",
    "hi",
    "hey",
    "bon matin",
    "bonne journee",
    "bonne soiree",
];

const THANKS: &[&str] = &[
    "merci",
    "thanks",
    "thank you",
    "remercie",
    "remerciements",
    "parfait merci",
    "super merci",
];

const GOODBYE: &[&str] = &[
    "au revoir",
    "a plus",
    "a bientot",
    "bye",
    "ciao",
    "bonne journee",
    "bonne soiree",
    "a tout a l heure",
];

const HOW_ARE_YOU: &[&str] = &[
    "comment vas tu",
    "comment allez vous",
    "ca va",
    "comment ca va",
    "tu vas bien",
    "vous allez bien",
];

const WHO_ARE_YOU: &[&str] = &[
    "qui es tu",
    "tu es qui",
    "c est quoi loggy",
    "presente toi",
    "ton nom",
    "qui etes vous",
];

const HELP: &[&str] = &[
    "aide",
    "help",
    "que peux tu faire",
    "que sais tu faire",
    "comment tu fonctionnes",
    "comment utiliser",
    "tes capacites",
];

fn contains_any(n: &str, words: &[&str]) -> bool {
    words.iter().any(|w| n.contains(w))
}

fn is_pure_social_message(n: &str) -> bool {
    if contains_any(n, BUSINESS_HINTS) {
        return false;
    }
    let words: Vec<&str> = n.split_whitespace().collect();
    if words.len() > 14 {
        return false;
    }
    contains_any(n, HELLO)
        || contains_any(n, THANKS)
        || contains_any(n, GOODBYE)
        || contains_any(n, HOW_ARE_YOU)
        || contains_any(n, WHO_ARE_YOU)
        || contains_any(n, HELP)
}

pub fn classify_greeting(message: &str) -> Option<GreetingKind> {
    let n = normalize_message(message);
    if !is_pure_social_message(&n) {
        return None;
    }
    if contains_any(&n, THANKS) {
        return Some(GreetingKind::Thanks);
    }
    if contains_any(&n, GOODBYE) {
        return Some(GreetingKind::Goodbye);
    }
    if contains_any(&n, HOW_ARE_YOU) {
        return Some(GreetingKind::HowAreYou);
    }
    if contains_any(&n, WHO_ARE_YOU) {
        return Some(GreetingKind::WhoAreYou);
    }
    if contains_any(&n, HELP) {
        return Some(GreetingKind::Help);
    }
    if contains_any(&n, HELLO) {
        return Some(GreetingKind::Hello);
    }
    None
}

pub fn wants_greeting_intent(message: &str) -> bool {
    classify_greeting(message).is_some()
}

fn pick_variant<'a>(variants: &'a [&'a str], seed: &str) -> &'a str {
    let idx = seed
        .bytes()
        .fold(0usize, |acc, b| acc.wrapping_add(b as usize))
        % variants.len();
    variants[idx]
}

fn pick_variant_owned(variants: &[String], seed: &str) -> String {
    let idx = seed
        .bytes()
        .fold(0usize, |acc, b| acc.wrapping_add(b as usize))
        % variants.len();
    variants[idx].clone()
}

fn time_greeting_fr() -> &'static str {
    let hour = chrono::Local::now().hour();
    match hour {
        5..=11 => "Bonjour",
        12..=17 => "Bonjour",
        18..=21 => "Bonsoir",
        _ => "Bonsoir",
    }
}

/// Message de bienvenue affiché à la connexion (toast Loggy personnifié).
pub fn format_login_greeting(user_name: &str, app_name: &str) -> String {
    let t = time_greeting_fr();
    let name = user_name.trim();
    let variants = if name.is_empty() {
        vec![
            format!("Connexion — {t} ! Bienvenue sur {app_name}. Loggy est prêt à vous accompagner."),
            format!("Connexion — {t} ! Ravi de vous retrouver sur {app_name}. Que souhaitez-vous faire ?"),
        ]
    } else {
        vec![
            format!("Connexion — {t} {name} ! Bienvenue sur {app_name}. Loggy est prêt à vous accompagner."),
            format!("Connexion — {t} {name} ! Content de vous revoir sur {app_name}. Dites-moi ce dont vous avez besoin."),
            format!("Connexion — {t} {name} ! Heureux de vous accueillir sur {app_name}. Je suis Loggy, à votre service."),
        ]
    };
    pick_variant_owned(&variants, name)
}

fn stats_snippet(_stats: &DashboardStats) -> String {
    "Consultez vos entités depuis le tableau de bord (barre de commande).".to_string()
}

pub fn format_greeting_reply(
    kind: GreetingKind,
    user_message: &str,
    stats: Option<&DashboardStats>,
    app_name: &str,
) -> String {
    let stats_line = stats.map(stats_snippet);

    match kind {
        GreetingKind::Hello => {
            let t = time_greeting_fr();
            let variants = [
                format!(
                    "{t} ! Je suis Loggy, votre assistant {app_name}. Je peux gérer vos entités métier — dites-moi ce dont vous avez besoin."
                ),
                format!(
                    "{t} ! Ravi de vous retrouver. Listes, créations, modifications, exports PDF : posez votre question en français."
                ),
                format!(
                    "{t} ! Loggy à votre service. Exemple : « gérer les tâches » ou « liste les enregistrements tache »."
                ),
            ];
            let idx = user_message
                .bytes()
                .fold(0usize, |acc, b| acc.wrapping_add(b as usize))
                % variants.len();
            let mut msg = variants[idx].clone();
            if let Some(s) = stats_line {
                msg.push_str("\n\n");
                msg.push_str(&s);
            }
            msg
        }
        GreetingKind::Thanks => {
            let variants = vec![
                format!("Avec plaisir ! N'hésitez pas si vous avez une autre question sur {app_name}."),
                "Je vous en prie. Loggy reste disponible pour la suite.".into(),
                "De rien ! Bonne continuation.".into(),
            ];
            pick_variant_owned(&variants, user_message)
        }
        GreetingKind::Goodbye => {
            let variants = vec![
                format!("Au revoir ! {app_name} reste ouvert quand vous en aurez besoin."),
                "À bientôt ! Passez une excellente journée.".into(),
                "Bonne continuation — je serai là pour vos prochaines questions.".into(),
            ];
            pick_variant_owned(&variants, user_message)
        }
        GreetingKind::HowAreYou => {
            let variants = vec![
                format!("Je fonctionne correctement, merci ! Et vous — que souhaitez-vous faire dans {app_name} aujourd'hui ?"),
                "Tout va bien de mon côté : prêt à lister, créer ou mettre à jour vos données. Comment puis-je vous aider ?".into(),
                "Opérationnel et à votre écoute. Une entité ou une action en tête ?".into(),
            ];
            pick_variant_owned(&variants, user_message)
        }
        GreetingKind::WhoAreYou => {
            let variants = vec![
                format!("Je suis Loggy, l'assistant local intégré à {app_name}. J'exécute des actions sur vos données (avec confirmation pour les modifications), 100 % hors ligne sur ce poste."),
                format!("Loggy — assistant IA de {app_name} : gestion des entités, listes, formulaires et exports. J'automatise le quotidien du logiciel."),
            ];
            pick_variant_owned(&variants, user_message)
        }
        GreetingKind::Help => {
            let mut msg = format!(
                "Voici ce que je peux faire dans {app_name} :\n\
                • **Gérer les entités** déclarées dans Paramètres (ex. « gérer les tâches »)\n\
                • **Lister / créer / modifier / supprimer** via outils dda_* (confirmation requise)\n\
                • **Répondre** aux questions pratiques (hors ligne ou avec recherche Internet si activée)\n\n\
                Exemple : « gérer tache » ou « liste les enregistrements de l'entité tache »."
            );
            if let Some(s) = stats_line {
                msg.push_str("\n\n");
                msg.push_str(&s);
            }
            msg
        }
        GreetingKind::Unknown => format!("Bonjour ! Comment puis-je vous aider avec {app_name} ?"),
    }
}
