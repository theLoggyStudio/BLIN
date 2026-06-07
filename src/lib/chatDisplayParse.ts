import type { ChatDisplayBlock } from "@/types/ai";
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

export function parseAssistantChatContent(content: string): {
  text: string;
  displayBlocks: ChatDisplayBlock[];
} {
  const displayBlocks = parseChatDisplayBlocks(content);
  return {
    text: stripMarkers(content),
    displayBlocks,
  };
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
