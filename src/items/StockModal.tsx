import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Alert } from "@/items/Alert";
import { DataScreen } from "@/engine/DataScreen";
import { StockDestockButton } from "@/items/StockDestockButton";
import { Modal } from "@/items/Modal";
import { Text } from "@/items/Text";
import type { ScreenConfigFile } from "@/types/screen";

const ENTITY_KEY = "stock";

interface StockModalProps {
  open: boolean;
  onClose: () => void;
}

/** Inventaire : quantités, articles périssables et alertes de déstockage. */
export function StockModal({ open, onClose }: StockModalProps) {
  const [config, setConfig] = useState<ScreenConfigFile | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      await invoke("entity_stock_scan_destock", {}).catch(() => undefined);
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
    <Modal open={open} onClose={onClose} title="Stock" size="2xl">
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
              Quantités synchronisées depuis vos entités. Cochez « Article périssable » et
              indiquez la date de péremption : une tâche de déstockage est créée automatiquement
              un mois avant expiration.
            </Text>
            <DataScreen
              config={config}
              key="stock-list"
              extraRowActions={(row, reload) => (
                <StockDestockButton row={row} onDone={reload} />
              )}
            />
          </>
        )}
      </div>
    </Modal>
  );
}
