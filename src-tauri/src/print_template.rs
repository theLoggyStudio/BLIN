//! Modèles HTML/CSS fiche objet unique et listes tabulaires (génération auto + rendu).

use crate::dda::config::{FieldDef, ScreenConfigFile};
use crate::entity::registry::EntityDef;
use crate::print_seed::{AUTO_PRINT_DESCRIPTION_PREFIX, LETTERHEAD_FOOTER, LETTERHEAD_HEADER};
use serde_json::Value;
use std::collections::HashMap;

pub use crate::print_seed::{FICHE_CSS, LIST_PRINT_CSS as LIST_CSS};

/// Nom d'attribut tableau dans les modèles : `eleve` → `eleves`, `cour` → `cours`, `stock` → `stock`.
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

const SUMMARY_FIELD_KEYS: &[&str] = &["intitule"];
const SIGNATURE_FIELD_KEYS: &[&str] = &[
    "statut_signature",
    "signe_par",
    "refuse_par",
    "motif_refus",
];

fn config_has_field(cfg: &ScreenConfigFile, key: &str) -> bool {
    cfg.fields.iter().any(|f| f.key == key)
}

fn append_objet_concerne_block(html: &mut String, screen_key: &str) {
    html.push_str(&format!(
        r#"  <section class="fiche-objet-concerne" data-field="intitule">
    <p class="fiche-objet-label">Objet concerné</p>
    <div class="fiche-objet-value fiche-value--multiline">{{{{{screen_key}.intitule}}}}</div>
  </section>
"#
    ));
}

fn append_signature_block(html: &mut String, screen_key: &str) {
    html.push_str(&format!(
        r#"  <section class="fiche-signature" data-section="signature">
    <span class="fiche-signature-badge">{{{{{screen_key}.statut_signature}}}}</span>
    <span class="fiche-signature-meta"><strong>Signé par :</strong> {{{{{screen_key}.signe_par}}}}</span>
    <span class="fiche-signature-meta"><strong>Refusé par :</strong> {{{{{screen_key}.refuse_par}}}}</span>
    <span class="fiche-signature-meta"><strong>Motif :</strong> {{{{{screen_key}.motif_refus}}}}</span>
  </section>
"#
    ));
}

fn skip_in_grid_field(key: &str, cfg: &ScreenConfigFile) -> bool {
    if SUMMARY_FIELD_KEYS.contains(&key) && config_has_field(cfg, "intitule") {
        return true;
    }
    if SIGNATURE_FIELD_KEYS.contains(&key) && config_has_field(cfg, "statut_signature") {
        return true;
    }
    false
}

fn field_value_class(field: &FieldDef) -> &'static str {
    if is_multiline_field(field) {
        " fiche-value--multiline"
    } else {
        ""
    }
}

