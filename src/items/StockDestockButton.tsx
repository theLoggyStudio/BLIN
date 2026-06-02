import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { PackageMinus } from "lucide-react";
import { Guard } from "@/components/Guard";
import { Button } from "@/items/Button";
import { Modal } from "@/items/Modal";
import { Input } from "@/items/Input";
import type { ScreenRow } from "@/types/screen";

interface StockDestockButtonProps {
  row: ScreenRow;
  onDone: () => void;
}

/** Retire une quantité du stock et met à jour l'enregistrement source lié. */
export function StockDestockButton({ row, onDone }: StockDestockButtonProps) {
  const [open, setOpen] = useState(false);
  const [qty, setQty] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const stockId = String(row.id ?? "");
  const available = Number(row.quantite ?? 0);

  const submit = async (removeAll: boolean) => {
    if (!stockId) return;
    setBusy(true);
    setError(null);
    try {
      const quantity = removeAll
        ? undefined
        : qty.trim() === ""
          ? undefined
          : Number(qty);
      if (!removeAll && (quantity == null || Number.isNaN(quantity) || quantity <= 0)) {
        setError("Indiquez une quantité positive.");
        setBusy(false);
        return;
      }
      if (!removeAll && quantity != null && quantity > available) {
        setError(`Maximum disponible : ${available}`);
        setBusy(false);
        return;
      }
      await invoke("entity_stock_destock", {
        payload: { stock_id: stockId, quantity },
      });
      setOpen(false);
      setQty("");
      onDone();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  if (available <= 0) return null;

  return (
    <Guard privilege="stock:modifier">
      <Button
        variant="ghost"
        size="sm"
        aria-label="Déstocker"
        title="Retirer du stock"
        onClick={(e) => {
          e.stopPropagation();
          setQty(String(available));
          setError(null);
          setOpen(true);
        }}
      >
        <PackageMinus className="h-4 w-4 text-secondary" />
      </Button>
      <Modal
        open={open}
        onClose={() => !busy && setOpen(false)}
        title="Déstockage"
        size="sm"
        footer={
          <div className="flex flex-wrap justify-end gap-2">
            <Button variant="ghost" disabled={busy} onClick={() => setOpen(false)}>
              Annuler
            </Button>
            <Button variant="secondary" disabled={busy} onClick={() => void submit(false)}>
              Retirer la quantité
            </Button>
            <Button disabled={busy} onClick={() => void submit(true)}>
              Tout déstocker ({available})
            </Button>
          </div>
        }
      >
        <p className="mb-3 text-sm text-muted">
          {String(row.libelle ?? "Article")} — disponible : <strong>{available}</strong>
        </p>
        <Input
          label="Quantité à retirer"
          type="number"
          min={0}
          step={1}
          value={qty}
          disabled={busy}
          error={error ?? undefined}
          onChange={(e) => setQty(e.target.value)}
        />
        <p className="mt-2 text-xs text-muted">
          La fiche source (entité d&apos;origine) sera mise à jour automatiquement.
        </p>
      </Modal>
    </Guard>
  );
}
