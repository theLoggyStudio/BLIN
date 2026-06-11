import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import { usePrivilege } from "@/hooks/usePrivilege";
import { EntityEmbedGroup, EntityEmbedListEditor } from "@/items/EntityEmbedField";
import { EntityRelationAutocomplete, fetchRelationLabels } from "@/items/EntityRelationAutocomplete";
import { EntityRelationCreateModal } from "@/items/EntityRelationCreateModal";
import { EntityRelationPickOrCreateModal } from "@/items/EntityRelationPickOrCreateModal";
import { TacheRolesVisibleField } from "@/items/TacheRolesVisibleField";
import { Button } from "@/items/Button";
import { Input } from "@/items/Input";
import { Select } from "@/items/Select";
import { FieldMessages } from "@/items/Alert";
import { ImageField } from "@/items/ImageField";
import { ImagesField } from "@/items/ImagesField";
import type { FieldDef, ScreenRow, ValidationIssue } from "@/types/screen";
import { FieldReadOnlyValue } from "@/engine/FieldReadOnlyValue";
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
  /** Affichage texte (pas de champs de saisie) — ex. objet signé. */
  displayOnly?: boolean;
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
  displayOnly?: boolean;
  screenKey: string;
  excludeRecordId?: string;
  fieldError?: ValidationIssue;
  onChange: (key: string, value: unknown) => void;
  onBlur?: (key: string) => void;
}) {
  const refEntity = field.form?.refEntity?.trim() ?? "";
  const canCreate = usePrivilege(refEntity ? `${refEntity}:creer` : "");
  const [createOpen, setCreateOpen] = useState(false);

  const handleCreated = (row: ScreenRow) => {
    const id = row.id != null ? String(row.id) : "";
    if (!id) return;
    onChange(field.key, id);
  };

  return (
    <>
      <EntityRelationAutocomplete
        label={field.label}
        screenKey={screenKey}
        fieldKey={field.key}
        excludeRecordId={excludeRecordId}
        value={value}
        disabled={readOnly}
        error={fieldError?.message}
        onSelect={(option) => onChange(field.key, option.value)}
        onBlur={() => onBlur?.(field.key)}
        onCreateNew={
          canCreate && refEntity && !readOnly ? () => setCreateOpen(true) : undefined
        }
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
  onChange,
  onBlur,
}: {
  field: FieldDef;
  value: unknown;
  readOnly?: boolean;
  displayOnly?: boolean;
  screenKey: string;
  excludeRecordId?: string;
  fieldError?: ValidationIssue;
  onChange: (key: string, value: unknown) => void;
  onBlur?: (key: string) => void;
}) {
  const [labels, setLabels] = useState<Map<string, string>>(new Map());
  const [pickOpen, setPickOpen] = useState(false);
  const rows = parseEntityRefListValue(value);
  const refEntity = field.form?.refEntity?.trim() ?? "";

  // Résolution des libellés des lignes sélectionnées uniquement (pas de chargement total).
  const rowsKey = rows.join("|");
  useEffect(() => {
    let cancelled = false;
    const ids = rowsKey ? rowsKey.split("|").filter(Boolean) : [];
    if (ids.length === 0) {
      setLabels(new Map());
      return;
    }
    void fetchRelationLabels(screenKey, field.key, ids, excludeRecordId)
      .then((map) => {
        if (!cancelled) setLabels(map);
      })
      .catch(() => {
        if (!cancelled) setLabels(new Map());
      });
    return () => {
      cancelled = true;
    };
  }, [rowsKey, screenKey, field.key, excludeRecordId]);

  const labelById = (id: string) => labels.get(id) ?? id;

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
          {!readOnly && (
            <Button
              size="sm"
              variant="ghost"
              onClick={() => {
                const next = rows.filter((_, i) => i !== idx);
                updateRows(next);
              }}
            >
              Retirer
            </Button>
          )}
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
          screenKey={screenKey}
          fieldKey={field.key}
          excludeRecordId={excludeRecordId}
          excludeIds={rows}
          onSelected={addRow}
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
  displayOnly,
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
  const autoGenerated = field.form?.autoGenerated;
  const hint = field.validation?.fixHint ?? field.form?.placeholder;
  const fieldPlaceholder = autoGenerated && ro ? undefined : field.form?.placeholder;
  const fieldHint = autoGenerated && ro ? undefined : !fieldError ? hint : undefined;
  const hasWarning = Boolean(fieldWarning && !fieldError);
  const storageFolder = defaultStorageFolder(field.form?.storageFolder, storageFolders);
  const entityId = mediaEntityId(
    values.id != null ? String(values.id) : undefined,
    uploadDraftId,
  );

  if (displayOnly && field.type !== "entity_embed" && field.type !== "entity_embed_list") {
    return (
      <FieldReadOnlyValue
        field={field}
        value={val}
        screenKey={screenKey}
        excludeRecordId={excludeRecordId}
      />
    );
  }

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
    const node = (
      <EntityEmbedGroup
        field={field}
        allFields={allFields}
        values={values}
        readOnly={ro}
        displayOnly={displayOnly}
        screenKey={screenKey}
        uploadDraftId={uploadDraftId}
        storageFolders={storageFolders}
        excludeRecordId={excludeRecordId}
        onChange={onChange}
        onBatchChange={onBatchChange}
        onBlur={onBlur}
        fieldErrors={fieldErrorsMap}
        fieldWarnings={fieldWarningsMap}
      />
    );
    return displayOnly ? node : wrap(node);
  }

  if (field.type === "entity_embed_list") {
    const node = (
      <EntityEmbedListEditor
        field={field}
        value={val}
        readOnly={ro}
        displayOnly={displayOnly}
        screenKey={screenKey}
        excludeRecordId={excludeRecordId}
        fieldError={fieldError}
        onChange={onChange}
        onBlur={onBlur}
      />
    );
    return displayOnly ? node : wrap(node);
  }

  if (field.type === "entity_ref") {
    return wrap(
      <EntityRefSelect
        field={field}
        value={String(val ?? "")}
        readOnly={ro}
        displayOnly={displayOnly}
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
        displayOnly={displayOnly}
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
        placeholder={fieldPlaceholder}
        error={fieldError?.message}
        hint={
          fieldHint ??
          (!fieldError && field.type === "stock"
            ? "Quantité en stock (synchronisée dans l’écran Stock)"
            : undefined)
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
        hint={fieldHint}
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
        placeholder={fieldPlaceholder ?? "HH:MM"}
        error={fieldError?.message}
        hint={fieldHint}
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
        hint={fieldHint}
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
      placeholder={fieldPlaceholder}
      error={fieldError?.message}
      hint={fieldHint}
      onChange={(e) => onChange(field.key, e.target.value)}
      onBlur={() => onBlur?.(field.key)}
    />,
  );
}
