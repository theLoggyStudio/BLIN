//! Rapports HTML imprimables (fiches bien, contrat) — utilisés par Loggy.

use crate::db::{BienRow, ContratRow, FinanceRow};

fn mois_label_fr(mois: u32) -> &'static str {
    match mois {
        1 => "janvier",
        2 => "février",
        3 => "mars",
        4 => "avril",
        5 => "mai",
        6 => "juin",
        7 => "juillet",
        8 => "août",
        9 => "septembre",
        10 => "octobre",
        11 => "novembre",
        12 => "décembre",
        _ => "mois",
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn build_bien_report_html(bien: &BienRow, contrats: &[ContratRow], app_name: &str) -> String {
    let now = {
        let dt = chrono::Local::now().naive_local();
        crate::date_format::format_naive_datetime(dt)
    };
    let prix = bien
        .prix_defaut
        .map(|p| format!("{p:.2} {}", bien.devise))
        .unwrap_or_else(|| "—".into());
    let surface = bien
        .surface_m2
        .map(|s| format!("{s} m²"))
        .unwrap_or_else(|| "—".into());

    let mut contrat_rows = String::new();
    if contrats.is_empty() {
        contrat_rows.push_str("<tr><td colspan=\"5\">Aucun contrat lié</td></tr>");
    } else {
        for c in contrats {
            contrat_rows.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{:.2} {}</td><td>{}</td></tr>",
                esc(&c.reference),
                esc(&c.locataire),
                esc(&c.statut),
                c.loyer_mensuel,
                esc(&c.devise),
                esc(&crate::date_format::format_iso_date_str(&c.date_debut)),
            ));
        }
    }

    let nom_block = if bien.nomenclature.is_empty() {
        String::new()
    } else {
        let mut s = String::from("<h2>Nomenclature</h2><ul>");
        for et in &bien.nomenclature {
            s.push_str(&format!("<li><strong>{}</strong><ul>", esc(&et.libelle)));
            for ch in &et.chambres {
                s.push_str(&format!(
                    "<li>{} — {}</li>",
                    esc(&ch.code),
                    esc(&ch.nom)
                ));
            }
            s.push_str("</ul></li>");
        }
        s.push_str("</ul>");
        s
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="fr">
<head>
<meta charset="utf-8"/>
<title>Fiche bien — {ref}</title>
<style>
body {{ font-family: "Segoe UI", system-ui, sans-serif; margin: 24px; color: #171717; max-width: 800px; }}
header {{ display: flex; justify-content: space-between; padding: 14px 18px; margin-bottom: 20px;
  border-radius: 8px; background: linear-gradient(135deg, #dc2626, #2563eb 55%, #06b6d4); color: #fff; }}
h1 {{ margin: 0 0 8px; font-size: 22px; border-bottom: 3px solid #06b6d4; padding-bottom: 8px; }}
.meta {{ color: #525252; font-size: 13px; margin-bottom: 20px; }}
table {{ width: 100%; border-collapse: collapse; font-size: 12px; margin-top: 8px; }}
th, td {{ border: 1px solid #d4d4d4; padding: 8px; text-align: left; }}
th {{ background: #f5f5f5; }}
.kv {{ display: grid; grid-template-columns: 1fr 1fr; gap: 12px; margin: 16px 0; }}
.kv div {{ padding: 10px; background: #fafafa; border-radius: 6px; border: 1px solid #e5e5e5; }}
.kv dt {{ font-size: 11px; color: #737373; }}
.kv dd {{ margin: 4px 0 0; font-weight: 600; }}
@media print {{ body {{ margin: 12px; }} }}
</style>
</head>
<body>
<header>
  <div><strong>{app_name}</strong><br/><span style="font-size:11px;opacity:.9">Fiche bien</span></div>
  <div style="text-align:right;font-size:11px">{date}</div>
</header>
<h1>{ref}</h1>
<p class="meta">{adresse}</p>
<dl class="kv">
  <div><dt>Type</dt><dd>{type_bien}</dd></div>
  <div><dt>Statut</dt><dd>{statut}</dd></div>
  <div><dt>Domaine</dt><dd>{domaine}</dd></div>
  <div><dt>Surface</dt><dd>{surface}</dd></div>
  <div><dt>Prix par défaut</dt><dd>{prix}</dd></div>
  <div><dt>Devise</dt><dd>{devise}</dd></div>
</dl>
{nom_block}
<h2>Contrats associés</h2>
<table>
<thead><tr><th>Réf.</th><th>Locataire</th><th>Statut</th><th>Loyer</th><th>Début</th></tr></thead>
<tbody>{contrat_rows}</tbody>
</table>
<p style="margin-top:24px;font-size:10px;color:#737373;text-align:center">
  Document généré par {app_name} — ouvrez dans le navigateur puis Ctrl+P pour enregistrer en PDF.
</p>
</body>
</html>"#,
        app_name = esc(app_name),
        ref = esc(&bien.reference),
        adresse = esc(&bien.adresse),
        type_bien = esc(&bien.type_bien),
        statut = esc(&bien.statut),
        domaine = esc(&bien.domaine),
        surface = esc(&surface),
        prix = esc(&prix),
        devise = esc(&bien.devise),
        date = esc(&now),
        nom_block = nom_block,
        contrat_rows = contrat_rows,
    )
}

pub fn build_finances_month_html(
    annee: i32,
    mois: u32,
    rows: &[FinanceRow],
    app_name: &str,
) -> String {
    let mois_label = mois_label_fr(mois);
    let mut body = String::new();
    let mut total = 0.0f64;
    for f in rows {
        total += f.montant;
        body.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{:.2} {}</td><td>{}</td><td>{}</td></tr>",
            esc(&f.reference),
            esc(&f.libelle),
            f.montant,
            esc(&f.devise),
            esc(&crate::date_format::format_iso_date_str(&f.date_echeance)),
            esc(&f.statut),
        ));
    }
    if body.is_empty() {
        body.push_str("<tr><td colspan=\"5\">Aucune écriture sur cette période</td></tr>");
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="fr"><head><meta charset="utf-8"/>
<title>Loyers {mois_label} {annee}</title>
<style>
body {{ font-family: "Segoe UI", sans-serif; margin: 24px; }}
h1 {{ border-bottom: 3px solid #06b6d4; padding-bottom: 8px; }}
table {{ width: 100%; border-collapse: collapse; font-size: 12px; }}
th, td {{ border: 1px solid #ccc; padding: 8px; }}
th {{ background: #f5f5f5; }}
</style></head><body>
<h1>Loyers — {mois_label} {annee}</h1>
<table><thead><tr><th>Réf.</th><th>Libellé</th><th>Montant</th><th>Échéance</th><th>Statut</th></tr></thead>
<tbody>{body}</tbody></table>
<p><strong>Total : {total:.2}</strong></p>
<p style="font-size:10px;color:#666">Ctrl+P pour PDF — {app_name}</p>
</body></html>"#,
        mois_label = mois_label,
        annee = annee,
        body = body,
        total = total,
        app_name = esc(app_name),
    )
}

pub fn write_export_file(data_dir: &std::path::Path, prefix: &str, html: &str) -> Result<std::path::PathBuf, String> {
    let exports_dir = data_dir.join("exports");
    std::fs::create_dir_all(&exports_dir).map_err(|e| e.to_string())?;
    let stamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let safe_prefix: String = prefix
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let path = exports_dir.join(format!("{safe_prefix}_{stamp}.html"));
    std::fs::write(&path, html).map_err(|e| e.to_string())?;
    Ok(path)
}
