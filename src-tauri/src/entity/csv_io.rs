//! Import / export CSV des objets d'une entité (séparateur `|`).

use std::collections::HashMap;

use rusqlite::OptionalExtension;
use serde_json::{Map, Value};

use crate::csv_util::{push_row_entity_csv, strip_bom, ENTITY_CSV_DELIMITER};
use crate::db::Database;
use crate::dda::config::{is_persisted_field, FieldDef};
use crate::dda::crud::{create_row_with_user_and_options, update_row, CreateRowOptions};
use crate::entity::compteur::{self, is_compteur_attr};
use crate::entity::load_screen_config;
use crate::entity::registry::{self, EntityRegistry};
use crate::entity::validation;

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
    let registry = registry::load(data_dir)?;
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
    let mut touched_ids: Vec<String> = Vec::new();
    let import_opts = CreateRowOptions {
        skip_post_create_hooks: true,
    };

    db.conn
        .execute_batch("BEGIN IMMEDIATE;")
        .map_err(|e| format!("Début transaction import : {e}"))?;

    let import_result = (|| {
        for (line_no, line) in lines.iter().skip(1).enumerate() {
            if line_no > 0 && line_no % 20 == 0 {
                std::thread::yield_now();
            }
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
                let hdr = hdr.trim();
                if hdr.is_empty() {
                    continue;
                }
                let Some(field_key) = resolve_csv_header_key(hdr, &field_by_key, &cfg, &registry) else {
                    continue;
                };
                let field = field_by_key.get(&field_key).expect("resolved key exists");
                if field.column == cfg.screen.primary_key {
                    if !val.trim().is_empty() {
                        id = Some(val.trim().to_string());
                    }
                    continue;
                }
                if val.trim().is_empty() {
                    continue;
                }
                data.insert(
                    field_key.clone(),
                    csv_value_to_json(val.trim(), field),
                );
            }

            let existing_id = find_existing_record_id(db, &cfg, &registry, &data, id.take());

            let result = if let Some(ref existing_id) = existing_id {
                update_row(db, &cfg, existing_id, &data).map(|_| {
                    touched_ids.push(existing_id.clone());
                    (false, true)
                })
            } else {
                create_row_with_user_and_options(db, &cfg, &data, None, import_opts).map(|row| {
                    if let Some(id) = row.get("id").and_then(|v| v.as_str()) {
                        touched_ids.push(id.to_string());
                    }
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
        Ok::<(), String>(())
    })();

    if let Err(e) = import_result {
        let _ = db.conn.execute_batch("ROLLBACK;");
        return Err(e);
    }
    db.conn
        .execute_batch("COMMIT;")
        .map_err(|e| format!("Validation import : {e}"))?;

    if let Err(e) = validation::reconcile_signature_tasks(db, data_dir) {
        errors.push(format!("Réconciliation des tâches de signature : {e}"));
    }
    reconcile_relation_impacts_after_import(db, data_dir, screen_key, &touched_ids);

    let error_count = errors.len() as u32;
    Ok(EntityCsvImportResult {
        success: error_count == 0,
        inserted,
        updated,
        error_count,
        errors,
    })
}

fn reconcile_relation_impacts_after_import(
    db: &Database,
    data_dir: &std::path::Path,
    screen_key: &str,
    record_ids: &[String],
) {
    if record_ids.is_empty() {
        return;
    }
    let registry = match registry::load(data_dir) {
        Ok(r) => r,
        Err(_) => return,
    };
    if crate::entity::record_signature::entity_requires_signature(&registry, screen_key) {
        return;
    }
    for record_id in record_ids {
        crate::entity::relation_impact::apply_after_create_if_ready(
            db,
            data_dir,
            screen_key,
            record_id,
        );
    }
}

/// En-tête CSV : accepte le nom d'attribut matricule/compteur (`reference`) ou la clé DDA (`reference_libelle`).
fn resolve_csv_header_key(
    hdr: &str,
    field_by_key: &HashMap<String, &FieldDef>,
    cfg: &crate::dda::config::ScreenConfigFile,
    registry: &EntityRegistry,
) -> Option<String> {
    if field_by_key.contains_key(hdr) {
        return Some(hdr.to_string());
    }
    let ent = registry.find(&cfg.screen.key)?;
    for attr in ent.attributs.iter().filter(|a| is_compteur_attr(a)) {
        if attr.nom == hdr {
            let libelle_key = compteur::column_libelle(attr);
            if field_by_key.contains_key(&libelle_key) {
                return Some(libelle_key);
            }
        }
    }
    None
}

fn csv_value_to_json(val: &str, field: &FieldDef) -> Value {
    match field.field_type.as_str() {
        "number" | "integer" | "float" | "stock" => {
            if let Ok(i) = val.parse::<i64>() {
                Value::Number(i.into())
            } else if let Ok(f) = val.parse::<f64>() {
                serde_json::Number::from_f64(f)
                    .map(Value::Number)
                    .unwrap_or_else(|| Value::String(val.to_string()))
            } else {
                Value::String(val.to_string())
            }
        }
        "boolean" => Value::Bool(matches!(
            val.to_ascii_lowercase().as_str(),
            "1" | "true" | "oui" | "yes"
        )),
        _ => Value::String(val.to_string()),
    }
}

fn find_existing_record_id(
    db: &Database,
    cfg: &crate::dda::config::ScreenConfigFile,
    registry: &EntityRegistry,
    data: &Map<String, Value>,
    pk_from_csv: Option<String>,
) -> Option<String> {
    let table = &cfg.screen.table;
    let pk = &cfg.screen.primary_key;

    if let Some(ref id) = pk_from_csv {
        if !id.trim().is_empty() {
            let sql = format!("SELECT {pk} FROM {table} WHERE {pk} = ?1 LIMIT 1");
            if let Ok(found) = db
                .conn
                .query_row(&sql, rusqlite::params![id.trim()], |row| row.get::<_, String>(0))
            {
                return Some(found);
            }
        }
    }

    let ent = registry.find(&cfg.screen.key)?;
    for attr in ent.attributs.iter().filter(|a| is_compteur_attr(a)) {
        let lib_col = compteur::column_libelle(attr);
        let lib_val = data
            .get(&lib_col)
            .or_else(|| data.get(&attr.nom))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let Some(v) = lib_val else {
            continue;
        };
        let sql = format!("SELECT {pk} FROM {table} WHERE {lib_col} = ?1 LIMIT 1");
        if let Ok(found) = db
            .conn
            .query_row(&sql, rusqlite::params![v], |row| row.get::<_, String>(0))
        {
            return Some(found);
        }
    }

    None
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::apply_registry;
    use crate::entity::registry::{EntityAttribute, EntityDef, EntityRegistry};
    use crate::entity::registry;
    use crate::db::Database;
    use std::fs;

    fn attr(nom: &str, attr_type: &str, required: bool) -> EntityAttribute {
        EntityAttribute {
            nom: nom.into(),
            attr_type: attr_type.into(),
            label: Some(nom.into()),
            required,
            r#ref: None,
            relation_multiple: false,
            relation_exclusive_parent: true,
            default: None,
            enum_options: None,
            relation_impact_source: None,
            relation_impact_target: None,
            relation_impact_action: None,
            relation_impact_defer: false,
        }
    }

    fn setup_client_registry(tmp: &std::path::Path) -> Database {
        fs::create_dir_all(tmp.join("entities")).unwrap();
        let client = EntityDef {
            nom: "client".into(),
            label: Some("Client".into()),
            description: None,
            ai_suggestions: false,
            requires_signature: false,
            signatory_role_ids: vec![],
            is_session: false,
            attributs: vec![
                attr("reference", "matricule", false),
                attr("prenom", "string", true),
                attr("nom", "string", true),
                attr("age", "integer", false),
                attr("ville", "string", false),
                attr("profession", "string", false),
                attr("email", "email", false),
                attr("telephone", "string", false),
            ],
        };
        let tache = EntityDef {
            nom: "tache".into(),
            label: Some("Tâche".into()),
            description: None,
            ai_suggestions: false,
            requires_signature: false,
            signatory_role_ids: vec![],
            is_session: false,
            attributs: vec![attr("libelle", "string", true)],
        };
        let reg = EntityRegistry {
            ecosysteme: None,
            slogan: None,
            logo_url: None,
            logo: None,
            entities: vec![tache, client],
        };
        registry::save(tmp, &reg).unwrap();
        let db = Database::open(tmp.to_path_buf()).unwrap();
        crate::dda::schema::ensure_dda_registry_table(&db).unwrap();
        apply_registry(&db, tmp, &EntityRegistry::default(), None).unwrap();
        db
    }

    #[test]
    fn import_clients_csv_inserts_rows() {
        let tmp = std::env::temp_dir().join(format!("blin-csv-import-{}", uuid::Uuid::new_v4()));
        let db = setup_client_registry(&tmp);
        let csv = "reference|prenom|nom|age|ville|profession|email|telephone\n\
CLI-01|David|Laurent|62|Strasbourg|Comptable|david.laurent@mail.fr|06 70 52 20 78\n\
CLI-02|Chloe|Roux|27|Nantes|Cadre|chloe.roux@mail.fr|06 22 54 50 38";
        let res = import_entity_csv(&db, &db.data_dir, "client", csv).unwrap();
        assert_eq!(res.error_count, 0, "{:?}", res.errors);
        assert_eq!(res.inserted, 2);
        assert_eq!(res.updated, 0);

        let csv2 = "reference|prenom|nom|age|ville|profession|email|telephone\n\
CLI-01|David|Laurent|63|Strasbourg|Comptable|david.laurent@mail.fr|06 70 52 20 78";
        let res2 = import_entity_csv(&db, &db.data_dir, "client", csv2).unwrap();
        assert_eq!(res2.error_count, 0, "{:?}", res2.errors);
        assert_eq!(res2.inserted, 0);
        assert_eq!(res2.updated, 1);

        let _ = fs::remove_dir_all(tmp);
    }

    #[test]
    fn resolve_matricule_header_alias() {
        let tmp = std::env::temp_dir().join(format!("blin-csv-header-{}", uuid::Uuid::new_v4()));
        let db = setup_client_registry(&tmp);
        let cfg = load_screen_config(&db.data_dir, "client").unwrap();
        let reg = registry::load(&db.data_dir).unwrap();
        let field_by_key: HashMap<String, _> =
            cfg.fields.iter().map(|f| (f.key.clone(), f)).collect();
        let key = resolve_csv_header_key("reference", &field_by_key, &cfg, &reg).unwrap();
        assert_eq!(key, "reference_libelle");
        let _ = fs::remove_dir_all(tmp);
    }
}
