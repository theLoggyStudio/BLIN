import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ReactNode } from "react";
import { usePrivilege } from "@/hooks/usePrivilege";
import { EntityRelationCreateModal } from "@/items/EntityRelationCreateModal";
import { EntityValidationModal } from "@/items/EntityValidationModal";
import { TacheRolesVisibleField } from "@/items/TacheRolesVisibleField";
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
  onBlur?: (key: string) => void;
  readOnly?: boolean;
  fieldError?: ValidationIssue;
  fieldWarning?: ValidationIssue;
  screenKey: string;
  uploadDraftId: string;
  storageFolders?: string[];
  /** Pour les liaisons entity_ref : exclure l'enregistrement en cours de la règle d'exclusivité parent. */
  excludeRecordId?: string;
}

/** Valeur réservée de la première option « Créer un nouveau ». */
export const ENTITY_REF_CREATE_NEW = "__blin_create_new__";

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
  const canViewRef = usePrivilege(refEntity ? `${refEntity}:voir` : "");
  const [options, setOptions] = useState<RelationSelectOption[]>([]);
  const [createOpen, setCreateOpen] = useState(false);
  const [validationTarget, setValidationTarget] = useState<string | null>(null);
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

  const optionByValue = (id: string) => merged.find((o) => o.value === id);

  const openValidationFor = (recordId: string) => {
    if (!canViewRef) return;
    const opt = optionByValue(recordId);
    if (opt?.validationStatus !== VALIDATION_STATUS_NON_VALIDE) return;
    setValidationTarget(recordId);
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
          const opt = optionByValue(v);
          if (opt?.validationStatus === VALIDATION_STATUS_NON_VALIDE) {
            if (canViewRef) {
              setValidationTarget(v);
            }
            return;
          }
          selectValueRef.current = v;
          onChange(field.key, v);
        }}
        onBlur={() => onBlur?.(field.key)}
        options={selectOptions}
      />
      {canViewRef &&
        value &&
        optionByValue(value)?.validationStatus === VALIDATION_STATUS_NON_VALIDE && (
        <button
          type="button"
          className="mt-1.5 text-left text-sm text-amber-400 underline-offset-2 hover:underline"
          onClick={() => openValidationFor(value)}
        >
          Non validé — cliquer pour consulter et valider
        </button>
      )}
      {refEntity && (
        <EntityRelationCreateModal
          entityKey={refEntity}
          open={createOpen}
          onClose={() => setCreateOpen(false)}
          onCreated={handleCreated}
        />
      )}
      {refEntity && canViewRef && validationTarget && (
        <EntityValidationModal
          entityKey={refEntity}
          recordId={validationTarget}
          open={Boolean(validationTarget)}
          onClose={() => setValidationTarget(null)}
          onValidated={() => {
            void load();
          }}
        />
      )}
    </>
  );
}

export function FieldRenderer({
  field,
  values,
  onChange,
  onBlur,
  readOnly,
  fieldError,
  fieldWarning,
  screenKey,
  uploadDraftId,
  storageFolders,
  excludeRecordId,
}: FieldRendererProps) {
  if (field.type === "hidden" || field.type === "detail_link" || !isFieldVisible(field, values)) {
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
