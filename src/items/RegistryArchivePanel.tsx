import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Check, ClipboardCopy } from "lucide-react";
import { Alert } from "@/items/Alert";
import { Button } from "@/items/Button";
import { Text } from "@/items/Text";
import { ENTITY_REGISTRY_SYNCED_EVENT } from "@/constants/events";
import { formatDateTimeFr } from "@/lib/formatDateTime";
import { cn } from "@/lib/utils";

export interface RegistryArchiveSummary {
  id: string;
  archivedAt: string;
}

/** Archives du registre entités — 5 dernières versions (copie JSON). */
export function RegistryArchivePanel() {
  const [rows, setRows] = useState<RegistryArchiveSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [copiedId, setCopiedId] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await invoke<RegistryArchiveSummary[]>("entity_registry_archive_list");
      setRows(data);
    } catch (e) {
      setError(String(e));
      setRows([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    const onSynced = () => void load();
    window.addEventListener(ENTITY_REGISTRY_SYNCED_EVENT, onSynced);
    return () => window.removeEventListener(ENTITY_REGISTRY_SYNCED_EVENT, onSynced);
  }, [load]);

  const copyArchive = async (id: string) => {
    try {
      const json = await invoke<string>("entity_registry_archive_get", { payload: { id } });
      await navigator.clipboard.writeText(json);
      setCopiedId(id);
      window.setTimeout(() => setCopiedId((cur) => (cur === id ? null : cur)), 2000);
    } catch (e) {
      setError(String(e));
    }
  };

  if (loading) {
    return <Text variant="muted">Chargement des archives…</Text>;
  }

  if (error) {
    return <Alert variant="danger" size="inline" message={error} />;
  }

  if (rows.length === 0) {
    return (
      <Text variant="muted" className="text-sm">
        Aucune archive pour l&apos;instant. Une version est enregistrée à chaque synchronisation du
        registre (avant remplacement).
      </Text>
    );
  }

  return (
    <ul className="space-y-2">
      {rows.map((row) => {
        const label = formatDateTimeFr(row.archivedAt);
        const copied = copiedId === row.id;
        return (
          <li
            key={row.id}
            className="flex items-center justify-between gap-3 rounded-lg border border-border bg-card px-3 py-2"
          >
            <span className="min-w-0 truncate text-sm text-foreground">
              Version {label || row.archivedAt}
            </span>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="shrink-0 px-2"
              title="Copier le JSON du registre"
              aria-label={`Copier la version ${label}`}
              onClick={() => void copyArchive(row.id)}
            >
              {copied ? (
                <Check className={cn("h-4 w-4 text-secondary")} />
              ) : (
                <ClipboardCopy className="h-4 w-4" />
              )}
            </Button>
          </li>
        );
      })}
    </ul>
  );
}
