import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ReactNode } from "react";
import { usePrivilege } from "@/hooks/usePrivilege";
import { EntityEmbedGroup, EntityEmbedListEditor } from "@/items/EntityEmbedField";
import { EntityRelationCreateModal } from "@/items/EntityRelationCreateModal";
import { EntityRelationPickOrCreateModal } from "@/items/EntityRelationPickOrCreateModal";
import { TacheRolesVisibleField } from "@/items/TacheRolesVisibleField";
import { Button } from "@/items/Button";
import { Input } from "@/items/Input";
import { Select } from "@/items/Select";
import { FieldMessages } from "@/items/FieldMessages";
import { ImageField } from "@/items/ImageField";
import { ImagesField } from "@/items/ImagesField";
import type { FieldDef, ScreenRow, ValidationIssue } from "@/types/screen";
import type { RelationSelectOption } from "@/types/entity";
import { defaultStorageFolder, mediaEntityId } from "./mediaUtils";
import { isFieldVisible } from "./screenUtils";
import { toDateInputValue, toDatetimeLocalValue, toTimeInputValue } from "@/lib/dateInputValues";
import { cn } from "@/lib/utils";

interface FieldRendererProps {
  field: FieldDef;
  values: ScreenRow;
  onChange: (key: string, value: unknown) => void;
  onBatchChange?: (updates: Record<string, unknown>) => void;
  onBlur?: (key: string) => void;
  readOnly?: boolean;
  fieldError?: ValidationIssue;
  fieldWarning?: ValidationIssue;
  screenKey: string;
  uploadDraftId: string;
  storageFolders?: string[];
  /** Pour les liaisons entity_ref : exclure l'enregistrement en cours de la règle d'exclusivité parent. */
  excludeRecordId?: string;
  /** Tous les champs de l'écran (groupes embarqués). */
  allFields?: FieldDef[];
  fieldErrorsMap?: Record<string, ValidationIssue>;
  fieldWarningsMap?: Record<string, ValidationIssue>;
}

/** Valeur réservée de la première option « Créer un nouveau ». */
export const ENTITY_REF_CREATE_NEW = "__blin_create_new__";

function parseEntityRefListValue(value: unknown): string[] {
  if (Array.isArray(value)) {
    return value.map((v) => String(v ?? "").trim()).filter(Boolean);
  }
  if (typeof value === "string") {
    const trimmed = value.trim();
    if (!trimmed) return [];
    try {
      const parsed = JSON.parse(trimmed);
      if (Array.isArray(parsed)) {
        return parsed.map((v) => String(v ?? "").trim()).filter(Boolean);
      }
    } catch {
      // support legacy comma-separated values
      return trimmed
        .split(",")
        .map((s) => s.trim())
        .filter(Boolean);
    }
  }
  return [];
}

