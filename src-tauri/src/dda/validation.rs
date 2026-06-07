use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::config::{is_persisted_field, FieldDef, FieldValidation, ScreenConfigFile, VisibleWhen};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationIssue {
    pub field: String,
    pub label: String,
    pub level: String,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationReport {
    pub valid: bool,
    pub errors: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
}

impl ValidationReport {
    pub fn ok() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn merge(&mut self, other: ValidationReport) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
        self.valid = self.errors.is_empty();
    }
}

pub fn validate_screen_data(
    cfg: &ScreenConfigFile,
    data: &Map<String, Value>,
    for_filter: bool,
) -> ValidationReport {
    let mut report = ValidationReport::ok();
    for field in &cfg.fields {
        if !is_persisted_field(field) {
            continue;
        }
        if for_filter && field.filter.as_ref().is_none_or(|f| !f.enabled) {
            continue;
        }
        if !for_filter && !is_field_visible(field, data) {
            continue;
        }
        let value = data.get(&field.key).or_else(|| data.get(&field.column));
        let field_report = validate_field(field, value, data);
        report.merge(field_report);
    }
    if !for_filter && cfg.screen.key == "tache" {
        report.merge(validate_tache_link_fields(data));
        report.merge(validate_tache_visibility_fields(data));
    }
    if !for_filter {
        report.merge(validate_embed_fields(cfg, data));
    }
    report
}

fn validate_embed_fields(cfg: &ScreenConfigFile, data: &Map<String, Value>) -> ValidationReport {
    let mut report = ValidationReport::ok();
    for field in &cfg.fields {
        if field.field_type == "entity_embed" {
            let mut any = false;
            for child in &cfg.fields {
                if child.form.as_ref().and_then(|m| m.embed_parent.as_deref())
                    != Some(field.key.as_str())
                {
                    continue;
                }
                if !is_field_visible(child, data) {
                    continue;
                }
                let v = data.get(&child.key).or_else(|| data.get(&child.column));
                if !is_empty_value(v) {
                    any = true;
                }
                if child.required && is_empty_value(v) {
                    report.errors.push(ValidationIssue {
                        field: child.key.clone(),
                        label: child.label.clone(),
                        level: "error".into(),
                        code: "required".into(),
                        message: format!("« {} » est obligatoire.", child.label),
                        fix_hint: None,
                    });
                }
            }
            if field.required && !any {
                report.errors.push(ValidationIssue {
                    field: field.key.clone(),
                    label: field.label.clone(),
                    level: "error".into(),
                    code: "required".into(),
                    message: format!(
                        "« {} » : choisissez ou renseignez au moins un champ.",
                        field.label
                    ),
                    fix_hint: None,
                });
            }
        }
        if field.field_type == "entity_embed_list" && field.required {
            let v = data.get(&field.key).or_else(|| data.get(&field.column));
            let empty_list = match v {
                None | Some(Value::Null) => true,
                Some(Value::String(s)) => s.trim().is_empty() || s.trim() == "[]",
                Some(Value::Array(a)) => a.is_empty(),
                _ => false,
            };
            if empty_list {
                report.errors.push(ValidationIssue {
                    field: field.key.clone(),
                    label: field.label.clone(),
                    level: "error".into(),
                    code: "required".into(),
                    message: format!("« {} » : ajoutez au moins un élément.", field.label),
                    fix_hint: None,
                });
            }
        }
    }
    report.valid = report.errors.is_empty();
    report
}

fn validate_tache_visibility_fields(data: &Map<String, Value>) -> ValidationReport {
    use crate::entity::tache_visibility::{parse_roles_csv, VIS_PERSONNALISEE};
    let vis = data
        .get(crate::entity::tache_visibility::COL_VISIBILITE)
        .and_then(|v| v.as_str())
        .unwrap_or(crate::entity::tache_visibility::VIS_PUBLIQUE);
    if vis != VIS_PERSONNALISEE {
        return ValidationReport::ok();
    }
    let roles = parse_roles_csv(
        data.get(crate::entity::tache_visibility::COL_ROLES_VISIBLES)
            .and_then(|v| v.as_str()),
    );
    if !roles.is_empty() {
        return ValidationReport::ok();
    }
    let mut report = ValidationReport::ok();
    report.errors.push(ValidationIssue {
        field: crate::entity::tache_visibility::COL_ROLES_VISIBLES.into(),
        label: "Rôles autorisés".into(),
        level: "error".into(),
        code: "required".into(),
        message: "Sélectionnez au moins un rôle pour une visibilité personnalisée.".into(),
        fix_hint: None,
    });
    report.valid = false;
    report
}

