import type { FieldDef, ScreenRow } from "@/types/screen";
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
  return v != null ? String(v) : "—";
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
  if (typeof value === "number") return String(value);
  return String(value);
}