function EntityRefSelect({
  field,
  value,
  readOnly,
  screenKey,
  excludeRecordId,
  fieldError,
  onChange,
  onBlur,
}: {
  field: FieldDef;
  value: string;
  readOnly?: boolean;
  screenKey: string;
  excludeRecordId?: string;
  fieldError?: ValidationIssue;
  onChange: (key: string, value: unknown) => void;
  onBlur?: (key: string) => void;
}) {
  const refEntity = field.form?.refEntity?.trim() ?? "";
  const canCreate = usePrivilege(refEntity ? `${refEntity}:creer` : "");
  const [options, setOptions] = useState<RelationSelectOption[]>([]);
  const [createOpen, setCreateOpen] = useState(false);
  const selectValueRef = useRef(value);

  useEffect(() => {
    selectValueRef.current = value;
  }, [value]);

  const load = useCallback(async () => {
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
      setOptions([{ value: "", label: "— Aucun —" }]);
    }
  }, [screenKey, field.key, excludeRecordId]);

  useEffect(() => {
    void load();
  }, [load]);

  const merged =
    value && !options.some((o) => o.value === value)
      ? [...options, { value, label: value }]
      : options;

  const selectOptions = [
    ...(canCreate && refEntity && !readOnly
      ? [{ value: ENTITY_REF_CREATE_NEW, label: "Créer un nouveau" }]
      : []),
    ...merged.map((o) => ({ value: o.value, label: o.label })),
  ];

  const handleCreated = (row: ScreenRow) => {
    const id = row.id != null ? String(row.id) : "";
    if (!id) return;
    onChange(field.key, id);
    void load();
  };

  return (
    <>
      <Select
        label={field.label}
        value={value}
        disabled={readOnly}
        error={fieldError?.message}
        onChange={(e) => {
          const v = e.target.value;
          if (v === ENTITY_REF_CREATE_NEW) {
            setCreateOpen(true);
            return;
          }
          selectValueRef.current = v;
          onChange(field.key, v);
        }}
        onBlur={() => onBlur?.(field.key)}
        options={selectOptions}
      />
      {refEntity && (
        <EntityRelationCreateModal
          entityKey={refEntity}
          open={createOpen}
          onClose={() => setCreateOpen(false)}
          onCreated={handleCreated}
        />
      )}
    </>
  );
}

function EntityRefListEditor({
  field,
  value,
  readOnly,
  screenKey,
  excludeRecordId,
  fieldError,
  onChange,
  onBlur,
}: {
  field: FieldDef;
  value: unknown;
  readOnly?: boolean;
  screenKey: string;
  excludeRecordId?: string;
  fieldError?: ValidationIssue;
  onChange: (key: string, value: unknown) => void;
  onBlur?: (key: string) => void;
}) {
  const [options, setOptions] = useState<RelationSelectOption[]>([]);
  const [pickOpen, setPickOpen] = useState(false);
  const rows = parseEntityRefListValue(value);
  const refEntity = field.form?.refEntity?.trim() ?? "";

  const load = useCallback(async () => {
    try {
      const fetched = await invoke<RelationSelectOption[]>("entity_relation_options", {
        payload: {
          screen_key: screenKey,
          field_key: field.key,
          exclude_record_id: excludeRecordId ?? null,
        },
      });
      setOptions(fetched);
    } catch {
      setOptions([{ value: "", label: "— Aucun —" }]);
    }
  }, [screenKey, field.key, excludeRecordId]);

  useEffect(() => {
    void load();
  }, [load]);

  const labelById = (id: string) =>
    options.find((o) => o.value === id)?.label ?? id;

  const updateRows = (next: string[]) => {
    onChange(field.key, next);
    onBlur?.(field.key);
  };

  const addRow = (id: string) => {
    if (!id || rows.includes(id)) return;
    updateRows([...rows, id]);
  };

  return (
    <div className="space-y-2 rounded-lg border border-border p-3">
      <p className="text-sm font-medium text-foreground">{field.label}</p>
      <p className="text-xs text-muted">
        Liaison multiple sous forme de tableau incrémentable.
      </p>
      {rows.length === 0 && (
        <p className="text-xs text-muted">Aucune ligne — cliquez sur « Ajouter ».</p>
      )}
      {rows.map((rowValue, idx) => (
        <div
          key={`${idx}-${rowValue}`}
          className="flex items-center justify-between gap-2 rounded-md border border-border bg-background px-3 py-2"
        >
          <span className="text-sm text-foreground">{labelById(rowValue)}</span>
          <Button
            size="sm"
            variant="ghost"
            disabled={readOnly}
            onClick={() => {
              const next = rows.filter((_, i) => i !== idx);
              updateRows(next);
            }}
          >
            Retirer
          </Button>
        </div>
      ))}
      {!readOnly && refEntity && (
        <div className="flex gap-2">
          <Button size="sm" variant="secondary" onClick={() => setPickOpen(true)}>
            Ajouter
          </Button>
        </div>
      )}
      {refEntity && (
        <EntityRelationPickOrCreateModal
          entityKey={refEntity}
          open={pickOpen}
          onClose={() => setPickOpen(false)}
          options={options}
          excludeIds={rows}
          onSelected={addRow}
          onOptionsRefresh={() => void load()}
        />
      )}
    </div>
  );
}

