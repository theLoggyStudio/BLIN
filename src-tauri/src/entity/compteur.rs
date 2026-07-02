//! Attributs « compteur » et « matricule » : libellé/base + date (jjmmaaaa) + numéro auto.

use chrono::Local;
use rusqlite::OptionalExtension;
use serde_json::{Map, Value};

use super::matricule_registry::{self, MatriculeRegistry};
use super::registry::{EntityAttribute, EntityDef, EntityRegistry};
use super::schema::{attr_column, table_name};
use crate::db::Database;

pub const COMPTEUR_ATTR_TYPE: &str = "compteur";
pub const MATRICULE_ATTR_TYPE: &str = "matricule";

pub fn is_compteur_attr(attr: &EntityAttribute) -> bool {
    attr.attr_type == COMPTEUR_ATTR_TYPE || attr.attr_type == MATRICULE_ATTR_TYPE
}

pub fn is_matricule_attr(attr: &EntityAttribute) -> bool {
    attr.attr_type == MATRICULE_ATTR_TYPE
}

pub fn column_base(attr: &EntityAttribute) -> String {
    format!("{}_base", attr_column(attr))
}

pub fn column_libelle(attr: &EntityAttribute) -> String {
    format!("{}_libelle", attr_column(attr))
}

pub fn column_jjmmaaaa(attr: &EntityAttribute) -> String {
    format!("{}_jjmmaaaa", attr_column(attr))
}

pub fn column_numero(attr: &EntityAttribute) -> String {
    format!("{}_numero", attr_column(attr))
}

pub fn all_sql_columns(attr: &EntityAttribute) -> Vec<String> {
    if is_matricule_attr(attr) {
        vec![
            column_base(attr),
            column_libelle(attr),
            column_jjmmaaaa(attr),
            column_numero(attr),
        ]
    } else {
        vec![
            column_libelle(attr),
            column_jjmmaaaa(attr),
            column_numero(attr),
        ]
    }
}

pub fn today_jjmmaaaa() -> String {
    Local::now().format("%d%m%Y").to_string()
}

fn libelle_value(attr: &EntityAttribute, ent: &EntityDef) -> String {
    attr.label
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            ent.label
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .unwrap_or_else(|| attr.nom.clone())
        })
}

fn matricule_def_for_attr<'a>(
    matricules: &'a MatriculeRegistry,
    attr: &EntityAttribute,
) -> Result<&'a matricule_registry::MatriculeDef, String> {
    matricules.resolve_for_attr(attr).ok_or_else(|| {
        if matricules.matricules.is_empty() {
            format!(
                "Aucune définition matricule — créez-en une dans Paramètres (attribut « {} »).",
                attr.label.as_deref().unwrap_or(&attr.nom)
            )
        } else {
            format!(
                "L'attribut « {} » (matricule) doit être lié à une définition dans Paramètres.",
                attr.label.as_deref().unwrap_or(&attr.nom)
            )
        }
    })
}

/// Format affiché : `<base><date jjmmaaaa><compteur>` — ex. MAT1203202601.
pub fn format_full_matricule(base: &str, date: &str, numero: i64) -> String {
    format!("{}{}{numero:02}", base.trim(), date.trim())
}

pub fn format_matricule_from_row(row: &Map<String, Value>, attr: &EntityAttribute) -> String {
    let base = row
        .get(&column_base(attr))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let base = if base.is_empty() {
        row.get(&column_libelle(attr))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
    } else {
        base
    };
    let date = row
        .get(&column_jjmmaaaa(attr))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let numero = row
        .get(&column_numero(attr))
        .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(0);
    if base.is_empty() && date.is_empty() && numero == 0 {
        return String::new();
    }
    if base.is_empty() {
        return format!("{date}{numero:02}");
    }
    format_full_matricule(base, date, numero)
}

pub fn hydrate_matricule_display(row: &mut Map<String, Value>, ent: &EntityDef) {
    for attr in ent.attributs.iter().filter(|a| is_matricule_attr(a)) {
        let display = format_matricule_from_row(row, attr);
        if !display.is_empty() {
            row.insert(attr.nom.clone(), Value::String(display));
        }
    }
}

fn next_numero(
    db: &Database,
    table: &str,
    col_date: &str,
    col_numero: &str,
    today: &str,
) -> Result<i64, String> {
    let sql = format!(
        "SELECT COALESCE(MAX({col_numero}), 0) FROM {table} WHERE {col_date} = ?1"
    );
    let max: i64 = db
        .conn
        .query_row(&sql, rusqlite::params![today], |r| r.get(0))
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or(0);
    Ok(max + 1)
}

fn next_matricule_numero(
    db: &Database,
    table: &str,
    attr: &EntityAttribute,
    base: &str,
    today: &str,
) -> Result<i64, String> {
    let col_base = column_base(attr);
    let col_date = column_jjmmaaaa(attr);
    let col_numero = column_numero(attr);
    let sql = format!(
        "SELECT COALESCE(MAX({col_numero}), 0) FROM {table} WHERE {col_base} = ?1 AND {col_date} = ?2"
    );
    let max: i64 = db
        .conn
        .query_row(&sql, rusqlite::params![base, today], |r| r.get(0))
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or(0);
    Ok(max + 1)
}

