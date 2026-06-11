import {
  useCallback,
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { ChevronDown, Plus } from "lucide-react";
import { Alert } from "@/items/Alert";
import { RELATION_SUGGESTIONS_LIMIT } from "@/constants/variable.constant";
import { cn } from "@/lib/utils";
import type { RelationSelectOption } from "@/types/entity";

const SEARCH_DEBOUNCE_MS = 200;

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
 * Recherche côté serveur, max RELATION_SUGGESTIONS_LIMIT résultats —
 * remplace les <select> qui chargeaient la totalité des objets.
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
  const containerRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [options, setOptions] = useState<RelationSelectOption[]>([]);
  const [loading, setLoading] = useState(false);
  const [highlighted, setHighlighted] = useState(0);
  const [resolvedLabel, setResolvedLabel] = useState("");

  // Résolution du libellé de la valeur courante (sans tout charger).
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

  // Recherche serveur débouncée (max RELATION_SUGGESTIONS_LIMIT résultats).
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
          if (cancelled) return;
          setOptions(rows.filter((o) => o.value));
          setHighlighted(0);
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

  // Fermeture au clic extérieur.
  useEffect(() => {
    if (!open) return;
    const onDocMouseDown = (e: MouseEvent) => {
      if (!containerRef.current?.contains(e.target as Node)) {
        setOpen(false);
        onBlur?.();
      }
    };
    document.addEventListener("mousedown", onDocMouseDown);
    return () => document.removeEventListener("mousedown", onDocMouseDown);
  }, [open, onBlur]);

  const suggestions = useMemo(() => {
    if (!excludeIds?.length) return options;
    const excluded = new Set(excludeIds);
    return options.filter((o) => !excluded.has(o.value));
  }, [options, excludeIds]);

  const currentLabel = displayLabel?.trim() || resolvedLabel;

  const closeAnd = useCallback(
    (option: RelationSelectOption | null) => {
      setOpen(false);
      setQuery("");
      if (option) onSelect(option);
      onBlur?.();
    },
    [onSelect, onBlur],
  );

  const openDropdown = () => {
    if (disabled) return;
    setQuery("");
    setOptions([]);
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
          aria-autocomplete="list"
          autoComplete="off"
          disabled={disabled}
          value={open ? query : currentLabel}
          placeholder={
            open
              ? "Rechercher…"
              : placeholder ?? (allowEmpty ? "— Aucun —" : "Choisir…")
          }
          className={cn(
            "w-full rounded-lg border bg-background px-3 py-2.5 pr-9 text-sm text-foreground",
            "placeholder:text-muted/60 transition-colors duration-200",
            "focus:border-secondary focus:ring-1 focus:ring-secondary",
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
        <ChevronDown
          className={cn(
            "pointer-events-none absolute right-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted transition-transform",
            open && "rotate-180",
          )}
        />
        {open && (
          <div className="absolute left-0 right-0 top-full z-50 mt-1 max-h-72 overflow-y-auto rounded-lg border border-border bg-card p-1 shadow-xl">
            {allowEmpty && (
              <button
                type="button"
                className="flex w-full items-center rounded-md px-3 py-2 text-left text-sm text-muted hover:bg-surface-elevated/80"
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => closeAnd({ value: "", label: "" })}
              >
                — Aucun —
              </button>
            )}
            {loading ? (
              <p className="px-3 py-2 text-sm text-muted">Recherche…</p>
            ) : suggestions.length === 0 ? (
              <p className="px-3 py-2 text-sm text-muted">
                {query.trim()
                  ? "Aucun résultat pour cette recherche."
                  : "Aucun enregistrement disponible."}
              </p>
            ) : (
              suggestions.map((option, idx) => (
                <button
                  key={option.value}
                  type="button"
                  className={cn(
                    "flex w-full items-center justify-between gap-2 rounded-md px-3 py-2 text-left text-sm text-foreground",
                    idx === highlighted
                      ? "bg-surface-elevated"
                      : "hover:bg-surface-elevated/80",
                    option.value === value && "text-secondary",
                  )}
                  onMouseDown={(e) => e.preventDefault()}
                  onMouseEnter={() => setHighlighted(idx)}
                  onClick={() => closeAnd(option)}
                >
                  <span className="truncate">{option.label}</span>
                </button>
              ))
            )}
            {!loading && suggestions.length >= RELATION_SUGGESTIONS_LIMIT && (
              <p className="border-t border-border px-3 py-1.5 text-xs text-muted">
                {RELATION_SUGGESTIONS_LIMIT} premiers résultats — affinez votre
                recherche.
              </p>
            )}
            {onCreateNew && (
              <button
                type="button"
                className="mt-1 flex w-full items-center gap-2 rounded-md border-t border-border px-3 py-2 text-left text-sm text-secondary hover:bg-surface-elevated/80"
                onMouseDown={(e) => e.preventDefault()}
                onClick={() => {
                  setOpen(false);
                  setQuery("");
                  onCreateNew();
                }}
              >
                <Plus className="h-4 w-4" />
                Créer un nouveau
              </button>
            )}
          </div>
        )}
      </div>
      {error && <Alert variant="danger" size="field" message={error} />}
    </div>
  );
}