export function FieldRenderer({
  field,
  values,
  onChange,
  onBatchChange,
  onBlur,
  readOnly,
  fieldError,
  fieldWarning,
  screenKey,
  uploadDraftId,
  storageFolders,
  excludeRecordId,
  allFields = [],
  fieldErrorsMap,
  fieldWarningsMap,
}: FieldRendererProps) {
  if (field.type === "hidden" || field.type === "detail_link" || !isFieldVisible(field, values)) {
    return null;
  }
  if (field.form?.embedParent) {
    return null;
  }

  const ro = readOnly || field.form?.readOnly;
  const val = values[field.key] ?? values[field.column] ?? "";
  const hint = field.validation?.fixHint ?? field.form?.placeholder;
  const hasWarning = Boolean(fieldWarning && !fieldError);
  const storageFolder = defaultStorageFolder(field.form?.storageFolder, storageFolders);
  const entityId = mediaEntityId(
    values.id != null ? String(values.id) : undefined,
    uploadDraftId,
  );

  const wrap = (node: ReactNode) => (
    <div className={cn(hasWarning && "rounded-lg ring-1 ring-amber-500/30 p-0.5 -m-0.5")}>
      {node}
      <FieldMessages error={fieldError} warning={fieldWarning} />
    </div>
  );

  if (
    screenKey === "tache" &&
    field.key === "roles_visibles"
  ) {
    return wrap(
      <TacheRolesVisibleField
        label={field.label}
        visibilite={values.visibilite}
        rolesCsv={String(val ?? "")}
        readOnly={ro}
        error={fieldError?.message}
        onChange={(csv) => onChange(field.key, csv)}
      />,
    );
  }

  if (field.type === "entity_embed") {
    return wrap(
      <EntityEmbedGroup
        field={field}
        allFields={allFields}
        values={values}
        readOnly={ro}
        screenKey={screenKey}
        uploadDraftId={uploadDraftId}
        storageFolders={storageFolders}
        excludeRecordId={excludeRecordId}
        onChange={onChange}
        onBatchChange={onBatchChange}
        onBlur={onBlur}
        fieldErrors={fieldErrorsMap}
        fieldWarnings={fieldWarningsMap}
      />,
    );
  }

  if (field.type === "entity_embed_list") {
    return wrap(
      <EntityEmbedListEditor
        field={field}
        value={val}
        readOnly={ro}
        screenKey={screenKey}
        excludeRecordId={excludeRecordId}
        fieldError={fieldError}
        onChange={onChange}
        onBlur={onBlur}
      />,
    );
  }

  if (field.type === "entity_ref") {
    return wrap(
      <EntityRefSelect
        field={field}
        value={String(val ?? "")}
        readOnly={ro}
        screenKey={screenKey}
        excludeRecordId={excludeRecordId}
        fieldError={fieldError}
        onChange={onChange}
        onBlur={onBlur}
      />,
    );
  }

  if (field.type === "entity_ref_list") {
    return wrap(
      <EntityRefListEditor
        field={field}
        value={val}
        readOnly={ro}
        screenKey={screenKey}
        excludeRecordId={excludeRecordId}
        fieldError={fieldError}
        onChange={onChange}
        onBlur={onBlur}
      />,
    );
  }

  if (field.type === "image") {
    return wrap(
      <ImageField
        label={field.label}
        value={String(val ?? "")}
        onChange={(path) => {
          onChange(field.key, path);
          onBlur?.(field.key);
        }}
        disabled={ro}
        screenKey={screenKey}
        entityId={entityId}
        storageFolder={storageFolder}
        accept={field.form?.accept}
        fieldError={fieldError}
        fieldWarning={fieldWarning}
      />,
    );
  }

  if (field.type === "images") {
    return wrap(
      <ImagesField
        label={field.label}
        value={val}
        onChange={(paths) => {
          onChange(field.key, paths);
          onBlur?.(field.key);
        }}
        disabled={ro}
        screenKey={screenKey}
        entityId={entityId}
        storageFolder={storageFolder}
        maxFiles={field.form?.maxFiles}
        accept={field.form?.accept}
        fieldError={fieldError}
        fieldWarning={fieldWarning}
      />,
    );
  }

  if (field.type === "select") {
    const opts = field.options?.map((o) => ({ value: o.value, label: o.label })) ?? [];
    const strVal = String(val ?? "");
    const selectValue =
      strVal || (field.default != null ? String(field.default) : opts[0]?.value ?? "");
    return wrap(
      <Select
        label={field.label}
        value={selectValue}
        disabled={ro}
        error={fieldError?.message}
        onChange={(e) => onChange(field.key, e.target.value)}
        onBlur={() => onBlur?.(field.key)}
        options={opts}
      />,
    );
  }

  if (field.type === "boolean") {
    return wrap(
      <Select
        label={field.label}
        value={val === true || val === 1 || val === "1" || val === "true" ? "1" : "0"}
        disabled={ro}
        error={fieldError?.message}
        onChange={(e) => onChange(field.key, e.target.value === "1")}
        onBlur={() => onBlur?.(field.key)}
        options={[
          { value: "0", label: "Non" },
          { value: "1", label: "Oui" },
        ]}
      />,
    );
  }

  if (field.type === "number" || field.type === "stock") {
    return wrap(
      <Input
        label={field.label}
        type="number"
        disabled={ro}
        value={val === "" || val == null ? "" : String(val)}
        placeholder={field.form?.placeholder}
        error={fieldError?.message}
        hint={
          !fieldError
            ? field.type === "stock"
              ? "Quantité en stock (synchronisée dans l’écran Stock)"
              : hint
            : undefined
        }
        onChange={(e) =>
          onChange(field.key, e.target.value === "" ? null : Number(e.target.value))
        }
        onBlur={() => onBlur?.(field.key)}
      />,
    );
  }

  if (field.type === "date") {
    return wrap(
      <Input
        label={field.label}
        type="date"
        disabled={ro}
        value={toDateInputValue(val)}
        error={fieldError?.message}
        hint={!fieldError ? hint : undefined}
        onChange={(e) => onChange(field.key, e.target.value || null)}
        onBlur={() => onBlur?.(field.key)}
      />,
    );
  }

  if (field.type === "time") {
    return wrap(
      <Input
        label={field.label}
        type="time"
        disabled={ro}
        value={toTimeInputValue(val)}
        placeholder={field.form?.placeholder ?? "HH:MM"}
        error={fieldError?.message}
        hint={!fieldError ? hint : undefined}
        onChange={(e) => onChange(field.key, e.target.value || null)}
        onBlur={() => onBlur?.(field.key)}
      />,
    );
  }

  if (field.type === "datetime") {
    return wrap(
      <Input
        label={field.label}
        type="datetime-local"
        disabled={ro}
        value={toDatetimeLocalValue(val)}
        error={fieldError?.message}
        hint={!fieldError ? hint : undefined}
        onChange={(e) => onChange(field.key, e.target.value || null)}
        onBlur={() => onBlur?.(field.key)}
      />,
    );
  }

  return wrap(
    <Input
      label={field.label}
      disabled={ro}
      value={String(val ?? "")}
      placeholder={field.form?.placeholder}
      error={fieldError?.message}
      hint={!fieldError ? hint : undefined}
      onChange={(e) => onChange(field.key, e.target.value)}
      onBlur={() => onBlur?.(field.key)}
    />,
  );
}
