import type { ChatColsRequest, ChatDisplayBlock } from "@/types/ai";
import { formatDateTimeFr } from "@/lib/formatDateTime";

const DISPLAY_START = "__BLIN_DISPLAY__\n";
const DISPLAY_END = "\n__END_BLIN_DISPLAY__";
const COLS_START = "__BLIN_ASK_COLS__\n";
const COLS_END = "\n__END_BLIN_ASK_COLS__";

function stripMarkers(content: string): string {
  let text = content;
  for (const [start, end] of [
    [DISPLAY_START, DISPLAY_END],
    [COLS_START, COLS_END],
  ] as const) {
    const s = text.indexOf(start);
    if (s >= 0) {
      const e = text.indexOf(end, s);
      text = text.slice(0, s).trimEnd() + (e >= 0 ? text.slice(e + end.length) : "");
    }
  }
  return text.trim();
}

export function parseChatDisplayBlocks(content: string): ChatDisplayBlock[] {
  const start = content.indexOf(DISPLAY_START);
  if (start < 0) return [];
  const end = content.indexOf(DISPLAY_END, start);
  const json = content.slice(start + DISPLAY_START.length, end >= 0 ? end : undefined).trim();
  try {
    const payload = JSON.parse(json) as { blocks?: ChatDisplayBlock[] };
    return payload.blocks ?? [];
  } catch {
    return [];
  }
}

export function parseChatColsRequest(content: string): ChatColsRequest | undefined {
  const start = content.indexOf(COLS_START);
  if (start < 0) return undefined;
  const end = content.indexOf(COLS_END, start);
  const json = content.slice(start + COLS_START.length, end >= 0 ? end : undefined).trim();
  try {
    const payload = JSON.parse(json) as {
      entityKey?: string;
      entity_key?: string;
      entityLabel?: string;
      entity_label?: string;
      available?: { key: string; label: string }[];
      filters?: Record<string, string>;
    };
    const entityKey = payload.entityKey ?? payload.entity_key;
    if (!entityKey || !payload.available?.length) return undefined;
    return {
      entityKey,
      entityLabel: payload.entityLabel ?? payload.entity_label ?? entityKey,
      available: payload.available,
      filters: payload.filters ?? {},
    };
  } catch {
    return undefined;
  }
}

export function hasLoggyAttachments(parsed: {
  displayBlocks?: ChatDisplayBlock[];
  colsRequest?: ChatColsRequest;
}): boolean {
  return (parsed.displayBlocks?.length ?? 0) > 0 || !!parsed.colsRequest;
}

export function parseAssistantChatContent(content: string): {
  text: string;
  displayBlocks: ChatDisplayBlock[];
  colsRequest?: ChatColsRequest;
} {
  const displayBlocks = parseChatDisplayBlocks(content);
  const colsRequest = parseChatColsRequest(content);
  let text = stripMarkers(content);
  text = stripJsonFences(text);
  text = stripToolCallLines(text);
  return {
    text: text.trim(),
    displayBlocks: displayBlocks.length > 0 ? displayBlocks : [],
    colsRequest,
  };
}

function stripJsonFences(text: string): string {
  let out = text;
  while (out.includes("```json")) {
    const start = out.indexOf("```json");
    const after = start + 7;
    const end = out.indexOf("```", after);
    if (end < 0) break;
    out = out.slice(0, start).trimEnd() + out.slice(end + 3);
  }
  return out.trim();
}

function stripToolCallLines(text: string): string {
  return text
    .split("\n")
    .filter((line) => {
      const t = line.trim();
      return !(t.startsWith("{") && t.includes('"tool"') && t.endsWith("}"));
    })
    .join("\n")
    .trim();
}

export function formatCellValue(value: unknown): string {
  if (value == null || value === "") return "—";
  if (typeof value === "boolean") return value ? "Oui" : "Non";
  if (Array.isArray(value)) return value.map(String).join(", ");
  if (typeof value === "object") return JSON.stringify(value);
  const asText = String(value);
  if (/^\d{4}-\d{2}-\d{2}/.test(asText) || /T\d{2}:\d{2}/.test(asText)) {
    return formatDateTimeFr(asText);
  }
  return asText;
}
