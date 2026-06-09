/** CSS impression — en-tête professionnel (aligné sur `print_seed.rs`). */
export const DEFAULT_FICHE_CSS = `
*, *::before, *::after { box-sizing: border-box; }
.doc, .page, .fiche {
  font-family: "Segoe UI", system-ui, -apple-system, sans-serif;
  width: 100%;
  max-width: 100%;
  margin: 0 auto;
  padding: 12px 8px 0;
  color: #1a1a1a;
  background: #ffffff;
  font-size: 13px;
  line-height: 1.5;
}
.lh-header { display: flex; align-items: stretch; gap: 0; margin-bottom: 28px; }
.lh-logo {
  flex: 0 0 auto; min-width: 140px; padding: 14px 20px;
  background: #2563eb; color: #ffffff; font-size: 15px; font-weight: 700;
  display: flex; align-items: center; justify-content: center; text-align: center;
}
.lh-header-line { flex: 1; align-self: center; height: 4px; background: #2563eb; }
.lh-title-row {
  display: flex; justify-content: space-between; align-items: flex-end;
  gap: 16px; margin-bottom: 6px;
}
.doc-title, .fiche-title {
  margin: 0; font-size: 22px; font-weight: 700; color: #1a1a1a;
  border-bottom: 3px solid #2563eb; padding-bottom: 8px; flex: 1;
}
.lh-date { margin: 0 0 8px; font-size: 12px; color: #525252; white-space: nowrap; }
.doc-sub, .fiche-meta { margin: 0 0 20px; font-size: 12px; color: #525252; }
.doc-body, .fiche-body { color: #1a1a1a; margin-bottom: 32px; }
.fiche-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px 24px; }
.fiche-field { margin: 0; padding: 10px 0; border-bottom: 1px solid #e5e5e5; color: #1a1a1a; }
.fiche-field--full { grid-column: 1 / -1; }
.fiche-label {
  display: block; font-size: 10px; font-weight: 700; text-transform: uppercase;
  letter-spacing: 0.06em; color: #2563eb; margin-bottom: 4px;
}
.fiche-value { display: block; color: #1a1a1a; font-size: 13px; }
.fiche-value--multiline { white-space: pre-wrap; line-height: 1.55; }
.fiche-objet-concerne {
  margin: 0 0 24px; padding: 16px 18px;
  border: 1px solid #cbd5e1; border-left: 4px solid #2563eb;
  border-radius: 4px; background: #f8fafc;
}
.fiche-objet-label {
  margin: 0 0 10px; font-size: 10px; font-weight: 700;
  text-transform: uppercase; letter-spacing: 0.06em; color: #2563eb;
}
.fiche-objet-value { margin: 0; font-size: 13px; color: #1a1a1a; }
.fiche-signature {
  display: flex; flex-wrap: wrap; align-items: center; gap: 10px 20px;
  margin: 0 0 20px; padding: 12px 16px; border-radius: 4px;
  background: #eff6ff; border: 1px solid #bfdbfe;
}
.fiche-signature-badge {
  display: inline-block; padding: 4px 12px; border-radius: 999px;
  font-size: 11px; font-weight: 700; text-transform: uppercase;
  letter-spacing: 0.04em; background: #2563eb; color: #ffffff;
}
.fiche-signature-meta { font-size: 12px; color: #1e40af; }
.fiche-signature-meta strong { font-weight: 600; }
.data-table {
  width: 100%; border-collapse: collapse; font-size: 11px; color: #1a1a1a;
  table-layout: fixed; word-wrap: break-word; overflow-wrap: anywhere;
}
.data-table th, .data-table td {
  border: 1px solid #cbd5e1; padding: 6px 8px; text-align: left; color: #1a1a1a;
}
.data-table th { background: #2563eb; color: #ffffff; font-weight: 600; }
.data-table tr:nth-child(even) td { background: #f8fafc; }
.data-table--wide { font-size: 8px; }
.data-table--wide th, .data-table--wide td { padding: 4px 5px; line-height: 1.25; }
.lh-footer { margin-top: 24px; }
.lh-footer-rule { height: 1px; background: #cbd5e1; margin-bottom: 16px; }
.lh-office-title {
  margin: 0 0 4px; font-size: 11px; font-weight: 700; color: #2563eb;
  text-transform: uppercase; letter-spacing: 0.05em;
}
.lh-office { margin: 0 0 14px; font-size: 12px; color: #525252; }
.lh-contacts {
  display: flex; flex-wrap: wrap; justify-content: center; gap: 16px 24px; margin-bottom: 12px;
}
.lh-contact { display: inline-flex; align-items: center; gap: 8px; font-size: 11px; color: #1a1a1a; }
.lh-icon {
  display: inline-flex; align-items: center; justify-content: center;
  width: 22px; height: 22px; border-radius: 50%; background: #2563eb; color: #ffffff; font-size: 10px;
}
.lh-bottom-bar { height: 10px; background: #2563eb; margin: 0; width: 100%; }
.lh-footer--page { margin-top: auto; padding-top: 10px; }
.lh-footer--page .lh-office-title, .lh-footer--page .lh-office { text-align: center; }
`;
