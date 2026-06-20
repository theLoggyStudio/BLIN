import html2canvas from "html2canvas";
import { jsPDF } from "jspdf";
import { DEFAULT_FICHE_CSS } from "@/lib/print/defaultCss";
import {
  LANDSCAPE_COLUMN_THRESHOLD,
  PDF_LIST_LAYOUT_CSS,
  PDF_MARGIN_MM,
  pageDimensions,
} from "@/lib/print/pdfLayout";
import type { PdfExportProgress } from "@/types/pdfExportProgress";

export class PdfExportCancelled extends Error {
  constructor() {
    super("Export PDF annulé.");
    this.name = "PdfExportCancelled";
  }
}

export interface PdfExportOptions {
  onProgress?: (progress: PdfExportProgress) => void;
  signal?: AbortSignal;
}

function throwIfAborted(signal?: AbortSignal): void {
  if (signal?.aborted) throw new PdfExportCancelled();
}

function reportProgress(
  onProgress: PdfExportOptions["onProgress"],
  progress: PdfExportProgress,
  signal?: AbortSignal,
): void {
  throwIfAborted(signal);
  onProgress?.(progress);
}

/** Isole le rendu PDF du thème sombre de l'application. */
const PRINT_ISOLATION_CSS = `
html, body, .print-root { margin: 0; background: #ffffff !important; color: #1a1a1a !important; }
.print-root .doc, .print-root .page, .print-root .fiche,
.print-root .doc-body, .print-root .fiche-body,
.print-root .fiche-field, .print-root .fiche-value, .print-root .fiche-label,
.print-root .lh-contact, .print-root .data-table td {
  color: #1a1a1a !important;
  background: transparent;
}
.print-root .fiche-label, .print-root .lh-office-title { color: #2563eb !important; }
.print-root .lh-date, .print-root .doc-sub, .print-root .fiche-meta, .print-root .lh-office {
  color: #525252 !important;
}
.print-root .lh-logo, .print-root .lh-icon, .print-root .data-table th {
  color: #ffffff !important;
}
.print-root .lh-logo, .print-root .lh-header-line, .print-root .lh-bottom-bar, .print-root .lh-icon {
  background: #2563eb !important;
}
`;

interface ParsedListDoc {
  headerHtml: string;
  footerHtml: string;
  theadHtml: string;
  rowHtmls: string[];
  colCount: number;
  title: string;
}

function isListDocument(html: string): boolean {
  return html.includes("data-table") && (html.includes('class="doc page"') || html.includes("doc-body"));
}

function parseListDocument(html: string): ParsedListDoc | null {
  const root = document.createElement("div");
  root.innerHTML = html.trim();
  const doc = root.querySelector(".doc");
  if (!doc) return null;

  const footer = doc.querySelector("footer.lh-footer");
  const table = doc.querySelector("table.data-table");
  if (!footer || !table) return null;

  const headerParts: string[] = [];
  for (const child of Array.from(doc.children)) {
    if (child === footer) break;
    if (child.tagName === "MAIN" || child.classList.contains("doc-body")) break;
    headerParts.push(child.outerHTML);
  }

  const theadHtml = table.querySelector("thead")?.outerHTML ?? "";
  const rowHtmls = Array.from(table.querySelectorAll("tbody tr")).map((tr) => tr.outerHTML);
  const colCount = table.querySelectorAll("thead th").length;
  const title =
    doc.querySelector(".doc-title, .fiche-title")?.textContent?.trim() || "Liste";

  let footerHtml = footer.outerHTML;
  if (!footerHtml.includes("lh-footer--page")) {
    footerHtml = footerHtml.replace("lh-footer", "lh-footer lh-footer--page");
  }

  return {
    headerHtml: headerParts.join("\n"),
    footerHtml,
    theadHtml,
    rowHtmls,
    colCount,
    title,
  };
}

