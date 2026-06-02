/** CSS fiche A4 — aligné sur `print_seed.rs` / charte Blin. */
export const DEFAULT_FICHE_CSS = `.page, .fiche {
  font-family: "Segoe UI", system-ui, sans-serif;
  width: 754px;
  box-sizing: border-box;
  margin: 0;
  padding: 16px 18px;
  color: #0a0a0a;
}
.fiche-head {
  padding: 14px 18px;
  margin-bottom: 18px;
  border-radius: 8px;
  background: linear-gradient(135deg, #dc2626 0%, #2563eb 55%, #06b6d4 100%);
  color: #fafafa;
}
.fiche-head h1 { margin: 0; font-size: 20px; font-weight: 700; }
.fiche-meta { margin: 6px 0 0; font-size: 11px; opacity: 0.9; }
.fiche-field { margin: 0 0 10px; font-size: 13px; line-height: 1.45; }
.fiche-label { color: #262626; }
.fiche-value { color: #171717; }
.fiche-foot {
  margin-top: 20px;
  padding-top: 10px;
  border-top: 1px solid #e5e5e5;
  font-size: 10px;
  color: #737373;
  text-align: center;
}
.data-table-wrap { width: 100%; max-width: 100%; }
.data-table {
  width: 100%;
  min-width: 100%;
  table-layout: auto;
  border-collapse: collapse;
  font-size: 11px;
}
.data-table th, .data-table td {
  border: 1px solid #d4d4d4;
  padding: 7px 9px;
  text-align: left;
  word-break: break-word;
}
.data-table th {
  background: linear-gradient(180deg, #f5f5f5 0%, #e5e5e5 100%);
  font-weight: 600;
}
.data-table tr:nth-child(even) td { background: #fafafa; }`;
