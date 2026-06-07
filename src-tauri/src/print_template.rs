//! Modèles HTML/CSS fiche objet unique et listes tabulaires (génération auto + rendu).

use crate::dda::config::{FieldDef, ScreenConfigFile};
use crate::entity::registry::EntityDef;
use crate::print_seed::{AUTO_PRINT_DESCRIPTION_PREFIX, LETTERHEAD_FOOTER, LETTERHEAD_HEADER};
use serde_json::Value;
use std::collections::HashMap;

pub use crate::print_seed::{FICHE_CSS, LIST_PRINT_CSS as LIST_CSS};

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

pub fn auto_print_description(kind: &str, screen_key: &str) -> String {
    format!("{AUTO_PRINT_DESCRIPTION_PREFIX} — {kind} — écran {screen_key}")
}

/// HTML fiche objet unique — en-tête type courrier professionnel.
pub fn build_fiche_html_from_config(cfg: &ScreenConfigFile) -> String {
    let title = escape_html_attr(&cfg.screen.label);
    let key = &cfg.screen.key;
    let mut html = format!(
        r#"<article class="fiche doc">
{header}
  <div class="lh-title-row">
    <h1 class="fiche-title">{title}</h1>
    <p class="lh-date">{{{{date.aujourdhui}}}}</p>
  </div>
  <p class="fiche-meta">Fiche — {key}</p>
  <section class="fiche-body">
    <div class="fiche-grid">"#,
        header = LETTERHEAD_HEADER,
        title = title,
        key = key,
    );
    for field in printable_fields(cfg) {
        let full = field.field_type == "entity_embed_list"
            || field.form.as_ref().and_then(|m| m.col_span).unwrap_or(1) >= 2;
        html.push_str(&format!(
            r#"<div class="fiche-field{full_class}" data-field="{key_attr}">
        <span class="fiche-label">{label}</span>
        <span class="fiche-value">{{{{{screen}.{field_key}}}}}</span>
      </div>"#,
            full_class = if full { " fiche-field--full" } else { "" },
            key_attr = field.key,
            label = escape_html_attr(&field.label),
            screen = key,
            field_key = field.key,
        ));
    }
    html.push_str(
        r#"    </div>
  </section>
"#,
    );
    html.push_str(LETTERHEAD_FOOTER);
    html.push_str("\n</article>");
    html
}

pub fn build_fiche_html_from_entity(ent: &EntityDef) -> String {
    let label = escape_html_attr(ent.label.as_deref().unwrap_or(&ent.nom));
    let mut html = format!(
        r#"<article class="fiche doc">
{header}
  <div class="lh-title-row">
    <h1 class="fiche-title">{label}</h1>
    <p class="lh-date">{{{{date.aujourdhui}}}}</p>
  </div>
  <section class="fiche-body">
    <div class="fiche-grid">"#,
        header = LETTERHEAD_HEADER,
        label = label,
    );
    for attr in &ent.attributs {
        if crate::entity::attr_types::is_reserved_attribute(attr) {
            continue;
        }
        let lbl = escape_html_attr(attr.label.as_deref().unwrap_or(&attr.nom));
        html.push_str(&format!(
            r#"<div class="fiche-field">
        <span class="fiche-label">{lbl}</span>
        <span class="fiche-value">{{{{{table}.{field}}}}}</span>
      </div>"#,
            lbl = lbl,
            table = ent.nom,
            field = attr.nom,
        ));
    }
    html.push_str(
        r#"    </div>
  </section>
"#,
    );
    html.push_str(LETTERHEAD_FOOTER);
    html.push_str("\n</article>");
    html
}

fn printable_fields<'a>(cfg: &'a ScreenConfigFile) -> Vec<&'a FieldDef> {
    cfg.fields
        .iter()
        .filter(|f| {
            f.field_type != "hidden"
                && f.field_type != "detail_link"
                && f.field_type != "entity_embed"
                && f.field_type != "entity_embed_list"
                && f.form.as_ref().and_then(|m| m.embed_parent.as_ref()).is_none()
        })
        .collect()
}

fn escape_html_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn apply_document_placeholders(html: &str, societe_nom: &str, societe_slogan: &str) -> String {
    let mut out = html.to_string();
    let now = chrono::Local::now();
    out = out.replace("{{date.aujourdhui}}", &now.format("%d/%m/%Y").to_string());
    out = out.replace("{{date.heure}}", &now.format("%H:%M").to_string());
    out = out.replace("{{societe.nom}}", &escape_html_attr(societe_nom));
    out = out.replace("{{societe.slogan}}", &escape_html_attr(societe_slogan));
    out
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
        let raw = row.get(&field.key).or_else(|| row.get(&field.column));
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

/// HTML modèle liste — même en-tête professionnel.
pub fn build_list_print_html(cfg: &ScreenConfigFile) -> String {
    let token = table_token_for_entity(&cfg.screen.key);
    let placeholder = format!("{{{{{token}}}}}");
    format!(
        r#"<div class="doc page">
  <header class="lh-header">
    <div class="lh-logo">{{{{societe.nom}}}}</div>
    <div class="lh-header-line"></div>
  </header>
  <div class="lh-title-row">
    <h1 class="doc-title">{{{{titre}}}}</h1>
    <p class="lh-date">{{{{date.aujourdhui}}}}</p>
  </div>
  <p class="doc-sub">{{{{sousTitre}}}}</p>
  <main class="doc-body liste data-table-wrap">{placeholder}</main>
  <footer class="lh-footer">
    <div class="lh-footer-rule"></div>
    <p class="lh-office-title">Coordonnées</p>
    <p class="lh-office">{{{{societe.slogan}}}}</p>
    <div class="lh-contacts">
      <span class="lh-contact"><span class="lh-icon">☎</span> Document interne</span>
      <span class="lh-contact"><span class="lh-icon">✉</span> {{{{societe.nom}}}}</span>
      <span class="lh-contact"><span class="lh-icon">◉</span> {{{{date.heure}}}}</span>
    </div>
    <div class="lh-bottom-bar"></div>
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

pub fn build_stock_list_print_html() -> String {
    r#"<div class="doc page">
  <header class="lh-header">
    <div class="lh-logo">{{societe.nom}}</div>
    <div class="lh-header-line"></div>
  </header>
  <div class="lh-title-row">
    <h1 class="doc-title">{{titre}}</h1>
    <p class="lh-date">{{date.aujourdhui}}</p>
  </div>
  <p class="doc-sub">{{sousTitre}}</p>
  <main class="doc-body liste data-table-wrap">{{stock}}</main>
  <footer class="lh-footer">
    <div class="lh-footer-rule"></div>
    <p class="lh-office-title">Coordonnées</p>
    <p class="lh-office">{{societe.slogan}}</p>
    <div class="lh-contacts">
      <span class="lh-contact"><span class="lh-icon">☎</span> Document interne</span>
      <span class="lh-contact"><span class="lh-icon">✉</span> {{societe.nom}}</span>
      <span class="lh-contact"><span class="lh-icon">◉</span> {{date.heure}}</span>
    </div>
    <div class="lh-bottom-bar"></div>
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
            } else if (s.starts_with('[') || s.starts_with('{')) && s.len() > 80 {
                format!("{} caractères (JSON)", s.len())
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
