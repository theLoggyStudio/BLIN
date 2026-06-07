use rusqlite::Row;

use super::registry::{EntityAttribute, EntityDef};
use super::schema::{attr_column, table_name};
use crate::dda::config::ScreenConfigFile;
use crate::db::Database;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregateOp {
    Count,
    Sum,
    Avg,
    Max,
    Min,
}

impl AggregateOp {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "count" | "nombre" | "compte" => Some(Self::Count),
            "sum" | "somme" => Some(Self::Sum),
            "avg" | "average" | "moyenne" => Some(Self::Avg),
            "max" | "maximum" => Some(Self::Max),
            "min" | "minimum" => Some(Self::Min),
            _ => None,
        }
    }
}

/// Résout l'agrégat (nouveau format + legacy `metric`).
pub fn resolve_aggregate(
    aggregate: Option<&str>,
    value_field: Option<&str>,
    metric: Option<&str>,
) -> Result<(AggregateOp, Option<String>), String> {
    if let Some(agg) = aggregate.filter(|s| !s.is_empty()) {
        let op = AggregateOp::parse(agg)
            .ok_or_else(|| format!("Agrégat « {agg} » inconnu (count, sum, avg, max, min)."))?;
        let vf = value_field.filter(|s| !s.is_empty()).map(|s| s.to_string());
        if op != AggregateOp::Count && vf.is_none() {
            return Err("Choisissez un champ numérique pour l'ordonnée.".to_string());
        }
        return Ok((op, vf));
    }
    let m = metric.unwrap_or("count").trim();
    if m.eq_ignore_ascii_case("count") {
        return Ok((AggregateOp::Count, None));
    }
    Ok((AggregateOp::Sum, Some(m.to_string())))
}

fn find_attr<'a>(ent: &'a EntityDef, key: &str) -> Result<&'a EntityAttribute, String> {
    ent.attributs
        .iter()
        .find(|a| a.nom == key || attr_column(a) == key)
        .ok_or_else(|| format!("Attribut « {key} » introuvable."))
}

fn resolve_group_column(cfg: &ScreenConfigFile, ent: &EntityDef, key: &str) -> Result<String, String> {
    if let Some(f) = cfg
        .fields
        .iter()
        .find(|f| f.key == key || f.column == key)
    {
        return Ok(f.column.clone());
    }
    let attr = find_attr(ent, key)?;
    Ok(attr_column(attr))
}

fn is_numeric_field_type(field_type: &str) -> bool {
    matches!(field_type, "number" | "integer" | "float" | "stock")
}

fn is_numeric_attr(attr: &EntityAttribute) -> bool {
    matches!(
        attr.attr_type.as_str(),
        "number" | "integer" | "float" | "stock"
    )
}

pub struct EntityStatRow {
    pub label: String,
    pub value: f64,
}

pub fn query_entity_stats(
    db: &Database,
    cfg: &ScreenConfigFile,
    ent: &EntityDef,
    group_by: &str,
    op: AggregateOp,
    value_field: Option<&str>,
) -> Result<Vec<EntityStatRow>, String> {
    let table = table_name(&ent.nom);
    let group_col = resolve_group_column(cfg, ent, group_by)?;

    let (select_expr, order_sql) = match op {
        AggregateOp::Count => (
            "COUNT(*)".to_string(),
            "ORDER BY val DESC, lbl ASC".to_string(),
        ),
        AggregateOp::Sum | AggregateOp::Avg | AggregateOp::Max | AggregateOp::Min => {
            let vf = value_field.ok_or_else(|| {
                "Champ ordonnée requis pour somme / moyenne / min / max.".to_string()
            })?;
            let val_col = resolve_group_column(cfg, ent, vf)?;
            let numeric = cfg
                .fields
                .iter()
                .find(|f| f.key == vf || f.column == vf)
                .map(|f| is_numeric_field_type(&f.field_type))
                .unwrap_or_else(|| find_attr(ent, vf).map(|a| is_numeric_attr(a)).unwrap_or(false));
            if !numeric {
                return Err(format!(
                    "Le champ « {vf} » n'est pas numérique (ordonnée)."
                ));
            }
            let agg = match op {
                AggregateOp::Sum => format!("SUM(COALESCE({val_col}, 0))"),
                AggregateOp::Avg => format!("AVG(COALESCE({val_col}, 0))"),
                AggregateOp::Max => format!("MAX(COALESCE({val_col}, 0))"),
                AggregateOp::Min => format!("MIN(COALESCE({val_col}, 0))"),
                _ => unreachable!(),
            };
            (agg, "ORDER BY val DESC, lbl ASC".to_string())
        }
    };

    let sql = format!(
        "SELECT CAST({group_col} AS TEXT) AS lbl, {select_expr} AS val FROM {table}
         WHERE {group_col} IS NOT NULL AND CAST({group_col} AS TEXT) != ''
         GROUP BY {group_col}
         {order_sql}
         LIMIT 48"
    );

    let mut stmt = db.conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], map_stat_row)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

fn map_stat_row(row: &Row<'_>) -> rusqlite::Result<EntityStatRow> {
    let label: String = row.get(0)?;
    let value: f64 = row.get(1)?;
    Ok(EntityStatRow {
        label: if label.trim().is_empty() {
            "—".to_string()
        } else {
            label
        },
        value,
    })
}
