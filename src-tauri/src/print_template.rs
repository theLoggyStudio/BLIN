//! Modèles HTML/CSS fiche objet unique et listes tabulaires (génération auto + rendu).

use crate::dda::config::{FieldDef, ScreenConfigFile};
use crate::entity::registry::EntityDef;
use crate::print_seed::{LIST_PRINT_CSS, PRINT_CSS};
use serde_json::Value;
use std::collections::HashMap;

pub const FICHE_CSS: &str = PRINT_CSS;
pub const LIST_CSS: &str = LIST_PRINT_CSS;

/// Nom de variable tableau dans les modèles : `eleve` → `eleves`, `cour` → `cours`, `stock` → `stock`.
pub fn table_token_for_entity(entity_nom: &str) -> String {
    let n = entity_nom.trim();
    if n == "stock" {
        return "stock".into();
    }
    if n.ends_with('s') {
        return n.to_string();
    }
    if n.ends_with('e') {
        return format!("{n}s");
    }
    if n.ends_with("ou") || n.ends_with("u") {
        return format!("{n}s");
    }
    format!("{n}s")
}

/// HTML fiche avec un placeholder `{{nom_attribut}}` par champ métier.
pub fn build_fiche_html_from_config(cfg: &ScreenConfigFile) -> String {
    let title = &cfg.screen.label;
    let key = &cfg.screen.key;
    let mut html = format!(
        r#"<article class="fiche">
  <header class="fiche-head">
    <h1>{title}</h1>
    <p class="fiche-meta">Écran : {key}</p>
  </header>
  <section class="fiche-body">"#
    );
    for field in printable_fields(cfg) {
        html.push_str(&format!(
            r#"<p class="fiche-field" data-field="{}"><strong class="fiche-label">{} :</strong> <span class="fiche-value">{{{{{key}.{field}}}}}</span></p>"#,
            field.key,
            escape_html_attr(&field.label),
            key = key,
            field = field.key
        ));
    }
    html.push_str(
        r#"  </section>
  <footer class="fiche-foot"><span>{{date.aujourdhui}}</span> — {{date.heure}}</footer>
</article>"#,
    );
    html
}

pub fn build_fiche_html_from_entity(ent: &EntityDef) -> String {
    let label = ent.label.as_deref().unwrap_or(&ent.nom);
    let mut html = format!(
        r#"<article class="fiche">
  <header class="fiche-head">
    <h1>{}</h1>
  </header>
  <section class="fiche-body">"#,
        escape_html_attr(label)
    );
    for attr in &ent.attributs {
        if crate::entity::attr_types::is_reserved_attribute(attr) {
            continue;
        }
        let lbl = attr.label.as_deref().unwrap_or(&attr.nom);
        html.push_str(&format!(
            r#"<p class="fiche-field"><strong class="fiche-label">{} :</strong> <span class="fiche-value">{{{{{table}.{field}}}}}</span></p>"#,
            escape_html_attr(lbl),
            table = ent.nom,
            field = attr.nom
        ));
    }
    html.push_str(
        r#"  </section>
  <footer class="fiche-foot"><span>{{date.aujourdhui}}</span> — {{date.heure}}</footer>
</article>"#,
    );
    html
}

fn printable_fields<'a>(cfg: &'a ScreenConfigFile) -> Vec<&'a FieldDef> {
    cfg.fields
        .iter()
        .filter(|f| f.field_type != "hidden" && f.field_type != "detail_link")
        .collect()
}

fn escape_html_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn substitute_row(
    html: &str,
    row: &HashMap<String, Value>,
    fields: &[FieldDef],
    screen_key: &str,
) -> String {
    let mut out = html.to_string();
    let now = chrono::Local::now();
    out = out.replace("{{date.aujourdhui}}", &now.format("%d/%m/%Y").to_string());
    out = out.replace("{{date.heure}}", &now.format("%H:%M").to_string());

    for field in fields {
        if field.field_type == "hidden" || field.field_type == "detail_link" {
            continue;
        }
        let raw = row
            .get(&field.key)
            .or_else(|| row.get(&field.column));
        let display = escape_html_attr(&value_to_display(raw, &field.field_type));
        let token = format!("{{{{{}}}}}", field.key);
        out = out.replace(&token, &display);
        let token_dotted = format!("{{{{{}.{}}}}}", screen_key, field.key);
        out = out.replace(&token_dotted, &display);
        let token_col = format!("{{{{{}}}}}", field.column);
        if token_col != token {
            out = out.replace(&token_col, &display);
        }
        if field.column != field.key {
            let token_col_dotted = format!("{{{{{}.{}}}}}", screen_key, field.column);
            out = out.replace(&token_col_dotted, &display);
        }
    }
    out
}

