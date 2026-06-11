import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Plus } from "lucide-react";
import { usePrivilege } from "@/hooks/usePrivilege";
import { Alert } from "@/items/Alert";
import { Button } from "@/items/Button";
import { Input } from "@/items/Input";
import { Modal } from "@/items/Modal";
import { EntityRelationCreateModal } from "@/items/EntityRelationCreateModal";
import { fetchRelationOptions } from "@/items/EntityRelationAutocomplete";
import { RELATION_SUGGESTIONS_LIMIT } from "@/constants/variable.constant";
import type { RelationSelectOption } from "@/types/entity";

const SEARCH_DEBOUNCE_MS = 200;

interface EntityRelationPickOrCreateModalProps {
  entityKey: string;
  open: boolean;
  onClose: () => void;
  /** Écran parent (clé) portant le champ de liaison. */
  screenKey: string;
  /** Clé du champ de liaison sur l'écran parent. */
  fieldKey: string;
  excludeRecordId?: string;
  excludeIds: string[];
  embedMode?: boolean;
  onSelected: (id: string) => void;
}

/**
 * Choisir ou créer un enregistrement d'une entité liée.
 * Recherche côté serveur, max RELATION_SUGGESTIONS_LIMIT résultats affichés.
 */
export function EntityRelationPickOrCreateModal({
  entityKey,
  open,
  onClose,
  screenKey,
  fieldKey,
  excludeRecordId,
  excludeIds,
  onSelected,
}: EntityRelationPickOrCreateModalProps) {
  const [query, setQuery] = useState("");
  const [options, setOptions] = useState<RelationSelectOption[]>([]);
  const [loading, setLoading] = useState(false);
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

  // Recherche serveur débouncée — ne charge jamais la totalité de l'entité cible.
  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    setLoading(true);
    const timer = window.setTimeout(() => {
      void fetchRelationOptions({
        screenKey,
        fieldKey,
        excludeRecordId,
        search: query.trim() || undefined,
        limit: RELATION_SUGGESTIONS_LIMIT,
      })
        .then((rows) => {
          if (!cancelled) setOptions(rows.filter((o) => o.value));
        })
        .catch(() => {
          if (!cancelled) setOptions([]);
        })
        .finally(() => {
          if (!cancelled) setLoading(false);
        });
    }, SEARCH_DEBOUNCE_MS);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [open, query, screenKey, fieldKey, excludeRecordId]);

  const available = useMemo(() => {
    const excluded = new Set(excludeIds);
    return options.filter((o) => o.value && !excluded.has(o.value));
  }, [options, excludeIds]);

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
            placeholder="Tapez pour rechercher…"
            autoFocus
          />
          <div className="max-h-64 space-y-1 overflow-y-auto rounded-lg border border-border p-1">
            {loading ? (
              <p className="px-3 py-6 text-center text-sm text-muted">Recherche…</p>
            ) : available.length === 0 ? (
              <Alert
                variant="info"
                size="box"
                centered
                className="px-3 py-6"
                message={
                  query.trim()
                    ? "Aucun résultat pour cette recherche."
                    : "Aucun objet signé disponible — créez-en un nouveau ou demandez la signature d'un objet existant."
                }
              />
            ) : (
              available.map((option) => (
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
          {!loading && available.length >= RELATION_SUGGESTIONS_LIMIT && (
            <p className="text-xs text-muted">
              {RELATION_SUGGESTIONS_LIMIT} premiers résultats affichés — affinez votre recherche.
            </p>
          )}
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
            onSelected(id);
            handleClose();
          }}
        />
      )}
    </>
  );
}
