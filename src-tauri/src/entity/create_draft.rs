//! Détection « créer une entité » + préremplissage partiel pour ouverture du modal.

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use super::attr_types::is_reserved_attribute;
use super::registry::{EntityAttribute, EntityDef, EntityRegistry};
use crate::ai::intent_filters::{normalize_message, wants_create_verb};
use crate::entity::intent::{entity_terms, score_match};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityCreateDraft {
    pub entity_key: String,
    pub entity_label: String,
    pub initial_data: Value,
    pub assistant_message: String,
}

/// Message utilisateur → brouillon de création (entité + champs connus).
pub fn match_create_draft(message: &str, registry: &EntityRegistry) -> Option<EntityCreateDraft> {
    let raw = message.trim();
    if raw.len() < 4 || !wants_create_verb(raw) {
        return None;
    }

    let msg = normalize_message(raw);
    let ent = best_entity_for_create(&msg, registry)?;
    let initial_data = extract_prefill(raw, &msg, &ent);
    let assistant_message = build_assistant_message(&ent, &initial_data);

    Some(EntityCreateDraft {
        entity_key: ent.nom.clone(),
        entity_label: ent.label.clone().unwrap_or_else(|| ent.nom.clone()),
        initial_data: Value::Object(initial_data),
        assistant_message,
    })
}

fn best_entity_for_create(msg: &str, registry: &EntityRegistry) -> Option<EntityDef> {
    let mut best: Option<(EntityDef, i32)> = None;
    for ent in &registry.entities {
        let score = score_match(msg, ent);
        if score < 28 {
            continue;
        }
        if best.as_ref().map(|(_, s)| score > *s).unwrap_or(true) {
            best = Some((ent.clone(), score));
        }
    }
    best.map(|(e, _)| e)
}

fn extract_prefill(raw: &str, msg: &str, ent: &EntityDef) -> Map<String, Value> {
    let mut data = Map::new();

    for attr in &ent.attributs {
        if is_reserved_attribute(attr) {
            continue;
        }
        if let Some(def) = &attr.default {
            data.insert(attr.nom.clone(), def.clone());
        }
    }

    let remainder = strip_create_prefix(raw, msg, ent);
    if let Some(text) = remainder {
        if let Some(attr) = primary_text_attribute(ent) {
            data.insert(attr.nom.clone(), json!(text));
        }
    }

    for attr in &ent.attributs {
        if is_reserved_attribute(attr) {
            continue;
        }
        if data.contains_key(&attr.nom) {
            continue;
        }
        if user_skips_field(msg, attr) {
            continue;
        }
        if let Some(v) = extract_attribute_value(raw, msg, attr) {
            data.insert(attr.nom.clone(), v);
        }
    }

    data
}

fn strip_create_prefix(_raw: &str, msg: &str, ent: &EntityDef) -> Option<String> {
    const PREFIXES: &[&str] = &[
        "creer une ",
        "creer un ",
        "cree une ",
        "cree un ",
        "ajouter une ",
        "ajouter un ",
        "ajoute une ",
        "ajoute un ",
        "nouvelle ",
        "nouveau ",
        "enregistrer une ",
        "enregistre une ",
    ];

    let mut rest = msg.to_string();
    for p in PREFIXES {
        if let Some(r) = rest.strip_prefix(p) {
            rest = r.trim().to_string();
            break;
        }
    }

    let terms: Vec<String> = entity_terms(ent)
        .into_iter()
        .map(|t| normalize_message(&t))
        .collect();
    terms.iter().for_each(|t| {
        if t.is_empty() {
            return;
        }
        if let Some(r) = rest.strip_prefix(t) {
            rest = r.trim().to_string();
        }
        if let Some(r) = rest.strip_prefix(&format!("{t} ")) {
            rest = r.trim().to_string();
        }
    });

    if let Some(idx) = rest.find(" sans ") {
        rest = rest[..idx].trim().to_string();
    }

    rest = rest
        .trim_matches(|c: char| c == ',' || c == '.' || c == ';' || c == ':')
        .trim()
        .to_string();

    if rest.len() >= 2 && !wants_create_verb(&rest) {
        Some(rest)
    } else {
        None
    }
}

