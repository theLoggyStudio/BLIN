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
}

export interface AiWebSearchConfig {
  enabled: boolean;
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
