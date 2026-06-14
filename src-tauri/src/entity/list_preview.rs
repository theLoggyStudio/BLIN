//! Affichage de listes entités dans le chat (tableau / liste) sans ouvrir le formulaire.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::Path;

use crate::ai::agent::{ChatColsRequest, ChatDisplayBlock, ChatDisplayColumn};
use crate::ai::intent_filters::{normalize_message, wants_list_intent};
use crate::dda::config::FieldDef;
use crate::dda::crud::{list_rows_with_options, ListRowsOptions};
use crate::dda::filters::{
    apply_exact_filters, extract_filters_from_message, filters_summary,
};
use crate::db::Database;
use crate::entity::intent;
use crate::entity::load_screen_config;
use crate::entity::registry::EntityRegistry;
use crate::privileges::has_privilege;
use crate::session::SessionUser;

const DISPLAY_MARKER_START: &str = "__BLIN_DISPLAY__\n";
const DISPLAY_MARKER_END: &str = "\n__END_BLIN_DISPLAY__";
const COLS_MARKER_START: &str = "__BLIN_ASK_COLS__\n";
const COLS_MARKER_END: &str = "\n__END_BLIN_ASK_COLS__";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredDisplayPayload {
    blocks: Vec<ChatDisplayBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredColsRequest {
    entity_key: String,
    available: Vec<ColMeta>,
    #[serde(default)]
    filters: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ColMeta {
    key: String,
    label: String,
}

#[derive(Debug, Clone)]
struct JoinHint {
    via_field: String,
    via_label: String,
    ref_entity: String,
    ref_label_field: String,
}

pub fn embed_display(message: &str, blocks: &[ChatDisplayBlock]) -> String {
    if blocks.is_empty() {
        return message.to_string();
    }
    let payload = StoredDisplayPayload {
        blocks: blocks.to_vec(),
    };
    let json = serde_json::to_string(&payload).unwrap_or_default();
    format!("{message}{DISPLAY_MARKER_START}{json}{DISPLAY_MARKER_END}")
}

pub fn parse_display_from_message(content: &str) -> (String, Vec<ChatDisplayBlock>) {
    let (visible, blocks) = parse_display_blocks(content);
    (strip_cols_marker(&visible), blocks)
}

/// Texte visible pour l'utilisateur : retire les marqueurs internes (JSON machine).
pub fn visible_chat_message(content: &str) -> (String, Vec<ChatDisplayBlock>) {
    let (text, blocks) = parse_display_from_message(content);
    let clean = crate::ai::format_display::sanitize_assistant_message(&text);
    (clean, blocks)
}

fn parse_display_blocks(content: &str) -> (String, Vec<ChatDisplayBlock>) {
    if let Some(start) = content.find(DISPLAY_MARKER_START) {
        let end = content.find(DISPLAY_MARKER_END).unwrap_or(content.len());
        let json = content[start + DISPLAY_MARKER_START.len()..end].trim();
        let visible = content[..start].trim_end().to_string();
        if let Ok(payload) = serde_json::from_str::<StoredDisplayPayload>(json) {
            return (visible, payload.blocks);
        }
        return (visible, vec![]);
    }
    (content.to_string(), vec![])
}

fn strip_cols_marker(content: &str) -> String {
    if let Some(start) = content.find(COLS_MARKER_START) {
        return content[..start].trim_end().to_string();
    }
    content.trim().to_string()
}

fn registry_entity_label(registry: &EntityRegistry, entity_key: &str) -> String {
    registry
        .find(entity_key)
        .and_then(|e| e.label.as_deref())
        .filter(|s| !s.is_empty())
        .unwrap_or(entity_key)
        .to_string()
}

fn make_cols_request(
    entity_key: &str,
    entity_label: &str,
    available: &[ColMeta],
    filters: &HashMap<String, String>,
) -> ChatColsRequest {
    ChatColsRequest {
        entity_key: entity_key.to_string(),
        entity_label: entity_label.to_string(),
        available: available
            .iter()
            .map(|c| ChatDisplayColumn {
                key: c.key.clone(),
                label: c.label.clone(),
            })
            .collect(),
        filters: filters.clone(),
    }
}

/// Résultat intent « liste les … » pour le chat dashboard.
pub struct ListPreviewResult {
    pub raw_message: String,
    pub display_blocks: Vec<ChatDisplayBlock>,
    pub cols_request: Option<ChatColsRequest>,
}

/// Reconstruit la demande de colonnes depuis un message stocké (historique).
pub fn cols_request_from_message(content: &str, registry: &EntityRegistry) -> Option<ChatColsRequest> {
    let stored = parse_cols_request(content)?;
    let entity_label = registry_entity_label(registry, &stored.entity_key);
    Some(make_cols_request(
        &stored.entity_key,
        &entity_label,
        &stored.available,
        &stored.filters,
    ))
}

fn embed_cols_request(
    message: &str,
    entity_key: &str,
    available: &[ColMeta],
    filters: &HashMap<String, String>,
) -> String {
    let payload = StoredColsRequest {
        entity_key: entity_key.to_string(),
        available: available.to_vec(),
        filters: filters.clone(),
    };
    let json = serde_json::to_string(&payload).unwrap_or_default();
    format!("{message}{COLS_MARKER_START}{json}{COLS_MARKER_END}")
}

fn parse_cols_request(content: &str) -> Option<StoredColsRequest> {
    let start = content.find(COLS_MARKER_START)?;
    let end = content.find(COLS_MARKER_END)?;
    let json = content[start + COLS_MARKER_START.len()..end].trim();
    serde_json::from_str(json).ok()
}

fn last_assistant_message(db: &Database, conv_id: &str) -> Option<String> {
    db.ai_list_messages(conv_id, 20)
        .ok()?
        .into_iter()
        .rev()
        .find(|m| m.role == "assistant")
        .map(|m| m.content)
}

fn match_list_entity(message: &str, registry: &EntityRegistry) -> Option<String> {
    let msg = normalize_message(message);
    if !wants_list_intent(&msg) {
        return None;
    }
    let mut best: Option<(String, i32)> = None;
    for ent in &registry.entities {
        let score = intent::score_match(&msg, ent);
        if score < 30 {
            continue;
        }
        if best.as_ref().map(|(_, s)| score > *s).unwrap_or(true) {
            best = Some((ent.nom.clone(), score));
        }
    }
    best.map(|(k, _)| k)
}

fn list_column_metas(cfg: &crate::dda::config::ScreenConfigFile) -> Vec<ColMeta> {
    let cols = cfg.list_columns();
    let source: Vec<&FieldDef> = if cols.is_empty() {
        cfg.persisted_fields()
            .into_iter()
            .filter(|f| f.key != "created_at")
            .take(8)
            .collect()
    } else {
        cols
    };
    source
        .into_iter()
        .map(|f| ColMeta {
            key: f.key.clone(),
            label: f.label.clone(),
        })
        .collect()
}

fn parse_requested_columns(text: &str, available: &[ColMeta]) -> Vec<String> {
    let norm = normalize_message(text);
    if norm.contains("tout") || norm.contains("toutes") || norm.contains("all") {
        return available.iter().map(|c| c.key.clone()).collect();
    }
    let mut picked = Vec::new();
    for col in available {
        let key_norm = normalize_message(&col.key);
        let label_norm = normalize_message(&col.label);
        if norm.contains(&key_norm) || norm.contains(&label_norm) {
            picked.push(col.key.clone());
        }
    }
    if picked.is_empty() {
        for part in text.split([',', ';', '|']) {
            let p = normalize_message(part);
            if p.is_empty() {
                continue;
            }
            if let Some(col) = available.iter().find(|c| {
                normalize_message(&c.key) == p || normalize_message(&c.label) == p
            }) {
                picked.push(col.key.clone());
            }
        }
    }
    picked
}

fn columns_from_message(message: &str, available: &[ColMeta]) -> Vec<String> {
    let cols = parse_requested_columns(message, available);
    if !cols.is_empty() {
        return cols;
    }
    if message.contains(" avec ") || message.contains(" colonne") {
        return parse_requested_columns(message, available);
    }
    vec![]
}

fn detect_joins(registry: &EntityRegistry, entity_key: &str, message: &str) -> Vec<JoinHint> {
    let msg = normalize_message(message);
    let ent = match registry.find(entity_key) {
        Some(e) => e,
        None => return vec![],
    };
    let mut joins = Vec::new();
    for attr in &ent.attributs {
        if attr.attr_type != "entity" || attr.relation_multiple {
            continue;
        }
        let Some(ref_key) = attr.r#ref.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
            continue;
        };
        let ref_ent = registry.find(ref_key);
        let terms = ref_ent
            .map(|e| intent::entity_terms(e))
            .unwrap_or_default();
        if terms.iter().any(|t| t.len() >= 3 && msg.contains(t)) {
            joins.push(JoinHint {
                via_field: attr.nom.clone(),
                via_label: attr.label.clone().unwrap_or_else(|| attr.nom.clone()),
                ref_entity: ref_key.to_string(),
                ref_label_field: "nom".into(),
            });
        }
    }
    joins
}

fn build_rows_with_joins(
    db: &Database,
    data_dir: &Path,
    entity_key: &str,
    column_keys: &[String],
    joins: &[JoinHint],
    filters: &HashMap<String, String>,
    user: &SessionUser,
) -> Result<(Vec<ChatDisplayBlock>, String), String> {
    let cfg = load_screen_config(data_dir, entity_key)?;
    let priv_key = format!("{entity_key}:voir");
    if !has_privilege(&user.privileges, &priv_key) {
        return Err(format!(
            "Vous n'avez pas le droit de consulter l'entité « {entity_key} »."
        ));
    }

    let rows = list_rows_with_options(
        db,
        &cfg,
        filters,
        ListRowsOptions {
            viewer_role_id: None,
            viewer_user_id: None,
            viewer_privileges: &user.privileges,
        },
        None,
    )?;
    let rows = apply_exact_filters(rows, &cfg, filters);

    let field_by_key: HashMap<String, &FieldDef> =
        cfg.fields.iter().map(|f| (f.key.clone(), f)).collect();

    let mut columns: Vec<ChatDisplayColumn> = column_keys
        .iter()
        .filter_map(|k| {
            field_by_key.get(k).map(|f| ChatDisplayColumn {
                key: f.key.clone(),
                label: f.label.clone(),
            })
        })
        .collect();

    let mut joins_resolved = joins.to_vec();
    for j in &mut joins_resolved {
        j.ref_label_field = load_screen_config(data_dir, &j.ref_entity)
            .map(|c| c.screen.label_field)
            .unwrap_or_else(|_| "nom".into());
        columns.push(ChatDisplayColumn {
            key: format!("{}__{}", j.ref_entity, j.ref_label_field),
            label: format!("{} ({})", j.via_label, j.ref_entity),
        });
    }

    let mut ref_labels: HashMap<String, HashMap<String, String>> = HashMap::new();
    for j in &joins_resolved {
        let ref_cfg = load_screen_config(data_dir, &j.ref_entity)?;
        let ref_table = ref_cfg.screen.table.clone();
        let pk = ref_cfg.screen.primary_key.clone();
        let label_col = j.ref_label_field.clone();
        let sql = format!("SELECT {pk}, {label_col} FROM {ref_table}");
        let mut stmt = db.conn.prepare(&sql).map_err(|e| e.to_string())?;
        let mut map = HashMap::new();
        let rows_ref = stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?;
        for pair in rows_ref.flatten() {
            map.insert(pair.0, pair.1);
        }
        ref_labels.insert(j.via_field.clone(), map);
    }

    let mut out_rows: Vec<Map<String, Value>> = Vec::new();
    for row in rows {
        let mut out = Map::new();
        for key in column_keys {
            if let Some(v) = row.get(key) {
                out.insert(key.clone(), v.clone());
            }
        }
        for j in &joins_resolved {
            let fk = row
                .get(&j.via_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            let label = ref_labels
                .get(&j.via_field)
                .and_then(|m| m.get(&fk))
                .cloned()
                .unwrap_or_else(|| fk.clone());
            out.insert(
                format!("{}__{}", j.ref_entity, j.ref_label_field),
                Value::String(label),
            );
        }
        out_rows.push(out);
    }

    let label = cfg.screen.label.clone();
    let kind = if out_rows.len() <= 3 && column_keys.len() == 1 {
        "list".into()
    } else {
        "table".into()
    };

    let count = out_rows.len();
    let block = ChatDisplayBlock {
        kind,
        entity_key: Some(entity_key.to_string()),
        columns,
        rows: out_rows,
    };
    let filter_hint = filters_summary(&cfg, filters);
    let msg = if count == 0 {
        if filter_hint.is_empty() {
            format!(
                "C'est fait ! Aucun enregistrement pour « {label} ». Clique sur le bouton ci-dessous pour voir le détail."
            )
        } else {
            format!(
                "C'est fait ! Aucun enregistrement pour « {label} » ({filter_hint}). Clique sur le bouton ci-dessous pour voir le détail."
            )
        }
    } else if filter_hint.is_empty() {
        format!(
            "C'est fait ! J'ai préparé {count} enregistrement(s) pour « {label} ». Clique sur le bouton ci-dessous pour afficher la liste dans une fenêtre."
        )
    } else {
        format!(
            "C'est fait ! J'ai préparé {count} enregistrement(s) pour « {label} » ({filter_hint}). Clique sur le bouton ci-dessous pour afficher la liste dans une fenêtre."
        )
    };
    Ok((vec![block], msg))
}

fn ask_columns_message(entity_label: &str) -> String {
    format!(
        "J'ai besoin de savoir quelles colonnes afficher pour « {entity_label} ». \
         Clique sur le bouton ci-dessous pour choisir dans une fenêtre."
    )
}

/// Tente une réponse liste/tableau dans le chat dashboard.
pub fn try_list_preview(
    db: &Database,
    user: &SessionUser,
    conv_id: &str,
    user_message: &str,
) -> Result<Option<ListPreviewResult>, String> {
    let registry = crate::entity::registry::load(&db.data_dir)?;
    let data_dir = &db.data_dir;

    if let Some(prev) = last_assistant_message(db, conv_id) {
        if let Some(req) = parse_cols_request(&prev) {
            let available = req.available;
            let entity_label = registry_entity_label(&registry, &req.entity_key);
            let (msg_filters, text_for_cols) =
                extract_filters_from_message(user_message, &load_screen_config(data_dir, &req.entity_key)?);
            let mut filters = req.filters.clone();
            filters.extend(msg_filters);
            let cols = parse_requested_columns(&text_for_cols, &available);
            if cols.is_empty() {
                let msg = ask_columns_message(&entity_label);
                return Ok(Some(ListPreviewResult {
                    raw_message: embed_cols_request(&msg, &req.entity_key, &available, &filters),
                    display_blocks: vec![],
                    cols_request: Some(make_cols_request(
                        &req.entity_key,
                        &entity_label,
                        &available,
                        &filters,
                    )),
                }));
            }
            let joins = detect_joins(&registry, &req.entity_key, user_message);
            let (blocks, msg) = build_rows_with_joins(
                db,
                data_dir,
                &req.entity_key,
                &cols,
                &joins,
                &filters,
                user,
            )?;
            return Ok(Some(ListPreviewResult {
                raw_message: embed_display(&msg, &blocks),
                display_blocks: blocks,
                cols_request: None,
            }));
        }
    }

    let Some(entity_key) = match_list_entity(user_message, &registry) else {
        return Ok(None);
    };

    let cfg = load_screen_config(data_dir, &entity_key)?;
    let entity_label = cfg.screen.label.clone();
    let (filters, text_for_cols) = extract_filters_from_message(user_message, &cfg);
    let available = list_column_metas(&cfg);
    let cols = columns_from_message(&text_for_cols, &available);

    if cols.is_empty() {
        let msg = ask_columns_message(&entity_label);
        return Ok(Some(ListPreviewResult {
            raw_message: embed_cols_request(&msg, &entity_key, &available, &filters),
            display_blocks: vec![],
            cols_request: Some(make_cols_request(
                &entity_key,
                &entity_label,
                &available,
                &filters,
            )),
        }));
    }

    let joins = detect_joins(&registry, &entity_key, user_message);
    let (blocks, msg) =
        build_rows_with_joins(db, data_dir, &entity_key, &cols, &joins, &filters, user)?;
    Ok(Some(ListPreviewResult {
        raw_message: embed_display(&msg, &blocks),
        display_blocks: blocks,
        cols_request: None,
    }))
}