fn assert_matricule_unique(
    db: &Database,
    table: &str,
    attr: &EntityAttribute,
    base: &str,
    date: &str,
    numero: i64,
) -> Result<(), String> {
    let col_base = column_base(attr);
    let col_date = column_jjmmaaaa(attr);
    let col_numero = column_numero(attr);
    let sql = format!(
        "SELECT 1 FROM {table} WHERE {col_base} = ?1 AND {col_date} = ?2 AND {col_numero} = ?3 LIMIT 1"
    );
    let exists: Option<i32> = db
        .conn
        .query_row(&sql, rusqlite::params![base, date, numero], |r| r.get(0))
        .optional()
        .map_err(|e| e.to_string())?;
    if exists.is_some() {
        return Err(format!(
            "Matricule déjà utilisé : {}{date}{numero:02}",
            base.trim()
        ));
    }
    Ok(())
}

/// Remplit les champs compteur / matricule dans `data` avant INSERT (création uniquement).
pub fn apply_compteurs_on_create(
    db: &Database,
    registry: &EntityRegistry,
    entity_key: &str,
    data: &mut Map<String, Value>,
) -> Result<(), String> {
    let Some(ent) = registry.find(entity_key) else {
        return Ok(());
    };
    let matricules = matricule_registry::load(&db.data_dir)?;
    let table = table_name(entity_key);
    let today = today_jjmmaaaa();

    for attr in ent.attributs.iter().filter(|a| is_compteur_attr(a)) {
        if is_matricule_attr(attr) {
            let def = matricule_def_for_attr(&matricules, attr)?;
            let base = def.base.clone();
            let libelle = def.libelle.clone();
            let numero = next_matricule_numero(db, &table, attr, &base, &today)?;
            assert_matricule_unique(db, &table, attr, &base, &today, numero)?;

            data.insert(column_base(attr).into(), Value::String(base.clone()));
            data.insert(column_libelle(attr).into(), Value::String(libelle));
            data.insert(column_jjmmaaaa(attr).into(), Value::String(today.clone()));
            data.insert(
                column_numero(attr).into(),
                Value::Number(serde_json::Number::from(numero)),
            );
            data.insert(
                attr.nom.clone(),
                Value::String(format_full_matricule(&base, &today, numero)),
            );
        } else {
            let libelle = libelle_value(attr, ent);
            let numero = next_numero(
                db,
                &table,
                &column_jjmmaaaa(attr),
                &column_numero(attr),
                &today,
            )?;

            data.insert(column_libelle(attr).into(), Value::String(libelle));
            data.insert(column_jjmmaaaa(attr).into(), Value::String(today.clone()));
            data.insert(
                column_numero(attr).into(),
                Value::Number(serde_json::Number::from(numero)),
            );
        }
    }
    Ok(())
}

/// Aperçu des champs auto pour le formulaire de création — avant enregistrement.
pub fn preview_compteurs_on_create(
    db: &Database,
    registry: &EntityRegistry,
    entity_key: &str,
) -> Result<Map<String, Value>, String> {
    let mut data = Map::new();
    let Some(ent) = registry.find(entity_key) else {
        return Ok(data);
    };
    let matricules = matricule_registry::load(&db.data_dir)?;
    let table = table_name(entity_key);
    let today = today_jjmmaaaa();

    for attr in ent.attributs.iter().filter(|a| is_compteur_attr(a)) {
        if is_matricule_attr(attr) {
            let Ok(def) = matricule_def_for_attr(&matricules, attr) else {
                continue;
            };
            let base = def.base.clone();
            let libelle = def.libelle.clone();
            let numero = next_matricule_numero(db, &table, attr, &base, &today)?;
            data.insert(column_base(attr).into(), Value::String(base.clone()));
            data.insert(column_libelle(attr).into(), Value::String(libelle));
            data.insert(column_jjmmaaaa(attr).into(), Value::String(today.clone()));
            data.insert(
                column_numero(attr).into(),
                Value::Number(serde_json::Number::from(numero)),
            );
            data.insert(
                attr.nom.clone(),
                Value::String(format_full_matricule(&base, &today, numero)),
            );
        } else {
            data.insert(
                column_libelle(attr).into(),
                Value::String(libelle_value(attr, ent)),
            );
            let numero = next_numero(
                db,
                &table,
                &column_jjmmaaaa(attr),
                &column_numero(attr),
                &today,
            )?;
            data.insert(column_jjmmaaaa(attr).into(), Value::String(today.clone()));
            data.insert(
                column_numero(attr).into(),
                Value::Number(serde_json::Number::from(numero)),
            );
        }
    }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn column_names_from_attr() {
        let attr = EntityAttribute {
            nom: "ref_doc".into(),
            attr_type: COMPTEUR_ATTR_TYPE.into(),
            label: Some("Référence".into()),
            required: false,
            r#ref: None,
            relation_multiple: false,
            relation_exclusive_parent: true,
            default: None,
            enum_options: None,
            ..Default::default()
        };
        assert_eq!(column_libelle(&attr), "ref_doc_libelle");
        assert_eq!(column_jjmmaaaa(&attr), "ref_doc_jjmmaaaa");
        assert_eq!(column_numero(&attr), "ref_doc_numero");
    }

    #[test]
    fn format_full_matricule_example() {
        assert_eq!(format_full_matricule("MAT", "12032026", 1), "MAT1203202601");
    }
}
