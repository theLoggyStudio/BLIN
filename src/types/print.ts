export interface PrintModelRow {
  id: string;
  name: string;
  description: string;
  screen_key: string | null;
  created_at: string;
  updated_at: string;
}

export interface PrintModelDetail extends PrintModelRow {
  html_content: string;
  css_content: string;
}

export interface PrintRowRenderResult {
  html: string;
  css: string;
  file_name: string;
  model_name: string;
}

export interface PrintTemplateDefaults {
  html: string;
  css: string;
}

export interface PrintListRenderPayload {
  screen_key: string;
  visible_columns: string[];
  filters: Record<string, string>;
  date_field?: string | null;
  date_from?: string | null;
  date_to?: string | null;
  entity_source_filter?: string | null;
  titre?: string | null;
  sous_titre?: string | null;
}
