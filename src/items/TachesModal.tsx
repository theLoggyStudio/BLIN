import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Alert } from "@/items/Alert";
import { DataScreen } from "@/engine/DataScreen";
import { Modal } from "@/items/Modal";
import { Text } from "@/items/Text";
import type { ScreenConfigFile, ScreenRow } from "@/types/screen";

const ENTITY_KEY = "tache";

interface TachesModalProps {
  open: boolean;
  onClose: () => void;
  initialCreateValues?: ScreenRow;
  onInitialCreateApplied?: () => void;
}

/** Liste et CRUD des tâches dans un modal (pas de navigation plein écran). */
export function TachesModal({
  open,
  onClose,
  initialCreateValues,
  onInitialCreateApplied,
}: TachesModalProps) {
  const [config, setConfig] = useState<ScreenConfigFile | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const cfg = await invoke<ScreenConfigFile>("entity_get_screen_config", {
        payload: { entity_key: ENTITY_KEY },
      });
      setConfig(cfg);
    } catch (e) {
      setError(String(e));
      setConfig(null);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (open) {
      void load();
    } else {
      setConfig(null);
      setError(null);
    }
  }, [open, load]);

  return (
    <Modal open={open} onClose={onClose} title="Tâches" size="2xl">
      <div className="pr-1">
        {loading && (
          <div className="flex justify-center py-12">
            <div className="h-10 w-10 animate-spin rounded-full border-2 border-secondary border-t-transparent" />
          </div>
        )}
        {!loading && error && <Alert variant="danger" size="box" message={error} />}
        {!loading && config && (
          <>
            <Text variant="muted" className="mb-4 text-sm">
              Validations, créations et suivi — les formulaires s&apos;ouvrent dans des modals
              imbriqués.
            </Text>
            <DataScreen
              config={config}
              compactList
              listRowClick="detail"
              initialCreateValues={initialCreateValues}
              onInitialCreateApplied={onInitialCreateApplied}
              key={
                initialCreateValues
                  ? `tache-create-${JSON.stringify(initialCreateValues)}`
                  : "tache-list"
              }
            />
          </>
        )}
      </div>
    </Modal>
  );
}
