import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Plus, RefreshCw, Trash2 } from "lucide-react";
import { Alert } from "@/items/Alert";
import { Button } from "@/items/Button";
import { CollapsiblePanel } from "@/items/CollapsiblePanel";
import { Select } from "@/items/Select";
import { StatChart, type StatChartMultiDatum, type StatChartSeriesDef, type StatChartType } from "@/items/StatChart";
import { isOrphanEntityKey } from "@/lib/orphanEntities";
import {
  abscissaFields,
  aggregateNeedsValueField,
  mergeStatSeries,
  numericFields,
  SERIES_COLORS,
  STAT_AGGREGATE_OPTIONS,
  type StatAggregate,
} from "@/lib/entityStats";
import type { EntityRegistryResponse, EntityStatRow } from "@/types/entity";
import type { ScreenConfigFile } from "@/types/screen";

const CHART_TYPES: { value: StatChartType; label: string }[] = [
  { value: "bar", label: "Barres" },
  { value: "line", label: "Courbes" },
  { value: "area", label: "Aires" },
  { value: "pie", label: "Secteurs" },
];

const MAX_SERIES = 4;

interface StatSeriesDraft {
  id: string;
  entityKey: string;
  groupBy: string;
  aggregate: StatAggregate;
  valueField: string;
}

