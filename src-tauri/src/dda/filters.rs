//! Filtres DDA — opérateur par type d'attribut, SQL et extraction chat Loggy.

use std::collections::HashMap;

use regex::Regex;
use rusqlite::types::Value as SqlValue;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::config::{FieldDef, ScreenConfigFile};
use crate::ai::intent_filters::normalize_message;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterFieldRule {
    pub key: String,
    pub column: String,
    pub label: String,
    pub field_type: String,
    pub attr_type: String,
    pub operator: String,
    pub enabled: bool,
    pub chat_operators: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterCatalog {
    pub screen_key: String,
    pub entity_label: String,
    pub fields: Vec<FilterFieldRule>,
}

/// Opérateur SQL par type d'attribut registre entité (trigger auto à chaque sync).
pub fn filter_operator_for_attr_type(attr_type: &str) -> Option<&'static str> {
    match attr_type {
        "string" | "email" | "matricule" => Some("contains"),
        "number" | "integer" | "float" | "stock" | "compteur" => Some("equals"),
        "enum" | "boolean" | "bool" | "date" | "time" | "datetime" => Some("equals"),
        "entity" => Some("equals"),
        _ => None,
    }
}

/// Active le filtre UI + chat pour cet attribut ?
pub fn filter_enabled_for_attr_type(attr_type: &str, list_enabled: bool) -> bool {
    if !list_enabled {
        return false;
    }
    matches!(
        attr_type,
        "string"
            | "email"
            | "enum"
            | "date"
            | "time"
            | "datetime"
            | "number"
            | "integer"
            | "float"
            | "stock"
            | "boolean"
            | "bool"
            | "entity"
            | "matricule"
            | "compteur"
    )
}

pub fn filter_meta_for_attribute(
    attr_type: &str,
    list_enabled: bool,
) -> Option<super::config::FieldFilterMeta> {
    if !filter_enabled_for_attr_type(attr_type, list_enabled) {
        return None;
    }
    Some(super::config::FieldFilterMeta {
        enabled: true,
        operator: filter_operator_for_attr_type(attr_type).map(String::from),
    })
}

pub fn effective_operator(field: &FieldDef) -> &str {
    field
        .filter
        .as_ref()
        .and_then(|f| f.operator.as_deref())
        .unwrap_or("contains")
}

pub fn build_filter_catalog(cfg: &ScreenConfigFile) -> FilterCatalog {
    let fields = cfg
        .fields
        .iter()
        .filter(|f| f.filter.as_ref().is_some_and(|x| x.enabled))
        .map(|f| {
            let attr_type = infer_attr_type_from_field(f);
            FilterFieldRule {
                key: f.key.clone(),
                column: f.column.clone(),
                label: f.label.clone(),
                field_type: f.field_type.clone(),
                attr_type,
                operator: effective_operator(f).to_string(),
                enabled: true,
                chat_operators: vec![
                    "=".into(),
                    "==".into(),
                    "egal".into(),
                    "egale".into(),
                    "est".into(),
                    "vaut".into(),
                ],
            }
        })
        .collect();
    FilterCatalog {
        screen_key: cfg.screen.key.clone(),
        entity_label: cfg.screen.label.clone(),
        fields,
    }
}

fn infer_attr_type_from_field(field: &FieldDef) -> String {
    match field.field_type.as_str() {
        "number" => "number".into(),
        "boolean" => "boolean".into(),
        "select" => "enum".into(),
        "date" | "time" | "datetime" => field.field_type.clone(),
        "entity_ref" => "entity".into(),
        _ => "string".into(),
    }
}

pub fn format_filter_knowledge(cfg: &ScreenConfigFile, catalog: &FilterCatalog) -> String {
    let mut s = format!(
        "=== FILTRES — {} ({}) ===\n\
         Généré automatiquement par trigger_filters à chaque sync entité.\n\
         FilterBar UI + chat Loggy : « liste les {key} de {{label}} = {{valeur}} ».\n\n",
        cfg.screen.label,
        cfg.screen.key,
        key = cfg.screen.key,
    );
    for rule in &catalog.fields {
        s.push_str(&format!(
            "- {} (key={}, type={}, op={}) — exemple chat : « liste les {} où {} = … »\n",
            rule.label, rule.key, rule.attr_type, rule.operator, cfg.screen.key, rule.label,
        ));
    }
    if catalog.fields.is_empty() {
        s.push_str("(Aucun attribut filtrable sur cette entité.)\n");
    }
    s
}

