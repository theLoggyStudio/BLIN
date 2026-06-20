//! Catalogue statistiques DDA — abscisses / ordonnées / agrégats par entité.
//! Généré automatiquement par `trigger_stats` à chaque sync d'écran : alimente
//! le panneau « Statistiques » de chaque écran et la mémoire RAG de Loggy.

use serde::{Deserialize, Serialize};

use super::config::{FieldDef, ScreenConfigFile};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsField {
    pub key: String,
    pub column: String,
    pub label: String,
    pub field_type: String,
    /// Vrai pour date / datetime / time (tri chronologique sur l'axe X).
    pub temporal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsAggregate {
    pub value: String,
    pub label: String,
    pub needs_value_field: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsCatalog {
    pub screen_key: String,
    pub entity_label: String,
    pub abscissa_fields: Vec<StatsField>,
    pub value_fields: Vec<StatsField>,
    pub aggregates: Vec<StatsAggregate>,
    pub default_abscissa: Option<String>,
    pub default_value_field: Option<String>,
}

/// Types d'attribut utilisables en abscisse (group_by). Aligné sur `entityStats.ts`.
const ABSCISSA_TYPES: &[&str] = &[
    "text",
    "select",
    "datetime",
    "date",
    "time",
    "boolean",
    "entity_ref",
    "email",
];

/// Types d'attribut numériques utilisables en ordonnée (sum / avg / max / min).
const NUMERIC_TYPES: &[&str] = &["number", "stock", "compteur", "matricule"];

fn is_excluded(f: &FieldDef) -> bool {
    matches!(
        f.field_type.as_str(),
        "hidden" | "detail_link" | "entity_embed" | "entity_embed_list"
    ) || f
        .form
        .as_ref()
        .and_then(|m| m.embed_parent.as_ref())
        .is_some()
        || f.key == "id"
        || f.key == "created_at"
}

fn is_temporal(field_type: &str) -> bool {
    matches!(field_type, "date" | "datetime" | "time")
}

fn to_field(f: &FieldDef) -> StatsField {
    StatsField {
        key: f.key.clone(),
        column: f.column.clone(),
        label: f.label.clone(),
        field_type: f.field_type.clone(),
        temporal: is_temporal(&f.field_type),
    }
}

pub fn abscissa_fields(cfg: &ScreenConfigFile) -> Vec<StatsField> {
    cfg.fields
        .iter()
        .filter(|f| !is_excluded(f) && ABSCISSA_TYPES.contains(&f.field_type.as_str()))
        .map(to_field)
        .collect()
}

pub fn value_fields(cfg: &ScreenConfigFile) -> Vec<StatsField> {
    cfg.fields
        .iter()
        .filter(|f| !is_excluded(f) && NUMERIC_TYPES.contains(&f.field_type.as_str()))
        .map(to_field)
        .collect()
}

pub fn aggregates() -> Vec<StatsAggregate> {
    vec![
        StatsAggregate {
            value: "count".into(),
            label: "Nombre d'enregistrements".into(),
            needs_value_field: false,
        },
        StatsAggregate {
            value: "sum".into(),
            label: "Somme".into(),
            needs_value_field: true,
        },
        StatsAggregate {
            value: "avg".into(),
            label: "Moyenne".into(),
            needs_value_field: true,
        },
        StatsAggregate {
            value: "max".into(),
            label: "Maximum".into(),
            needs_value_field: true,
        },
        StatsAggregate {
            value: "min".into(),
            label: "Minimum".into(),
            needs_value_field: true,
        },
    ]
}

pub fn build_stats_catalog(cfg: &ScreenConfigFile) -> StatsCatalog {
    let abscissa = abscissa_fields(cfg);
    let values = value_fields(cfg);
    StatsCatalog {
        screen_key: cfg.screen.key.clone(),
        entity_label: cfg.screen.label.clone(),
        default_abscissa: abscissa.first().map(|f| f.key.clone()),
        default_value_field: values.first().map(|f| f.key.clone()),
        abscissa_fields: abscissa,
        value_fields: values,
        aggregates: aggregates(),
    }
}

pub fn format_stats_knowledge(cfg: &ScreenConfigFile, catalog: &StatsCatalog) -> String {
    let mut s = format!(
        "=== STATISTIQUES — {} ({}) ===\n\
         Généré automatiquement par trigger_stats à chaque sync entité.\n\
         Panneau « Statistiques » présent sur chaque écran : abscisse (group_by) + ordonnée (agrégat) + type de graphe.\n\
         Outil : entity_stats {{ entity_key: \"{key}\", group_by, metric: count|sum|avg|max|min, value_field? }}.\n\n",
        cfg.screen.label,
        cfg.screen.key,
        key = cfg.screen.key,
    );
    s.push_str("Abscisses possibles (axe X / group_by) :\n");
    if catalog.abscissa_fields.is_empty() {
        s.push_str("  (aucune — ajoutez un attribut date, enum, booléen, texte ou liaison)\n");
    } else {
        for f in &catalog.abscissa_fields {
            s.push_str(&format!(
                "  - {} (key={}, type={}{})\n",
                f.label,
                f.key,
                f.field_type,
                if f.temporal { ", temporel" } else { "" }
            ));
        }
    }
    s.push_str("Ordonnées numériques (axe Y / sum|avg|max|min) :\n");
    if catalog.value_fields.is_empty() {
        s.push_str("  (aucune — seul l'agrégat « Nombre » est disponible)\n");
    } else {
        for f in &catalog.value_fields {
            s.push_str(&format!(
                "  - {} (key={}, type={})\n",
                f.label, f.key, f.field_type
            ));
        }
    }
    s
}
