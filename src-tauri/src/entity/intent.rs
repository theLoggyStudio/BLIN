use super::registry::{EntityDef, EntityRegistry};
use crate::ai::intent_filters::normalize_message;

const ACTION_PREFIXES: &[&str] = &[
    "gerer les ",
    "gere les ",
    "gérer les ",
    "gerer ",
    "gere ",
    "gérer ",
    "ouvrir ",
    "afficher ",
    "voir ",
    "consulter ",
    "liste ",
    "lister ",
    "acceder ",
    "accéder ",
    "acceder a ",
    "accéder à ",
    "acceder aux ",
    "accéder aux ",
    "travailler sur ",
    "tableau ",
    "ecran ",
    "écran ",
    "entite ",
    "entité ",
    "entity ",
];

fn token_key(s: &str) -> String {
    s.replace(' ', "_")
}

fn singular_token(token: &str) -> String {
    let chars: Vec<char> = token.trim().chars().collect();
    if chars.len() > 3 && chars.last() == Some(&'s') {
        chars[..chars.len() - 1].iter().collect()
    } else {
        token.trim().to_string()
    }
}

pub(crate) fn entity_terms(ent: &EntityDef) -> Vec<String> {
    let nom = normalize_message(&ent.nom);
    let label = normalize_message(ent.label.as_deref().unwrap_or(&ent.nom));
    let mut terms = vec![nom.clone(), label.clone(), token_key(&nom), token_key(&label)];
    terms.push(singular_token(&nom));
    terms.push(singular_token(&label));
    match nom.as_str() {
        "users" | "user" => {
            terms.extend([
                "user".into(),
                "users".into(),
                "utilisateur".into(),
                "utilisateurs".into(),
            ]);
        }
        "clients" | "client" => {
            terms.extend(["client".into(), "clients".into()]);
        }
        "tache" | "taches" => {
            terms.extend([
                "tache".into(),
                "taches".into(),
                "task".into(),
                "tasks".into(),
            ]);
        }
        _ => {}
    }
    terms.retain(|t| !t.is_empty());
    terms.sort_by_key(|t| std::cmp::Reverse(t.len()));
    terms.dedup();
    terms
}

pub(crate) fn score_match(msg: &str, ent: &EntityDef) -> i32 {
    let nom = normalize_message(&ent.nom);
    let label = normalize_message(ent.label.as_deref().unwrap_or(&ent.nom));
    let mut score = 0i32;

    if msg == nom || msg == label || msg == token_key(&nom) {
        return 120;
    }

    for prefix in ACTION_PREFIXES {
        if let Some(rest) = msg.strip_prefix(prefix) {
            let rest = rest.trim();
            if rest == nom || rest == label || rest == singular_token(&nom) || rest == singular_token(&label) {
                score = score.max(110);
            }
        }
    }

    if msg.starts_with("gerer ") || msg.starts_with("gere ") {
        let rest = msg.split_once(' ').map(|(_, r)| r.trim()).unwrap_or("");
        if rest == nom || rest == label {
            score = score.max(105);
        }
    }

    for term in entity_terms(ent) {
        if term.len() < 2 {
            continue;
        }
        if msg == term {
            score = score.max(100);
        } else if msg.contains(&term) {
            let bonus = (term.len() as i32).min(40) + 25;
            score = score.max(bonus);
        } else if term.len() >= 4 && msg.len() >= 3 && (term.contains(msg) || msg.contains(&term)) {
            score = score.max(35);
        }
    }

    for word in msg.split_whitespace() {
        if word.len() < 2 {
            continue;
        }
        let w = singular_token(word);
        if w == nom || w == label || w == singular_token(&nom) {
            score = score.max(90);
        }
        for term in entity_terms(ent) {
            if w == term || singular_token(&term) == w {
                score = score.max(85);
            }
        }
    }

    score
}

/// Correspondance locale (sans LLM) — accepte une requête libre (« ecole », « gérer clients », etc.).
pub fn match_intent(message: &str, registry: &EntityRegistry) -> Option<String> {
    let msg = normalize_message(message);
    if msg.is_empty() || registry.entities.is_empty() {
        return None;
    }

    let mut best: Option<(String, i32)> = None;
    for ent in &registry.entities {
        if !ent.ai_suggestions {
            continue;
        }
        let score = score_match(&msg, ent);
        if score < 30 {
            continue;
        }
        if best.as_ref().map(|(_, s)| score > *s).unwrap_or(true) {
            best = Some((ent.nom.clone(), score));
        }
    }

    best.map(|(k, _)| k)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::registry::EntityDef;

    fn reg(entities: Vec<EntityDef>) -> EntityRegistry {
        EntityRegistry {
            ecosysteme: None,
            slogan: None,
            logo_url: None,
            logo: None,
            entities,
        }
    }

    #[test]
    fn match_single_word_entity_name() {
        let r = reg(vec![        EntityDef {
            nom: "ecole".into(),
            label: Some("École".into()),
            description: None,
            ai_suggestions: true,
            requires_signature: false,
            signatory_role_ids: vec![],
            is_session: false,
            attributs: vec![],
        }]);
        assert_eq!(match_intent("ecole", &r).as_deref(), Some("ecole"));
        assert_eq!(match_intent("école", &r).as_deref(), Some("ecole"));
    }

    #[test]
    fn match_gerer_label() {
        let r = reg(vec![EntityDef {
            nom: "clients".into(),
            label: Some("Clients".into()),
            description: None,
            ai_suggestions: true,
            requires_signature: false,
            signatory_role_ids: vec![],
            is_session: false,
            attributs: vec![],
        }]);
        assert_eq!(match_intent("gerer clients", &r).as_deref(), Some("clients"));
    }
}