/// Ajoute une clause SQL AND pour un filtre champ/valeur.
pub fn append_field_filter_sql(
    sql: &mut String,
    params: &mut Vec<SqlValue>,
    field: &FieldDef,
    value: &str,
    force_exact: bool,
) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return;
    }
    let op = if force_exact {
        "equals"
    } else {
        effective_operator(field)
    };
    match op {
        "equals" => {
            if matches!(field.field_type.as_str(), "number" | "integer" | "float") {
                if let Ok(n) = trimmed.replace(',', ".").parse::<f64>() {
                    sql.push_str(&format!(" AND {} = ?", field.column));
                    params.push(SqlValue::Real(n));
                    return;
                }
            }
            sql.push_str(&format!(" AND {} = ?", field.column));
            params.push(SqlValue::Text(trimmed.to_string()));
        }
        "gte" => {
            sql.push_str(&format!(" AND {} >= ?", field.column));
            params.push(SqlValue::Text(trimmed.to_string()));
        }
        "lte" => {
            sql.push_str(&format!(" AND {} <= ?", field.column));
            params.push(SqlValue::Text(trimmed.to_string()));
        }
        _ => {
            sql.push_str(&format!(" AND {} LIKE ?", field.column));
            params.push(SqlValue::Text(format!("%{trimmed}%")));
        }
    }
}

fn field_term_regex(term: &str) -> String {
    term.split_whitespace()
        .map(regex::escape)
        .collect::<Vec<_>>()
        .join(r"\s+")
}

fn match_filter_clause(text: &str, field_term: &str) -> Option<(String, String)> {
    let term_pat = field_term_regex(field_term);
    let re = Regex::new(&format!(
        r"(?i)(?:\bde\s+|\bavec\s+|\bwhere\s+|\bou\s+)?{term_pat}\s*(?:=|==|egal|egale|egaux|est|vaut|>=|<=|>|<)\s*(?:['']([^'']+)['']|([^\s,;]+))"
    ))
    .ok()?;
    let caps = re.captures(text)?;
    let full = caps.get(0)?.as_str().to_string();
    let value = caps
        .get(1)
        .or_else(|| caps.get(2))?
        .as_str()
        .trim()
        .trim_matches(|c| c == '.' || c == '?')
        .to_string();
    if value.is_empty() {
        return None;
    }
    Some((value, full))
}

/// Extrait les filtres « label = valeur » depuis une demande liste chat.
pub fn extract_filters_from_message(
    message: &str,
    cfg: &ScreenConfigFile,
) -> (HashMap<String, String>, String) {
    let mut filters = HashMap::new();
    let mut remaining = message.to_string();

    let mut field_terms: Vec<(&FieldDef, String, String)> = cfg
        .fields
        .iter()
        .filter(|f| f.filter.as_ref().is_some_and(|x| x.enabled))
        .map(|f| (f, normalize_message(&f.label), normalize_message(&f.key)))
        .collect();
    field_terms.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| b.2.len().cmp(&a.2.len())));

    for (field, label_norm, key_norm) in field_terms {
        for term in [label_norm.as_str(), key_norm.as_str()] {
            if term.len() < 2 {
                continue;
            }
            if let Some((value, matched)) = match_filter_clause(&remaining, term) {
                filters.insert(field.key.clone(), value);
                remaining = remaining.replacen(&matched, " ", 1);
                break;
            }
        }
    }

    (filters, remaining.trim().to_string())
}

pub fn cell_matches_filter(val: Option<&Value>, expected: &str, operator: &str) -> bool {
    let expected = expected.trim();
    let force_exact = operator == "equals" || operator == "gte" || operator == "lte";
    let Some(v) = val else {
        return expected.is_empty();
    };
    match v {
        Value::Null => expected.is_empty(),
        Value::Number(n) => {
            if let (Some(a), Ok(b)) = (n.as_f64(), expected.replace(',', ".").parse::<f64>()) {
                if force_exact {
                    (a - b).abs() < f64::EPSILON * 10.0 || a == b
                } else {
                    n.to_string().contains(expected)
                }
            } else {
                n.to_string() == expected
            }
        }
        Value::Bool(b) => {
            let e = normalize_message(expected);
            (*b && matches!(e.as_str(), "1" | "true" | "oui" | "vrai"))
                || (!*b && matches!(e.as_str(), "0" | "false" | "non" | "faux"))
        }
        Value::String(s) => {
            if force_exact {
                normalize_message(s) == normalize_message(expected)
            } else {
                normalize_message(s).contains(&normalize_message(expected))
            }
        }
        other => {
            if force_exact {
                normalize_message(&other.to_string()) == normalize_message(expected)
            } else {
                normalize_message(&other.to_string()).contains(&normalize_message(expected))
            }
        }
    }
}

