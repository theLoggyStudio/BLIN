/** Progression export PDF (liste paginée, fiche, etc.). */
export interface PdfExportProgress {
  phase: "prepare" | "layout" | "pages" | "save";
  current: number;
  total: number;
  label: string;
  /** Détail optionnel (ex. lignes intégrées). */
  detail?: string;
  done: boolean;
}
