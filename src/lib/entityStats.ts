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

const ABSCISSA_TYPES = new Set(["text", "select", "datetime", "boolean", "entity_ref"]);
const NUMERIC_TYPES = new Set(["number"]);

export function abscissaFields(cfg: ScreenConfigFile): FieldDef[] {
  return cfg.fields.filter(
    (f) =>
      f.type !== "hidden" &&
      f.type !== "detail_link" &&
      f.key !== "id" &&
      f.key !== "created_at" &&
      ABSCISSA_TYPES.has(f.type),
  );
}

export function numericFields(cfg: ScreenConfigFile): FieldDef[] {
  return cfg.fields.filter(
    (f) =>
      f.type !== "hidden" &&
      f.key !== "id" &&
      f.key !== "created_at" &&
      NUMERIC_TYPES.has(f.type),
  );
}

export function aggregateNeedsValueField(agg: StatAggregate): boolean {
  return agg !== "count";
}

export function mergeStatSeries(
  seriesResults: { seriesKey: string; rows: EntityStatRow[] }[],
): { data: StatChartMultiDatum[]; series: StatChartSeriesDef[] } {
  const labelSet = new Set<string>();
  for (const { rows } of seriesResults) {
    for (const r of rows) {
      labelSet.add(r.label);
    }
  }
  const labels = [...labelSet].sort((a, b) =>
    a.localeCompare(b, "fr", { sensitivity: "base" }),
  );

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
