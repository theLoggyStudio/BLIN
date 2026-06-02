import html2canvas from "html2canvas";
import { jsPDF } from "jspdf";

const A4_W_MM = 210;
const A4_H_MM = 297;

/**
 * Exporte un document HTML/CSS en PDF (logique inspirée de Loma).
 */
export async function exportHtmlToPdf(
  html: string,
  css: string,
  fileName: string,
): Promise<void> {
  const host = document.createElement("div");
  host.style.cssText =
    "position:fixed;left:-10000px;top:0;width:794px;background:#fff;z-index:-1;";
  const style = document.createElement("style");
  style.textContent = `${css}
@page { size: A4; margin: 12mm; }
body { margin: 0; background: #fff; }
`;
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
