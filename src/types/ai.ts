export interface AiToolResult {
  tool: string;
  success: boolean;
  message: string;
  data?: unknown;
  requires_confirmation: boolean;
  pending_id?: string;
  confirm_privilege?: string;
}

import type { EntityDef } from "@/types/entity";

export interface EntityCreateAction {
  entity_key: string;
  initial_data: Record<string, unknown>;
}

export interface RegistryEntityCreateAction {
  initial_entity: EntityDef;
}

export interface ChatDisplayColumn {
  key: string;
  label: string;
}

export interface ChatDisplayBlock {
  kind: "table" | "list" | string;
  entityKey?: string;
  columns: ChatDisplayColumn[];
  rows: Record<string, unknown>[];
}

export interface ChatColsRequest {
  entityKey: string;
  entityLabel?: string;
  available: ChatDisplayColumn[];
  filters?: Record<string, string>;
}

export interface AiChatReply {
  conversation_id: string;
  message: string;
  tool_results: AiToolResult[];
  display_blocks?: ChatDisplayBlock[];
  cols_request?: ChatColsRequest;
  open_entity_create?: EntityCreateAction;
  open_registry_entity_create?: RegistryEntityCreateAction;
}

export interface AiStatus {
  llama_bin: boolean;
  model_present: boolean;
  model_name: string;
  model_path: string;
  install_dir?: string | null;
  server_healthy: boolean;
  gpu_enabled: boolean;
  backend: string;
  gpu_layers: number;
  ctx_size: number;
  threads: number;
  profiled: boolean;
  profile_summary: string;
  offline_only: boolean;
  web_search_enabled: boolean;
  experience_entries: number;
  db_dir: string;
  db_path?: string;
  db_paths?: string[];
}

export interface AiWebSearchConfig {
  enabled: boolean;
}

export interface AiVisionConfigPublic {
  configured: boolean;
  provider: string;
  providerLabel: string;
  model: string;
  keyHint?: string | null;
}

export interface AiVisionAnalyzeReply {
  conversation_id: string;
  message: string;
}

/** Indication utilisateur : colonne / attribut et caractère obligatoire. */
export interface VisionAttributeHint {
  nom: string;
  required: boolean;
}

/** Options pour l'entité principale détectée lors d'une analyse vision. */
export interface VisionAnalyzeEntityOptions {
  requires_signature: boolean;
  ai_suggestions: boolean;
  signatory_role_ids: string[];
  /** Colonnes attendues et obligation (vide = laisser Loggy déduire depuis l'image). */
  attribute_hints: VisionAttributeHint[];
}

export interface AiRuntimeStatus {
  ready: boolean;
  configured: boolean;
  install_dir: string | null;
  default_install_dir: string;
  model_path: string;
  llama_bin: boolean;
  model_present: boolean;
}

export interface AiInstallProgress {
  phase: string;
  percent: number;
  message: string;
}

export interface ChatMessage {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  toolResults?: AiToolResult[];
}

export interface AiConversationSummary {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
  message_count: number;
}

export interface AiStoredMessage {
  role: string;
  content: string;
}