/// Liens signature / déstockage : champs obligatoires selon `type_tache`.
fn validate_tache_link_fields(data: &Map<String, Value>) -> ValidationReport {
    let type_tache = data
        .get("type_tache")
        .and_then(|v| v.as_str())
        .unwrap_or("generale");
    if type_tache != "signature" && type_tache != "destockage" {
        return ValidationReport::ok();
    }
    let mut report = ValidationReport::ok();
    let entity_key = data
        .get("entite_a_signer")
        .or_else(|| data.get("entite_a_valider"));
    let record_id = data.get("enregistrement_id");
    if is_empty_value(entity_key) {
        report.errors.push(ValidationIssue {
            field: "entite_a_signer".into(),
            label: "Entité à signer".into(),
            level: "error".into(),
            code: "required".into(),
            message: format!(
                "« Entité à signer » est obligatoire pour une tâche de type « {type_tache} »."
            ),
            fix_hint: None,
        });
    }
    if is_empty_value(record_id) {
        report.errors.push(ValidationIssue {
            field: "enregistrement_id".into(),
            label: "ID enregistrement".into(),
            level: "error".into(),
            code: "required".into(),
            message: format!(
                "« ID enregistrement » est obligatoire pour une tâche de type « {type_tache} »."
            ),
            fix_hint: None,
        });
    }
    if type_tache == "signature" {
        let value = data
            .get("role_signataire")
            .or_else(|| data.get("role_validateur"));
        if is_empty_value(value) {
            report.errors.push(ValidationIssue {
                field: "role_signataire".into(),
                label: "Rôle signataire".into(),
                level: "error".into(),
                code: "required".into(),
                message: "« Rôle signataire » est obligatoire pour une tâche de signature.".into(),
                fix_hint: None,
            });
        }
    }
    report.valid = report.errors.is_empty();
    report
}

fn is_field_visible(field: &FieldDef, data: &Map<String, Value>) -> bool {
    let Some(w) = &field.visible_when else {
        return true;
    };
    let current = data.get(&w.field).or_else(|| data.get(&w.field));
    match current {
        Some(Value::String(s)) => Value::String(s.clone()) == w.equals,
        Some(Value::Number(n)) => w.equals.as_f64().is_some_and(|e| n.as_f64() == Some(e)),
        Some(Value::Bool(b)) => w.equals.as_bool() == Some(*b),
        None => w.equals == Value::Null,
        _ => false,
    }
}

pub fn validate_field(
    field: &FieldDef,
    value: Option<&Value>,
    all: &Map<String, Value>,
) -> ValidationReport {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let rules = field.validation.as_ref();
    let empty = is_empty_value(value);

    let required = field.required || rules.is_some_and(|r| r.required);
    if required && empty {
        let msg = rules
            .and_then(|r| r.required_message.clone())
            .unwrap_or_else(|| format!("« {} » est obligatoire.", field.label));
        let fix = rules
            .and_then(|r| r.fix_hint.clone())
            .or_else(|| field.form.as_ref().and_then(|f| f.placeholder.clone()));
        errors.push(issue(
            field,
            "error",
            "required",
            msg,
            fix,
        ));
    }

    if field.field_type == "images" {
        if let Some(max) = field.form.as_ref().and_then(|f| f.max_files) {
            let count = image_paths_count(value);
            if count > max as usize {
                errors.push(issue(
                    field,
                    "error",
                    "max_files",
                    format!(
                        "Maximum {} photo(s) dans « {} ».",
                        max, field.label
                    ),
                    field.form.as_ref().and_then(|f| f.placeholder.clone()),
                ));
            }
        }
    }

    if !empty && field.field_type != "image" && field.field_type != "images" {
        if let Some(rules) = rules {
            validate_rules(field, value.unwrap(), rules, &mut errors, &mut warnings);
            for warn_rule in &rules.warnings {
                if condition_matches(&warn_rule.when, all) {
                    warnings.push(issue(
                        field,
                        "warning",
                        "conditional",
                        warn_rule.message.clone(),
                        warn_rule.fix_hint.clone(),
                    ));
                }
            }
        } else {
            validate_inferred(field, value.unwrap(), &mut errors);
        }
    }

    ValidationReport {
        valid: errors.is_empty(),
        errors,
        warnings,
    }
}

