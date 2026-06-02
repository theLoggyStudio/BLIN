//! Modèles d'impression par défaut — charte Blin (rouge #dc2626, cyan #06b6d4, bleu #2563eb).

pub const PRINT_HTML: &str = r#"<div class="page">
  <header class="brand-header">
    <div>
      <p class="brand-name">Blin</p>
      <p class="brand-tag">Gestion immobilière</p>
    </div>
    <div class="brand-meta">
      <p>{{ date.aujourdhui }}</p>
      <p>{{ date.heure }}</p>
    </div>
  </header>
  <h1>{{ titre }}</h1>
  <p class="sub">{{ sousTitre }}</p>
  <div class="liste">{{ liste.contenu }}</div>
  <footer class="foot">
    <span>{{ societe.nom }}</span> — document généré localement
  </footer>
</div>"#;

pub const PRINT_CSS: &str = r#".page {
  font-family: "Segoe UI", system-ui, sans-serif;
  width: 754px;
  box-sizing: border-box;
  margin: 0;
  padding: 0;
  color: #0a0a0a;
}
.brand-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  padding: 14px 18px;
  margin-bottom: 22px;
  border-radius: 8px;
  background: linear-gradient(135deg, #dc2626 0%, #2563eb 55%, #06b6d4 100%);
  color: #fafafa;
}
.brand-name { margin: 0; font-size: 20px; font-weight: 700; letter-spacing: 0.02em; }
.brand-tag { margin: 4px 0 0; font-size: 11px; opacity: 0.9; }
.brand-meta { text-align: right; font-size: 11px; line-height: 1.5; }
.brand-meta p { margin: 0; }
h1 {
  margin: 0 0 6px;
  font-size: 22px;
  font-weight: 700;
  color: #171717;
  border-bottom: 3px solid #06b6d4;
  padding-bottom: 8px;
}
.sub {
  color: #525252;
  margin: 0 0 18px;
  font-size: 13px;
}
.liste table {
  width: 100%;
  border-collapse: collapse;
  font-size: 11px;
}
.liste th, .liste td {
  border: 1px solid #d4d4d4;
  padding: 7px 9px;
  text-align: left;
  vertical-align: top;
}
.liste th {
  background: linear-gradient(180deg, #f5f5f5 0%, #e5e5e5 100%);
  font-weight: 600;
  color: #262626;
}
.liste tr:nth-child(even) td { background: #fafafa; }
.data-table-wrap {
  width: 100%;
  max-width: 100%;
  overflow-x: visible;
}
.data-table {
  width: 100%;
  min-width: 100%;
  table-layout: auto;
  border-collapse: collapse;
  font-size: 11px;
}
.data-table th,
.data-table td {
  border: 1px solid #d4d4d4;
  padding: 7px 9px;
  text-align: left;
  vertical-align: top;
  word-break: break-word;
}
.data-table th {
  background: linear-gradient(180deg, #f5f5f5 0%, #e5e5e5 100%);
  font-weight: 600;
  color: #262626;
}
.data-table tr:nth-child(even) td { background: #fafafa; }
.data-table .empty-row {
  text-align: center;
  color: #737373;
  font-style: italic;
}
.empty-table { color: #737373; font-size: 12px; }
.foot {
  margin-top: 24px;
  padding-top: 10px;
  border-top: 1px solid #e5e5e5;
  font-size: 10px;
  color: #737373;
  text-align: center;
}"#;

/// CSS listes tabulaires pleine largeur (variable `{{eleves}}`, `{{stock}}`, …).
pub const LIST_PRINT_CSS: &str = PRINT_CSS;

pub struct PrintModelSeed<'a> {
    pub screen_key: &'a str,
    pub name: &'a str,
    pub description: &'a str,
}

/// Modèle liste inventaire stock (injecté aussi via `ensure_list_print_model`).
pub const STOCK_LIST_MODEL: PrintModelSeed<'static> = PrintModelSeed {
    screen_key: "stock",
    name: "Liste Stock",
    description: "Inventaire tabulaire — variable {{stock}}, filtres entité source et colonnes",
};

/// Modèles d'impression par défaut au premier seed global.
pub const ALL_SCREEN_MODELS: &[PrintModelSeed<'static>] = &[STOCK_LIST_MODEL];
