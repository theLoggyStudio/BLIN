import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  startTransition,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2, MessageCircle, Plus, RefreshCw, Trash2 } from "lucide-react";
import { Alert } from "@/items/Alert";
import { Button } from "@/items/Button";
import { CollapsiblePanel } from "@/items/CollapsiblePanel";
import { Select } from "@/items/Select";
import { StatChart, type StatChartMultiDatum, type StatChartSeriesDef, type StatChartType } from "@/items/StatChart";
import { StatsLoggyChatModal } from "@/items/StatsLoggyChatModal";
import { isOrphanEntityKey } from "@/lib/orphanEntities";
import {
  abscissaFields,
  aggregateNeedsValueField,
  isTemporalAbscissa,
  mergeStatSeries,
  numericFields,
  SERIES_COLORS,
  STAT_AGGREGATE_OPTIONS,
  type StatAggregate,
} from "@/lib/entityStats";
import {
  fallbackStatsInterpretation,
  enrichStatsInterpretationWithAi,
  type StatsInterpretPayload,
} from "@/lib/statsInterpret";
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

interface StatSeriesResult {
  seriesKey: string;
  rows: EntityStatRow[];
  def: StatChartSeriesDef;
  draft: StatSeriesDraft;
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
  const [seriesResults, setSeriesResults] = useState<StatSeriesResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [interpretation, setInterpretation] = useState("");
  const [interpretLoading, setInterpretLoading] = useState(false);
  const [loggyModalOpen, setLoggyModalOpen] = useState(false);
  const interpretRequestId = useRef(0);

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
      setSeriesResults([]);
      return;
    }

    setLoading(true);
    try {
      const temporal = valid.some((s) =>
        isTemporalAbscissa(configs[s.entityKey] ?? null, s.groupBy),
      );

      const results = await Promise.all(
        valid.map(async (s, i) => {
          await loadConfig(s.entityKey);
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
          return {
            seriesKey,
            rows,
            draft: s,
            def: {
              key: seriesKey,
              name: entLabel,
              color: SERIES_COLORS[i % SERIES_COLORS.length],
            },
          } satisfies StatSeriesResult;
        }),
      );

      const merged = mergeStatSeries(
        results.map((r) => ({ seriesKey: r.seriesKey, rows: r.rows })),
        { temporal },
      );
      setMultiData(merged.data);
      setChartSeries(results.map((r) => r.def));
      setSeriesResults(results);
    } catch (e) {
      setError(String(e));
      setMultiData([]);
      setChartSeries([]);
      setSeriesResults([]);
    } finally {
      setLoading(false);
    }
  }, [series, configs, entities, loadConfig]);

  useEffect(() => {
    const t = setTimeout(() => {
      void refreshChart();
    }, 400);
    return () => clearTimeout(t);
  }, [refreshChart]);

  const xLabel = series[0]?.groupBy
    ? (configs[series[0].entityKey]
        ? abscissaFields(configs[series[0].entityKey]).find((f) => f.key === series[0].groupBy)?.label
        : series[0].groupBy) ?? "Abscisse"
    : "Abscisse";

  const yLabel =
    series[0]?.aggregate === "count"
      ? "Nombre"
      : STAT_AGGREGATE_OPTIONS.find((o) => o.value === series[0]?.aggregate)?.label ?? "Ordonnée";

  const statsDataVersion = useMemo(() => {
    if (seriesResults.length === 0 || multiData.length === 0) return "";
    return JSON.stringify({
      multiData,
      series: seriesResults.map((r) => ({
        seriesKey: r.seriesKey,
        entityKey: r.draft.entityKey,
        groupBy: r.draft.groupBy,
        aggregate: r.draft.aggregate,
        valueField: r.draft.valueField,
        rows: r.rows,
      })),
      xLabel,
      yLabel,
    });
  }, [seriesResults, multiData, xLabel, yLabel]);

  const interpretPayload = useMemo((): StatsInterpretPayload | null => {
    if (!statsDataVersion) return null;
    return {
      chart_type: chartType,
      x_label: xLabel,
      y_label: yLabel,
      series: seriesResults.map((r) => ({
        name: r.def.name,
        entity_key: r.draft.entityKey,
        aggregate: r.draft.aggregate,
        group_by: r.draft.groupBy,
        value_field: aggregateNeedsValueField(r.draft.aggregate) ? r.draft.valueField : null,
        points: multiData.map((row) => ({
          label: row.label,
          value: Number(row[r.seriesKey] ?? 0),
        })),
      })),
    };
  }, [statsDataVersion, chartType, xLabel, yLabel, seriesResults, multiData]);

  const interpretPayloadRef = useRef(interpretPayload);
  interpretPayloadRef.current = interpretPayload;

  useEffect(() => {
    if (!statsDataVersion) {
      setInterpretation("");
      setInterpretLoading(false);
      return;
    }
    if (loading) {
      interpretRequestId.current += 1;
      setInterpretLoading(false);
      return;
    }

    const payload = interpretPayloadRef.current;
    if (!payload) return;

    const requestId = ++interpretRequestId.current;
    setInterpretLoading(true);

    const fallbackTimer = window.setTimeout(() => {
      const text = fallbackStatsInterpretation(payload);
      startTransition(() => {
        if (interpretRequestId.current === requestId) {
          setInterpretation(text);
        }
      });
    }, 0);

    const aiTimer = window.setTimeout(() => {
      void enrichStatsInterpretationWithAi(payload)
        .then((enriched) => {
          if (interpretRequestId.current !== requestId) return;
          if (enriched) {
            startTransition(() => setInterpretation(enriched));
          }
        })
        .finally(() => {
          if (interpretRequestId.current === requestId) {
            setInterpretLoading(false);
          }
        });
    }, 600);

    return () => {
      clearTimeout(fallbackTimer);
      clearTimeout(aiTimer);
    };
  }, [statsDataVersion, loading]);

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

        {error && <Alert variant="danger" size="box" message={error} />}

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

      <div className="flex flex-wrap items-center justify-between gap-3">
        <p className="text-xs text-muted">
          {interpretLoading
            ? "Loggy prépare son analyse en arrière-plan…"
            : statsDataVersion
              ? "Discutez avec Loggy des paliers et variations de cette courbe."
              : "Ajustez l'abscisse et l'ordonnée pour activer l'analyse."}
        </p>
        <Button
          size="sm"
          variant="secondary"
          disabled={!statsDataVersion || loading}
          onClick={() => setLoggyModalOpen(true)}
        >
          {interpretLoading ? (
            <Loader2 className="h-4 w-4 animate-spin" aria-hidden />
          ) : (
            <MessageCircle className="h-4 w-4" aria-hidden />
          )}
          Demander l&apos;avis de Loggy
        </Button>
      </div>

      <StatsLoggyChatModal
        open={loggyModalOpen}
        onClose={() => setLoggyModalOpen(false)}
        interpretPayload={interpretPayload}
        interpretation={interpretation}
        interpretLoading={interpretLoading}
        statsDataVersion={statsDataVersion}
      />
      </div>
    </CollapsiblePanel>
  );
}
