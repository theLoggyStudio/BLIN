import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Plus } from "lucide-react";
import { FieldRenderer } from "@/engine/FieldRenderer";
import { FieldReadOnlyValue } from "@/engine/FieldReadOnlyValue";
import { EntityRelationAutocomplete } from "@/items/EntityRelationAutocomplete";
import { EntityRelationCreateModal } from "@/items/EntityRelationCreateModal";
import { EntityRelationPickOrCreateModal } from "@/items/EntityRelationPickOrCreateModal";
import { Alert } from "@/items/Alert";
import { Button } from "@/items/Button";
import { CollapsiblePanel } from "@/items/CollapsiblePanel";
import { Input } from "@/items/Input";
import { embedRefKey } from "@/lib/createFormLines";
import { blurActiveElement } from "@/lib/focus";
import type { RelationSelectOption } from "@/types/entity";
import type { FieldDef, ScreenRow, ValidationIssue } from "@/types/screen";

interface EmbedImpactMeta {
  qtyField: string;
  action: "increment" | "decrement";
  label: string;
  originEntityKey: string;
  qtyReadOnly: boolean;
  childEntityKey: string;
  stockCapField?: string | null;
}

const DA_ENTITY_KEY = "demande_d'achat";
const EMBED_SOURCE_ID = "_source_record_id";
const EMBED_STOCK_CAP = "_embedStockCap";

function rowSourceId(row: Record<string, unknown>): string | null {
  const id = row[EMBED_SOURCE_ID];
  return id != null && String(id).trim() ? String(id).trim() : null;
}

function rowCapKey(row: Record<string, unknown>, idx: number): string {
  return rowSourceId(row) ?? String(row.reference ?? row.nom ?? idx);
}

function clampQtyInput(raw: string, max?: number): string {
  const trimmed = raw.trim();
  if (!trimmed) return trimmed;
  const n = Number(trimmed);
  if (!Number.isFinite(n)) return raw;
  let v = Math.max(0, n);
  if (max != null && Number.isFinite(max)) v = Math.min(v, max);
  return Number.isInteger(v) ? String(Math.trunc(v)) : String(v);
}

function pickDefaultImpactQty(
  child: Record<string, unknown>,
  qtyField: string,
  max?: number,
): string {
  const direct = child[qtyField];
  if (direct != null && String(direct).trim() !== "") {
    return clampQtyInput(String(direct), max);
  }
  const cap = child[EMBED_STOCK_CAP];
  if (cap != null && String(cap).trim() !== "") {
    return clampQtyInput(String(cap), max);
  }
  return "";
}

function parseEmbedListValue(value: unknown): Record<string, unknown>[] {
  if (Array.isArray(value)) {
    return value.filter((v) => v && typeof v === "object") as Record<string, unknown>[];
  }
  if (typeof value === "string") {
    const trimmed = value.trim();
    if (!trimmed) return [];
    try {
      const parsed = JSON.parse(trimmed);
      if (Array.isArray(parsed)) {
        return parsed.filter((v) => v && typeof v === "object") as Record<string, unknown>[];
      }
    } catch {
      return [];
    }
  }
  return [];
}

function EmbedNestedArticlesReadonly({
  articles,
  qtyField = "qte_initial",
}: {
  articles: unknown;
  qtyField?: string;
}) {
  const rows = parseEmbedListValue(articles);
  if (rows.length === 0) return null;
  return (
    <ul className="m-0 list-none space-y-1 border-l-2 border-border pl-3 pt-1">
      {rows.map((art, i) => (
        <li key={`nested-art-${i}`} className="text-xs text-muted">
          {rowLabel(art)}
          {art[qtyField] != null && String(art[qtyField]).trim() !== "" && (
            <span className="text-foreground">
              {" "}
              — {embedFieldLabel(qtyField)} : {String(art[qtyField])}
            </span>
          )}
        </li>
      ))}
    </ul>
  );
}

function readChildValue(values: ScreenRow, field: FieldDef): unknown {
  return values[field.key] ?? values[field.column ?? ""] ?? undefined;
}

