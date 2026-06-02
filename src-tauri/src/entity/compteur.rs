//! Attribut type « compteur » : libellé + date du jour (jjmmaaaa) + numéro d'incrémentation auto.

use chrono::Local;
use rusqlite::OptionalExtension;
use serde_json::{Map, Value};

use super::registry::{EntityAttribute, EntityDef, EntityRegistry};
use super::schema::{attr_column, table_name};
use crate::db::Database;

pub const COMPTEUR_ATTR_TYPE: &str = "compteur";

pub fn is_compteur_attr(attr: &EntityAttribute) -> bool {
    attr.attr_type == COMPTEUR_ATTR_TYPE
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

pub fn all_sql_columns(attr: &EntityAttribute) -> [String; 3] {
    [
        column_libelle(attr),
        column_jjmmaaaa(attr),
        column_numero(attr),
    ]
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

/// Remplit les champs compteur dans `data` avant INSERT (création uniquement).
pub fn apply_compteurs_on_create(
    db: &Database,
    registry: &EntityRegistry,
    entity_key: &str,
    data: &mut Map<String, Value>,
) -> Result<(), String> {
    let Some(ent) = registry.find(entity_key) else {
        return Ok(());
    };
    let table = table_name(entity_key);
    let today = today_jjmmaaaa();

    for attr in ent.attributs.iter().filter(|a| is_compteur_attr(a)) {
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
    Ok(())
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
            default: None,
            enum_options: None,
        };
        assert_eq!(column_libelle(&attr), "ref_doc_libelle");
        assert_eq!(column_jjmmaaaa(&attr), "ref_doc_jjmmaaaa");
        assert_eq!(column_numero(&attr), "ref_doc_numero");
    }
}