function buildListPageHtml(
  dims: ReturnType<typeof pageDimensions>,
  parts: {
    headerHtml: string;
    footerHtml: string;
    theadHtml: string;
    rowHtmls: string[];
    colCount: number;
    title: string;
    continued: boolean;
  },
): string {
  const wide = parts.colCount >= LANDSCAPE_COLUMN_THRESHOLD;
  const tableClass = wide ? "data-table data-table--wide" : "data-table";
  const continued = parts.continued
    ? `<p class="print-pdf-continued">${parts.title} — suite</p>`
    : "";
  // L'en-tête complet (logo + titre) est répété sur CHAQUE page ; la mention
  // « suite » est ajoutée sous l'en-tête à partir de la 2ᵉ page.
  const header = `${parts.headerHtml}${continued}`;

  return `<div class="print-pdf-sheet doc page${dims.landscape ? " doc--landscape" : ""}" style="min-height:${Math.round(dims.contentHpx)}px">
${header}
<main class="doc-body liste data-table-wrap">
<table class="${tableClass}">
${parts.theadHtml}
<tbody>${parts.rowHtmls.join("")}</tbody>
</table>
</main>
${parts.footerHtml}
</div>`;
}

function createRenderHost(contentWpx: number, css: string): HTMLDivElement {
  const host = document.createElement("div");
  host.style.cssText = `position:fixed;left:0;top:0;width:${Math.round(contentWpx)}px;background:#fff;color:#1a1a1a;z-index:-9999;opacity:0;pointer-events:none;overflow:hidden;`;
  const style = document.createElement("style");
  style.textContent = css;
  const body = document.createElement("div");
  body.className = "print-root";
  host.append(style, body);
  document.body.appendChild(host);
  return host;
}

function measureBlockHeight(host: HTMLDivElement, innerHtml: string, minHeightPx: number): number {
  const body = host.querySelector(".print-root") as HTMLDivElement;
  body.innerHTML = innerHtml;
  const sheet = body.firstElementChild as HTMLElement | null;
  if (sheet) sheet.style.minHeight = `${minHeightPx}px`;
  return body.scrollHeight;
}

/** Hauteur naturelle du contenu (sans min-height de page), pour calibrer la pagination. */
function measureNaturalHeight(host: HTMLDivElement, innerHtml: string): number {
  return measureBlockHeight(host, innerHtml, 0);
}

async function renderPageToPdf(
  pdf: jsPDF,
  host: HTMLDivElement,
  pageHtml: string,
  dims: ReturnType<typeof pageDimensions>,
  isFirstPage: boolean,
): Promise<void> {
  const body = host.querySelector(".print-root") as HTMLDivElement;
  body.innerHTML = pageHtml;
  const sheet = body.firstElementChild as HTMLElement;
  if (sheet) sheet.style.minHeight = `${Math.round(dims.contentHpx)}px`;

  const canvas = await html2canvas(body, {
    scale: 2,
    useCORS: true,
    backgroundColor: "#ffffff",
    logging: false,
    width: Math.round(dims.contentWpx),
    windowWidth: Math.round(dims.contentWpx),
  });

  const naturalW = dims.contentWmm;
  const naturalH = (canvas.height * naturalW) / canvas.width;

  // Ajustement PROPORTIONNEL : on ne déforme jamais le contenu. Si la page
  // dépasse la hauteur A4 (cas limite), on réduit largeur ET hauteur du même
  // facteur et on centre horizontalement, plutôt que d'écraser la hauteur.
  let drawW = naturalW;
  let drawH = naturalH;
  if (naturalH > dims.contentHmm) {
    const scale = dims.contentHmm / naturalH;
    drawW = naturalW * scale;
    drawH = dims.contentHmm;
  }
  const offsetX = PDF_MARGIN_MM + (naturalW - drawW) / 2;

  if (!isFirstPage) {
    pdf.addPage(dims.landscape ? "a4" : "a4", dims.landscape ? "landscape" : "portrait");
  }

  pdf.addImage(
    canvas.toDataURL("image/jpeg", 0.92),
    "JPEG",
    offsetX,
    PDF_MARGIN_MM,
    drawW,
    drawH,
  );
}

/**
 * Pagination MESURÉE : on ajoute les lignes une à une et on déclenche un saut de
 * page dès que la hauteur réelle dépasserait l'A4. Aucune estimation, donc aucune
 * page ne déborde → ni réduction d'échelle, ni déformation, ni police rapetissée.
 */