function hasCopiedEmbedData(
  values: ScreenRow,
  childFields: FieldDef[],
): boolean {
  return childFields.some((cf) => {
    const v = readChildValue(values, cf);
    return v != null && String(v).trim() !== "";
  });
}

function labelFromCopiedFields(
  values: ScreenRow,
  childFields: FieldDef[],
): string {
  for (const cf of childFields) {
    for (const key of ["libelle", "nom", "titre", "reference", "intitule", cf.key]) {
      const v = values[key] ?? readChildValue(values, cf);
      if (v != null && String(v).trim()) return String(v).trim();
    }
  }
  return "Client embarqué";
}

function rowLabel(row: Record<string, unknown>): string {
  for (const key of ["libelle", "nom", "titre", "reference", "intitule"]) {
    const v = row[key];
    if (v != null && String(v).trim()) return String(v).trim();
  }
  const first = Object.values(row).find((v) => v != null && String(v).trim());
  return first != null ? String(first).trim() : "Élément";
}

function embedFieldLabel(key: string): string {
  const labels: Record<string, string> = {
    nom: "Nom",
    qte: "Quantité",
    libelle: "Libellé",
    reference: "Référence",
    intitule: "Intitulé",
  };
  return labels[key] ?? key;
}

function formatEmbedRowText(
  row: Record<string, unknown>,
  keys: readonly string[],
): string {
  const filled = keys
    .map((key) => {
      const v = row[key];
      if (v == null || !String(v).trim()) return null;
      return { key, text: String(v).trim() };
    })
    .filter(Boolean) as { key: string; text: string }[];

  if (filled.length === 0) return rowLabel(row);
  if (filled.length === 1) return filled[0].text;
  return filled.map(({ key, text }) => `${embedFieldLabel(key)} : ${text}`).join(" · ");
}

function EmbedGroupDisplayOnly({
  field,
  childFields,
  values,
  screenKey,
  excludeRecordId,
}: {
  field: FieldDef;
  childFields: FieldDef[];
  values: ScreenRow;
  screenKey: string;
  excludeRecordId?: string;
}) {
  const visibleFields = childFields.filter((cf) => {
    const v = readChildValue(values, cf);
    return v != null && String(v).trim() !== "";
  });

  return (
    <div className="sm:col-span-2 space-y-2">
      <p className="text-xs font-medium uppercase tracking-wide text-muted">{field.label}</p>
      {visibleFields.length === 0 ? (
        <p className="text-sm text-foreground">—</p>
      ) : (
        <dl className="grid gap-3 sm:grid-cols-2">
          {visibleFields.map((childField) => (
            <FieldReadOnlyValue
              key={childField.key}
              field={childField}
              value={readChildValue(values, childField)}
              screenKey={screenKey}
              excludeRecordId={excludeRecordId}
            />
          ))}
        </dl>
      )}
    </div>
  );
}