pub fn apply_exact_filters(
    rows: Vec<serde_json::Map<String, Value>>,
    cfg: &ScreenConfigFile,
    filters: &HashMap<String, String>,
) -> Vec<serde_json::Map<String, Value>> {
    if filters.is_empty() {
        return rows;
    }
    let field_by_key: HashMap<&str, &FieldDef> = cfg.fields.iter().map(|f| (f.key.as_str(), f)).collect();
    rows.into_iter()
        .filter(|row| {
            filters.iter().all(|(key, expected)| {
                let op = field_by_key
                    .get(key.as_str())
                    .map(|f| effective_operator(f))
                    .unwrap_or("equals");
                cell_matches_filter(row.get(key), expected, op)
            })
        })
        .collect()
}

pub fn filters_summary(cfg: &ScreenConfigFile, filters: &HashMap<String, String>) -> String {
    if filters.is_empty() {
        return String::new();
    }
    let field_by_key: HashMap<&str, &FieldDef> = cfg.fields.iter().map(|f| (f.key.as_str(), f)).collect();
    filters
        .iter()
        .map(|(key, val)| {
            let label = field_by_key
                .get(key.as_str())
                .map(|f| f.label.as_str())
                .unwrap_or(key.as_str());
            format!("{label} = {val}")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dda::config::{
        FieldFilterMeta, FieldFormMeta, FieldListMeta, ScreenLayout, ScreenMeta, ScreenPrivileges,
    };

    fn sample_cfg() -> ScreenConfigFile {
        ScreenConfigFile {
            screen: ScreenMeta {
                key: "article".into(),
                label: "Article".into(),
                label_plural: None,
                icon: None,
                route: "/article".into(),
                system: false,
                ai_editable: false,
                table: "entity_article".into(),
                primary_key: "id".into(),
                label_field: "nom".into(),
                default_order_by: None,
                privileges: ScreenPrivileges {
                    view: "article:voir".into(),
                    create: "article:creer".into(),
                    update: "article:modifier".into(),
                    delete: "article:supprimer".into(),
                    import: None,
                    export: None,
                },
                print: None,
                storage: None,
            },
            fields: vec![FieldDef {
                key: "qte_initial".into(),
                column: "qte_initial".into(),
                field_type: "number".into(),
                label: "qte initial".into(),
                required: false,
                default: None,
                options: vec![],
                list: Some(FieldListMeta {
                    enabled: true,
                    sortable: true,
                }),
                filter: Some(FieldFilterMeta {
                    enabled: true,
                    operator: Some("equals".into()),
                }),
                form: Some(FieldFormMeta {
                    col_span: None,
                    placeholder: None,
                    min: None,
                    step: None,
                    read_only: None,
                    auto_generated: None,
                    storage_folder: None,
                    max_files: None,
                    accept: None,
                    ref_entity: None,
                    relation_exclusive_parent: None,
                    relation_multiple: None,
                    embed_parent: None,
                }),
                visible_when: None,
                validation: None,
            }],
            layout: ScreenLayout {
                list: crate::dda::config::ListLayout {
                    title: "Articles".into(),
                    subtitle: None,
                    actions: vec![],
                    row_click: None,
                },
                forms: None,
            },
        }
    }

    #[test]
    fn number_attr_gets_equals_operator() {
        assert_eq!(filter_operator_for_attr_type("number"), Some("equals"));
        assert_eq!(filter_operator_for_attr_type("string"), Some("contains"));
    }

    #[test]
    fn extract_filter_from_chat_message() {
        let cfg = sample_cfg();
        let (filters, _) = extract_filters_from_message("liste les article de qte initial = 50", &cfg);
        assert_eq!(filters.get("qte_initial"), Some(&"50".to_string()));
    }
}