function paginateRowsMeasured(
  host: HTMLDivElement,
  dims: ReturnType<typeof pageDimensions>,
  parsed: ParsedListDoc,
): string[][] {
  if (parsed.rowHtmls.length === 0) return [[]];

  // Petite réserve pour les arrondis / bordures.
  const limit = dims.contentHpx - 6;
  const pages: string[][] = [];
  let current: string[] = [];

  for (const row of parsed.rowHtmls) {
    const trial = [...current, row];
    const html = buildListPageHtml(dims, {
      ...parsed,
      rowHtmls: trial,
      continued: pages.length > 0,
    });
    const height = measureNaturalHeight(host, html);
    if (height > limit && current.length > 0) {
      pages.push(current);
      current = [row];
    } else {
      current = trial;
    }
  }

  if (current.length > 0) pages.push(current);
  return pages.length > 0 ? pages : [[]];
}

async function exportListPdf(
  html: string,
  css: string,
  fileName: string,
  options?: PdfExportOptions,
): Promise<void> {
  const { onProgress, signal } = options ?? {};
  reportProgress(onProgress, {
    phase: "layout",
    current: 0,
    total: 1,
    label: "Analyse du tableau…",
    done: false,
  }, signal);

  const parsed = parseListDocument(html);
  if (!parsed) {
    await exportSimplePdf(html, css, fileName, options);
    return;
  }

  const landscape = parsed.colCount >= LANDSCAPE_COLUMN_THRESHOLD;
  const dims = pageDimensions(landscape);
  const mergedCss = `${DEFAULT_FICHE_CSS}\n${css}\n${PRINT_ISOLATION_CSS}\n${PDF_LIST_LAYOUT_CSS}`;

  const host = createRenderHost(dims.contentWpx, mergedCss);

  try {
    throwIfAborted(signal);
    // Saut de page automatique fondé sur la hauteur réelle de chaque page.
    const rowPages = paginateRowsMeasured(host, dims, parsed);

    const totalPages = rowPages.length;
    const totalRows = parsed.rowHtmls.length;
    let rowsDone = 0;

    reportProgress(onProgress, {
      phase: "pages",
      current: 0,
      total: totalPages,
      label: `Génération de ${totalPages} page(s) PDF…`,
      detail: `${totalRows} ligne(s) à intégrer`,
      done: false,
    }, signal);

    const pdf = new jsPDF({
      orientation: dims.landscape ? "landscape" : "portrait",
      unit: "mm",
      format: "a4",
    });

    for (let i = 0; i < rowPages.length; i++) {
      throwIfAborted(signal);
      const pageHtml = buildListPageHtml(dims, {
        ...parsed,
        rowHtmls: rowPages[i],
        continued: i > 0,
      });
      await renderPageToPdf(pdf, host, pageHtml, dims, i === 0);
      rowsDone += rowPages[i].length;
      reportProgress(onProgress, {
        phase: "pages",
        current: i + 1,
        total: totalPages,
        label: `Page ${i + 1} sur ${totalPages}`,
        detail: `${rowsDone} / ${totalRows} ligne(s) intégrée(s)`,
        done: false,
      }, signal);
    }

    reportProgress(onProgress, {
      phase: "save",
      current: 0,
      total: 1,
      label: "Enregistrement du fichier PDF…",
      detail: `${totalPages} page(s), ${totalRows} ligne(s)`,
      done: false,
    }, signal);

    pdf.save(fileName.endsWith(".pdf") ? fileName : `${fileName}.pdf`);

    reportProgress(onProgress, {
      phase: "save",
      current: 1,
      total: 1,
      label: "PDF enregistré",
      detail: fileName,
      done: true,
    }, signal);
  } finally {
    document.body.removeChild(host);
  }
}

interface ParsedFicheDoc {
  headerHtml: string;
  footerHtml: string;
  leadHtml: string;
  fieldHtmls: string[];
  bodyClass: string;
  gridClass: string;
}

function isFicheDocument(html: string): boolean {
  return html.includes("fiche-body") || html.includes('class="fiche');
}

