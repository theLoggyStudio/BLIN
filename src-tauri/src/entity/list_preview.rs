//! Affichage de listes entités dans le chat (tableau / liste) sans ouvrir le formulaire.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::Path;

use crate::ai::agent::ChatDisplayBlock;
use crate::ai::agent::ChatDisplayColumn;
use crate::ai::intent_filters::{normalize_message, wants_list_intent};
use crate::dda::config::FieldDef;
use crate::dda::crud::{list_rows_with_options, ListRowsOptions};
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

fn embed_cols_request(message: &str, entity_key: &str, available: &[ColMeta]) -> String {
    let payload = StoredColsRequest {
        entity_key: entity_key.to_string(),
        available: available.to_vec(),
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
        &HashMap::new(),
        ListRowsOptions {
            viewer_role_id: None,
            viewer_privileges: &user.privileges,
        },
    )?;

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
    let msg = format!("Voici {count} enregistrement(s) pour « {label} ».");
    Ok((vec![block], msg))
}

fn ask_columns_message(entity_key: &str, available: &[ColMeta]) -> String {
    let labels: Vec<String> = available.iter().map(|c| c.label.clone()).collect();
    format!(
        "Quelles colonnes souhaitez-vous afficher pour « {entity_key} » ?\n\
         Colonnes disponibles : {}.\n\
         Répondez avec les noms séparés par des virgules, ou « toutes » pour tout afficher.",
        labels.join(", ")
    )
}

/// Tente une réponse liste/tableau dans le chat dashboard.
pub fn try_list_preview(
    db: &Database,
    user: &SessionUser,
    conv_id: &str,
    user_message: &str,
) -> Result<Option<(String, Vec<ChatDisplayBlock>)>, String> {
    let registry = crate::entity::registry::load(&db.data_dir)?;
    let data_dir = &db.data_dir;

    if let Some(prev) = last_assistant_message(db, conv_id) {
        if let Some(req) = parse_cols_request(&prev) {
            let available = req.available;
            let cols = parse_requested_columns(user_message, &available);
            if cols.is_empty() {
                let msg = ask_columns_message(&req.entity_key, &available);
                return Ok(Some((
                    embed_cols_request(&msg, &req.entity_key, &available),
                    vec![],
                )));
            }
            let joins = detect_joins(&registry, &req.entity_key, user_message);
            let (blocks, msg) = build_rows_with_joins(
                db,
                data_dir,
                &req.entity_key,
                &cols,
                &joins,
                user,
            )?;
            return Ok(Some((embed_display(&msg, &blocks), blocks)));
        }
    }

    let Some(entity_key) = match_list_entity(user_message, &registry) else {
        return Ok(None);
    };

    let cfg = load_screen_config(data_dir, &entity_key)?;
    let available = list_column_metas(&cfg);
    let cols = columns_from_message(user_message, &available);

    if cols.is_empty() {
        let msg = ask_columns_message(&entity_key, &available);
        return Ok(Some((
            embed_cols_request(&msg, &entity_key, &available),
            vec![],
        )));
    }

    let joins = detect_joins(&registry, &entity_key, user_message);
    let (blocks, msg) =
        build_rows_with_joins(db, data_dir, &entity_key, &cols, &joins, user)?;
    Ok(Some((embed_display(&msg, &blocks), blocks)))
}
