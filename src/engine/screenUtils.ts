import type { FieldDef, ScreenRow } from "@/types/screen";
import { entityRowDisplayLabel } from "@/lib/entityRowLabel";
import { formatDateFr, formatDateTimeFr, formatJjmmaaaaFr, formatTimeFr } from "@/lib/formatDateTime";
import { parseImagesValue } from "./mediaUtils";

function fieldValuesMatch(current: unknown, expected: unknown): boolean {
  if (current === expected) return true;
  const truthy = (v: unknown) =>
    v === true || v === 1 || v === "1" || v === "true";
  if (truthy(current) && truthy(expected)) return true;
  return String(current ?? "") === String(expected ?? "");
}

export function isFieldVisible(field: FieldDef, values: ScreenRow): boolean {
  if (!field.visibleWhen) return true;
  const current = values[field.visibleWhen.field];
  return fieldValuesMatch(current, field.visibleWhen.equals);
}

export function rowLabel(row: ScreenRow, labelField: string): string {
  const v = row[labelField];
  if (v != null && String(v).trim()) {
    const s = String(v).trim();
    if (s.startsWith("[") || s.startsWith("{")) {
      const fromEmbed = entityRowDisplayLabel(row);
      if (fromEmbed !== "—") return fromEmbed;
    }
    return s;
  }
  return entityRowDisplayLabel(row);
}

export function formatCellValue(field: FieldDef | undefined, value: unknown): string {
  if (field?.type === "image") {
    return value && String(value).trim() ? "Photo" : "—";
  }
  if (field?.type === "images") {
    const n = parseImagesValue(value).length;
    return n > 0 ? `${n} photo${n > 1 ? "s" : ""}` : "—";
  }
  if (value == null || value === "") return "—";
  if (field?.key.endsWith("_jjmmaaaa")) return formatJjmmaaaaFr(value);
  if (field?.type === "date") return formatDateFr(value);
  if (field?.type === "time") return formatTimeFr(value);
  if (field?.type === "datetime") return formatDateTimeFr(value);
  if (typeof value === "number") return String(value);
  const asText = String(value);
  if (/^\d{4}-\d{2}-\d{2}/.test(asText) || /T\d{2}:\d{2}/.test(asText)) {
    return formatDateTimeFr(asText);
  }
  return asText;
}