/**
 * Découpe une fiche objet unique en : en-tête (`.lh-header`) et pied
 * (`.lh-footer`) répétés sur chaque page, blocs d'introduction (titre, méta,
 * signature…) en page 1 uniquement, et champs de la grille à paginer.
 */
function parseFicheDocument(html: string): ParsedFicheDoc | null {
  const root = document.createElement("div");
  root.innerHTML = html.trim();
  const doc = root.querySelector(".fiche, .doc");
  if (!doc) return null;

  const header = doc.querySelector(".lh-header");
  const footer = doc.querySelector(".lh-footer");
  const body = doc.querySelector(".fiche-body");
  const grid = body?.querySelector(".fiche-grid") ?? null;
  // Sans grille de champs, on ne sait pas découper sans risque de perte :
  // on laisse le filet de sécurité (tranche image) gérer ce document.
  if (!body || !grid) return null;

  const headerHtml = header?.outerHTML ?? "";
  let footerHtml = footer?.outerHTML ?? "";
  if (footerHtml && !footerHtml.includes("lh-footer--page")) {
    footerHtml = footerHtml.replace("lh-footer", "lh-footer lh-footer--page");
  }

  const leadParts: string[] = [];
  for (const child of Array.from(doc.children)) {
    if (child === header || child === footer || child === body) continue;
    leadParts.push(child.outerHTML);
  }

  return {
    headerHtml,
    footerHtml,
    leadHtml: leadParts.join("\n"),
    fieldHtmls: Array.from(grid.children).map((c) => c.outerHTML),
    bodyClass: body.getAttribute("class") ?? "fiche-body",
    gridClass: grid.getAttribute("class") ?? "fiche-grid",
  };
}

function buildFichePageHtml(
  dims: ReturnType<typeof pageDimensions>,
  parts: ParsedFicheDoc & { fieldHtmls: string[]; isFirst: boolean },
): string {
  return `<div class="print-pdf-sheet fiche doc" style="min-height:${Math.round(dims.contentHpx)}px">
${parts.headerHtml}
${parts.isFirst ? parts.leadHtml : ""}
<section class="${parts.bodyClass}"><div class="${parts.gridClass}">${parts.fieldHtmls.join("")}</div></section>
${parts.footerHtml}
</div>`;
}

/** Répartit les champs sur des pages A4 (mesure réelle, hauteurs variables). */
function paginateFields(
  host: HTMLDivElement,
  dims: ReturnType<typeof pageDimensions>,
  parsed: ParsedFicheDoc,
): string[][] {
  if (parsed.fieldHtmls.length === 0) return [[]];

  const pages: string[][] = [];
  let current: string[] = [];

  for (const field of parsed.fieldHtmls) {
    const trial = [...current, field];
    const html = buildFichePageHtml(dims, {
      ...parsed,
      fieldHtmls: trial,
      isFirst: pages.length === 0,
    });
    const height = measureBlockHeight(host, html, dims.contentHpx);
    if (height > dims.contentHpx && current.length > 0) {
      pages.push(current);
      current = [field];
    } else {
      current = trial;
    }
  }

  if (current.length > 0) pages.push(current);
  return pages.length > 0 ? pages : [[]];
}

async function exportFichePdf(
  html: string,
  css: string,
  fileName: string,
  options?: PdfExportOptions,
): Promise<void> {
  const parsed = parseFicheDocument(html);
  if (!parsed) {
    await exportSimplePdf(html, css, fileName, options);
    return;
  }

  const dims = pageDimensions(false);
  const mergedCss = `${DEFAULT_FICHE_CSS}\n${css}\n${PRINT_ISOLATION_CSS}\n${PDF_LIST_LAYOUT_CSS}`;
  const host = createRenderHost(dims.contentWpx, mergedCss);

  try {
    const pages = paginateFields(host, dims, parsed);

    const pdf = new jsPDF({ orientation: "portrait", unit: "mm", format: "a4" });

    for (let i = 0; i < pages.length; i++) {
      const pageHtml = buildFichePageHtml(dims, {
        ...parsed,
        fieldHtmls: pages[i],
        isFirst: i === 0,
      });
      await renderPageToPdf(pdf, host, pageHtml, dims, i === 0);
    }

    pdf.save(fileName.endsWith(".pdf") ? fileName : `${fileName}.pdf`);
  } finally {
    document.body.removeChild(host);
  }
}