fn regex_escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if "()[]{}.*+?^$|\\".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

fn primary_text_attribute(ent: &EntityDef) -> Option<&EntityAttribute> {
    const PRIORITY: &[&str] = &[
        "libelle",
        "titre",
        "intitule",
        "nom",
        "objet",
        "contenu",
        "description",
        "motif",
        "appreciation",
        "sujet",
    ];
    for key in PRIORITY {
        if let Some(a) = ent.attributs.iter().find(|a| a.nom == *key) {
            if !is_reserved_attribute(a) && a.attr_type == "string" {
                return Some(a);
            }
        }
    }
    ent.attributs
        .iter()
        .find(|a| !is_reserved_attribute(a) && matches!(a.attr_type.as_str(), "string" | "email"))
}

fn user_skips_field(msg: &str, attr: &EntityAttribute) -> bool {
    let n = normalize_message(&attr.nom);
    let label = normalize_message(attr.label.as_deref().unwrap_or(&attr.nom));
    for hint in [
        format!("sans {n}"),
        format!("sans {label}"),
        format!("pas de {n}"),
        format!("pas de {label}"),
        format!("sans l {n}"),
        format!("sans la {n}"),
        format!("sans le {n}"),
    ] {
        if msg.contains(&hint) {
            return true;
        }
    }
    false
}

fn extract_attribute_value(raw: &str, msg: &str, attr: &EntityAttribute) -> Option<Value> {
    let nom = &attr.nom;
    let label = attr.label.as_deref().unwrap_or(nom);

    if let Some(v) = extract_quoted_after_keys(raw, &[nom, label, "intitule", "titre"]) {
        return Some(json!(v));
    }

    match attr.attr_type.as_str() {
        "boolean" => {
            if msg.contains(&format!("{nom} oui"))
                || msg.contains("oui pour")
                || msg.contains("urgent")
            {
                return Some(json!(true));
            }
            if msg.contains(&format!("{nom} non")) || msg.contains("pas urgent") {
                return Some(json!(false));
            }
            None
        }
        "integer" | "number" | "float" => {
            let re = Regex::new(&format!(
                r"(?i)(?:{nom}|{label})\s*[:=]?\s*(\d+(?:[.,]\d+)?)"
            ))
            .ok()?;
            re.captures(raw)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().replace(',', "."))
                .and_then(|s| s.parse::<f64>().ok())
                .map(|n| {
                    if attr.attr_type == "integer" {
                        json!(n as i64)
                    } else {
                        json!(n)
                    }
                })
        }
        "date" => extract_date(raw).map(|d| json!(d)),
        "time" | "datetime" => extract_time(raw).map(|t| json!(t)),
        "email" => {
            let re = Regex::new(r"(?i)[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}").ok()?;
            re.find(raw).map(|m| json!(m.as_str()))
        }
        _ => None,
    }
}

