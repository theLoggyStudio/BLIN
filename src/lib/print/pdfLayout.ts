/** Dimensions et styles dédiés à l'export PDF (listes paginées). */

export const PDF_MARGIN_MM = 0;
export const LANDSCAPE_COLUMN_THRESHOLD = 6;
export const MM_TO_PX = 96 / 25.4;

export function mmToPx(mm: number): number {
  return mm * MM_TO_PX;
}

export function pageDimensions(landscape: boolean) {
  const pageWmm = landscape ? 297 : 210;
  const pageHmm = landscape ? 210 : 297;
  const contentWmm = pageWmm - 2 * PDF_MARGIN_MM;
  const contentHmm = pageHmm - 2 * PDF_MARGIN_MM;
  return {
    landscape,
    pageWmm,
    pageHmm,
    contentWmm,
    contentHmm,
    contentWpx: mmToPx(contentWmm),
    contentHpx: mmToPx(contentHmm),
  };
}

/** CSS additionnel : pleine largeur, pied de page répété, mode paysage. */
export const PDF_LIST_LAYOUT_CSS = `
.print-root { display: flex; justify-content: center; background: #fff !important; }
.print-pdf-sheet {
  width: 100%;
  max-width: 100%;
  margin: 0 auto;
  padding: 0;
  display: flex;
  flex-direction: column;
  box-sizing: border-box;
  background: #fff;
}
.print-pdf-sheet .doc-body,
.print-pdf-sheet .fiche-body { flex: 1 1 auto; margin-bottom: 8px; }
.print-pdf-sheet .lh-header { flex-shrink: 0; }
.print-pdf-continued {
  margin: 0 0 10px;
  font-size: 11px;
  font-weight: 600;
  color: #525252;
  text-align: center;
}
.lh-footer--page {
  margin-top: auto;
  padding-top: 10px;
  flex-shrink: 0;
}
.lh-footer--page .lh-office-title,
.lh-footer--page .lh-office {
  text-align: center;
}
.lh-footer--page .lh-contacts {
  justify-content: center;
  gap: 12px 20px;
  margin-bottom: 8px;
}
.lh-footer--page .lh-footer-rule { margin-bottom: 10px; }
.lh-bottom-bar {
  margin: 0 !important;
  width: 100% !important;
}
.data-table-wrap { width: 100%; overflow: visible; }
.data-table {
  width: 100%;
  table-layout: fixed;
  word-wrap: break-word;
  overflow-wrap: anywhere;
}
.data-table--wide { font-size: 8px; }
.data-table--wide th,
.data-table--wide td { padding: 4px 5px; line-height: 1.25; }
.data-table--wide th { font-size: 8px; }
.doc--landscape .lh-header { margin-bottom: 16px; }
.doc--landscape .doc-title { font-size: 18px; }
.doc--landscape .doc-sub { margin-bottom: 12px; }
`;