fn validate_rules(
    field: &FieldDef,
    value: &Value,
    rules: &FieldValidation,
    errors: &mut Vec<ValidationIssue>,
    warnings: &mut Vec<ValidationIssue>,
) {
    let text = value_as_string(value);

    if let Some(min_len) = rules.min_length {
        if (text.chars().count() as u32) < min_len {
            errors.push(issue(
                field,
                "error",
                "min_length",
                rules
                    .min_length_message
                    .clone()
                    .unwrap_or_else(|| {
                        format!(
                            "« {} » : minimum {} caractères (actuellement {}).",
                            field.label,
                            min_len,
                            text.chars().count()
                        )
                    }),
                rules.fix_hint.clone(),
            ));
        }
    }

    if let Some(max_len) = rules.max_length {
        if (text.chars().count() as u32) > max_len {
            errors.push(issue(
                field,
                "error",
                "max_length",
                rules
                    .max_length_message
                    .clone()
                    .unwrap_or_else(|| {
                        format!(
                            "« {} » : maximum {} caractères.",
                            field.label, max_len
                        )
                    }),
                rules.fix_hint.clone(),
            ));
        }
    }

    if field.field_type == "number" || field.field_type == "integer" {
        if let Some(n) = value.as_f64().or_else(|| text.parse::<f64>().ok()) {
            let min = rules.min.or(field.form.as_ref().and_then(|f| f.min));
            let max = rules.max;
            if let Some(m) = min {
                if n < m {
                    errors.push(issue(
                        field,
                        "error",
                        "min",
                        rules.min_message.clone().unwrap_or_else(|| {
                            format!("« {} » doit être ≥ {}.", field.label, m)
                        }),
                        rules.fix_hint.clone(),
                    ));
                }
            }
            if let Some(m) = max {
                if n > m {
                    errors.push(issue(
                        field,
                        "error",
                        "max",
                        rules.max_message.clone().unwrap_or_else(|| {
                            format!("« {} » doit être ≤ {}.", field.label, m)
                        }),
                        rules.fix_hint.clone(),
                    ));
                }
            }
        } else if !text.is_empty() {
            errors.push(issue(
                field,
                "error",
                "not_a_number",
                format!("« {} » doit être un nombre valide.", field.label),
                rules.fix_hint.clone(),
            ));
        }
    }

    if let Some(pattern) = &rules.pattern {
        if let Ok(re) = Regex::new(pattern) {
            if !re.is_match(&text) {
                errors.push(issue(
                    field,
                    "error",
                    "pattern",
                    rules.pattern_message.clone().unwrap_or_else(|| {
                        format!("« {} » : format invalide.", field.label)
                    }),
                    rules
                        .fix_hint
                        .clone()
                        .or_else(|| Some(format!("Format attendu : {pattern}"))),
                ));
            }
        }
    }

    if let Some(allowed) = &rules.one_of {
        if !allowed.is_empty() && !allowed.iter().any(|v| v == &text) {
            errors.push(issue(
                field,
                "error",
                "one_of",
                rules.one_of_message.clone().unwrap_or_else(|| {
                    format!(
                        "« {} » : valeur non autorisée. Valeurs acceptées : {}.",
                        field.label,
                        allowed.join(", ")
                    )
                }),
                rules.fix_hint.clone(),
            ));
        }
    }

    let _ = warnings;
}

fn validate_inferred(field: &FieldDef, value: &Value, errors: &mut Vec<ValidationIssue>) {
    if field.field_type == "select" && !field.options.is_empty() {
        let text = value_as_string(value);
        if !text.is_empty() && !field.options.iter().any(|o| o.value == text) {
            errors.push(issue(
                field,
                "error",
                "invalid_option",
                format!("« {} » : choisissez une valeur dans la liste.", field.label),
                Some(
                    field
                        .options
                        .iter()
                        .map(|o| o.value.clone())
                        .collect::<Vec<_>>()
                        .join(", "),
                ),
            ));
        }
    }
}

fn condition_matches(when: &VisibleWhen, data: &Map<String, Value>) -> bool {
    let current = data.get(&when.field);
    match current {
        Some(Value::String(s)) => Value::String(s.clone()) == when.equals,
        Some(Value::Number(n)) => when.equals.as_f64().is_some_and(|e| n.as_f64() == Some(e)),
        Some(Value::Bool(b)) => when.equals.as_bool() == Some(*b),
        None => when.equals == Value::Null,
        _ => false,
    }
}

fn is_empty_value(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => true,
        Some(Value::String(s)) => {
            let t = s.trim();
            t.is_empty() || t == "[]"
        }
        Some(Value::Array(a)) => a.is_empty(),
        _ => false,
    }
}

fn image_paths_count(value: Option<&Value>) -> usize {
    match value {
        Some(Value::Array(items)) => items.len(),
        Some(Value::String(s)) => {
            if s.trim().is_empty() || s.trim() == "[]" {
                0
            } else if let Ok(Value::Array(items)) = serde_json::from_str(s) {
                items.len()
            } else {
                1
            }
        }
        _ => 0,
    }
}

fn value_as_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.trim().to_string(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => value.to_string(),
    }
}

fn issue(
    field: &FieldDef,
    level: &str,
    code: &str,
    message: String,
    fix_hint: Option<String>,
) -> ValidationIssue {
    ValidationIssue {
        field: field.key.clone(),
        label: field.label.clone(),
        level: level.to_string(),
        code: code.to_string(),
        message,
        fix_hint,
    }
}

