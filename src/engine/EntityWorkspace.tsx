import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { X } from "lucide-react";
import { Alert } from "@/items/Alert";
import { DataScreen } from "@/engine/DataScreen";
import { Button } from "@/items/Button";
import { EntityStatsPanel } from "@/items/EntityStatsPanel";
import { StatChartGrid } from "@/items/StatChartGrid";
import { StatKpiCard } from "@/items/StatKpiCard";
import { Text } from "@/items/Text";
import type { ScreenConfigFile, ScreenRow } from "@/types/screen";

interface EntityWorkspaceProps {
  entityKey: string;
  onClose: () => void;
  initialCreateValues?: ScreenRow;
  onCreateDraftConsumed?: () => void;
}

/** Formulaire + liste dynamiques pour une entité (pas d'écran dédié). */
export function EntityWorkspace({
  entityKey,
  onClose,
  initialCreateValues,
  onCreateDraftConsumed,
}: EntityWorkspaceProps) {
  const [config, setConfig] = useState<ScreenConfigFile | null>(null);
  const [rowCount, setRowCount] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const attributeCount = useMemo(() => {
    if (!config) return 0;
    return config.fields.filter(
      (f) =>
        f.type !== "hidden" &&
        f.type !== "detail_link" &&
        f.type !== "entity_embed" &&
        f.type !== "entity_embed_list" &&
        !f.form?.embedParent,
    ).length;
  }, [config]);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const cfg = await invoke<ScreenConfigFile>("entity_get_screen_config", {
        payload: { entity_key: entityKey },
      });
      setConfig(cfg);
      const rows = await invoke<Record<string, unknown>[]>("dda_list", {
        payload: { screen_key: entityKey, filters: {} },
      });
      setRowCount(rows.length);

    } catch (e) {
      setError(String(e));
      setConfig(null);
    } finally {
      setLoading(false);
    }
  }, [entityKey]);

  useEffect(() => {
    void load();
  }, [load]);

  if (loading) {
    return (
      <div className="flex justify-center py-16">
        <div className="h-10 w-10 animate-spin rounded-full border-2 border-secondary border-t-transparent" />
      </div>
    );
  }

  if (error || !config) {
    return (
      <Alert
        variant="danger"
        size="box"
        className="card-panel mx-6 my-8 rounded-xl p-6"
        message={error ?? "Configuration introuvable."}
      />
    );
  }

  const label = config.screen.label;

  return (
    <div className="flex flex-col gap-6 px-6 py-6">
      <div className="flex items-start justify-between gap-4">
        <div>
          <Text variant="title" as="h2" className="screen-title-gradient !text-2xl">
            {label}
          </Text>
          <Text variant="muted" className="mt-1">
            Entité « {entityKey} » — liste, formulaire modal et statistiques
          </Text>
        </div>
        <Button variant="ghost" size="sm" onClick={onClose} title="Fermer">
          <X className="h-4 w-4" />
          Fermer
        </Button>
      </div>

      <StatChartGrid columns={3}>
        <StatKpiCard label="Enregistrements" value={rowCount} />
        <StatKpiCard label="Attributs" value={attributeCount} />
        <StatKpiCard label="Table" value={config.screen.table} hint="SQLite dynamique" />
      </StatChartGrid>

      <EntityStatsPanel defaultEntityKey={entityKey} />

      <DataScreen
        config={config}
        initialCreateValues={initialCreateValues}
        onInitialCreateApplied={onCreateDraftConsumed}
        key={
          initialCreateValues
            ? `${entityKey}-create-${JSON.stringify(initialCreateValues)}`
            : entityKey
        }
      />
    </div>
  );
}
