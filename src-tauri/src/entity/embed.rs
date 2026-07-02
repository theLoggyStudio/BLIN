//! Embarquement des valeurs d'une entité fille dans la table mère (single-table, sans FK).

use serde_json::{Map, Value};

use super::attr_types::is_reserved_attribute;
use super::compteur::{self, is_compteur_attr};
use super::registry::{EntityAttribute, EntityDef, EntityRegistry};
use super::schema::attr_column;

/// ID source conservé dans une ligne embarquée (copie depuis un enregistrement existant).
pub const EMBED_SOURCE_RECORD_ID: &str = "_source_record_id";

/// Plafond de quantité d'impact (stock article au moment de la copie, session UI).
pub const EMBED_STOCK_CAP: &str = "_embedStockCap";

pub fn ref_entity_key(attr: &EntityAttribute) -> Option<String> {
    if attr.attr_type != "entity" {
        return None;
    }
    attr.r#ref
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase().replace(' ', "_"))
}

pub fn resolve_child<'a>(
    registry: &'a EntityRegistry,
    attr: &EntityAttribute,
) -> Option<&'a EntityDef> {
    ref_entity_key(attr).and_then(|k| registry.find(&k))
}

/// Attributs copiables de l'entité fille (pas de liaison imbriquée ni id système).
pub fn copyable_child_attributes<'a>(child: &'a EntityDef) -> Vec<&'a EntityAttribute> {
    child
        .attributs
        .iter()
        .filter(|a| !is_reserved_attribute(a))
        .filter(|a| a.attr_type != "entity")
        .collect()
}

pub fn embedded_prefix(parent_attr: &EntityAttribute) -> String {
    attr_column(parent_attr)
}

pub fn embedded_column_name(parent_attr: &EntityAttribute, child_attr: &EntityAttribute) -> String {
    let prefix = embedded_prefix(parent_attr);
    if is_compteur_attr(child_attr) {
        format!("{prefix}_{}", attr_column(child_attr))
    } else {
        format!("{prefix}_{}", attr_column(child_attr))
    }
}

/// Colonnes SQLite à créer sur la table mère pour une liaison `entity`.
pub fn sql_columns_for_entity_attr(
    attr: &EntityAttribute,
    registry: &EntityRegistry,
) -> Vec<(String, &'static str)> {
    let Some(child) = resolve_child(registry, attr) else {
        return vec![(attr_column(attr), "TEXT")];
    };
    if attr.relation_multiple {
        return vec![];
    }
    let mut cols = Vec::new();
    for child_attr in copyable_child_attributes(child) {
        if is_compteur_attr(child_attr) {
            let embed_base = embedded_column_name(attr, child_attr);
            let child_col = super::schema::attr_column(child_attr);
            for col in compteur::all_sql_columns(child_attr) {
                let suffix = col
                    .strip_prefix(&format!("{child_col}_"))
                    .unwrap_or("");
                let mapped = if suffix.is_empty() {
                    embed_base.clone()
                } else {
                    format!("{embed_base}_{suffix}")
                };
                let col_type = if mapped.ends_with("_numero") {
                    "INTEGER"
                } else {
                    "TEXT"
                };
                cols.push((mapped, col_type));
            }
        } else {
            cols.push((
                embedded_column_name(attr, child_attr),
                sqlite_type(&child_attr.attr_type),
            ));
        }
    }
    cols
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::registry::{EntityAttribute, EntityDef, EntityRegistry};

    #[test]
    fn one_to_one_embed_uses_prefixed_columns_only() {
        let child = EntityDef {
            nom: "client".into(),
            label: None,
            description: None,
            ai_suggestions: true,
        requires_signature: false,
        signatory_role_ids: vec![],
            is_session: false,
            attributs: vec![EntityAttribute {
                nom: "nom".into(),
                attr_type: "string".into(),
                label: None,
                required: true,
                r#ref: None,
                relation_multiple: false,
                relation_exclusive_parent: true,
                default: None,
                enum_options: None,
                ..Default::default()
            }],
        };
        let parent_attr = EntityAttribute {
            nom: "client".into(),
            attr_type: "entity".into(),
            label: None,
            required: true,
            r#ref: Some("client".into()),
            relation_multiple: false,
            relation_exclusive_parent: true,
            default: None,
            enum_options: None,
            ..Default::default()
        };
        let registry = EntityRegistry {
            ecosysteme: None,
            slogan: None,
            logo_url: None,
            logo: None,
            entities: vec![child],
        };
        let cols: Vec<String> = sql_columns_for_entity_attr(&parent_attr, &registry)
            .into_iter()
            .map(|(c, _)| c)
            .collect();
        assert!(!cols.contains(&"client".to_string()));
        assert!(cols.iter().any(|c| c.starts_with("client_")));
    }
}

fn sqlite_type(attr_type: &str) -> &'static str {
    match attr_type {
        "number" | "integer" | "float" | "stock" => "REAL",
        "boolean" | "bool" => "INTEGER",
        "photo" | "image" => "TEXT",
        _ => "TEXT",
    }
}

/// Copie les valeurs d'un enregistrement fille vers les clés préfixées du parent.
pub fn values_from_child_row(
    parent_attr: &EntityAttribute,
    child: &EntityDef,
    child_row: &Map<String, Value>,
) -> Map<String, Value> {
    let mut out = Map::new();
    for child_attr in copyable_child_attributes(child) {
        if is_compteur_attr(child_attr) {
            let base = embedded_column_name(parent_attr, child_attr);
            for (suffix, key) in [
                ("libelle", compteur::column_libelle(child_attr)),
                ("jjmmaaaa", compteur::column_jjmmaaaa(child_attr)),
                ("numero", compteur::column_numero(child_attr)),
            ] {
                let col = format!("{base}_{suffix}");
                if let Some(v) = child_row.get(&key).or_else(|| child_row.get(key.as_str())) {
                    out.insert(col, v.clone());
                }
            }
        } else {
            let col = embedded_column_name(parent_attr, child_attr);
            let key = child_attr.nom.as_str();
            if let Some(v) = child_row.get(key).or_else(|| child_row.get(attr_column(child_attr).as_str()))
            {
                out.insert(col, v.clone());
            }
        }
    }
    out
}