fn extract_quoted_after_keys(raw: &str, keys: &[&str]) -> Option<String> {
    for key in keys {
        let patterns = [
            format!(r#"(?i){}\s*[:=]\s*["']([^"']+)["']"#, regex_escape(key)),
            format!(r#"(?i){}\s+["']([^"']+)["']"#, regex_escape(key)),
            format!(r"(?i){}\s*[:=]\s*«\s*([^»]+)\s*»", regex_escape(key)),
        ];
        for p in patterns {
            if let Ok(re) = Regex::new(&p) {
                if let Some(c) = re.captures(raw) {
                    if let Some(m) = c.get(1) {
                        let t = m.as_str().trim();
                        if !t.is_empty() {
                            return Some(t.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

fn extract_date(raw: &str) -> Option<String> {
    let re = Regex::new(r"(?i)(\d{1,2})[/.-](\d{1,2})[/.-](\d{2,4})").ok()?;
    let cap = re.captures(raw)?;
    let d = cap.get(1)?.as_str();
    let m = cap.get(2)?.as_str();
    let y = cap.get(3)?.as_str();
    let year = if y.len() == 2 {
        format!("20{y}")
    } else {
        y.to_string()
    };
    let month: u32 = m.parse().ok()?;
    let day: u32 = d.parse().ok()?;
    Some(format!("{year}-{month:02}-{day:02}"))
}

fn extract_time(raw: &str) -> Option<String> {
    let re = Regex::new(r"(?i)(\d{1,2})[:h](\d{2})").ok()?;
    let cap = re.captures(raw)?;
    let h = cap.get(1)?.as_str();
    let m = cap.get(2)?.as_str();
    let hour: u32 = h.parse().ok()?;
    let minute: u32 = m.parse().ok()?;
    Some(format!("{hour:02}:{minute:02}"))
}

fn build_assistant_message(ent: &EntityDef, data: &Map<String, Value>) -> String {
    let label = ent.label.as_deref().unwrap_or(&ent.nom);
    let mut missing: Vec<String> = Vec::new();
    for attr in &ent.attributs {
        if is_reserved_attribute(attr) || !attr.required {
            continue;
        }
        if is_empty_value(data.get(&attr.nom)) {
            let lbl = attr.label.as_deref().unwrap_or(&attr.nom);
            missing.push(lbl.to_string());
        }
    }

    let mut parts = vec![format!(
        "J'ouvre le formulaire de création pour « {label} »."
    )];

    let filled: Vec<String> = data
        .iter()
        .filter(|(_, v)| !is_empty_value(Some(v)))
        .map(|(k, v)| {
            let lbl = ent
                .attributs
                .iter()
                .find(|a| &a.nom == k)
                .and_then(|a| a.label.as_deref())
                .unwrap_or(k.as_str());
            format!("{lbl} : {v}")
        })
        .collect();
    if !filled.is_empty() {
        parts.push(format!("J'ai prérempli : {}.", filled.join(", ")));
    }

    if missing.is_empty() {
        parts.push("Vérifiez les informations puis enregistrez.".into());
    } else {
        parts.push(format!(
            "Merci de compléter les champs obligatoires restants : {}.",
            missing.join(", ")
        ));
    }

    parts.join(" ")
}

fn is_empty_value(v: Option<&Value>) -> bool {
    match v {
        None => true,
        Some(Value::Null) => true,
        Some(Value::String(s)) => s.trim().is_empty(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::registry::EntityDef;

    fn sample_registry() -> EntityRegistry {
        EntityRegistry {
            entities: vec![            EntityDef {
                nom: "tache".into(),
                label: Some("Tâche".into()),
                description: None,
                ai_suggestions: true,
                requires_signature: false,
                signatory_role_ids: vec![],
                is_session: false,
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
                    },
                    EntityAttribute {
                        nom: "heure_debut".into(),
                        attr_type: "time".into(),
                        label: Some("Heure".into()),
                        required: true,
                        r#ref: None,
                        relation_multiple: false,
                        relation_exclusive_parent: true,
                        default: None,
                        enum_options: None,
                    },
                ],
            }],
            ..Default::default()
        }
    }

    #[test]
    fn match_create_tache_without_time() {
        let r = sample_registry();
        let d = match_create_draft("creer une tache reviser le chapitre 3 sans heure", &r).unwrap();
        assert_eq!(d.entity_key, "tache");
        assert_eq!(
            d.initial_data.get("libelle").and_then(|v| v.as_str()),
            Some("reviser le chapitre 3")
        );
        assert!(d.initial_data.get("heure_debut").is_none());
        assert!(d.assistant_message.contains("Heure"));
    }
}
