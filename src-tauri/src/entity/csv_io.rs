//! Import / export CSV des objets d'une entité (séparateur `|`).

use std::collections::HashMap;

use serde_json::{Map, Value};

use crate::csv_util::{push_row_entity_csv, strip_bom, ENTITY_CSV_DELIMITER};
use crate::db::Database;
use crate::dda::config::is_persisted_field;
use crate::dda::crud::{create_row, update_row};
use crate::entity::load_screen_config;

#[derive(Debug, serde::Serialize)]
pub struct EntityCsvImportResult {
    pub success: bool,
    pub inserted: u32,
    pub updated: u32,
    pub error_count: u32,
    pub errors: Vec<String>,
}

pub fn export_entity_csv(
    db: &Database,
    data_dir: &std::path::Path,
    screen_key: &str,
) -> Result<(String, String), String> {
    let cfg = load_screen_config(data_dir, screen_key)?;
    let export_fields: Vec<_> = cfg
        .fields
        .iter()
        .filter(|f| {
            is_persisted_field(f)
                && f.field_type != "entity_embed"
                && f.field_type != "entity_embed_list"
                && f.form.as_ref().and_then(|m| m.embed_parent.as_ref()).is_none()
                && f.field_type != "hidden"
                && f.field_type != "detail_link"
        })
        .collect();

    let table = &cfg.screen.table;
    let pk = &cfg.screen.primary_key;
    let cols: Vec<String> = export_fields.iter().map(|f| f.column.clone()).collect();
    let header: Vec<String> = export_fields.iter().map(|f| f.key.clone()).collect();

    let sql = format!(
        "SELECT {} FROM {table} ORDER BY {pk}",
        cols.join(", ")
    );
    let mut stmt = db.conn.prepare(&sql).map_err(|e| e.to_string())?;
    let col_count = cols.len();
    let rows = stmt
        .query_map([], |row| {
            let mut vals = Vec::with_capacity(col_count);
            for i in 0..col_count {
                let v: rusqlite::types::Value = row.get(i)?;
                vals.push(sql_value_to_string(&v));
            }
            Ok(vals)
        })
        .map_err(|e| e.to_string())?
        .flatten()
        .collect::<Vec<_>>();

    let mut w = String::from("\u{feff}");
    push_row_entity_csv(&mut w, &header);
    for row in rows {
        push_row_entity_csv(&mut w, &row);
    }
    let file_name = format!("blin_{screen_key}.csv");
    Ok((w, file_name))
}

pub fn import_entity_csv(
    db: &Database,
    data_dir: &std::path::Path,
    screen_key: &str,
    csv_text: &str,
) -> Result<EntityCsvImportResult, String> {
    let cfg = load_screen_config(data_dir, screen_key)?;
    let lines: Vec<&str> = strip_bom(csv_text)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if lines.is_empty() {
        return Err("Fichier CSV vide.".into());
    }

    let headers = parse_csv_line(lines[0])?;
    let field_by_key: HashMap<String, _> = cfg
        .fields
        .iter()
        .map(|f| (f.key.clone(), f))
        .collect();

    let mut inserted = 0u32;
    let mut updated = 0u32;
    let mut errors = Vec::new();

    for (line_no, line) in lines.iter().skip(1).enumerate() {
        let cells = parse_csv_line(line)?;
        if cells.len() != headers.len() {
            errors.push(format!(
                "Ligne {} : {} colonne(s) attendue(s), {} reçue(s).",
                line_no + 2,
                headers.len(),
                cells.len()
            ));
            continue;
        }
        let mut data = Map::new();
        let mut id: Option<String> = None;
        for (hdr, val) in headers.iter().zip(cells.iter()) {
            let key = hdr.trim();
            if key.is_empty() {
                continue;
            }
            let Some(field) = field_by_key.get(key) else {
                continue;
            };
            if field.column == cfg.screen.primary_key {
                if !val.trim().is_empty() {
                    id = Some(val.trim().to_string());
                }
                continue;
            }
            if val.trim().is_empty() {
                continue;
            }
            data.insert(key.to_string(), Value::String(val.trim().to_string()));
        }

        let result = if let Some(ref existing_id) = id {
            update_row(db, &cfg, existing_id, &data).map(|_| (false, true))
        } else {
            create_row(db, &cfg, &data).map(|row| {
                let _ = row;
                (true, false)
            })
        };

        match result {
            Ok((ins, upd)) => {
                if ins {
                    inserted += 1;
                }
                if upd {
                    updated += 1;
                }
            }
            Err(e) => errors.push(format!("Ligne {} : {e}", line_no + 2)),
        }
    }

    let error_count = errors.len() as u32;
    Ok(EntityCsvImportResult {
        success: error_count == 0,
        inserted,
        updated,
        error_count,
        errors,
    })
}

fn sql_value_to_string(v: &rusqlite::types::Value) -> String {
    match v {
        rusqlite::types::Value::Null => String::new(),
        rusqlite::types::Value::Integer(i) => i.to_string(),
        rusqlite::types::Value::Real(f) => f.to_string(),
        rusqlite::types::Value::Text(s) => s.clone(),
        rusqlite::types::Value::Blob(b) => String::from_utf8_lossy(b).to_string(),
    }
}

fn parse_csv_line(line: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if !in_quotes => in_quotes = true,
            '"' if in_quotes => {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    cur.push('"');
                } else {
                    in_quotes = false;
                }
            }
            c if c == ENTITY_CSV_DELIMITER && !in_quotes => {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            other => cur.push(other),
        }
    }
    out.push(cur.trim().to_string());
    Ok(out)
}
