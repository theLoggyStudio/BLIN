import { useEffect, useState } from "react";
import { Columns3 } from "lucide-react";
import { Button } from "@/items/Button";
import { Modal } from "@/items/Modal";
import type { ChatColsRequest } from "@/types/ai";

interface ChatColsPickerModalProps {
  request: ChatColsRequest;
  open: boolean;
  onClose: () => void;
  onConfirm: (message: string) => void;
}

export function ChatColsPickerModal({
  request,
  open,
  onClose,
  onConfirm,
}: ChatColsPickerModalProps) {
  const [selected, setSelected] = useState<Set<string>>(() => new Set());

  useEffect(() => {
    if (open) {
      setSelected(new Set());
    }
  }, [open, request.entityKey]);

  const toggle = (key: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  const selectAll = () => {
    setSelected(new Set(request.available.map((c) => c.key)));
  };

  const handleConfirm = () => {
    if (selected.size === 0) return;
    const all = selected.size === request.available.length;
    const msg = all
      ? "toutes"
      : request.available
          .filter((c) => selected.has(c.key))
          .map((c) => c.label)
          .join(", ");
    onConfirm(msg);
    onClose();
  };

  const label = request.entityLabel ?? request.entityKey;

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={`Colonnes — ${label}`}
      size="md"
      footer={
        <div className="flex w-full flex-col-reverse gap-2 sm:flex-row sm:justify-end">
          <Button variant="ghost" onClick={onClose}>
            Annuler
          </Button>
          <Button variant="primary" disabled={selected.size === 0} onClick={handleConfirm}>
            Afficher la liste
          </Button>
        </div>
      }
    >
      <p className="mb-4 text-sm text-muted">
        Cochez les colonnes à afficher pour « {label} ».
      </p>
      <div className="mb-3">
        <Button type="button" size="sm" variant="secondary" onClick={selectAll}>
          <Columns3 className="mr-1.5 h-4 w-4" />
          Tout sélectionner
        </Button>
      </div>
      <ul className="max-h-[min(50vh,20rem)] space-y-2 overflow-y-auto">
        {request.available.map((col) => (
          <li key={col.key}>
            <label className="flex cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 hover:bg-muted/40">
              <input
                type="checkbox"
                className="h-4 w-4 accent-teal-500"
                checked={selected.has(col.key)}
                onChange={() => toggle(col.key)}
              />
              <span className="text-sm text-foreground">{col.label}</span>
            </label>
          </li>
        ))}
      </ul>
    </Modal>
  );
}
