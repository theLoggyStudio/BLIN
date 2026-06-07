import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Plus } from "lucide-react";
import { usePrivilege } from "@/hooks/usePrivilege";
import { Button } from "@/items/Button";
import { Input } from "@/items/Input";
import { Modal } from "@/items/Modal";
import { EntityRelationCreateModal } from "@/items/EntityRelationCreateModal";
import type { RelationSelectOption } from "@/types/entity";

interface EntityRelationPickOrCreateModalProps {
  entityKey: string;
  open: boolean;
  onClose: () => void;
  options: RelationSelectOption[];
  excludeIds: string[];
  embedMode?: boolean;
  onSelected: (id: string) => void;
  onOptionsRefresh: () => void;
}

export function EntityRelationPickOrCreateModal({
  entityKey,
  open,
  onClose,
  options,
  excludeIds,
  onSelected,
  onOptionsRefresh,
}: EntityRelationPickOrCreateModalProps) {
  const [query, setQuery] = useState("");
  const [createOpen, setCreateOpen] = useState(false);
  const canCreate = usePrivilege(`${entityKey}:creer`);
  const [entityLabel, setEntityLabel] = useState<string | null>(null);

  const loadLabel = useCallback(async () => {
    try {
      const cfg = await invoke<{ screen?: { label?: string } }>("entity_get_screen_config", {
        payload: { entity_key: entityKey },
      });
      setEntityLabel(cfg.screen?.label ?? null);
    } catch {
      setEntityLabel(null);
    }
  }, [entityKey]);

  useEffect(() => {
    if (open) void loadLabel();
  }, [open, loadLabel]);

  const available = useMemo(() => {
    const excluded = new Set(excludeIds);
    return options.filter((o) => o.value && !excluded.has(o.value));
  }, [options, excludeIds]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return available;
    return available.filter(
      (o) => o.label.toLowerCase().includes(q) || o.value.toLowerCase().includes(q),
    );
  }, [available, query]);

  const handleClose = () => {
    setQuery("");
    setCreateOpen(false);
    onClose();
  };

  const trySelect = (option: RelationSelectOption) => {
    onSelected(option.value);
    handleClose();
  };

  const title = entityLabel
    ? `Choisir ou créer — ${entityLabel}`
    : `Choisir ou créer — ${entityKey}`;

  return (
    <>
      <Modal open={open && !createOpen} onClose={handleClose} title={title} size="md">
        <div className="space-y-4">
          <Input
            label="Rechercher"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Filtrer la liste…"
            autoFocus
          />
          <div className="max-h-64 space-y-1 overflow-y-auto rounded-lg border border-border p-1">
            {filtered.length === 0 ? (
              <p className="px-3 py-6 text-center text-sm text-muted" role="alert">
                {available.length === 0
                  ? "Aucun objet signé disponible — créez-en un nouveau ou demandez la signature d'un objet existant."
                  : "Aucun résultat pour cette recherche."}
              </p>
            ) : (
              filtered.map((option) => (
                <button
                  key={option.value}
                  type="button"
                  className="flex w-full items-center justify-between gap-2 rounded-md px-3 py-2 text-left text-sm hover:bg-surface-elevated/80"
                  onClick={() => trySelect(option)}
                >
                  <span className="text-foreground">{option.label}</span>
                </button>
              ))
            )}
          </div>
          {canCreate && (
            <Button
              type="button"
              variant="secondary"
              className="w-full"
              onClick={() => setCreateOpen(true)}
            >
              <Plus className="mr-2 h-4 w-4" />
              Créer un nouveau
            </Button>
          )}
        </div>
      </Modal>

      {canCreate && (
        <EntityRelationCreateModal
          entityKey={entityKey}
          open={createOpen}
          onClose={() => setCreateOpen(false)}
          onCreated={(row) => {
            const id = row.id != null ? String(row.id) : "";
            if (!id) return;
            onOptionsRefresh();
            onSelected(id);
            handleClose();
          }}
        />
      )}
    </>
  );
}