function newSeriesId(): string {
  return `s-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
}

function defaultSeries(entityKey: string, cfg: ScreenConfigFile | null): StatSeriesDraft {
  const xFields = cfg ? abscissaFields(cfg) : [];
  const yFields = cfg ? numericFields(cfg) : [];
  return {
    id: newSeriesId(),
    entityKey,
    groupBy: xFields[0]?.key ?? "",
    aggregate: "count",
    valueField: yFields[0]?.key ?? "",
  };
}

interface EntityStatsPanelProps {
  defaultEntityKey: string;
}

/** Panneau statistiques : abscisse / ordonnée, type de graphe, comparaison multi-entités. */
export function EntityStatsPanel({ defaultEntityKey }: EntityStatsPanelProps) {
  const [entities, setEntities] = useState<{ value: string; label: string }[]>([]);
  const [configs, setConfigs] = useState<Record<string, ScreenConfigFile>>({});
  const [series, setSeries] = useState<StatSeriesDraft[]>([]);
  const [chartType, setChartType] = useState<StatChartType>("bar");
  const [multiData, setMultiData] = useState<StatChartMultiDatum[]>([]);
  const [chartSeries, setChartSeries] = useState<StatChartSeriesDef[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const entityOptions = useMemo(
    () => [{ value: "", label: "— Choisir —" }, ...entities],
    [entities],
  );

  const loadRegistry = useCallback(async () => {
    try {
      const reg = await invoke<EntityRegistryResponse>("entity_registry_get");
      setEntities(
        reg.entities
          .filter((e) => !isOrphanEntityKey(e.nom))
          .map((e) => ({
          value: e.nom,
          label: e.label?.trim() || e.nom,
        })),
      );
    } catch {
      setEntities([]);
    }
  }, []);

  const loadConfig = useCallback(async (entityKey: string): Promise<ScreenConfigFile | null> => {
    if (!entityKey) return null;
    try {
      const cfg = await invoke<ScreenConfigFile>("entity_get_screen_config", {
        payload: { entity_key: entityKey },
      });
      setConfigs((prev) => (prev[entityKey] ? prev : { ...prev, [entityKey]: cfg }));
      return cfg;
    } catch {
      return null;
    }
  }, []);

  useEffect(() => {
    void loadRegistry();
  }, [loadRegistry]);

  useEffect(() => {
    void (async () => {
      const cfg = await loadConfig(defaultEntityKey);
      setSeries([defaultSeries(defaultEntityKey, cfg)]);
    })();
  }, [defaultEntityKey]);

  const refreshChart = useCallback(async () => {
    setError(null);
    const valid = series.filter((s) => s.entityKey && s.groupBy);
    if (valid.length === 0) {
      setMultiData([]);
      setChartSeries([]);
      return;
    }

    setLoading(true);
    try {
      const results: { seriesKey: string; rows: EntityStatRow[]; def: StatChartSeriesDef }[] = [];

      for (let i = 0; i < valid.length; i++) {
        const s = valid[i];
        const cfg = configs[s.entityKey] ?? (await loadConfig(s.entityKey));
        const entLabel =
          entities.find((e) => e.value === s.entityKey)?.label ?? s.entityKey;
        const seriesKey = `s${i}`;
        const rows = await invoke<EntityStatRow[]>("entity_stats", {
          payload: {
            entity_key: s.entityKey,
            group_by: s.groupBy,
            aggregate: s.aggregate,
            value_field: aggregateNeedsValueField(s.aggregate) ? s.valueField || null : null,
          },
        });
        results.push({
          seriesKey,
          rows,
          def: {
            key: seriesKey,
            name: entLabel,
            color: SERIES_COLORS[i % SERIES_COLORS.length],
          },
        });
      }

      const merged = mergeStatSeries(
        results.map((r) => ({ seriesKey: r.seriesKey, rows: r.rows })),
      );
      setMultiData(merged.data);
      setChartSeries(results.map((r) => r.def));
    } catch (e) {
      setError(String(e));
      setMultiData([]);
      setChartSeries([]);
    } finally {
      setLoading(false);
    }
  }, [series, configs, entities, loadConfig]);

  useEffect(() => {
    const t = setTimeout(() => {
      void refreshChart();
    }, 400);
    return () => clearTimeout(t);
  }, [refreshChart, chartType]);

  const updateSeries = (id: string, patch: Partial<StatSeriesDraft>) => {
    setSeries((prev) => prev.map((s) => (s.id === id ? { ...s, ...patch } : s)));
  };

  const onEntityChange = async (id: string, entityKey: string) => {
    const cfg = await loadConfig(entityKey);
    const x = cfg ? abscissaFields(cfg) : [];
    const y = cfg ? numericFields(cfg) : [];
    updateSeries(id, {
      entityKey,
      groupBy: x[0]?.key ?? "",
      valueField: y[0]?.key ?? "",
    });
  };

  const xLabel = series[0]?.groupBy
    ? (configs[series[0].entityKey]
        ? abscissaFields(configs[series[0].entityKey]).find((f) => f.key === series[0].groupBy)?.label
        : series[0].groupBy) ?? "Abscisse"
    : "Abscisse";

  const yLabel =
    series[0]?.aggregate === "count"
      ? "Nombre"
      : STAT_AGGREGATE_OPTIONS.find((o) => o.value === series[0]?.aggregate)?.label ?? "Ordonnée";

  return (
    <CollapsiblePanel
      title="Statistiques"
      subtitle={`Choisissez l'abscisse et l'ordonnée, le type de graphe, et comparez jusqu'à ${MAX_SERIES} entités (courbes ou barres de couleurs différentes).`}
      defaultOpen={false}
      headerAction={
        <Button size="sm" variant="ghost" disabled={loading} onClick={() => void refreshChart()}>
          <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
          <span className="sr-only">Actualiser</span>
        </Button>
      }
    >
      <div className="space-y-4">
        <div className="flex flex-wrap justify-end gap-2">
          <Select
            label="Type de graphe"
            value={chartType}
            onChange={(e) => setChartType(e.target.value as StatChartType)}
            options={CHART_TYPES}
          />
          <div className="flex items-end">
            <Button size="sm" variant="secondary" disabled={loading} onClick={() => void refreshChart()}>
              <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
              Actualiser
            </Button>
          </div>
        </div>

        {error && <Alert variant="danger" size="inline" message={error} />}

        <div className="space-y-3">
        {series.map((s, index) => {
          const cfg = configs[s.entityKey];
          const xOpts = cfg
            ? abscissaFields(cfg).map((f) => ({ value: f.key, label: f.label }))
            : [];
          const yOpts = cfg
            ? numericFields(cfg).map((f) => ({ value: f.key, label: f.label }))
            : [];
          const color = SERIES_COLORS[index % SERIES_COLORS.length];

          return (
            <div
              key={s.id}
              className="grid gap-3 rounded-lg border border-border p-3 sm:grid-cols-2 lg:grid-cols-5"
              style={{ borderLeftWidth: 4, borderLeftColor: color }}
            >
              <Select
                label={`Série ${index + 1} — Entité`}
                value={s.entityKey}
                onChange={(e) => void onEntityChange(s.id, e.target.value)}
                options={entityOptions.filter((o) => o.value !== "")}
              />
              <Select
                label="Abscisse (X)"
                value={s.groupBy}
                onChange={(e) => updateSeries(s.id, { groupBy: e.target.value })}
                options={
                  xOpts.length > 0
                    ? xOpts
                    : [{ value: "", label: "— Aucun champ —" }]
                }
                disabled={!s.entityKey}
              />
              <Select
                label="Ordonnée (Y)"
                value={s.aggregate}
                onChange={(e) =>
                  updateSeries(s.id, { aggregate: e.target.value as StatAggregate })
                }
                options={STAT_AGGREGATE_OPTIONS}
              />
              <Select
                label="Champ numérique"
                value={s.valueField}
                onChange={(e) => updateSeries(s.id, { valueField: e.target.value })}
                options={
                  yOpts.length > 0
                    ? yOpts
                    : [{ value: "", label: "— Aucun —" }]
                }
                disabled={!aggregateNeedsValueField(s.aggregate) || !s.entityKey}
              />
              <div className="flex items-end justify-end">
                {series.length > 1 && (
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => setSeries((prev) => prev.filter((x) => x.id !== s.id))}
                    title="Retirer cette série"
                  >
                    <Trash2 className="h-4 w-4 text-primary" />
                  </Button>
                )}
              </div>
            </div>
          );
        })}
      </div>

      {series.length < MAX_SERIES && (
        <Button
          size="sm"
          variant="secondary"
          onClick={() => {
            const used = new Set(series.map((s) => s.entityKey));
            const nextKey =
              entities.find((e) => e.value && !used.has(e.value))?.value ?? defaultEntityKey;
            const cfg = configs[nextKey] ?? null;
            setSeries((prev) => [...prev, defaultSeries(nextKey, cfg)]);
          }}
        >
          <Plus className="h-4 w-4" />
          Comparer une autre entité
        </Button>
      )}

      <StatChart
        title={
          chartSeries.length > 1
            ? "Comparaison multi-entités"
            : chartSeries[0]
              ? `Statistiques — ${chartSeries[0].name}`
              : "Statistiques"
        }
        type={chartType}
        multiData={multiData}
        series={chartSeries}
        xLabel={xLabel}
        yLabel={yLabel}
        height={320}
      />
      </div>
    </CollapsiblePanel>
  );
}
