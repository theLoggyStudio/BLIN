import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Plus } from "lucide-react";
import { FieldRenderer } from "@/engine/FieldRenderer";
import { FieldReadOnlyValue } from "@/engine/FieldReadOnlyValue";
import { EntityRelationCreateModal } from "@/items/EntityRelationCreateModal";
import { EntityRelationPickOrCreateModal } from "@/items/EntityRelationPickOrCreateModal";
import { Alert } from "@/items/Alert";
import { Button } from "@/items/Button";
import { CollapsiblePanel } from "@/items/CollapsiblePanel";
import { Input } from "@/items/Input";
import { Select } from "@/items/Select";
import { embedRefKey } from "@/lib/createFormLines";
import type { RelationSelectOption } from "@/types/entity";
import type { FieldDef, ScreenRow, ValidationIssue } from "@/types/screen";

const NO_OPTIONS_ALERT =
  "Aucun enregistrement disponible — créez-en un nouveau via le bouton « Créer ».";

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
  const [options, setOptions] = useState<RelationSelectOption[]>([]);
  const [selectedRecordId, setSelectedRecordId] = useState("");
  const [pickOpen, setPickOpen] = useState(false);
  const [createOpen, setCreateOpen] = useState(false);

  const copiedLabel = useMemo(() => {
    if (!hasCopiedEmbedData(values, childFields)) return "";
    return labelFromCopiedFields(values, childFields);
  }, [values, childFields]);

  const selectValue = useMemo(() => {
    if (selectedRecordId) return selectedRecordId;
    if (copiedLabel) return `__copied__${field.key}`;
    return "";
  }, [selectedRecordId, copiedLabel, field.key]);

  const loadOptions = useCallback(async () => {
    if (!refEntity) return;
    try {
      const rows = await invoke<RelationSelectOption[]>("entity_relation_options", {
        payload: {
          screen_key: screenKey,
          field_key: field.key,
          exclude_record_id: excludeRecordId ?? null,
        },
      });
      setOptions(rows);
    } catch {
      setOptions([]);
    }
  }, [screenKey, field.key, excludeRecordId, refEntity]);

  useEffect(() => {
    void loadOptions();
  }, [loadOptions]);

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
    void loadOptions();
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
              <Button size="sm" variant="secondary" type="button" onClick={() => setPickOpen(true)}>
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
          {!readOnly && !displayOnly && refEntity && (options.some((o) => o.value) || copiedLabel) && (
            <Select
              label={`Choisir — ${field.label}`}
              value={selectValue}
              onChange={(e) => {
                const id = e.target.value;
                if (!id || id.startsWith("__copied__")) {
                  if (!id) {
                    setSelectedRecordId("");
                    if (onBatchChange) onBatchChange({ [storedRefKey]: "" });
                    else onChange(storedRefKey, "");
                  }
                  return;
                }
                const opt = options.find((o) => o.value === id);
                if (opt) void tryPick(opt);
              }}
              options={[
                { value: "", label: `— Choisir un ${refEntity} —` },
                ...(copiedLabel && !selectedRecordId
                  ? [{ value: `__copied__${field.key}`, label: copiedLabel }]
                  : []),
                ...options
                  .filter((o) => o.value)
                  .map((o) => ({ value: o.value, label: o.label })),
                ...(selectedRecordId && !options.some((o) => o.value === selectedRecordId)
                  ? [{ value: selectedRecordId, label: copiedLabel || selectedRecordId }]
                  : []),
              ]}
            />
          )}
          {!readOnly && !displayOnly && refEntity && options.filter((o) => o.value).length === 0 && (
            <Alert variant="warning" size="box" message={NO_OPTIONS_ALERT} />
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
          options={options}
          excludeIds={[]}
          embedMode
          onSelected={(id) => void tryPick({ value: id, label: id })}
          onOptionsRefresh={() => void loadOptions()}
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
  fieldError,
  onChange,
  onBlur,
}: EntityEmbedListEditorProps) {
  const refEntity = field.form?.refEntity?.trim() ?? "";
  const rows = parseEmbedListValue(value);
  const [options, setOptions] = useState<RelationSelectOption[]>([]);
  const [optionsError, setOptionsError] = useState<string | null>(null);
  const [pickOpen, setPickOpen] = useState(false);

  const load = useCallback(async () => {
    if (!refEntity) return;
    try {
      const fetched = await invoke<RelationSelectOption[]>("entity_relation_options", {
        payload: {
          screen_key: screenKey,
          field_key: field.key,
          exclude_record_id: excludeRecordId ?? null,
        },
      });
      setOptions(fetched);
      setOptionsError(null);
    } catch (e) {
      setOptions([]);
      setOptionsError(
        e instanceof Error ? e.message : "Impossible de charger les enregistrements liés.",
      );
    }
  }, [screenKey, field.key, excludeRecordId, refEntity]);

  useEffect(() => {
    void load();
  }, [load]);

  const persist = (next: Record<string, unknown>[]) => {
    onChange(field.key, JSON.stringify(next));
    onBlur?.(field.key);
  };

  const appendFromRecord = async (recordId: string) => {
    const child = await invoke<Record<string, unknown>>("entity_embed_child_from_record", {
      payload: {
        screen_key: screenKey,
        field_key: field.key,
        record_id: recordId,
      },
    });
    persist([...rows, child]);
  };

  const tryPick = async (option: RelationSelectOption) => {
    if (!option.value) return;
    await appendFromRecord(option.value);
    setPickOpen(false);
  };

  const listSubtitle =
    refEntity === "articles"
      ? "Plusieurs articles embarqués"
      : refEntity === "demande_dachat"
        ? "Demandes d'achat embarquées"
        : refEntity
          ? `Éléments embarqués (${refEntity})`
          : "Liste embarquée (one-to-many)";

  const rowEditKeys = useMemo(() => {
    const keys = ["nom", "qte", "libelle"] as const;
    return keys.filter(
      (k) => refEntity === "articles" || rows.some((r) => r[k] !== undefined && r[k] !== null),
    );
  }, [refEntity, rows]);

  const updateRow = (idx: number, key: string, val: string) => {
    const next = rows.map((r, i) => (i === idx ? { ...r, [key]: val } : r));
    persist(next);
  };

  const locked = readOnly || displayOnly;

  if (displayOnly) {
    return (
      <EmbedListDisplayOnly field={field} rows={rows} rowEditKeys={rowEditKeys} />
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
            <Button size="sm" variant="secondary" type="button" onClick={() => setPickOpen(true)}>
              Ajouter
            </Button>
          ) : undefined
        }
      >
        <div className="space-y-2">
          {optionsError && <Alert variant="danger" size="box" message={optionsError} />}
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
              </div>
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
          ))}
        </div>
      </CollapsiblePanel>
      {pickOpen && refEntity && (
        <EntityRelationPickOrCreateModal
          entityKey={refEntity}
          open={pickOpen}
          onClose={() => setPickOpen(false)}
          options={options}
          excludeIds={[]}
          embedMode
          onSelected={(id) => void tryPick({ value: id, label: id })}
          onOptionsRefresh={() => void load()}
        />
      )}
    </>
  );
}
