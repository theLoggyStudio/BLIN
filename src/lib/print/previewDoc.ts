import { DEFAULT_FICHE_CSS } from "@/lib/print/defaultCss";

/** Document iframe pour aperçu modèle HTML/CSS. */
export function buildPrintPreviewSrcDoc(html: string, css: string): string {
  const merged = `${DEFAULT_FICHE_CSS}\n${css}`;
  const isolation = `
html, body { margin: 0; background: #fff; color: #1a1a1a; }
.doc, .page, .fiche, .doc-body, .fiche-body, .fiche-field, .fiche-value, .fiche-label,
.lh-contact, .data-table td { color: #1a1a1a; }
.lh-logo, .lh-icon, .data-table th { color: #ffffff; }
`;
  return `<!DOCTYPE html><html><head><meta charset="utf-8"/><style>${merged}${isolation}</style></head><body>${html}</body></html>`;
}
