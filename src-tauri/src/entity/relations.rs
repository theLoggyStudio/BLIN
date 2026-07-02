use std::collections::HashSet;

use serde::Serialize;
use serde_json::{Map, Value};

use super::load_screen_config;
use super::record_signature::{self, RelationSelectOptionExt, SIGNATURE_STATUS_COLUMN, STATUS_NON_SIGNE, STATUS_REFUSE, STATUS_SIGNE};
use super::schema::table_has_column;
use super::registry::{EntityAttribute, EntityDef, EntityRegistry};
use crate::dda::config::FieldDef;
use crate::db::Database;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationPanelField {
    pub key: String,
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationPanel {
    pub entity_key: String,
    pub label: String,
    pub primary: bool,
    pub via_field: Option<String>,
    pub fields: Vec<RelationPanelField>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationDetailResponse {
    pub panels: Vec<RelationPanel>,
}

fn humanize_entity_name(nom: &str) -> String {
    let s = nom.replace('_', " ");
    let mut chars = s.chars();
    match chars.next() {
        None => nom.to_string(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn stub_entity(nom: &str) -> EntityDef {
    EntityDef {
        nom: nom.to_string(),
        label: Some(humanize_entity_name(nom)),
        description: None,
        ai_suggestions: true,
        requires_signature: false,
        signatory_role_ids: vec![],
        is_session: false,
        attributs: vec![EntityAttribute {
            nom: "nom".into(),
            attr_type: "string".into(),
            label: Some("Nom".into()),
            required: false,
            r#ref: None,
            relation_multiple: false,
            relation_exclusive_parent: true,
            default: None,
            enum_options: None,
            ..Default::default()
        }],
    }
}

/// Trigger : crée automatiquement les entités référencées par des attributs `entity`.
pub fn ensure_referenced_entities(registry: &mut EntityRegistry) -> Vec<String> {
    let mut existing: HashSet<String> = registry.entities.iter().map(|e| e.nom.clone()).collect();
    let mut pending: Vec<String> = Vec::new();

    for ent in &registry.entities {
        for attr in &ent.attributs {
            if attr.attr_type != "entity" {
                continue;
            }
            let Some(ref_key) = attr.r#ref.as_deref().map(str::trim).filter(|s| !s.is_empty())
            else {
                continue;
            };
            let key = ref_key.to_lowercase().replace(' ', "_");
            if !existing.contains(&key) && !pending.contains(&key) {
                pending.push(key);
            }
        }
    }

    let mut created = Vec::new();
    for key in pending {
        registry.entities.push(stub_entity(&key));
        existing.insert(key.clone());
        created.push(key);
    }
    created
}

pub fn field_ref_entity(field: &FieldDef) -> Option<&str> {
    field
        .form
        .as_ref()
        .and_then(|f| f.ref_entity.as_deref())
        .filter(|s| !s.is_empty())
}

fn format_relation_option_detail(
    ref_cfg: &crate::dda::config::ScreenConfigFile,
    row: &Map<String, Value>,
) -> String {
    ref_cfg
        .fields
        .iter()
        .filter(|f| {
            f.field_type != "hidden"
                && f.field_type != "detail_link"
                && f.field_type != "entity_embed"
                && f.field_type != "entity_embed_list"
                && f.field_type != "image"
                && f.field_type != "images"
                && f.form.as_ref().and_then(|m| m.embed_parent.as_ref()).is_none()
        })
        .filter_map(|f| {
            let raw = row.get(&f.key).or_else(|| row.get(&f.column))?;
            let val = display_value(raw);
            if val == "—" || val.trim().is_empty() {
                return None;
            }
            Some(format!("{} : {}", f.label, val))
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

fn relation_option_from_id(
    db: &Database,
    ref_cfg: &crate::dda::config::ScreenConfigFile,
    id: String,
    fallback_label: String,
) -> RelationSelectOptionExt {
    let label = if fallback_label.trim().is_empty() {
        id.clone()
    } else {
        fallback_label
    };
    let detail = crate::dda::crud::get_row(db, ref_cfg, &id)
        .map(|row| format_relation_option_detail(ref_cfg, &row))
        .unwrap_or_default();
    let detail = if detail.is_empty() {
        label.clone()
    } else {
        detail
    };
    RelationSelectOptionExt {
        value: id,
        label,
        detail,
    }
}

fn display_value(v: &Value) -> String {
    match v {
        Value::Null => "—".into(),
        Value::Bool(b) => if *b { "Oui" } else { "Non" }.into(),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_string()
            } else if let Some(f) = n.as_f64() {
                if (f - f.round()).abs() < f64::EPSILON {
                    format!("{}", f as i64)
                } else {
                    n.to_string()
                }
            } else {
                n.to_string()
            }
        }
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

pub fn row_to_panel_fields(
    cfg: &crate::dda::config::ScreenConfigFile,
    row: &Map<String, Value>,
) -> Vec<RelationPanelField> {
    cfg.fields
        .iter()
        .filter(|f| {
            f.field_type != "hidden"
                && f.field_type != "detail_link"
                && f.field_type != "entity_embed"
                && f.field_type != "entity_embed_list"
                && f.form.as_ref().and_then(|m| m.embed_parent.as_ref()).is_none()
        })
        .map(|f| {
            let raw = row.get(&f.key).or_else(|| row.get(&f.column));
            RelationPanelField {
                key: f.key.clone(),
                label: f.label.clone(),
                value: raw.map(display_value).unwrap_or_else(|| "—".into()),
            }
        })
        .collect()
}

fn embed_panel_fields(
    cfg: &crate::dda::config::ScreenConfigFile,
    parent: &Map<String, Value>,
    field_key: &str,
) -> Vec<RelationPanelField> {
    cfg.fields
        .iter()
        .filter(|f| {
            f.form
                .as_ref()
                .and_then(|m| m.embed_parent.as_deref())
                == Some(field_key)
        })
        .map(|f| {
            let raw = parent.get(&f.key).or_else(|| parent.get(&f.column));
            RelationPanelField {
                key: f.key.clone(),
                label: f.label.clone(),
                value: raw.map(display_value).unwrap_or_else(|| "—".into()),
            }
        })
        .collect()
}

fn parse_embed_list_objects(raw: Option<&Value>) -> Vec<Map<String, Value>> {
    let Some(v) = raw else {
        return Vec::new();
    };
    if let Value::Array(items) = v {
        return items
            .iter()
            .filter_map(|item| item.as_object().cloned())
            .collect();
    }
    if let Value::String(s) = v {
        let t = s.trim();
        if t.is_empty() {
            return Vec::new();
        }
        if let Ok(Value::Array(items)) = serde_json::from_str::<Value>(t) {
            return items
                .iter()
                .filter_map(|item| item.as_object().cloned())
                .collect();
        }
    }
    Vec::new()
}

fn embed_list_item_fields(
    ref_cfg: &crate::dda::config::ScreenConfigFile,
    obj: &Map<String, Value>,
) -> Vec<RelationPanelField> {
    let labeled: Vec<RelationPanelField> = ref_cfg
        .fields
        .iter()
        .filter(|f| {
            f.field_type != "hidden"
                && f.field_type != "detail_link"
                && f.form.as_ref().and_then(|m| m.embed_parent.as_ref()).is_none()
        })
        .filter_map(|f| {
            let raw = obj.get(&f.key).or_else(|| obj.get(&f.column));
            raw.map(|v| RelationPanelField {
                key: f.key.clone(),
                label: f.label.clone(),
                value: display_value(v),
            })
        })
        .collect();
    if !labeled.is_empty() {
        return labeled;
    }
    obj.iter()
        .map(|(k, v)| RelationPanelField {
            key: k.clone(),
            label: k.clone(),
            value: display_value(v),
        })
        .collect()
}

fn append_embed_panels_from_row(
    data_dir: &std::path::Path,
    cfg: &crate::dda::config::ScreenConfigFile,
    row: &Map<String, Value>,
    panels: &mut Vec<RelationPanel>,
    label_suffix: &str,
) -> Result<(), String> {
    for field in &cfg.fields {
        if field.field_type == "entity_embed" {
            let Some(ref_key) = field_ref_entity(field) else {
                continue;
            };
            let embed_fields = embed_panel_fields(cfg, row, &field.key);
            if embed_fields.is_empty() {
                continue;
            }
            let ref_cfg = load_screen_config(data_dir, ref_key)?;
            panels.push(RelationPanel {
                entity_key: ref_key.to_string(),
                label: format!("{}{label_suffix}", ref_cfg.screen.label),
                primary: false,
                via_field: Some(field.key.clone()),
                fields: embed_fields,
            });
            continue;
        }

        if field.field_type == "entity_embed_list" {
            let Some(ref_key) = field_ref_entity(field) else {
                continue;
            };
            let ref_cfg = load_screen_config(data_dir, ref_key)?;
            let raw = row.get(&field.key).or_else(|| row.get(&field.column));
            let items = parse_embed_list_objects(raw);
            if items.is_empty() {
                continue;
            }
            for (idx, obj) in items.into_iter().enumerate() {
                panels.push(RelationPanel {
                    entity_key: ref_key.to_string(),
                    label: format!(
                        "{} — élément {}{label_suffix}",
                        ref_cfg.screen.label,
                        idx + 1
                    ),
                    primary: false,
                    via_field: Some(field.key.clone()),
                    fields: embed_list_item_fields(&ref_cfg, &obj),
                });
            }
        }
    }
    Ok(())
}

fn append_linked_panels(
    db: &Database,
    data_dir: &std::path::Path,
    cfg: &crate::dda::config::ScreenConfigFile,
    row: &Map<String, Value>,
    panels: &mut Vec<RelationPanel>,
) -> Result<(), String> {
    for field in &cfg.fields {
        if field.field_type != "entity_ref" {
            continue;
        }
        let Some(ref_key) = field_ref_entity(field) else {
            continue;
        };
        let fk = row
            .get(&field.key)
            .or_else(|| row.get(&field.column))
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    Some(v.to_string().trim_matches('"').to_string())
                }
            })
            .filter(|s| !s.is_empty());
        let Some(fk_id) = fk else {
            continue;
        };

        let ref_cfg = load_screen_config(data_dir, ref_key)?;
        let ref_row = crate::dda::crud::get_row(db, &ref_cfg, &fk_id)?;
        panels.push(RelationPanel {
            entity_key: ref_key.to_string(),
            label: ref_cfg.screen.label.clone(),
            primary: false,
            via_field: Some(field.key.clone()),
            fields: row_to_panel_fields(&ref_cfg, &ref_row),
        });
    }

    append_embed_panels_from_row(data_dir, cfg, row, panels, "")?;
    Ok(())
}

fn append_extra_line_panels(
    data_dir: &std::path::Path,
    cfg: &crate::dda::config::ScreenConfigFile,
    row: &Map<String, Value>,
    panels: &mut Vec<RelationPanel>,
) -> Result<(), String> {
    let extras = parse_embed_list_objects(row.get(super::parent_lignes::CREATE_LINES_FORM_KEY));
    let extra_lines: Vec<Map<String, Value>> = if !extras.is_empty() {
        extras
    } else {
        super::parent_lignes::parse_lignes_items(row.get(super::parent_lignes::LIGNES_COLUMN))
            .into_iter()
            .skip(1)
            .collect()
    };
    for (idx, line) in extra_lines.into_iter().enumerate() {
        let line_num = idx + 2;
        let suffix = format!(" — ligne {line_num}");
        let line_fields = row_to_panel_fields(cfg, &line);
        if !line_fields.is_empty() {
            panels.push(RelationPanel {
                entity_key: cfg.screen.key.clone(),
                label: format!("{}{suffix}", cfg.screen.label),
                primary: false,
                via_field: Some(super::parent_lignes::LIGNES_COLUMN.into()),
                fields: line_fields,
            });
        }
        append_embed_panels_from_row(data_dir, cfg, &line, panels, &suffix)?;
    }
    Ok(())
}

/// Panneaux lecture seule : entité mère + liaisons + embed + lignes supplémentaires.
pub fn build_relation_panels(
    db: &Database,
    data_dir: &std::path::Path,
    cfg: &crate::dda::config::ScreenConfigFile,
    row: &Map<String, Value>,
) -> Result<Vec<RelationPanel>, String> {
    let mut panels = vec![RelationPanel {
        entity_key: cfg.screen.key.clone(),
        label: cfg.screen.label.clone(),
        primary: true,
        via_field: None,
        fields: row_to_panel_fields(cfg, row),
    }];
    append_linked_panels(db, data_dir, cfg, row, &mut panels)?;
    append_extra_line_panels(data_dir, cfg, row, &mut panels)?;
    Ok(panels)
}

pub fn relation_detail(
    db: &Database,
    data_dir: &std::path::Path,
    screen_key: &str,
    record_id: &str,
) -> Result<RelationDetailResponse, String> {
    let cfg = load_screen_config(data_dir, screen_key)?;
    let parent = crate::dda::crud::get_row(db, &cfg, record_id)?;
    let panels = build_relation_panels(db, data_dir, &cfg, &parent)?;
    Ok(RelationDetailResponse { panels })
}

/// Échappe les jokers LIKE (`%`, `_`, `\`) d'une recherche utilisateur.
fn escape_like_pattern(raw: &str) -> String {
    raw.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

/// Colonnes SQLite sur lesquelles la recherche de liaison porte (tous attributs persistés).
fn relation_search_columns(ref_cfg: &crate::dda::config::ScreenConfigFile) -> Vec<String> {
    let pk = ref_cfg.screen.primary_key.clone();
    let mut cols: Vec<String> = ref_cfg
        .persisted_fields()
        .iter()
        .map(|f| f.column.clone())
        .collect();
    if !cols.iter().any(|c| c == &pk) {
        cols.push(pk);
    }
    cols.sort();
    cols.dedup();
    cols
}

fn relation_search_where_clause(
    ref_cfg: &crate::dda::config::ScreenConfigFile,
    label_col: &str,
) -> String {
    let cols = relation_search_columns(ref_cfg);
    if cols.is_empty() {
        return format!(" WHERE {label_col} LIKE ?1 ESCAPE '\\'");
    }
    let parts = cols
        .iter()
        .map(|c| format!("CAST({c} AS TEXT) LIKE ?1 ESCAPE '\\'"))
        .collect::<Vec<_>>()
        .join(" OR ");
    format!(" WHERE ({parts})")
}

/// Options pour un champ entity_ref : enregistrements libres (non déjà liés sur cette entité parente).
///
/// - `search` : filtre LIKE sur tous les attributs persistés de l'entité cible.
/// - `limit` : nombre max de suggestions (0 = pas de listing général, seulement `include_ids`).
/// - `include_ids` : IDs toujours retournés (valeur courante / résolution de libellés),
///   indépendamment du filtre de recherche et de l'exclusivité parent.
pub fn relation_select_options(
    db: &Database,
    data_dir: &std::path::Path,
    screen_key: &str,
    field_key: &str,
    exclude_record_id: Option<&str>,
    search: Option<&str>,
    limit: Option<usize>,
    include_ids: Option<&[String]>,
) -> Result<Vec<RelationSelectOptionExt>, String> {
    fn parse_relation_ids(raw: &str) -> Vec<String> {
        let t = raw.trim();
        if t.is_empty() {
            return Vec::new();
        }
        if let Ok(Value::Array(items)) = serde_json::from_str::<Value>(t) {
            return items
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
                .filter(|s| !s.is_empty())
                .collect();
        }
        vec![t.to_string()]
    }

    let parent_cfg = load_screen_config(data_dir, screen_key)?;
    let field = parent_cfg
        .fields
        .iter()
        .find(|f| f.key == field_key)
        .ok_or_else(|| format!("Champ « {field_key} » introuvable."))?;
    if field.field_type != "entity_ref"
        && field.field_type != "entity_ref_list"
        && field.field_type != "entity_embed"
        && field.field_type != "entity_embed_list"
    {
        return Err(format!("Le champ « {field_key} » n'est pas une liaison entité."));
    }
    let is_embed = field.field_type == "entity_embed" || field.field_type == "entity_embed_list";
    let ref_key = field_ref_entity(field)
        .ok_or_else(|| format!("Liaison « {field_key} » sans entité cible (refEntity)."))?;
    let ref_cfg = load_screen_config(data_dir, ref_key)?;
    let registry = super::registry::load(data_dir)?;
    let ref_requires_signature = record_signature::entity_requires_signature(&registry, ref_key);
    let parent_table = &parent_cfg.screen.table;
    let fk_column = &field.column;
    let pk = &parent_cfg.screen.primary_key;

    let exclusive_parent = field
        .form
        .as_ref()
        .and_then(|f| f.relation_exclusive_parent)
        .unwrap_or(true);
    let mut used: HashSet<String> = HashSet::new();
    if exclusive_parent && !is_embed {
        let sql = format!(
            "SELECT {pk}, {fk_column} FROM {parent_table} WHERE {fk_column} IS NOT NULL AND TRIM({fk_column}) != ''"
        );
        let mut stmt = db.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?;
        for (parent_id, fk_raw) in rows.flatten() {
            if exclude_record_id.map(|x| x == parent_id).unwrap_or(false) {
                continue;
            }
            for id in parse_relation_ids(&fk_raw) {
                used.insert(id);
            }
        }
    }

    let ref_table = &ref_cfg.screen.table;
    let label_col = &ref_cfg.screen.label_field;
    let has_status_col =
        ref_requires_signature && table_has_column(db, ref_table, SIGNATURE_STATUS_COLUMN)?;

    let current_fks: HashSet<String> = if is_embed {
        HashSet::new()
    } else {
        exclude_record_id
            .and_then(|parent_id| {
                let sql = format!(
                    "SELECT {fk_column} FROM {parent_table} WHERE {pk} = ?1",
                    fk_column = fk_column,
                    parent_table = parent_table,
                    pk = pk,
                );
                db.conn
                    .query_row(&sql, rusqlite::params![parent_id], |row| {
                        row.get::<_, String>(0)
                    })
                    .ok()
            })
            .map(|raw| parse_relation_ids(&raw).into_iter().collect())
            .unwrap_or_default()
    };

    let mut options = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    let listing_limit = match limit {
        Some(0) => 0,
        Some(n) => n,
        None => 50,
    };
    if listing_limit > 0 {
        options.push(RelationSelectOptionExt {
            value: "".into(),
            label: "— Aucun —".into(),
            detail: String::new(),
        });

        let search_term = search.map(str::trim).filter(|s| !s.is_empty());
        let status_select = if has_status_col {
            format!(", {SIGNATURE_STATUS_COLUMN}")
        } else {
            String::new()
        };
        let where_clause = if search_term.is_some() {
            relation_search_where_clause(&ref_cfg, label_col)
        } else {
            String::new()
        };
        let list_sql = format!(
            "SELECT {pk}, {label_col}{status_select} FROM {ref_table}{where_clause} ORDER BY {label_col}"
        );
        let mut stmt = db.conn.prepare(&list_sql).map_err(|e| e.to_string())?;
        let mut sql_rows = if let Some(q) = search_term {
            stmt.query(rusqlite::params![format!("%{}%", escape_like_pattern(q))])
        } else {
            stmt.query([])
        }
        .map_err(|e| e.to_string())?;

        // Lecture en flux : on s'arrête dès que la limite est atteinte (pas de chargement total).
        let mut count = 0usize;
        while let Some(row) = sql_rows.next().map_err(|e| e.to_string())? {
            if count >= listing_limit {
                break;
            }
            let id: String = row.get(0).map_err(|e| e.to_string())?;
            let label: String = row.get(1).map_err(|e| e.to_string())?;
            let statut: Option<String> = if has_status_col {
                row.get(2).map_err(|e| e.to_string())?
            } else {
                None
            };
            let non_signe = !is_embed && statut.as_deref() == Some(STATUS_NON_SIGNE);
            let refuse = !is_embed && statut.as_deref() == Some(STATUS_REFUSE);
            if non_signe || refuse {
                continue;
            }
            let is_current = current_fks.contains(id.as_str());
            if used.contains(&id) && !is_current {
                continue;
            }
            let display = if label.trim().is_empty() {
                id.clone()
            } else {
                label
            };
            seen.insert(id.clone());
            options.push(relation_option_from_id(db, &ref_cfg, id, display));
            count += 1;
        }
    }

    // IDs explicitement demandés (valeur courante / résolution de libellés) : toujours retournés.
    if let Some(ids) = include_ids {
        let wanted: Vec<String> = ids
            .iter()
            .map(|id| id.trim().to_string())
            .filter(|id| !id.is_empty() && !seen.contains(id.as_str()))
            .collect();
        if !wanted.is_empty() {
            let placeholders = wanted
                .iter()
                .enumerate()
                .map(|(i, _)| format!("?{}", i + 1))
                .collect::<Vec<_>>()
                .join(", ");
            let sql = format!(
                "SELECT {pk}, {label_col} FROM {ref_table} WHERE {pk} IN ({placeholders})"
            );
            let mut stmt = db.conn.prepare(&sql).map_err(|e| e.to_string())?;
            let params = rusqlite::params_from_iter(wanted.iter());
            let rows = stmt
                .query_map(params, |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|e| e.to_string())?;
            for (id, label) in rows.flatten() {
                if seen.contains(id.as_str()) {
                    continue;
                }
                let display = if label.trim().is_empty() {
                    id.clone()
                } else {
                    label
                };
                seen.insert(id.clone());
                options.push(relation_option_from_id(db, &ref_cfg, id, display));
            }
        }
    }

    Ok(options)
}

/// Copie les valeurs d'un enregistrement fille vers les clés embarquées du parent (1-1).
pub fn embed_values_from_record(
    db: &Database,
    data_dir: &std::path::Path,
    screen_key: &str,
    field_key: &str,
    record_id: &str,
) -> Result<Map<String, Value>, String> {
    let registry = super::registry::load(data_dir)?;
    let parent_ent = registry
        .find(screen_key)
        .ok_or_else(|| format!("Entité « {screen_key} » introuvable."))?;
    let parent_attr = parent_ent
        .attributs
        .iter()
        .find(|a| a.nom == field_key)
        .ok_or_else(|| format!("Attribut « {field_key} » introuvable."))?;
    if parent_attr.attr_type != "entity" {
        return Err(format!("« {field_key} » n'est pas une liaison entité."));
    }
    let child = super::embed::resolve_child(&registry, parent_attr)
        .ok_or_else(|| format!("Entité fille introuvable pour « {field_key} »."))?;
    let ref_cfg = load_screen_config(data_dir, &child.nom)?;
    let row = crate::dda::crud::get_row(db, &ref_cfg, record_id)?;
    Ok(super::embed::values_from_child_row(parent_attr, child, &row))
}

/// Objet fille (clés sans préfixe) pour liste embarquée.
/// Entité d'origine du circuit d'impact stock (première mère, ex. demande d'achat).
pub const IMPACT_ORIGIN_ENTITY_KEY: &str = "demande_d'achat";

/// Métadonnées d'impact stock pour une liaison multiple (ligne embarquée) :
/// champ quantité côté ligne + sens (incrément / décrément) pour afficher l'input.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbedImpactMeta {
    /// Nom du champ (côté fille) qui porte la quantité à appliquer au stock.
    pub qty_field: String,
    /// "increment" | "decrement".
    pub action: String,
    /// Libellé du champ stock de la fille (pour l'input).
    pub label: String,
    /// Entité où la quantité est saisie une seule fois (demande d'achat).
    pub origin_entity_key: String,
    /// Quantité non modifiable (aval du circuit ou DA déjà signée).
    pub qty_read_only: bool,
    /// Entité fille référencée (ex. article).
    pub child_entity_key: String,
    /// Champ stock sur la fille servant de plafond (ex. qte_initial), si décrément.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stock_cap_field: Option<String>,
}

/// Renvoie l'impact stock configuré sur la liaison multiple, le cas échéant.
pub fn embed_impact_meta(
    db: &Database,
    registry: &EntityRegistry,
    screen_key: &str,
    field_key: &str,
    record_id: Option<&str>,
) -> Option<EmbedImpactMeta> {
    let parent = registry.find(screen_key)?;
    let attr = parent.attributs.iter().find(|a| a.nom == field_key)?;
    if attr.attr_type != "entity" || !attr.relation_multiple {
        return None;
    }
    let source = attr
        .relation_impact_source
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())?;
    let action = match attr
        .relation_impact_action
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "increment" | "incrementer" | "incrémenter" => "increment",
        "decrement" | "decrementer" | "décrémenter" => "decrement",
        _ => return None,
    };
    let child = super::embed::resolve_child(registry, attr)?;
    let leaf = source.rsplit('.').next().unwrap_or(source);
    let child_attr = child.attributs.iter().find(|a| a.nom == leaf)?;
    if !super::relation_impact::is_numeric_impactable(&child_attr.attr_type) {
        return None;
    }
    let label = child_attr
        .label
        .clone()
        .unwrap_or_else(|| child_attr.nom.clone());
    let qty_field = if source.contains('.') {
        leaf.to_string()
    } else {
        source.to_string()
    };
    let origin_entity_key = if attr.relation_impact_defer {
        IMPACT_ORIGIN_ENTITY_KEY.to_string()
    } else {
        screen_key.to_string()
    };
    let on_origin = screen_key == origin_entity_key;
    let qty_read_only = if !on_origin {
        true
    } else if let Some(rid) = record_id.filter(|s| !s.trim().is_empty()) {
        super::record_signature::is_record_signed(db, screen_key, rid, registry)
            .unwrap_or(false)
    } else {
        false
    };
    Some(EmbedImpactMeta {
        qty_field,
        action: action.to_string(),
        label: if source.contains('.') {
            format!("{source} ({label})")
        } else {
            label
        },
        origin_entity_key,
        qty_read_only,
        child_entity_key: child.nom.clone(),
        stock_cap_field: if action == "decrement" {
            Some(
                attr.relation_impact_target
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .unwrap_or(leaf)
                    .to_string(),
            )
        } else {
            None
        },
    })
}

pub fn embed_child_object_from_record(
    db: &Database,
    data_dir: &std::path::Path,
    screen_key: &str,
    field_key: &str,
    record_id: &str,
) -> Result<Map<String, Value>, String> {
    let registry = super::registry::load(data_dir)?;
    let parent_ent = registry
        .find(screen_key)
        .ok_or_else(|| format!("Entité « {screen_key} » introuvable."))?;
    let parent_attr = parent_ent
        .attributs
        .iter()
        .find(|a| a.nom == field_key)
        .ok_or_else(|| format!("Attribut « {field_key} » introuvable."))?;
    let child = super::embed::resolve_child(&registry, parent_attr)
        .ok_or_else(|| format!("Entité fille introuvable pour « {field_key} »."))?;
    let ref_cfg = load_screen_config(data_dir, &child.nom)?;
    let row = crate::dda::crud::get_row(db, &ref_cfg, record_id)?;
    let mut out = super::embed::child_object_from_row_for_embed_list(&child, &row);
    if let Some(cap_field) = parent_attr
        .relation_impact_target
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if let Some(n) = row.get(cap_field).and_then(super::relation_impact::json_to_f64) {
            if let Some(num) = serde_json::Number::from_f64(n) {
                out.insert(super::embed::EMBED_STOCK_CAP.to_string(), Value::Number(num));
            }
        }
    }
    for (list_attr, list_child) in super::child_table::embed_list_attrs(&child, &registry) {
        let items =
            super::child_table::load_embed_list(db, &child.nom, record_id, list_child)?;
        out.insert(
            list_attr.nom.clone(),
            Value::Array(items.into_iter().map(Value::Object).collect()),
        );
    }
    Ok(out)
}

/// Lit un champ numérique sur une entité fille (par id ou référence).
pub fn read_child_numeric_field(
    db: &Database,
    data_dir: &std::path::Path,
    entity_key: &str,
    field_key: &str,
    record_id: Option<&str>,
    reference: Option<&str>,
) -> Result<Option<f64>, String> {
    use super::schema::table_name;
    let registry = super::registry::load(data_dir)?;
    let child = registry
        .find(entity_key)
        .ok_or_else(|| format!("Entité « {entity_key} » introuvable."))?;
    let table = table_name(entity_key);
    let id = if let Some(rid) = record_id.map(str::trim).filter(|s| !s.is_empty()) {
        Some(rid.to_string())
    } else if let Some(r) = reference.map(str::trim).filter(|s| !s.is_empty()) {
        super::relation_impact::lookup_child_id_by_reference(db, &table, r)?
    } else {
        None
    };
    let id = id.ok_or_else(|| "Enregistrement fille introuvable.".to_string())?;
    let cfg = load_screen_config(data_dir, &child.nom)?;
    let row = crate::dda::crud::get_row(db, &cfg, &id)?;
    Ok(row.get(field_key).and_then(super::relation_impact::json_to_f64))
}

pub fn registry_entity_ref_targets(registry: &EntityRegistry) -> Vec<(String, String)> {
    registry
        .entities
        .iter()
        .map(|e| {
            let label = e.label.clone().unwrap_or_else(|| e.nom.clone());
            (e.nom.clone(), label)
        })
        .collect()
}
