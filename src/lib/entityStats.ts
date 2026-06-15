import type { StatChartMultiDatum, StatChartSeriesDef } from "@/items/StatChart";
import type { EntityStatRow } from "@/types/entity";
import type { FieldDef, ScreenConfigFile } from "@/types/screen";

export type StatAggregate = "count" | "sum" | "avg" | "max" | "min";

export const STAT_AGGREGATE_OPTIONS: { value: StatAggregate; label: string }[] = [
  { value: "count", label: "Nombre d'enregistrements" },
  { value: "sum", label: "Somme" },
  { value: "avg", label: "Moyenne" },
  { value: "max", label: "Maximum" },
  { value: "min", label: "Minimum" },
];

export const SERIES_COLORS = [
  "#4DB6AC",
  "#2563eb",
  "#dc2626",
  "#f59e0b",
  "#8b5cf6",
  "#ec4899",
];

const ABSCISSA_TYPES = new Set([
  "text",
  "select",
  "datetime",
  "date",
  "time",
  "boolean",
  "entity_ref",
  "email",
]);
const NUMERIC_TYPES = new Set(["number", "stock", "compteur", "matricule"]);

export function abscissaFields(cfg: ScreenConfigFile): FieldDef[] {
  return cfg.fields.filter(
    (f) =>
      f.type !== "hidden" &&
      f.type !== "detail_link" &&
      f.type !== "entity_embed" &&
      f.type !== "entity_embed_list" &&
      !f.form?.embedParent &&
      f.key !== "id" &&
      f.key !== "created_at" &&
      ABSCISSA_TYPES.has(f.type),
  );
}

export function numericFields(cfg: ScreenConfigFile): FieldDef[] {
  return cfg.fields.filter(
    (f) =>
      f.type !== "hidden" &&
      f.type !== "detail_link" &&
      !f.form?.embedParent &&
      f.key !== "id" &&
      f.key !== "created_at" &&
      NUMERIC_TYPES.has(f.type),
  );
}

export function aggregateNeedsValueField(agg: StatAggregate): boolean {
  return agg !== "count";
}

function labelSortKey(row: EntityStatRow): string {
  return row.sort_key ?? row.label;
}

function compareStatLabels(
  a: { label: string; sortKey: string },
  b: { label: string; sortKey: string },
  temporal: boolean,
): number {
  if (temporal) {
    return a.sortKey.localeCompare(b.sortKey, undefined, { numeric: true });
  }
  return a.label.localeCompare(b.label, "fr", { sensitivity: "base" });
}

export function mergeStatSeries(
  seriesResults: { seriesKey: string; rows: EntityStatRow[] }[],
  options?: { temporal?: boolean },
): { data: StatChartMultiDatum[]; series: StatChartSeriesDef[] } {
  const temporal = options?.temporal ?? false;
  const labelMap = new Map<string, string>();

  for (const { rows } of seriesResults) {
    for (const r of rows) {
      if (!labelMap.has(r.label)) {
        labelMap.set(r.label, labelSortKey(r));
      }
    }
  }

  const labels = [...labelMap.entries()]
    .map(([label, sortKey]) => ({ label, sortKey }))
    .sort((a, b) => compareStatLabels(a, b, temporal))
    .map((e) => e.label);

  const data: StatChartMultiDatum[] = labels.map((label) => {
    const row: StatChartMultiDatum = { label };
    for (const { seriesKey, rows } of seriesResults) {
      const hit = rows.find((r) => r.label === label);
      row[seriesKey] = hit?.value ?? 0;
    }
    return row;
  });

  return { data, series: [] };
}

export function isTemporalAbscissa(cfg: ScreenConfigFile | null, groupBy: string): boolean {
  if (!cfg || !groupBy) return false;
  const field = abscissaFields(cfg).find((f) => f.key === groupBy);
  return field?.type === "date" || field?.type === "datetime" || field?.type === "time";
}