/// Rapport JSON compact pour le trigger knowledge / fichier validations.
pub fn build_validation_catalog(cfg: &ScreenConfigFile) -> Value {
    let mut fields = Map::new();
    for f in &cfg.fields {
        if f.field_type == "hidden" {
            continue;
        }
        let mut entry = Map::new();
        entry.insert("label".into(), Value::String(f.label.clone()));
        entry.insert("required".into(), Value::Bool(f.required));
        if let Some(v) = &f.validation {
            if let Ok(j) = serde_json::to_value(v) {
                entry.insert("rules".into(), j);
            }
        }
        fields.insert(f.key.clone(), Value::Object(entry));
    }
    let mut root = Map::new();
    root.insert("screenKey".into(), Value::String(cfg.screen.key.clone()));
    root.insert("fields".into(), Value::Object(fields));
    Value::Object(root)
}

pub fn format_validation_knowledge(cfg: &ScreenConfigFile) -> String {
    let mut out = format!(
        "=== VALIDATIONS {} (auto DDA) ===\n",
        cfg.screen.key.to_uppercase()
    );
    for f in &cfg.fields {
        if f.field_type == "hidden" {
            continue;
        }
        out.push_str(&format!("\n• {} ({}) — colonne {}\n", f.label, f.key, f.column));
        if f.required {
            let msg = f
                .validation
                .as_ref()
                .and_then(|v| v.required_message.clone())
                .unwrap_or_else(|| format!("Champ obligatoire."));
            out.push_str(&format!("  ERREUR si vide : {msg}\n"));
        }
        if let Some(v) = &f.validation {
            if let Some(h) = &v.fix_hint {
                out.push_str(&format!("  Conseil : {h}\n"));
            }
            if let Some(p) = &v.pattern {
                out.push_str(&format!("  Format (regex) : {p}\n"));
            }
            for w in &v.warnings {
                out.push_str(&format!(
                    "  AVERTISSEMENT si {} = {:?} : {}\n",
                    w.when.field, w.when.equals, w.message
                ));
            }
        }
    }
    out.push_str("\nToute création/modification doit respecter ces règles.\n");
    out
}

pub fn validation_error_json(report: &ValidationReport) -> String {
    serde_json::to_string(report).unwrap_or_else(|_| report.errors[0].message.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dda::config::{
        FieldDef, FieldFormMeta, FieldListMeta, ListLayout, ScreenConfigFile, ScreenLayout,
        ScreenMeta,
    };
    use serde_json::json;

    fn embed_list_field(required: bool) -> FieldDef {
        FieldDef {
            key: "article".into(),
            column: "article".into(),
            field_type: "entity_embed_list".into(),
            label: "Articles".into(),
            required,
            default: None,
            options: vec![],
            list: Some(FieldListMeta {
                enabled: false,
                sortable: false,
            }),
            filter: None,
            form: Some(FieldFormMeta {
                col_span: Some(2),
                placeholder: None,
                min: None,
                step: None,
                read_only: None,
                auto_generated: None,
                storage_folder: None,
                max_files: None,
                accept: None,
                ref_entity: Some("articles".into()),
                relation_exclusive_parent: Some(true),
                relation_multiple: Some(true),
                embed_parent: None,
            }),
            visible_when: None,
            validation: None,
        }
    }

    fn minimal_cfg(field: FieldDef) -> ScreenConfigFile {
        ScreenConfigFile {
            screen: ScreenMeta {
                key: "demande_dachat".into(),
                label: "DA".into(),
                label_plural: None,
                icon: None,
                route: "/entite/demande_dachat".into(),
                system: false,
                ai_editable: false,
                table: "ent_demande_dachat".into(),
                primary_key: "id".into(),
                label_field: "id".into(),
                default_order_by: None,
                privileges: crate::dda::config::ScreenPrivileges {
                    view: "demande_dachat:voir".into(),
                    create: "demande_dachat:creer".into(),
                    update: "demande_dachat:modifier".into(),
                    delete: "demande_dachat:supprimer".into(),
                    import: None,
                    export: None,
                },
                print: None,
                storage: None,
            },
            layout: ScreenLayout {
                list: ListLayout {
                    title: "DA".into(),
                    subtitle: None,
                    actions: vec![],
                    row_click: None,
                },
                forms: None,
            },
            fields: vec![field],
        }
    }

    #[test]
    fn required_embed_list_rejects_empty() {
        let cfg = minimal_cfg(embed_list_field(true));
        let mut data = Map::new();
        data.insert("article".into(), json!("[]"));
        let report = validate_screen_data(&cfg, &data, false);
        assert!(!report.valid);
        assert!(report.errors.iter().any(|e| e.code == "required"));
    }
}
