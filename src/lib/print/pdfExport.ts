import html2canvas from "html2canvas";
import { jsPDF } from "jspdf";
import { DEFAULT_FICHE_CSS } from "@/lib/print/defaultCss";

const A4_W_MM = 210;
const A4_H_MM = 297;

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

/**
 * Exporte un document HTML/CSS en PDF.
 */
export async function exportHtmlToPdf(
  html: string,
  css: string,
  fileName: string,
): Promise<void> {
  const host = document.createElement("div");
  host.style.cssText =
    "position:fixed;left:-10000px;top:0;width:794px;background:#fff;color:#1a1a1a;z-index:-1;";
  const style = document.createElement("style");
  const mergedCss = `${DEFAULT_FICHE_CSS}\n${css}\n${PRINT_ISOLATION_CSS}
@page { size: A4; margin: 12mm; }
body { margin: 0; background: #fff; color: #1a1a1a; }
`;
  style.textContent = mergedCss;
  const body = document.createElement("div");
  body.className = "print-root";
  body.innerHTML = html;
  host.append(style, body);
  document.body.appendChild(host);

  try {
    const canvas = await html2canvas(body, {
      scale: 2,
      useCORS: true,
      backgroundColor: "#ffffff",
      logging: false,
    });
    const pdf = new jsPDF({ orientation: "portrait", unit: "mm", format: "a4" });
    const imgW = A4_W_MM;
    const pageH = A4_H_MM;
    const slicePx = (canvas.width * pageH) / imgW;
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
      const imgData = pageCanvas.toDataURL("image/jpeg", 0.92);
      const hMm = (sliceH * imgW) / canvas.width;
      if (page > 0) pdf.addPage();
      pdf.addImage(imgData, "JPEG", 0, 0, imgW, hMm);
      offset += sliceH;
      page += 1;
    }
    pdf.save(fileName.endsWith(".pdf") ? fileName : `${fileName}.pdf`);
  } finally {
    document.body.removeChild(host);
  }
}