async function exportSimplePdf(
  html: string,
  css: string,
  fileName: string,
  options?: PdfExportOptions,
): Promise<void> {
  const { onProgress, signal } = options ?? {};
  throwIfAborted(signal);
  reportProgress(onProgress, {
    phase: "pages",
    current: 0,
    total: 1,
    label: "Génération du PDF…",
    done: false,
  }, signal);

  const dims = pageDimensions(false);
  const mergedCss = `${DEFAULT_FICHE_CSS}\n${css}\n${PRINT_ISOLATION_CSS}\n${PDF_LIST_LAYOUT_CSS}`;
  const host = createRenderHost(dims.contentWpx, mergedCss);

  try {
    const body = host.querySelector(".print-root") as HTMLDivElement;
    body.innerHTML = `<div class="print-pdf-sheet doc page" style="width:100%;padding:6px 4px">${html}</div>`;

    const canvas = await html2canvas(body, {
      scale: 2,
      useCORS: true,
      backgroundColor: "#ffffff",
      logging: false,
      width: Math.round(dims.contentWpx),
      windowWidth: Math.round(dims.contentWpx),
    });

    const pdf = new jsPDF({ orientation: "portrait", unit: "mm", format: "a4" });
    const imgW = dims.contentWmm;
    const slicePx = (canvas.width * dims.contentHmm) / imgW;
    const totalPages = Math.max(1, Math.ceil(canvas.height / slicePx));
    let offset = 0;
    let page = 0;

    while (offset < canvas.height) {
      throwIfAborted(signal);
      const sliceH = Math.min(slicePx, canvas.height - offset);
      const pageCanvas = document.createElement("canvas");
      pageCanvas.width = canvas.width;
      pageCanvas.height = sliceH;
      const ctx = pageCanvas.getContext("2d");
      if (!ctx) break;
      ctx.drawImage(canvas, 0, offset, canvas.width, sliceH, 0, 0, canvas.width, sliceH);
      const hMm = (sliceH * imgW) / canvas.width;
      if (page > 0) pdf.addPage();
      pdf.addImage(
        pageCanvas.toDataURL("image/jpeg", 0.92),
        "JPEG",
        PDF_MARGIN_MM,
        PDF_MARGIN_MM,
        imgW,
        hMm,
      );
      offset += sliceH;
      page += 1;
      reportProgress(onProgress, {
        phase: "pages",
        current: page,
        total: totalPages,
        label: `Page ${page} sur ${totalPages}`,
        done: false,
      }, signal);
    }

    reportProgress(onProgress, {
      phase: "save",
      current: 0,
      total: 1,
      label: "Enregistrement du fichier PDF…",
      done: false,
    }, signal);

    pdf.save(fileName.endsWith(".pdf") ? fileName : `${fileName}.pdf`);

    reportProgress(onProgress, {
      phase: "save",
      current: 1,
      total: 1,
      label: "PDF enregistré",
      done: true,
    }, signal);
  } finally {
    document.body.removeChild(host);
  }
}

/**
 * Exporte un document HTML/CSS en PDF A4.
 * En-tête (`.lh-header`) et pied (`.lh-footer`) sont répétés sur chaque page.
 * - Listes : pagination des lignes, paysage auto si ≥ 6 colonnes.
 * - Fiches : pagination des champs de la grille.
 * - Autres documents : découpage en tranches A4 (filet de sécurité).
 */
export async function exportHtmlToPdf(
  html: string,
  css: string,
  fileName: string,
  options?: PdfExportOptions,
): Promise<void> {
  if (isListDocument(html)) {
    await exportListPdf(html, css, fileName, options);
    return;
  }
  if (isFicheDocument(html)) {
    await exportFichePdf(html, css, fileName, options);
    return;
  }
  await exportSimplePdf(html, css, fileName, options);
}