fn is_multiline_field(field: &FieldDef) -> bool {
    matches!(
        field.key.as_str(),
        "intitule" | "description" | "motif_refus"
    ) || field.field_type == "textarea"
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
"#,
        header = LETTERHEAD_HEADER,
        title = title,
        key = key,
    );
    if config_has_field(cfg, "statut_signature") {
        append_signature_block(&mut html, key);
    }
    if config_has_field(cfg, "intitule") {
        append_objet_concerne_block(&mut html, key);
    }
    html.push_str(
        r#"  <section class="fiche-body">
    <div class="fiche-grid">"#,
    );
    for field in printable_fields(cfg) {
        if skip_in_grid_field(&field.key, cfg) {
            continue;
        }
        let full = field.field_type == "entity_embed_list"
            || is_multiline_field(field)
            || field.form.as_ref().and_then(|m| m.col_span).unwrap_or(1) >= 2;
        let value_class = field_value_class(field);
        html.push_str(&format!(
            r#"<div class="fiche-field{full_class}" data-field="{key_attr}">
        <span class="fiche-label">{label}</span>
        <span class="fiche-value{value_class}">{{{{{screen}.{field_key}}}}}</span>
      </div>"#,
            full_class = if full { " fiche-field--full" } else { "" },
            key_attr = field.key,
            label = escape_html_attr(&field.label),
            screen = key,
            field_key = field.key,
            value_class = value_class,
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
    let table = &ent.nom;
    let has_intitule = ent.attributs.iter().any(|a| a.nom == "intitule");
    let mut html = format!(
        r#"<article class="fiche doc">
{header}
  <div class="lh-title-row">
    <h1 class="fiche-title">{label}</h1>
    <p class="lh-date">{{{{date.aujourdhui}}}}</p>
  </div>
"#,
        header = LETTERHEAD_HEADER,
        label = label,
    );
    if ent.requires_signature {
        append_signature_block(&mut html, table);
    }
    if has_intitule {
        append_objet_concerne_block(&mut html, table);
    }
    html.push_str(
        r#"  <section class="fiche-body">
    <div class="fiche-grid">"#,
    );
    for attr in &ent.attributs {
        if crate::entity::attr_types::is_reserved_attribute(attr) {
            continue;
        }
        if has_intitule && attr.nom == "intitule" {
            continue;
        }
        if ent.requires_signature && SIGNATURE_FIELD_KEYS.contains(&attr.nom.as_str()) {
            continue;
        }
        let lbl = escape_html_attr(attr.label.as_deref().unwrap_or(&attr.nom));
        let multiline = matches!(attr.nom.as_str(), "description" | "motif_refus")
            || attr.attr_type == "textarea";
        let value_class = if multiline {
            " fiche-value--multiline"
        } else {
            ""
        };
        let full = multiline || attr.attr_type == "entity";
        html.push_str(&format!(
            r#"<div class="fiche-field{full_class}">
        <span class="fiche-label">{lbl}</span>
        <span class="fiche-value{value_class}">{{{{{table}.{field}}}}}</span>
      </div>"#,
            full_class = if full { " fiche-field--full" } else { "" },
            lbl = lbl,
            table = table,
            field = attr.nom,
            value_class = value_class,
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
    out = out.replace("{{date.aujourdhui}}", &crate::date_format::format_local_now_date());
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
    out = out.replace("{{date.aujourdhui}}", &crate::date_format::format_local_now_date());
    out = out.replace("{{date.heure}}", &now.format("%H:%M").to_string());

    for field in fields {
        if field.field_type == "hidden" || field.field_type == "detail_link" {
            continue;
        }
        let raw = row.get(&field.key).or_else(|| row.get(&field.column));
        let display = format_field_display(raw, field);
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
    let wide = cols.len() >= 6;
    let table_class = if wide {
        "data-table data-table--wide"
    } else {
        "data-table"
    };
    let mut html = String::from(&format!(r#"<table class="{table_class}"><thead><tr>"#));
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
                let display = format_list_cell_display(raw, f);
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
    out = out.replace("{{date.aujourdhui}}", &crate::date_format::format_local_now_date());
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

fn format_field_display(value: Option<&Value>, field: &FieldDef) -> String {
    let text = value_to_display(value, &field.field_type, &field.key);
    if is_multiline_field(field) {
        escape_html_multiline(&text)
    } else {
        escape_html_attr(&text)
    }
}

fn format_list_cell_display(value: Option<&Value>, field: &FieldDef) -> String {
    let text = value_to_list_display(value, &field.field_type, &field.key);
    let truncated = if field.key == "intitule" && text.len() > 120 {
        format!("{}…", &text[..120])
    } else {
        text
    };
    escape_html_attr(&truncated)
}

fn value_to_list_display(value: Option<&Value>, field_type: &str, field_key: &str) -> String {
    let raw = value_to_display(value, field_type, field_key);
    if field_type == "date" || field_key.ends_with("_jjmmaaaa") {
        return format_iso_date(&raw);
    }
    if field_type == "datetime" || field_key.ends_with("_at") || field_key.contains("date") {
        return format_iso_datetime(&raw);
    }
    if field_type == "number" || field_type == "integer" || field_type == "float" {
        return trim_trailing_zero_decimal(&raw);
    }
    raw
}

fn trim_trailing_zero_decimal(s: &str) -> String {
    if let Some((int_part, frac)) = s.split_once('.') {
        if frac.chars().all(|c| c == '0') && !int_part.is_empty() {
            return int_part.to_string();
        }
    }
    s.to_string()
}

fn format_iso_date(s: &str) -> String {
    crate::date_format::format_iso_date_str(s)
}

fn format_iso_datetime(s: &str) -> String {
    crate::date_format::format_iso_datetime_str(s)
}

fn map_signature_status(raw: &str) -> String {
    match raw.trim() {
        "signe" => "Signé".into(),
        "non_signe" => "Non signé".into(),
        "refuse" => "Refusé".into(),
        other if other.is_empty() => "—".into(),
        other => other.to_string(),
    }
}

fn escape_html_multiline(s: &str) -> String {
    escape_html_attr(s).replace('\n', "<br>")
}

fn value_to_display(value: Option<&Value>, field_type: &str, field_key: &str) -> String {
    let Some(v) = value else {
        return "—".to_string();
    };
    match v {
        Value::Null => "—".to_string(),
        Value::Bool(b) => if *b { "Oui" } else { "Non" }.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => {
            if field_key == "statut_signature" {
                return map_signature_status(s);
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dda::config::{FieldDef, FieldFormMeta, FieldListMeta};

    #[test]
    fn signature_status_maps_to_french() {
        assert_eq!(
            value_to_display(
                Some(&Value::String("signe".into())),
                "select",
                "statut_signature"
            ),
            "Signé"
        );
    }

    #[test]
    fn list_datetime_formats_readable() {
        let out = format_iso_datetime("2026-06-08T10:35:52.007736600+00:00");
        assert!(out.contains("08 juin 2026"));
        assert!(out.contains("10:35") || out.contains("12:35"));
    }

    #[test]
    fn list_number_trims_trailing_zeros() {
        assert_eq!(trim_trailing_zero_decimal("2024.0"), "2024");
    }

    #[test]
    fn multiline_intitule_renders_line_breaks() {
        let field = FieldDef {
            key: "intitule".into(),
            column: "intitule".into(),
            field_type: "string".into(),
            label: "Objet".into(),
            required: false,
            default: None,
            options: vec![],
            list: Some(FieldListMeta {
                enabled: true,
                sortable: false,
            }),
            filter: None,
            form: Some(FieldFormMeta {
                col_span: Some(2),
                placeholder: None,
                min: None,
                step: None,
                read_only: Some(true),
                auto_generated: None,
                storage_folder: None,
                max_files: None,
                accept: None,
                ref_entity: None,
                relation_exclusive_parent: None,
                relation_multiple: None,
                embed_parent: None,
            }),
            visible_when: None,
            validation: None,
        };
        let out = format_field_display(
            Some(&Value::String("Ligne 1\nLigne 2".into())),
            &field,
        );
        assert!(out.contains("Ligne 1<br>Ligne 2"));
    }
}
