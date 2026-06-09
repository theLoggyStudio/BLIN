import { invoke } from "@tauri-apps/api/core";
import type { PrintRowRenderResult } from "@/types/print";
import { exportHtmlToPdf } from "@/lib/print/pdfExport";
import { tableTokenForEntity } from "@/lib/print/templateAttributes";

export interface PrintListRenderOptions {
  screenKey: string;
  visibleColumns: string[];
  filters: Record<string, string>;
  dateField?: string;
  dateFrom?: string;
  dateTo?: string;
  entitySourceFilter?: string;
  titre?: string;
  sousTitre?: string;
}

export async function printEntityListPdf(options: PrintListRenderOptions): Promise<void> {
  const doc = await invoke<PrintRowRenderResult>("print_list_render", {
    payload: {
      screen_key: options.screenKey,
      visible_columns: options.visibleColumns,
      filters: options.filters,
      date_field: options.dateField || null,
      date_from: options.dateFrom || null,
      date_to: options.dateTo || null,
      entity_source_filter: options.entitySourceFilter || null,
      titre: options.titre || null,
      sous_titre: options.sousTitre || null,
    },
  });
  await exportHtmlToPdf(doc.html, doc.css, doc.file_name);
}

export function defaultListPdfTitle(screenLabel: string): string {
  return `Liste — ${screenLabel}`;
}

export function defaultListPdfSubtitle(
  screenKey: string,
  rowCount: number,
  extra?: string,
): string {
  const token = tableTokenForEntity(screenKey);
  const base = `${rowCount} ligne(s) — tableau {{${token}}}`;
  return extra ? `${base} — ${extra}` : base;
}