/// HTML modèle liste : variable tableau `{{eleves}}`, `{{stock}}`, etc.
pub fn build_list_print_html(cfg: &ScreenConfigFile) -> String {
    let token = table_token_for_entity(&cfg.screen.key);
    let placeholder = format!("{{{{{token}}}}}");
    format!(
        r#"<div class="page">
  <header class="brand-header">
    <div>
      <p class="brand-name">{{{{societe.nom}}}}</p>
      <p class="brand-tag">{{{{societe.slogan}}}}</p>
    </div>
    <div class="brand-meta">
      <p>{{{{date.aujourdhui}}}}</p>
      <p>{{{{date.heure}}}}</p>
    </div>
  </header>
  <h1>{{{{titre}}}}</h1>
  <p class="sub">{{{{sousTitre}}}}</p>
  <div class="liste data-table-wrap">{placeholder}</div>
  <footer class="foot">
    <span>{{{{societe.nom}}}}</span> — document généré localement
  </footer>
</div>"#
    )
    .replace("{{{{societe.nom}}}}", "{{societe.nom}}")
    .replace("{{{{societe.slogan}}}}", "{{societe.slogan}}")
    .replace("{{{{date.aujourdhui}}}}", "{{date.aujourdhui}}")
    .replace("{{{{date.heure}}}}", "{{date.heure}}")
    .replace("{{{{titre}}}}", "{{titre}}")
    .replace("{{{{sousTitre}}}}", "{{sousTitre}}")
}

/// Modèle liste dédié stock (filtre entité source documenté dans le sous-titre).
pub fn build_stock_list_print_html() -> String {
    r#"<div class="page">
  <header class="brand-header">
    <div>
      <p class="brand-name">{{societe.nom}}</p>
      <p class="brand-tag">{{societe.slogan}}</p>
    </div>
    <div class="brand-meta">
      <p>{{date.aujourdhui}}</p>
      <p>{{date.heure}}</p>
    </div>
  </header>
  <h1>{{titre}}</h1>
  <p class="sub">{{sousTitre}}</p>
  <div class="liste data-table-wrap">{{stock}}</div>
  <footer class="foot">
    <span>{{societe.nom}}</span> — inventaire — document généré localement
  </footer>
</div>"#
    .to_string()
}

pub fn render_data_table_html(
    rows: &[HashMap<String, Value>],
    fields: &[FieldDef],
    visible_keys: &[String],
) -> String {
    let cols: Vec<&FieldDef> = if visible_keys.is_empty() {
        fields
            .iter()
            .filter(|f| f.field_type != "hidden" && f.field_type != "detail_link")
            .collect()
    } else {
        visible_keys
            .iter()
            .filter_map(|k| fields.iter().find(|f| f.key == *k || f.column == *k))
            .collect()
    };
    if cols.is_empty() {
        return r#"<p class="empty-table">Aucune colonne sélectionnée.</p>"#.to_string();
    }
    let mut html = String::from(r#"<table class="data-table"><thead><tr>"#);
    for f in &cols {
        html.push_str(&format!(
            r#"<th scope="col">{}</th>"#,
            escape_html_attr(&f.label)
        ));
    }
    html.push_str("</tr></thead><tbody>");
    if rows.is_empty() {
        html.push_str(&format!(
            r#"<tr><td colspan="{}" class="empty-row">Aucun enregistrement pour ces critères.</td></tr>"#,
            cols.len()
        ));
    } else {
        for row in rows {
            html.push_str("<tr>");
            for f in &cols {
                let raw = row.get(&f.key).or_else(|| row.get(&f.column));
                let display = escape_html_attr(&value_to_display(raw, &f.field_type));
                html.push_str(&format!(r#"<td>{}</td>"#, display));
            }
            html.push_str("</tr>");
        }
    }
    html.push_str("</tbody></table>");
    html
}

pub fn substitute_list_document(
    html: &str,
    screen_key: &str,
    table_html: &str,
    titre: &str,
    sous_titre: &str,
    societe_nom: &str,
    societe_slogan: &str,
) -> String {
    let mut out = html.to_string();
    let now = chrono::Local::now();
    out = out.replace("{{date.aujourdhui}}", &now.format("%d/%m/%Y").to_string());
    out = out.replace("{{date.heure}}", &now.format("%H:%M").to_string());
    out = out.replace("{{titre}}", &escape_html_attr(titre));
    out = out.replace("{{sousTitre}}", &escape_html_attr(sous_titre));
    out = out.replace("{{societe.nom}}", &escape_html_attr(societe_nom));
    out = out.replace("{{societe.slogan}}", &escape_html_attr(societe_slogan));

    let token = table_token_for_entity(screen_key);
    out = out.replace(&format!("{{{{{token}}}}}",), table_html);
    out = out.replace(&format!("{{{{{screen_key}}}}}",), table_html);
    out = out.replace("{{liste.contenu}}", table_html);
    out = out.replace(&format!("{{{{{screen_key}.contenu}}}}",), table_html);
    out = out.replace(&format!("{{{{{token}.contenu}}}}",), table_html);
    out
}

fn value_to_display(value: Option<&Value>, field_type: &str) -> String {
    let Some(v) = value else {
        return "—".to_string();
    };
    match v {
        Value::Null => "—".to_string(),
        Value::Bool(b) => if *b { "Oui" } else { "Non" }.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => {
            if field_type == "image" && !s.is_empty() {
                "[Image]".to_string()
            } else if s.is_empty() {
                "—".to_string()
            } else {
                s.clone()
            }
        }
        Value::Array(a) => {
            if a.is_empty() {
                "—".to_string()
            } else {
                format!("{} élément(s)", a.len())
            }
        }
        other => other.to_string(),
    }
}
