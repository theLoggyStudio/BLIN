//! Pont Loggy ↔ CRUD DDA (écrans pilotés par JSON).

use serde_json::{Map, Value};

use crate::dda::config::{FieldDef, ScreenConfigFile};
use crate::dda::crud;
use crate::dda::load_screen_config;
use crate::db::Database;

use super::crud::resolve_bien_id;

const BIENS_SCREEN: &str = "biens";

pub fn biens_uses_dda() -> bool {
    load_screen_config(BIENS_SCREEN).is_ok()
}

pub fn execute_bien_write_via_dda(
    db: &Database,
    tool: &str,
    params: &Value,
) -> Result<String, String> {
    let cfg = load_screen_config(BIENS_SCREEN)?;
    match tool {
        "create_bien" => {
            let data = params_to_dda_map(&cfg, params);
            let row = crud::create_row(db, &cfg, &data)?;
            let reference = row
                .get(&cfg.screen.label_field)
                .or_else(|| row.get("reference"))
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            Ok(format!(
                "Bien {reference} créé (DDA). Les photos s'ajoutent via l'écran Biens ou l'app mobile."
            ))
        }
        "update_bien" => {
            let id = resolve_bien_id(db, params)?;
            let existing = crud::get_row(db, &cfg, &id)?;
            let mut data = existing;
            merge_dda_params(&cfg, &mut data, params);
            let row = crud::update_row(db, &cfg, &id, &data)?;
            let reference = row
                .get(&cfg.screen.label_field)
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            Ok(format!("Bien {reference} mis à jour (DDA)."))
        }
        "delete_bien" => {
            let id = resolve_bien_id(db, params)?;
            let existing = crud::get_row(db, &cfg, &id)?;
            let type_bien = existing
                .get("type_bien")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if type_bien == "hangar" {
                return Err("C'est un hangar : utilisez delete_hangar.".into());
            }
            let reference = existing
                .get("reference")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            crud::delete_row(db, &cfg, &id)?;
            Ok(format!("Bien {reference} supprimé (DDA)."))
        }
        other => Err(format!("Outil DDA biens non géré : {other}")),
    }
}

fn params_to_dda_map(cfg: &ScreenConfigFile, params: &Value) -> Map<String, Value> {
    let mut map = Map::new();
    let obj = params.as_object();
    for field in &cfg.fields {
        if field.field_type == "hidden" {
            continue;
        }
        let Some(raw) = obj
            .and_then(|o| o.get(&field.key))
            .or_else(|| obj.and_then(|o| o.get(&field.column)))
        else {
            continue;
        };
        if raw.is_null() {
            continue;
        }
        map.insert(field.key.clone(), normalize_field_value(field, raw));
    }
    map
}

fn merge_dda_params(cfg: &ScreenConfigFile, target: &mut Map<String, Value>, params: &Value) {
    let patch = params_to_dda_map(cfg, params);
    for (k, v) in patch {
        if !value_is_empty(&v) {
            target.insert(k, v);
        }
    }
}

fn normalize_field_value(field: &FieldDef, raw: &Value) -> Value {
    match field.field_type.as_str() {
        "images" => normalize_images_value(raw),
        "number" => {
            if let Some(n) = raw.as_f64() {
                serde_json::Number::from_f64(n)
                    .map(Value::Number)
                    .unwrap_or(Value::Null)
            } else if let Some(s) = raw.as_str() {
                s.parse::<f64>()
                    .ok()
                    .and_then(|n| serde_json::Number::from_f64(n).map(Value::Number))
                    .unwrap_or_else(|| Value::String(s.to_string()))
            } else {
                raw.clone()
            }
        }
        "boolean" => Value::Bool(raw.as_bool().unwrap_or(false)),
        _ => {
            if let Some(s) = raw.as_str() {
                Value::String(s.to_string())
            } else {
                raw.clone()
            }
        }
    }
}

fn normalize_images_value(raw: &Value) -> Value {
    match raw {
        Value::Array(items) => Value::Array(
            items
                .iter()
                .filter_map(|x| x.as_str().map(|s| Value::String(s.to_string())))
                .collect(),
        ),
        Value::String(s) => {
            let t = s.trim();
            if t.starts_with('[') {
                serde_json::from_str(t).unwrap_or_else(|_| Value::Array(vec![]))
            } else if t.is_empty() {
                Value::Array(vec![])
            } else {
                Value::Array(vec![Value::String(t.to_string())])
            }
        }
        _ => Value::Array(vec![]),
    }
}

fn value_is_empty(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::String(s) => s.trim().is_empty(),
        Value::Array(a) => a.is_empty(),
        _ => false,
    }
}