function EmbedListDisplayOnly({
  field,
  rows,
  rowEditKeys,
}: {
  field: FieldDef;
  rows: Record<string, unknown>[];
  rowEditKeys: readonly string[];
}) {
  const keys =
    rowEditKeys.length > 0
      ? rowEditKeys
      : (["libelle", "nom", "qte", "reference"] as const);

  return (
    <div className="sm:col-span-2 space-y-2">
      <p className="text-xs font-medium uppercase tracking-wide text-muted">{field.label}</p>
      {rows.length === 0 ? (
        <p className="text-sm text-foreground">—</p>
      ) : (
        <ul className="m-0 list-none space-y-1 p-0">
          {rows.map((row, idx) => (
            <li key={`${field.key}-${idx}`} className="text-sm text-foreground">
              {formatEmbedRowText(row, keys)}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

interface EntityEmbedGroupProps {
  field: FieldDef;
  allFields: FieldDef[];
  values: ScreenRow;
  readOnly?: boolean;
  displayOnly?: boolean;
  screenKey: string;
  uploadDraftId: string;
  storageFolders?: string[];
  excludeRecordId?: string;
  onChange: (key: string, value: unknown) => void;
  onBatchChange?: (updates: Record<string, unknown>) => void;
  onBlur?: (key: string) => void;
  fieldErrors?: Record<string, ValidationIssue>;
  fieldWarnings?: Record<string, ValidationIssue>;
}

/** One-to-one : champs dupliqués de l'entité fille dans la table mère. */
export function EntityEmbedGroup({
  field,
  allFields,
  values,
  readOnly,
  displayOnly,
  screenKey,
  uploadDraftId,
  storageFolders,
  excludeRecordId,
  onChange,
  onBatchChange,
  onBlur,
  fieldErrors,
  fieldWarnings,
}: EntityEmbedGroupProps) {
  const refEntity = field.form?.refEntity?.trim() ?? "";
  const childFields = useMemo(
    () => allFields.filter((f) => f.form?.embedParent === field.key),
    [allFields, field.key],
  );
  const storedRefKey = embedRefKey(field.key);
  const [selectedRecordId, setSelectedRecordId] = useState("");
  const [pickOpen, setPickOpen] = useState(false);
  const [createOpen, setCreateOpen] = useState(false);

  const copiedLabel = useMemo(() => {
    if (!hasCopiedEmbedData(values, childFields)) return "";
    return labelFromCopiedFields(values, childFields);
  }, [values, childFields]);

  useEffect(() => {
    const stored = values[storedRefKey];
    if (stored != null && String(stored).trim()) {
      setSelectedRecordId(String(stored));
      return;
    }
    if (!hasCopiedEmbedData(values, childFields)) {
      setSelectedRecordId("");
    }
  }, [values, storedRefKey, childFields]);

  const applyCopied = async (recordId: string) => {
    const copied = await invoke<Record<string, unknown>>("entity_embed_values_from_record", {
      payload: {
        screen_key: screenKey,
        field_key: field.key,
        record_id: recordId,
      },
    });
    const updates = { ...copied, [storedRefKey]: recordId };
    if (onBatchChange) {
      onBatchChange(updates);
    } else {
      for (const [k, v] of Object.entries(updates)) onChange(k, v);
    }
  };

  const tryPick = async (option: RelationSelectOption) => {
    if (!option.value) return;
    await applyCopied(option.value);
    setSelectedRecordId(option.value);
    setPickOpen(false);
  };

  const handleCreated = async (row: ScreenRow) => {
    const id = row.id != null ? String(row.id) : "";
    if (!id) return;
    await applyCopied(id);
    setSelectedRecordId(id);
    setCreateOpen(false);
  };

  if (displayOnly) {
    return (
      <EmbedGroupDisplayOnly
        field={field}
        childFields={childFields}
        values={values}
        screenKey={screenKey}
        excludeRecordId={excludeRecordId}
      />
    );
  }

  return (
    <>
      <CollapsiblePanel
        title={field.label}
        subtitle="Copie embarquée (one-to-one) — indépendante des autres liaisons"
        defaultOpen
        headerAction={
          !readOnly && !displayOnly && refEntity ? (
            <div className="flex gap-2">
              <Button size="sm" variant="secondary" type="button" onClick={() => {
                blurActiveElement();
                setPickOpen(true);
              }}>
                Choisir
              </Button>
              <Button size="sm" variant="outline" type="button" onClick={() => setCreateOpen(true)}>
                <Plus className="mr-1 h-3.5 w-3.5" />
                Créer
              </Button>
            </div>
          ) : undefined
        }
      >
        <div className="space-y-3">
          {!displayOnly && (
            <p className="text-xs text-muted">
              Choisissez un enregistrement existant : ses champs sont copiés ici (pas de liaison par ID).
              Replier ce bloc n&apos;affecte pas les autres liaisons (ex. articles).
            </p>
          )}
          {!readOnly && !displayOnly && refEntity && (
            <EntityRelationAutocomplete
              label={`Choisir — ${field.label}`}
              screenKey={screenKey}
              fieldKey={field.key}
              excludeRecordId={excludeRecordId}
              value={selectedRecordId}
              displayLabel={!selectedRecordId ? copiedLabel : undefined}
              placeholder={`— Choisir un ${refEntity} —`}
              onSelect={(option) => {
                if (!option.value) {
                  setSelectedRecordId("");
                  if (onBatchChange) onBatchChange({ [storedRefKey]: "" });
                  else onChange(storedRefKey, "");
                  return;
                }
                void tryPick(option);
              }}
              onCreateNew={() => setCreateOpen(true)}
            />
          )}
          {childFields.map((childField) => (
            <FieldRenderer
              key={childField.key}
              field={childField}
              allFields={allFields}
              values={values}
              onChange={onChange}
              onBatchChange={onBatchChange}
              onBlur={onBlur}
              readOnly={readOnly}
              displayOnly={displayOnly}
              fieldError={fieldErrors?.[childField.key]}
              fieldWarning={fieldWarnings?.[childField.key]}
              screenKey={screenKey}
              uploadDraftId={uploadDraftId}
              storageFolders={storageFolders}
              excludeRecordId={excludeRecordId}
            />
          ))}
        </div>
      </CollapsiblePanel>

      {pickOpen && (
        <EntityRelationPickOrCreateModal
          entityKey={refEntity}
          open={pickOpen}
          onClose={() => setPickOpen(false)}
          screenKey={screenKey}
          fieldKey={field.key}
          excludeRecordId={excludeRecordId}
          excludeIds={[]}
          embedMode
          onSelected={(id) => void tryPick({ value: id, label: id })}
        />
      )}
      {refEntity && (
        <EntityRelationCreateModal
          entityKey={refEntity}
          open={createOpen}
          onClose={() => setCreateOpen(false)}
          onCreated={(row) => void handleCreated(row)}
        />
      )}
    </>
  );
}

interface EntityEmbedListEditorProps {
  field: FieldDef;
  value: unknown;
  readOnly?: boolean;
  displayOnly?: boolean;
  screenKey: string;
  excludeRecordId?: string;
  /** Enregistrement parent en cours (verrouillage quantité après signature DA). */
  recordId?: string;
  fieldError?: ValidationIssue;
  onChange: (key: string, value: unknown) => void;
  onBlur?: (key: string) => void;
}

/** One-to-many : tableau JSON de copies embarquées. */
export function EntityEmbedListEditor({
  field,
  value,
  readOnly,
  displayOnly,
  screenKey,
  excludeRecordId,
  recordId,
  fieldError,
  onChange,
  onBlur,
}: EntityEmbedListEditorProps) {
  const refEntity = field.form?.refEntity?.trim() ?? "";
  const rows = parseEmbedListValue(value);
  const [pickOpen, setPickOpen] = useState(false);
  const [impactMeta, setImpactMeta] = useState<EmbedImpactMeta | null>(null);
  const [stockCaps, setStockCaps] = useState<Record<string, number>>({});
  const editingRecordId = recordId ?? excludeRecordId;

  useEffect(() => {
    if (!refEntity || !screenKey || !field.key) {
      setImpactMeta(null);
      return;
    }
    let cancelled = false;
    void (async () => {
      try {
        const meta = await invoke<EmbedImpactMeta | null>("entity_embed_impact_meta", {
          payload: {
            screen_key: screenKey,
            field_key: field.key,
            record_id: editingRecordId ?? null,
          },
        });
        if (!cancelled) setImpactMeta(meta);
      } catch {
        if (!cancelled) setImpactMeta(null);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [refEntity, screenKey, field.key, editingRecordId]);

  useEffect(() => {
    if (!impactMeta?.stockCapField || impactMeta.action !== "decrement") {
      setStockCaps({});
      return;
    }
    let cancelled = false;
    void (async () => {
      const caps: Record<string, number> = {};
      for (let idx = 0; idx < rows.length; idx++) {
        const row = rows[idx];
        const key = rowCapKey(row, idx);
        const embeddedCap = row[EMBED_STOCK_CAP];
        if (embeddedCap != null && String(embeddedCap).trim() !== "") {
          const n = Number(embeddedCap);
          if (Number.isFinite(n)) caps[key] = n;
        }
        try {
          const live = await invoke<number | null>("entity_child_numeric_field", {
            payload: {
              entity_key: impactMeta.childEntityKey,
              field_key: impactMeta.stockCapField,
              record_id: rowSourceId(row),
              reference:
                row.reference != null && String(row.reference).trim()
                  ? String(row.reference)
                  : null,
            },
          });
          if (live != null && Number.isFinite(live)) caps[key] = live;
        } catch {
          /* stock live indisponible — garde le plafond embarqué */
        }
      }
      if (!cancelled) setStockCaps(caps);
    })();
    return () => {
      cancelled = true;
    };
  }, [rows, impactMeta]);

  const qtyLabel = impactMeta
    ? impactMeta.action === "decrement"
      ? `${impactMeta.label} (à retirer)`
      : `${impactMeta.label} (à ajouter)`
    : "";

  const qtyLocked = Boolean(impactMeta?.qtyReadOnly);

  const persist = useCallback(
    (next: Record<string, unknown>[]) => {
      onChange(field.key, JSON.stringify(next));
      onBlur?.(field.key);
    },
    [field.key, onChange, onBlur],
  );

  const appendFromRecord = async (pickedId: string) => {
    const child = await invoke<Record<string, unknown>>("entity_embed_child_from_record", {
      payload: {
        screen_key: screenKey,
        field_key: field.key,
        record_id: pickedId,
      },
    });
    child[EMBED_SOURCE_ID] = pickedId;
    if (impactMeta && screenKey === impactMeta.originEntityKey) {
      const capKey = rowCapKey(child, rows.length);
      const cap =
        child[EMBED_STOCK_CAP] != null
          ? Number(child[EMBED_STOCK_CAP])
          : undefined;
      const defaultQty = pickDefaultImpactQty(
        child,
        impactMeta.qtyField,
        cap != null && Number.isFinite(cap) ? cap : stockCaps[capKey],
      );
      if (defaultQty) child[impactMeta.qtyField] = defaultQty;
      if (cap != null && Number.isFinite(cap)) {
        setStockCaps((prev) => ({ ...prev, [capKey]: cap }));
      }
    }
    persist([...rows, child]);
  };

  const tryPick = async (option: RelationSelectOption) => {
    if (!option.value) return;
    await appendFromRecord(option.value);
    setPickOpen(false);
  };

  const listSubtitle =
    refEntity === "articles"
      ? "Plusieurs articles embarqués — quantité saisie ici, figée après signature DA"
      : refEntity === "demande_d'achat" || refEntity === "demande_dachat"
        ? "Demandes d'achat embarquées (articles et quantités hérités)"
        : refEntity
          ? `Éléments embarqués (${refEntity})`
          : "Liste embarquée (one-to-many)";

  const rowEditKeys = useMemo(() => {
    const keys = ["nom", "qte", "libelle"] as const;
    const base = keys.filter(
      (k) => refEntity === "articles" || rows.some((r) => r[k] !== undefined && r[k] !== null),
    );
    const qtyKey = impactMeta?.qtyField ?? "qte_initial";
    if (rows.some((r) => r[qtyKey] !== undefined && r[qtyKey] !== null)) {
      return [...base, qtyKey];
    }
    return base;
  }, [refEntity, rows, impactMeta?.qtyField]);

  const updateRow = (idx: number, key: string, val: string) => {
    let nextVal = val;
    if (impactMeta && key === impactMeta.qtyField) {
      const cap = stockCaps[rowCapKey(rows[idx], idx)];
      nextVal = clampQtyInput(val, cap);
    }
    const next = rows.map((r, i) => (i === idx ? { ...r, [key]: nextVal } : r));
    persist(next);
  };

  const locked = readOnly || displayOnly;
  const showDaNestedArticles =
    refEntity === DA_ENTITY_KEY || refEntity === "demande_dachat";

  if (displayOnly) {
    return (
      <CollapsiblePanel title={field.label} subtitle={listSubtitle} defaultOpen>
        <div className="space-y-2">
          {rows.length === 0 ? (
            <p className="text-sm text-foreground">—</p>
          ) : (
            rows.map((row, idx) => (
              <div
                key={`embed-ro-${idx}`}
                className="rounded-md border border-border bg-background px-3 py-2"
              >
                <p className="text-sm text-foreground">
                  {formatEmbedRowText(row, rowEditKeys)}
                </p>
                {showDaNestedArticles && (
                  <EmbedNestedArticlesReadonly
                    articles={row.articles}
                    qtyField={impactMeta?.qtyField ?? "qte_initial"}
                  />
                )}
              </div>
            ))
          )}
        </div>
      </CollapsiblePanel>
    );
  }

  return (
    <>
      <CollapsiblePanel
        title={field.label}
        subtitle={listSubtitle}
        defaultOpen
        headerAction={
          !readOnly && !displayOnly && refEntity ? (
            <Button size="sm" variant="secondary" type="button" onClick={() => {
              blurActiveElement();
              setPickOpen(true);
            }}>
              Ajouter
            </Button>
          ) : undefined
        }
      >
        <div className="space-y-2">
          {!refEntity && (
            <Alert
              variant="danger"
              size="inline"
              message="Entité liée introuvable dans la configuration."
            />
          )}
          {fieldError?.message && (
            <Alert variant="danger" size="box" message={fieldError.message} />
          )}
          {rows.length === 0 && (
            <p className="text-xs text-muted">Aucune ligne — cliquez sur « Ajouter ».</p>
          )}
          {rows.map((row, idx) => (
            <div
              key={`embed-row-${idx}`}
              className="flex flex-col gap-2 rounded-md border border-border bg-background px-3 py-2 sm:flex-row sm:items-start sm:justify-between"
            >
              <div className="min-w-0 flex-1 space-y-2">
                {!locked && rowEditKeys.length > 0 ? (
                  rowEditKeys.map((key) => (
                    <Input
                      key={`${idx}-${key}`}
                      label={key === "qte" ? "Quantité" : key === "nom" ? "Nom" : "Libellé"}
                      value={String(row[key] ?? "")}
                      onChange={(e) => updateRow(idx, key, e.target.value)}
                      onBlur={() => onBlur?.(field.key)}
                    />
                  ))
                ) : (
                  <span className="text-sm text-foreground">{rowLabel(row)}</span>
                )}
                {showDaNestedArticles && (
                  <EmbedNestedArticlesReadonly
                    articles={row.articles}
                    qtyField={impactMeta?.qtyField ?? "qte_initial"}
                  />
                )}
              </div>
              <div className="flex shrink-0 items-end gap-2 sm:justify-end">
                {impactMeta && (
                  <div className="w-36">
                    <Input
                      type="number"
                      label={qtyLabel}
                      value={String(row[impactMeta.qtyField] ?? "")}
                      readOnly={locked || qtyLocked}
                      min={0}
                      max={stockCaps[rowCapKey(row, idx)]}
                      step={1}
                      hint={
                        stockCaps[rowCapKey(row, idx)] != null
                          ? `Max : ${stockCaps[rowCapKey(row, idx)]} (stock article)`
                          : undefined
                      }
                      onChange={(e) => updateRow(idx, impactMeta.qtyField, e.target.value)}
                      onBlur={() => onBlur?.(field.key)}
                    />
                  </div>
                )}
                {!locked && (
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => persist(rows.filter((_, i) => i !== idx))}
                  >
                    Retirer
                  </Button>
                )}
              </div>
            </div>
          ))}
        </div>
      </CollapsiblePanel>
      {pickOpen && refEntity && (
        <EntityRelationPickOrCreateModal
          entityKey={refEntity}
          open={pickOpen}
          onClose={() => setPickOpen(false)}
          screenKey={screenKey}
          fieldKey={field.key}
          excludeRecordId={excludeRecordId}
          excludeIds={[]}
          embedMode
          onSelected={(id) => void tryPick({ value: id, label: id })}
        />
      )}
    </>
  );
}
