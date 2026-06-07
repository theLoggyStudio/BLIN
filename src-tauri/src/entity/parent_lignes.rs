//! Lignes multiples d'une même entité mère (même matricule) — colonne `lignes` (JSON).

use serde_json::{Map, Value};

use super::embed;
use super::registry::{EntityDef, EntityRegistry};
use crate::dda::config::{FieldDef, ScreenConfigFile};

pub const LIGNES_COLUMN: &str = "lignes";
pub const CREATE_LINES_FORM_KEY: &str = "__create_lines__";

pub fn entity_has_embed_children(ent: &EntityDef, registry: &EntityRegistry) -> bool {
    ent.attributs.iter().any(|a| {
        a.attr_type == "entity" && embed::resolve_child(registry, a).is_some()
    })
}

pub fn parse_lignes_items(raw: Option<&Value>) -> Vec<Map<String, Value>> {
    match raw {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|v| v.as_object().cloned())
            .collect(),
        Some(Value::String(s)) if !s.trim().is_empty() => {
            serde_json::from_str::<Value>(s)
                .ok()
                .and_then(|v| {
                    if let Value::Array(items) = v {
                        Some(
                            items
                                .iter()
                                .filter_map(|x| x.as_object().cloned())
                                .collect(),
                        )
                    } else {
                        None
                    }
                })
                .unwrap_or_default()
        }
        _ => Vec::new(),
    }
}

fn embed_snapshot_fields(cfg: &ScreenConfigFile) -> Vec<&FieldDef> {
    cfg.fields
        .iter()
        .filter(|f| {
            f.field_type == "entity_embed_list"
                || f
                    .form
                    .as_ref()
                    .and_then(|m| m.embed_parent.as_ref())
                    .is_some()
        })
        .collect()
}

fn field_value(data: &Map<String, Value>, field: &FieldDef) -> Option<Value> {
    data.get(&field.key)
        .or_else(|| data.get(&field.column))
        .cloned()
}

fn snapshot_from_data(data: &Map<String, Value>, fields: &[&FieldDef]) -> Map<String, Value> {
    let mut snap = Map::new();
    for f in fields {
        if let Some(v) = field_value(data, f) {
            snap.insert(f.key.clone(), v);
        }
    }
    snap
}

/// Avant INSERT/UPDATE : fusionne ligne 1 (aplatie) + `__create_lines__` → colonne `lignes`.
pub fn merge_create_lines_into_data(data: &mut Map<String, Value>, cfg: &ScreenConfigFile) {
    let fields = embed_snapshot_fields(cfg);
    if fields.is_empty() {
        data.remove(CREATE_LINES_FORM_KEY);
        return;
    }

    let line1 = snapshot_from_data(data, &fields);
    let extras_raw = data.remove(CREATE_LINES_FORM_KEY);
    let extras = parse_lignes_items(extras_raw.as_ref());

    let mut all = vec![line1];
    all.extend(extras);
    if all.len() > 1 {
        data.insert(
            LIGNES_COLUMN.into(),
            Value::Array(all.into_iter().map(Value::Object).collect()),
        );
    } else if all.len() == 1 && !all[0].is_empty() {
        data.insert(
            LIGNES_COLUMN.into(),
            Value::Array(vec![Value::Object(all[0].clone())]),
        );
    } else {
        data.remove(LIGNES_COLUMN);
    }
}

/// Après lecture : expose `__create_lines__` (lignes 2+) pour le formulaire.
/// La ligne 1 reste sur la racine (colonnes DB + listes embarquées hydratées).
pub fn hydrate_form_lines(row: &mut Map<String, Value>, cfg: &ScreenConfigFile) {
    let fields = embed_snapshot_fields(cfg);
    if fields.is_empty() {
        return;
    }
    let items = parse_lignes_items(row.get(LIGNES_COLUMN));
    if items.len() <= 1 {
        return;
    }
    row.insert(
        CREATE_LINES_FORM_KEY.into(),
        Value::Array(
            items[1..]
                .iter()
                .cloned()
                .map(Value::Object)
                .collect(),
        ),
    );
}
