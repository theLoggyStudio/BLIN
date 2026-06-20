import { invoke } from "@tauri-apps/api/core";
import type { PrintRowRenderResult } from "@/types/print";
import { exportHtmlToPdf, PdfExportCancelled, type PdfExportOptions } from "@/lib/print/pdfExport";
import { tableTokenForEntity } from "@/lib/print/templateAttributes";
import type { PdfExportProgress } from "@/types/pdfExportProgress";

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
  onProgress?: (progress: PdfExportProgress) => void;
  signal?: AbortSignal;
}

export async function printEntityListPdf(options: PrintListRenderOptions): Promise<void> {
  const { onProgress, signal, ...payload } = options;

  onProgress?.({
    phase: "prepare",
    current: 0,
    total: 1,
    label: "Préparation des données et du modèle…",
    done: false,
  });

  if (signal?.aborted) {
    throw new PdfExportCancelled();
  }

  const doc = await invoke<PrintRowRenderResult>("print_list_render", {
    payload: {
      screen_key: payload.screenKey,
      visible_columns: payload.visibleColumns,
      filters: payload.filters,
      date_field: payload.dateField || null,
      date_from: payload.dateFrom || null,
      date_to: payload.dateTo || null,
      entity_source_filter: payload.entitySourceFilter || null,
      titre: payload.titre || null,
      sous_titre: payload.sousTitre || null,
    },
  });

  onProgress?.({
    phase: "prepare",
    current: 1,
    total: 1,
    label: "Données prêtes — génération du PDF…",
    done: false,
  });

  const exportOpts: PdfExportOptions = { onProgress, signal };
  await exportHtmlToPdf(doc.html, doc.css, doc.file_name, exportOpts);
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
