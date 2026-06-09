import html2canvas from "html2canvas";
import { jsPDF } from "jspdf";
import { DEFAULT_FICHE_CSS } from "@/lib/print/defaultCss";
import {
  LANDSCAPE_COLUMN_THRESHOLD,
  PDF_LIST_LAYOUT_CSS,
  PDF_MARGIN_MM,
  pageDimensions,
} from "@/lib/print/pdfLayout";

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
  const header = parts.continued ? continued : parts.headerHtml;

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

  const imgW = dims.contentWmm;
  const imgH = (canvas.height * imgW) / canvas.width;

  if (!isFirstPage) {
    pdf.addPage(dims.landscape ? "a4" : "a4", dims.landscape ? "landscape" : "portrait");
  }

  pdf.addImage(
    canvas.toDataURL("image/jpeg", 0.92),
    "JPEG",
    PDF_MARGIN_MM,
    PDF_MARGIN_MM,
    imgW,
    Math.min(imgH, dims.contentHmm),
  );
}

function paginateRows(
  dims: ReturnType<typeof pageDimensions>,
  rowHtmls: string[],
  firstPageFixedH: number,
  nextPageFixedH: number,
  rowH: number,
): string[][] {
  if (rowHtmls.length === 0) return [[]];

  const firstCapacity = Math.max(1, Math.floor((dims.contentHpx - firstPageFixedH) / rowH));
  const nextCapacity = Math.max(1, Math.floor((dims.contentHpx - nextPageFixedH) / rowH));

  const pages: string[][] = [];
  let index = 0;
  pages.push(rowHtmls.slice(index, index + firstCapacity));
  index += firstCapacity;

  while (index < rowHtmls.length) {
    pages.push(rowHtmls.slice(index, index + nextCapacity));
    index += nextCapacity;
  }

  return pages;
}

async function exportListPdf(html: string, css: string, fileName: string): Promise<void> {
  const parsed = parseListDocument(html);
  if (!parsed) {
    await exportSimplePdf(html, css, fileName);
    return;
  }

  const landscape = parsed.colCount >= LANDSCAPE_COLUMN_THRESHOLD;
  const dims = pageDimensions(landscape);
  const mergedCss = `${DEFAULT_FICHE_CSS}\n${css}\n${PRINT_ISOLATION_CSS}\n${PDF_LIST_LAYOUT_CSS}`;

  const host = createRenderHost(dims.contentWpx, mergedCss);

  try {
    const sampleRow = parsed.rowHtmls[0] ?? "<tr><td>—</td></tr>";
    const firstEmpty = buildListPageHtml(dims, {
      ...parsed,
      rowHtmls: [],
      continued: false,
    });
    const firstOneRow = buildListPageHtml(dims, {
      ...parsed,
      rowHtmls: [sampleRow],
      continued: false,
    });
    const nextEmpty = buildListPageHtml(dims, {
      ...parsed,
      rowHtmls: [],
      continued: true,
    });
    const nextOneRow = buildListPageHtml(dims, {
      ...parsed,
      rowHtmls: [sampleRow],
      continued: true,
    });

    const firstPageFixedH = measureBlockHeight(host, firstEmpty, dims.contentHpx);
    const firstPageOneRowH = measureBlockHeight(host, firstOneRow, dims.contentHpx);
    const nextPageFixedH = measureBlockHeight(host, nextEmpty, dims.contentHpx);
    const nextPageOneRowH = measureBlockHeight(host, nextOneRow, dims.contentHpx);

    const rowH = Math.max(
      16,
      firstPageOneRowH - firstPageFixedH,
      nextPageOneRowH - nextPageFixedH,
    );

    const rowPages = paginateRows(
      dims,
      parsed.rowHtmls,
      firstPageFixedH,
      nextPageFixedH,
      rowH,
    );

    const pdf = new jsPDF({
      orientation: dims.landscape ? "landscape" : "portrait",
      unit: "mm",
      format: "a4",
    });

    for (let i = 0; i < rowPages.length; i++) {
      const pageHtml = buildListPageHtml(dims, {
        ...parsed,
        rowHtmls: rowPages[i],
        continued: i > 0,
      });
      await renderPageToPdf(pdf, host, pageHtml, dims, i === 0);
    }

    pdf.save(fileName.endsWith(".pdf") ? fileName : `${fileName}.pdf`);
  } finally {
    document.body.removeChild(host);
  }
}

async function exportSimplePdf(html: string, css: string, fileName: string): Promise<void> {
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
    let offset = 0;
    let page = 0;

    while (offset < canvas.height) {
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
    }

    pdf.save(fileName.endsWith(".pdf") ? fileName : `${fileName}.pdf`);
  } finally {
    document.body.removeChild(host);
  }
}

/**
 * Exporte un document HTML/CSS en PDF.
 * Listes : pagination avec pied de page par page, paysage auto si ≥ 6 colonnes.
 */
export async function exportHtmlToPdf(
  html: string,
  css: string,
  fileName: string,
): Promise<void> {
  if (isListDocument(html)) {
    await exportListPdf(html, css, fileName);
    return;
  }
  await exportSimplePdf(html, css, fileName);
}
