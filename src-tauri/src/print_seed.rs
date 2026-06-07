//! Modèles d'impression — en-tête professionnel (charte Blin, bleu #2563eb).

/// En-tête type courrier : bloc logo + barre + date.
pub const LETTERHEAD_HEADER: &str = r#"<header class="lh-header">
  <div class="lh-logo">{{societe.nom}}</div>
  <div class="lh-header-line"></div>
</header>"#;

/// Pied de page : adresse, contacts, barre bleue.
pub const LETTERHEAD_FOOTER: &str = r#"<footer class="lh-footer">
  <div class="lh-footer-rule"></div>
  <p class="lh-office-title">Coordonnées</p>
  <p class="lh-office">{{societe.slogan}}</p>
  <div class="lh-contacts">
    <span class="lh-contact"><span class="lh-icon">☎</span> Document interne</span>
    <span class="lh-contact"><span class="lh-icon">✉</span> {{societe.nom}}</span>
    <span class="lh-contact"><span class="lh-icon">◉</span> {{date.aujourdhui}}</span>
  </div>
  <div class="lh-bottom-bar"></div>
</footer>"#;

pub const PRINT_HTML: &str = r#"<div class="doc page">
  <header class="lh-header">
    <div class="lh-logo">{{societe.nom}}</div>
    <div class="lh-header-line"></div>
  </header>
  <div class="lh-title-row">
    <h1 class="doc-title">{{titre}}</h1>
    <p class="lh-date">{{date.aujourdhui}}</p>
  </div>
  <p class="doc-sub">{{sousTitre}}</p>
  <main class="doc-body liste data-table-wrap">{{liste.contenu}}</main>
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
</div>"#;

/// CSS unique — fiches, listes et aperçu (contraste garanti sur fond blanc).
pub const CORPORATE_PRINT_CSS: &str = r#"
*, *::before, *::after { box-sizing: border-box; }
.doc, .page, .fiche {
  font-family: "Segoe UI", system-ui, -apple-system, sans-serif;
  width: 754px;
  max-width: 100%;
  margin: 0 auto;
  padding: 28px 32px 0;
  color: #1a1a1a;
  background: #ffffff;
  font-size: 13px;
  line-height: 1.5;
}

/* —— En-tête type courrier —— */
.lh-header {
  display: flex;
  align-items: stretch;
  gap: 0;
  margin-bottom: 28px;
}
.lh-logo {
  flex: 0 0 auto;
  min-width: 140px;
  padding: 14px 20px;
  background: #2563eb;
  color: #ffffff;
  font-size: 15px;
  font-weight: 700;
  letter-spacing: 0.02em;
  display: flex;
  align-items: center;
  justify-content: center;
  text-align: center;
}
.lh-header-line {
  flex: 1;
  align-self: center;
  height: 4px;
  background: #2563eb;
  margin-left: 0;
}

.lh-title-row {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  gap: 16px;
  margin-bottom: 6px;
}
.doc-title, .fiche-title {
  margin: 0;
  font-size: 22px;
  font-weight: 700;
  color: #1a1a1a;
  border-bottom: 3px solid #2563eb;
  padding-bottom: 8px;
  flex: 1;
}
.lh-date {
  margin: 0 0 8px;
  font-size: 12px;
  color: #525252;
  white-space: nowrap;
}
.doc-sub, .fiche-meta {
  margin: 0 0 20px;
  font-size: 12px;
  color: #525252;
}

.doc-body, .fiche-body {
  color: #1a1a1a;
  margin-bottom: 32px;
}

/* —— Champs fiche —— */
.fiche-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px 24px;
}
.fiche-field {
  margin: 0;
  padding: 10px 0;
  border-bottom: 1px solid #e5e5e5;
  font-size: 13px;
  line-height: 1.45;
  color: #1a1a1a;
}
.fiche-field--full { grid-column: 1 / -1; }
.fiche-label {
  display: block;
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: #2563eb;
  margin-bottom: 4px;
}
.fiche-value {
  display: block;
  color: #1a1a1a;
  font-size: 13px;
}

/* —— Tableaux listes —— */
.liste table, .data-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 11px;
  color: #1a1a1a;
}
.liste th, .liste td,
.data-table th, .data-table td {
  border: 1px solid #cbd5e1;
  padding: 8px 10px;
  text-align: left;
  vertical-align: top;
  color: #1a1a1a;
}
.liste th, .data-table th {
  background: #2563eb;
  color: #ffffff;
  font-weight: 600;
  font-size: 11px;
}
.liste tr:nth-child(even) td,
.data-table tr:nth-child(even) td {
  background: #f8fafc;
}
.data-table-wrap { width: 100%; overflow-x: visible; }
.data-table .empty-row,
.empty-table {
  text-align: center;
  color: #64748b;
  font-style: italic;
}

/* —— Pied de page —— */
.lh-footer {
  margin-top: 36px;
  padding-bottom: 0;
}
.lh-footer-rule {
  height: 1px;
  background: #cbd5e1;
  margin-bottom: 16px;
}
.lh-office-title {
  margin: 0 0 4px;
  font-size: 11px;
  font-weight: 700;
  color: #2563eb;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}
.lh-office {
  margin: 0 0 14px;
  font-size: 12px;
  color: #525252;
}
.lh-contacts {
  display: flex;
  flex-wrap: wrap;
  gap: 20px 32px;
  margin-bottom: 20px;
}
.lh-contact {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  font-size: 11px;
  color: #1a1a1a;
}
.lh-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 50%;
  background: #2563eb;
  color: #ffffff;
  font-size: 10px;
  flex-shrink: 0;
}
.lh-bottom-bar {
  height: 12px;
  background: #2563eb;
  margin: 0 -32px;
  width: calc(100% + 64px);
}

.fiche-foot {
  margin-top: 24px;
  font-size: 10px;
  color: #64748b;
  text-align: center;
}

/* Legacy aliases (anciens modèles) */
.brand-header { display: none; }
.foot { display: none; }
.fiche-head { display: none; }
h1.sub { color: #525252; }
"#;

pub const PRINT_CSS: &str = CORPORATE_PRINT_CSS;
pub const FICHE_CSS: &str = CORPORATE_PRINT_CSS;
pub const LIST_PRINT_CSS: &str = CORPORATE_PRINT_CSS;

/// Marqueur pour identifier les modèles auto-générés (mise à jour à la sync).
pub const AUTO_PRINT_DESCRIPTION_PREFIX: &str = "Modèle auto DDA";

pub struct PrintModelSeed<'a> {
    pub screen_key: &'a str,
    pub name: &'a str,
    pub description: &'a str,
}

pub const STOCK_LIST_MODEL: PrintModelSeed<'static> = PrintModelSeed {
    screen_key: "stock",
    name: "Liste Stock",
    description: "Inventaire tabulaire — variable {{stock}}",
};

pub const ALL_SCREEN_MODELS: &[PrintModelSeed<'static>] = &[STOCK_LIST_MODEL];
