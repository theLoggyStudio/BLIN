import { useCallback, useEffect, useId, useLayoutEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { invoke } from "@tauri-apps/api/core";
import { ChevronDown, Plus, Search } from "lucide-react";
import { Alert } from "@/items/Alert";
import { RelationOptionRow } from "@/items/RelationOptionLine";
import { RELATION_SUGGESTIONS_LIMIT } from "@/constants/variable.constant";
import { floatingMenuZIndex } from "@/lib/modalStack";
import { cn } from "@/lib/utils";
import type { RelationSelectOption } from "@/types/entity";

export interface RelationOptionsQuery {
  screenKey: string;
  fieldKey: string;
  excludeRecordId?: string | null;
  search?: string;
  limit?: number;
  includeIds?: string[];
}

/** Appel `entity_relation_options` avec recherche + limite serveur. */
export async function fetchRelationOptions(
  q: RelationOptionsQuery,
): Promise<RelationSelectOption[]> {
  return invoke<RelationSelectOption[]>("entity_relation_options", {
    payload: {
      screen_key: q.screenKey,
      field_key: q.fieldKey,
      exclude_record_id: q.excludeRecordId ?? null,
      search: q.search ?? null,
      limit: q.limit ?? null,
      include_ids: q.includeIds ?? null,
    },
  });
}

/** Résout les libellés d'une liste d'IDs sans charger toute l'entité cible. */
export async function fetchRelationLabels(
  screenKey: string,
  fieldKey: string,
  ids: string[],
  excludeRecordId?: string,
): Promise<Map<string, string>> {
  const clean = ids.map((id) => id.trim()).filter(Boolean);
  if (clean.length === 0) return new Map();
  const rows = await fetchRelationOptions({
    screenKey,
    fieldKey,
    excludeRecordId,
    limit: 0,
    includeIds: clean,
  });
  return new Map(rows.filter((o) => o.value).map((o) => [o.value, o.label]));
}

interface EntityRelationAutocompleteProps {
  label?: string;
  screenKey: string;
  fieldKey: string;
  excludeRecordId?: string;
  /** ID de l'enregistrement sélectionné ("" si aucun). */
  value: string;
  /** Libellé affiché forcé (ex. copie embarquée sans ID). */
  displayLabel?: string;
  disabled?: boolean;
  error?: string;
  placeholder?: string;
  /** IDs masqués des suggestions (déjà ajoutés dans une liste). */
  excludeIds?: string[];
  /** Propose l'option « — Aucun — » (défaut : true). */
  allowEmpty?: boolean;
  onSelect: (option: RelationSelectOption) => void;
  onBlur?: () => void;
  /** Action « Créer un nouveau » en bas des suggestions. */
  onCreateNew?: () => void;
}

/**
 * Champ avec suggestions pour choisir un enregistrement d'une entité liée.
 * Recherche côté serveur au clic sur la loupe uniquement.
 */
export function EntityRelationAutocomplete({
  label,
  screenKey,
  fieldKey,
  excludeRecordId,
  value,
  displayLabel,
  disabled,
  error,
  placeholder,
  excludeIds,
  allowEmpty = true,
  onSelect,
  onBlur,
  onCreateNew,
}: EntityRelationAutocompleteProps) {
  const inputId = useId();
  const listboxId = useId();
  const containerRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [menuRect, setMenuRect] = useState<DOMRect | null>(null);
  const [query, setQuery] = useState("");
  const [options, setOptions] = useState<RelationSelectOption[]>([]);
  const [loading, setLoading] = useState(false);
  const [hasSearched, setHasSearched] = useState(false);
  const [highlighted, setHighlighted] = useState(0);
  const [resolvedLabel, setResolvedLabel] = useState("");

  useEffect(() => {
    let cancelled = false;
    const v = value.trim();
    if (!v) {
      setResolvedLabel("");
      return;
    }
    void fetchRelationLabels(screenKey, fieldKey, [v], excludeRecordId)
      .then((map) => {
        if (!cancelled) setResolvedLabel(map.get(v) ?? v);
      })
      .catch(() => {
        if (!cancelled) setResolvedLabel(v);
      });
    return () => {
      cancelled = true;
    };
  }, [value, screenKey, fieldKey, excludeRecordId]);

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
      setHighlighted(0);
    } catch {
      setOptions([]);
    } finally {
      setLoading(false);
    }
  }, [screenKey, fieldKey, excludeRecordId, query]);

  const updateMenuRect = useCallback(() => {
    const el = containerRef.current;
    if (!el) return;
    setMenuRect(el.getBoundingClientRect());
  }, []);

  useLayoutEffect(() => {
    if (!open) {
      setMenuRect(null);
      return;
    }
    updateMenuRect();
    const onScrollOrResize = () => updateMenuRect();
    window.addEventListener("scroll", onScrollOrResize, true);
    window.addEventListener("resize", onScrollOrResize);
    return () => {
      window.removeEventListener("scroll", onScrollOrResize, true);
      window.removeEventListener("resize", onScrollOrResize);
    };
  }, [open, updateMenuRect]);

  useEffect(() => {
    if (!open) return;
    const onDocMouseDown = (e: MouseEvent) => {
      const t = e.target as Node;
      if (containerRef.current?.contains(t)) return;
      const menu = document.getElementById(listboxId);
      if (menu?.contains(t)) return;
      setOpen(false);
      onBlur?.();
    };
    document.addEventListener("mousedown", onDocMouseDown);
    return () => document.removeEventListener("mousedown", onDocMouseDown);
  }, [open, onBlur, listboxId]);

  const suggestions = useMemo(() => {
    if (!excludeIds?.length) return options;
    const excluded = new Set(excludeIds);
    return options.filter((o) => !excluded.has(o.value));
  }, [options, excludeIds]);

  const currentLabel = displayLabel?.trim() || resolvedLabel;

  const resetSearchState = () => {
    setQuery("");
    setOptions([]);
    setHasSearched(false);
    setHighlighted(0);
  };

  const closeAnd = useCallback(
    (option: RelationSelectOption | null) => {
      setOpen(false);
      resetSearchState();
      if (option) onSelect(option);
      onBlur?.();
    },
    [onSelect, onBlur],
  );

  const openDropdown = () => {
    if (disabled) return;
    if (!open) resetSearchState();
    setOpen(true);
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (!open) {
      if (e.key === "ArrowDown" || e.key === "Enter") {
        e.preventDefault();
        openDropdown();
      }
      return;
    }
    if (!hasSearched || loading) return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setHighlighted((h) => Math.min(h + 1, suggestions.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setHighlighted((h) => Math.max(h - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      const opt = suggestions[highlighted];
      if (opt) closeAnd(opt);
    } else if (e.key === "Escape") {
      e.preventDefault();
      setOpen(false);
      resetSearchState();
      onBlur?.();
    }
  };

  return (
    <div className="flex flex-col gap-1.5">
      {label && (
        <label htmlFor={inputId} className="text-sm font-medium text-muted">
          {label}
        </label>
      )}
      <div ref={containerRef} className="relative">
        <input
          id={inputId}
          role="combobox"
          aria-expanded={open}
          aria-controls={open ? listboxId : undefined}
          aria-autocomplete="list"
          type={open ? "search" : "text"}
          autoComplete="off"
          disabled={disabled}
          value={open ? query : currentLabel}
          placeholder={
            open
              ? "Tapez puis cliquez sur la loupe…"
              : placeholder ?? (allowEmpty ? "— Aucun —" : "Choisir…")
          }
          className={cn(
            "w-full rounded-lg border bg-background py-2.5 text-sm text-foreground select-text",
            open ? "pl-3 pr-11" : "px-3 pr-9",
            "placeholder:text-muted/60 transition-colors duration-200",
            "focus:border-secondary focus:ring-1 focus:ring-secondary focus:outline-none",
            disabled && "cursor-not-allowed opacity-60",
            error ? "border-primary" : "border-border",
          )}
          aria-invalid={!!error}
          onFocus={openDropdown}
          onClick={() => {
            if (!open) openDropdown();
          }}
          onChange={(e) => {
            if (!open) setOpen(true);
            setQuery(e.target.value);
          }}
          onKeyDown={handleKeyDown}
        />
        {open ? (
          <button
            type="button"
            className={cn(
              "absolute right-2 top-1/2 flex h-8 w-8 -translate-y-1/2 items-center justify-center rounded-md",
              "text-foreground transition-colors hover:bg-surface-elevated hover:text-secondary",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-secondary",
              "disabled:cursor-not-allowed disabled:opacity-60",
              loading && "pointer-events-none opacity-60",
            )}
            aria-label="Lancer la recherche"
            disabled={disabled || loading}
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => void runSearch()}
          >
            <Search className="h-4 w-4" aria-hidden />
          </button>
        ) : (
          <ChevronDown
            className="pointer-events-none absolute right-3 top-1/2 h-4 w-4 -translate-y-1/2 text-foreground"
          />
        )}
        {open &&
          menuRect &&
          createPortal(
            <div
              id={listboxId}
              role="listbox"
              aria-labelledby={inputId}
              className="max-h-[min(20rem,70vh)] overflow-y-auto rounded-lg border border-border bg-card shadow-xl"
              style={{
                position: "fixed",
                top: menuRect.bottom + 4,
                left: menuRect.left,
                width: menuRect.width,
                zIndex: floatingMenuZIndex(),
              }}
            >
              {allowEmpty && (
                <button
                  type="button"
                  className="flex w-full min-w-0 border-b border-border/50 px-4 py-3 text-left text-sm text-muted transition-colors hover:bg-surface-elevated/70 last:border-b-0"
                  onMouseDown={(e) => e.preventDefault()}
                  onClick={() => closeAnd({ value: "", label: "" })}
                >
                  — Aucun —
                </button>
              )}
              {!hasSearched ? (
                <p className="px-4 py-3 text-sm text-muted">
                  Saisissez un terme puis cliquez sur la loupe pour lancer la recherche.
                </p>
              ) : loading ? (
                <p className="px-4 py-3 text-sm text-muted">Recherche…</p>
              ) : suggestions.length === 0 ? (
                <p className="px-4 py-3 text-sm text-muted">
                  {query.trim()
                    ? "Aucun résultat pour cette recherche."
                    : "Aucun enregistrement disponible."}
                </p>
              ) : (
                suggestions.map((option, idx) => (
                  <RelationOptionRow
                    key={option.value}
                    option={option}
                    active={idx === highlighted}
                    selected={option.value === value}
                    onMouseDown={(e) => e.preventDefault()}
                    onMouseEnter={() => setHighlighted(idx)}
                    onClick={() => closeAnd(option)}
                  />
                ))
              )}
              {hasSearched && !loading && suggestions.length >= RELATION_SUGGESTIONS_LIMIT && (
                <p className="border-t border-border px-3 py-1.5 text-xs text-muted">
                  {RELATION_SUGGESTIONS_LIMIT} premiers résultats — affinez votre recherche.
                </p>
              )}
              {onCreateNew && (
                <button
                  type="button"
                  className="mt-1 flex w-full items-center gap-2 rounded-md border-t border-border px-3 py-2 text-left text-sm text-secondary hover:bg-surface-elevated/80"
                  onMouseDown={(e) => e.preventDefault()}
                  onClick={() => {
                    setOpen(false);
                    resetSearchState();
                    onCreateNew();
                  }}
                >
                  <Plus className="h-4 w-4" />
                  Créer un nouveau
                </button>
              )}
            </div>,
            document.body,
          )}
      </div>
      {error && <Alert variant="danger" size="field" message={error} />}
    </div>
  );
}
