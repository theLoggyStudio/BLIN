/** Document iframe pour aperçu modèle HTML/CSS. */
export function buildPrintPreviewSrcDoc(html: string, css: string): string {
  return `<!DOCTYPE html><html><head><meta charset="utf-8"/><style>${css}</style></head><body>${html}</body></html>`;
}
