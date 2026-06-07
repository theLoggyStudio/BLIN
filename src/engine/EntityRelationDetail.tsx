import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Alert } from "@/items/Alert";
import { CollapsiblePanel } from "@/items/CollapsiblePanel";
import { Modal } from "@/items/Modal";
import { Text } from "@/items/Text";
import type { RelationDetailResponse } from "@/types/entity";

interface EntityRelationDetailProps {
  screenKey: string;
  recordId: string;
  open: boolean;
  onClose: () => void;
  title?: string;
}

/** Fiche détail : un panneau rétractable par entité (parent + liaisons). */
export function EntityRelationDetail({
  screenKey,
  recordId,
  open,
  onClose,
  title = "Fiche détaillée",
}: EntityRelationDetailProps) {
  const [data, setData] = useState<RelationDetailResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!recordId) return;
    setLoading(true);
    setError(null);
    try {
      const res = await invoke<RelationDetailResponse>("entity_relation_detail", {
        payload: { screen_key: screenKey, record_id: recordId },
      });
      setData(res);
    } catch (e) {
      setError(String(e));
      setData(null);
    } finally {
      setLoading(false);
    }
  }, [screenKey, recordId]);

  useEffect(() => {
    if (open) void load();
  }, [open, load]);

  return (
    <Modal open={open} onClose={onClose} title={title} size="xl">
      {loading && <Text variant="muted">Chargement…</Text>}
      {error && <Alert variant="danger" size="inline" message={error} />}
      {data && (
        <div className="max-h-[70vh] space-y-3 overflow-y-auto pr-1">
          {data.panels.map((panel) => (
            <CollapsiblePanel
              key={`${panel.entityKey}-${panel.viaField ?? "primary"}`}
              title={panel.label}
              subtitle={
                panel.viaField
                  ? `Liaison via le champ « ${panel.viaField} »`
                  : "Entité principale"
              }
              defaultOpen={panel.primary}
            >
              <dl className="grid gap-3 sm:grid-cols-2">
                {panel.fields.map((f) => (
                  <div key={f.key}>
                    <dt className="text-xs font-medium uppercase tracking-wide text-muted">{f.label}</dt>
                    <dd className="mt-0.5 text-sm text-foreground">{f.value || "—"}</dd>
                  </div>
                ))}
              </dl>
            </CollapsiblePanel>
          ))}
        </div>
      )}
    </Modal>
  );
}