/// Extrait un objet fille (clés sans préfixe) depuis les valeurs du formulaire parent.
pub fn child_object_from_parent_values(
    parent_attr: &EntityAttribute,
    child: &EntityDef,
    parent_values: &Map<String, Value>,
) -> Map<String, Value> {
    let mut out = Map::new();
    for child_attr in copyable_child_attributes(child) {
        if is_compteur_attr(child_attr) {
            let base = embedded_column_name(parent_attr, child_attr);
            for (suffix, dest) in [
                ("libelle", compteur::column_libelle(child_attr)),
                ("jjmmaaaa", compteur::column_jjmmaaaa(child_attr)),
                ("numero", compteur::column_numero(child_attr)),
            ] {
                let col = format!("{base}_{suffix}");
                if let Some(v) = parent_values.get(&col) {
                    out.insert(dest, v.clone());
                }
            }
        } else {
            let col = embedded_column_name(parent_attr, child_attr);
            if let Some(v) = parent_values.get(&col) {
                out.insert(child_attr.nom.clone(), v.clone());
            }
        }
    }
    out
}

/// Objet fille (clés = noms d'attributs) depuis une ligne SQLite de l'entité fille.
pub fn child_object_from_row(child: &EntityDef, child_row: &Map<String, Value>) -> Map<String, Value> {
    child_object_from_row_inner(child, child_row, false)
}

/// Copie pour liste embarquée : sans champs compteur/matricule (allège le JSON).
/// Si la fille n'a aucun libellé naturel, on lui donne un nom générique
/// « <entité> No. <matricule complète> » (ou l'identifiant à défaut de matricule).
pub fn child_object_from_row_for_embed_list(
    child: &EntityDef,
    child_row: &Map<String, Value>,
) -> Map<String, Value> {
    let mut out = child_object_from_row_inner(child, child_row, true);
    ensure_generic_embed_label(child, &mut out, child_row);
    if let Some(id) = child_row
        .get("id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        out.insert(
            EMBED_SOURCE_RECORD_ID.into(),
            Value::String(id.to_string()),
        );
    }
    out
}

/// Clés considérées comme libellé naturel d'une fille embarquée.
const EMBED_LABEL_KEYS: &[&str] = &["libelle", "nom", "titre", "reference", "intitule"];

fn value_non_empty(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::String(s) => !s.trim().is_empty(),
        _ => true,
    }
}

fn entity_display_label(child: &EntityDef) -> String {
    child
        .label
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| child.nom.clone())
}

fn numero_to_string(v: &Value) -> Option<String> {
    match v {
        Value::Number(n) => Some(n.to_string()),
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        _ => None,
    }
}

/// « Matricule complète » : `<base><date jjmmaaaa><compteur>`.
fn full_matricule(child: &EntityDef, child_row: &Map<String, Value>) -> Option<String> {
    for attr in child.attributs.iter().filter(|a| is_compteur_attr(a)) {
        if compteur::is_matricule_attr(attr) {
            let s = compteur::format_matricule_from_row(child_row, attr);
            if !s.is_empty() {
                return Some(s);
            }
            continue;
        }
        let date = child_row
            .get(&compteur::column_jjmmaaaa(attr))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let numero = child_row
            .get(&compteur::column_numero(attr))
            .and_then(numero_to_string)
            .map(|n| n.parse::<i64>().map(|x| format!("{x:04}")).unwrap_or(n));
        match (date, numero) {
            (Some(d), Some(n)) => return Some(format!("{d}-{n}")),
            (Some(d), None) => return Some(d),
            (None, Some(n)) => return Some(n),
            (None, None) => {}
        }
    }
    None
}

fn ensure_generic_embed_label(
    child: &EntityDef,
    out: &mut Map<String, Value>,
    child_row: &Map<String, Value>,
) {
    let has_label = EMBED_LABEL_KEYS
        .iter()
        .any(|k| out.get(*k).map(value_non_empty).unwrap_or(false));
    if has_label {
        return;
    }
    let entity_label = entity_display_label(child);
    let matricule = full_matricule(child, child_row).or_else(|| {
        child_row
            .get("id")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    });
    let generic = match matricule {
        Some(m) => format!("{entity_label} No. {m}"),
        None => entity_label,
    };
    out.insert("libelle".into(), Value::String(generic));
}

fn child_object_from_row_inner(
    child: &EntityDef,
    child_row: &Map<String, Value>,
    skip_compteur: bool,
) -> Map<String, Value> {
    let mut out = Map::new();
    for child_attr in copyable_child_attributes(child) {
        if skip_compteur && is_compteur_attr(child_attr) {
            continue;
        }
        if is_compteur_attr(child_attr) {
            for key in [
                compteur::column_libelle(child_attr),
                compteur::column_jjmmaaaa(child_attr),
                compteur::column_numero(child_attr),
            ] {
                if let Some(v) = child_row.get(&key) {
                    out.insert(key, v.clone());
                }
            }
        } else {
            let key = child_attr.nom.as_str();
            if let Some(v) = child_row
                .get(key)
                .or_else(|| child_row.get(attr_column(child_attr).as_str()))
            {
                out.insert(child_attr.nom.clone(), v.clone());
            }
        }
    }
    out
}
