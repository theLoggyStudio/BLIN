//! Détection « créer une entité dans le registre » (Paramètres) via Loggy.

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::registry::{EntityAttribute, EntityDef, EntityRegistry};
use crate::ai::intent_filters::{normalize_message, wants_create_verb};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntityCreateDraft {
    pub initial_entity: EntityDef,
    pub assistant_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryCreateMatchResult {
    pub matched: bool,
    pub allowed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft: Option<RegistryEntityCreateDraft>,
}

const REGISTRY_MARKERS: &[&str] = &[
    "entite ",
    "entites ",
    "entite",
    "nouvelle entite",
    "nouvel entite",
    "au registre",
    "dans parametres",
    "dans les parametres",
    "parametres entite",
    "parametre entite",
    "ecran entite",
    "definition entite",
    "definir une entite",
    "definir entite",
];

fn wants_registry_entity_create(msg: &str) -> bool {
    let n = normalize_message(msg);
    if !wants_create_verb(&n) {
        return false;
    }
    REGISTRY_MARKERS.iter().any(|m| n.contains(m))
}

fn slugify_key(raw: &str) -> String {
    let s: String = raw
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    s.split('_')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

fn humanize_label(key: &str) -> String {
    let words: Vec<&str> = key.split('_').filter(|w| !w.is_empty()).collect();
    if words.is_empty() {
        return key.to_string();
    }
    words
        .iter()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn strip_registry_prefix(msg: &str) -> String {
    let n = normalize_message(msg);
    let mut rest: &str = &n;
    const PREFIXES: &[&str] = &[
        "creer une entite ",
        "creer un entite ",
        "creer entite ",
        "cree une entite ",
        "cree entite ",
        "ajouter une entite ",
        "ajouter entite ",
        "ajoute une entite ",
        "ajoute entite ",
        "nouvelle entite ",
        "nouvel entite ",
        "definir une entite ",
        "definir entite ",
        "enregistrer une entite ",
        "enregistre une entite ",
    ];
    for p in PREFIXES {
        if let Some(r) = rest.strip_prefix(p) {
            rest = r;
            break;
        }
    }
    rest.trim().to_string()
}

fn extract_entity_name(msg: &str) -> Option<String> {
    let rest = strip_registry_prefix(msg);
    if rest.is_empty() {
        return None;
    }
    let stop_re = Regex::new(
        r"(?i)\b(avec|sans|comprenant|incluant|ayant|pour|dans|au|les|des|du|de la|de)\b",
    )
    .ok()?;
    let cut = stop_re.find(&rest).map(|m| m.start()).unwrap_or(rest.len());
    let name_part = rest[..cut].trim();
    if name_part.is_empty() {
        return None;
    }
    let key = slugify_key(name_part);
    if key.len() < 2 {
        return None;
    }
    Some(key)
}

fn default_entity_def(nom: &str) -> EntityDef {
    let label = humanize_label(nom);
    EntityDef {
        nom: nom.to_string(),
        label: Some(label.clone()),
        description: Some(format!("Entité « {label} » — à compléter dans le formulaire.")),
        ai_suggestions: true,
        requires_signature: false,
        signatory_role_ids: vec![],
        is_session: true,
        attributs: vec![
            EntityAttribute {
                nom: "id".into(),
                attr_type: "uuid".into(),
                label: None,
                required: true,
                r#ref: None,
                relation_multiple: false,
                relation_exclusive_parent: true,
                default: None,
                enum_options: None,
                relation_impact_source: None,
                relation_impact_target: None,
                relation_impact_action: None,
                relation_impact_defer: false,
                stock_alert_threshold: None,
                matricule_ref: None,
            },
            EntityAttribute {
                nom: "libelle".into(),
                attr_type: "string".into(),
                label: Some("Intitulé".into()),
                required: true,
                r#ref: None,
                relation_multiple: false,
                relation_exclusive_parent: true,
                default: None,
                enum_options: None,
                relation_impact_source: None,
                relation_impact_target: None,
                relation_impact_action: None,
                relation_impact_defer: false,
                stock_alert_threshold: None,
                matricule_ref: None,
            },
        ],
    }
}

fn build_assistant_message(ent: &EntityDef) -> String {
    let label = ent.label.as_deref().unwrap_or(&ent.nom);
    format!(
        "Je t'ouvre le formulaire pour créer l'entité « {label} » dans le registre. \
         Complète les champs à gauche, puis enregistre quand c'est prêt."
    )
}

/// Message utilisateur → brouillon de définition d'entité (registre).
pub fn match_registry_create_draft(
    message: &str,
    registry: &EntityRegistry,
) -> Option<RegistryEntityCreateDraft> {
    if !wants_registry_entity_create(message) {
        return None;
    }
    let nom = extract_entity_name(message)?;
    if registry.entities.iter().any(|e| e.nom == nom) {
        return None;
    }
    let initial_entity = default_entity_def(&nom);
    let assistant_message = build_assistant_message(&initial_entity);
    Some(RegistryEntityCreateDraft {
        initial_entity,
        assistant_message,
    })
}

pub fn match_registry_create_with_access(
    message: &str,
    registry: &EntityRegistry,
    allowed: bool,
) -> RegistryCreateMatchResult {
    let Some(draft) = match_registry_create_draft(message, registry) else {
        return RegistryCreateMatchResult {
            matched: false,
            allowed,
            draft: None,
        };
    };
    RegistryCreateMatchResult {
        matched: true,
        allowed,
        draft: if allowed { Some(draft) } else { None },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_registry() -> EntityRegistry {
        EntityRegistry {
            entities: vec![],
            ..Default::default()
        }
    }

    #[test]
    fn detects_registry_intent() {
        let r = empty_registry();
        let d = match_registry_create_draft("creer une entite fournisseur", &r).unwrap();
        assert_eq!(d.initial_entity.nom, "fournisseur");
    }

    #[test]
    fn rejects_existing_entity() {
        let mut r = empty_registry();
        r.entities.push(EntityDef {
            nom: "fournisseur".into(),
            label: Some("Fournisseur".into()),
            description: None,
            ai_suggestions: true,
            requires_signature: false,
            signatory_role_ids: vec![],
            is_session: true,
            attributs: vec![],
        });
        assert!(match_registry_create_draft("creer une entite fournisseur", &r).is_none());
    }

    #[test]
    fn ignores_record_create_without_entite_word() {
        let r = empty_registry();
        assert!(match_registry_create_draft("creer un client dupont", &r).is_none());
    }
}
