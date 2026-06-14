import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Plus } from "lucide-react";
import { usePrivilege } from "@/hooks/usePrivilege";
import { Alert } from "@/items/Alert";
import { Button } from "@/items/Button";
import { Modal } from "@/items/Modal";
import { SearchInput } from "@/items/SearchInput";
import { RelationOptionRow } from "@/items/RelationOptionLine";
import { EntityRelationCreateModal } from "@/items/EntityRelationCreateModal";
import { fetchRelationOptions } from "@/items/EntityRelationAutocomplete";
import { RELATION_SUGGESTIONS_LIMIT } from "@/constants/variable.constant";
import { blurActiveElement } from "@/lib/focus";

import type { RelationSelectOption } from "@/types/entity";

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
  const [hasSearched, setHasSearched] = useState(false);
  const [createOpen, setCreateOpen] = useState(false);
  const canCreate = usePrivilege(`${entityKey}:creer`);
  const [entityLabel, setEntityLabel] = useState<string | null>(null);
  const searchRef = useRef<HTMLInputElement>(null);

  useLayoutEffect(() => {
    if (!open) return;
    blurActiveElement();
    const timer = window.setTimeout(() => {
      searchRef.current?.focus({ preventScroll: true });
    }, 0);
    return () => window.clearTimeout(timer);
  }, [open]);

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

  const runSearch = useCallback(async () => {
    setHasSearched(true);
    setLoading(true);
    try {
      const rows = await fetchRelationOptions({
        screenKey,
        fieldKey,
        excludeRecordId,
        search: query.trim() || undefined,
        limit: RELATION_SUGGESTIONS_LIMIT,
      });
      setOptions(rows.filter((o) => o.value));
    } catch {
      setOptions([]);
    } finally {
      setLoading(false);
    }
  }, [screenKey, fieldKey, excludeRecordId, query]);

  const available = useMemo(() => {
    const excluded = new Set(excludeIds);
    return options.filter((o) => o.value && !excluded.has(o.value));
  }, [options, excludeIds]);

  const handleClose = () => {
    setQuery("");
    setOptions([]);
    setHasSearched(false);
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
          <SearchInput
            ref={searchRef}
            id="entity-relation-pick-search"
            label="Rechercher"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Tapez puis cliquez sur la loupe…"
            loading={loading}
            onSearch={() => void runSearch()}
          />
          <div className="max-h-80 overflow-y-auto rounded-lg border border-border bg-background/40">
            {!hasSearched ? (
              <p className="px-4 py-8 text-center text-sm text-muted">
                Saisissez un terme puis cliquez sur la loupe pour lancer la recherche.
              </p>
            ) : loading ? (
              <p className="px-4 py-8 text-center text-sm text-muted">Recherche…</p>
            ) : available.length === 0 ? (
              <Alert
                variant="info"
                size="box"
                centered
                className="mx-2 my-4 px-3 py-6"
                message={
                  query.trim()
                    ? "Aucun résultat pour cette recherche."
                    : "Aucun objet signé disponible — créez-en un nouveau ou demandez la signature d'un objet existant."
                }
              />
            ) : (
              available.map((option) => (
                <RelationOptionRow
                  key={option.value}
                  option={option}
                  onClick={() => trySelect(option)}
                />
              ))
            )}
          </div>
          {hasSearched && !loading && available.length >= RELATION_SUGGESTIONS_LIMIT && (
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
