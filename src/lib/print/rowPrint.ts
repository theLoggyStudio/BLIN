import { invoke } from "@tauri-apps/api/core";
import type { PrintRowRenderResult } from "@/types/print";
import { exportHtmlToPdf } from "@/lib/print/pdfExport";

export async function printEntityRowPdf(
  screenKey: string,
  recordId: string,
): Promise<void> {
  const doc = await invoke<PrintRowRenderResult>("print_row_render", {
    payload: { screen_key: screenKey, record_id: recordId },
  });
  await exportHtmlToPdf(doc.html, doc.css, doc.file_name);
}
