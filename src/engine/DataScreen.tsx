import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ExternalLink, Eye, Pencil, Printer, Trash2 } from "lucide-react";
import { Guard } from "@/components/Guard";
import { Button } from "@/items/Button";
import { FilterBar } from "@/items/FilterBar";
import { FieldRenderer } from "@/engine/FieldRenderer";
import { EntityRelationDetail } from "@/engine/EntityRelationDetail";
import { EntitySignatureModal } from "@/items/EntitySignatureModal";
import { Modal } from "@/items/Modal";
import { Offpanel } from "@/items/Offpanel";
import { ScreenHeader } from "@/items/ScreenHeader";
import { Table, type Column } from "@/items/Table";
import { TableImageCell } from "@/items/TableImageCell";
import { FieldReadOnlyValue } from "@/engine/FieldReadOnlyValue";
import { formatCellValue, isFieldVisible } from "@/engine/screenUtils";
import {
  issuesByField,
  parseEmbedListValue,
  parseValidationReportFromError,
  validateScreenForm,
} from "@/engine/validation";
import { Alert, ValidationBanner } from "@/items/Alert";
import {
  EntityCreateLineForm,
  type EntityCreateLineFormHandle,
} from "@/items/EntityCreateLineForm";
import {
  embedHeaderFields,
  expandRowsForLignes,
  isSharedListColumn,
  normalizeEditFormValues,
  parseParentLignes,
  scalarFormFields,
  type ListLineRow,
} from "@/lib/createFormLines";
import type { Privilege } from "@/types/auth";
import { PrintListPdfModal } from "@/items/PrintListPdfModal";
import { EntityCsvImportModal } from "@/items/EntityCsvImportModal";
import { printEntityRowPdf } from "@/lib/print/rowPrint";
import { exportEntityCsv } from "@/lib/entityCsv";
import { notifyEntitySuccess, type EntitySuccessAction } from "@/lib/entitySuccessAlert";
import {
  canCreatorEditRecord,
  hasSignatureWorkflow,
  isRecordRefused,
  isRecordSigned,
  isSignatureRecordReadOnly,
} from "@/lib/entitySignature";
import { cn } from "@/lib/utils";
import { useAlert } from "@/contexts/AlertContext";
import { useAuth } from "@/hooks/useAuth";
import type { ReactNode } from "react";
import {
  BUSINESS_SESSION_CHANGED_EVENT,
  ENTITY_CSV_IMPORT_OPEN_EVENT,
  TASK_REMINDERS_REFRESH_EVENT,
} from "@/constants/events";
import { clearTaskReminderKeys } from "@/lib/taskReminders";
import type { ScreenConfigFile, ScreenRow, ValidationIssue } from "@/types/screen";

interface DataScreenProps {
  config: ScreenConfigFile;
  /** Préremplit le modal de création puis l’ouvre une fois (ex. demande Loggy). */
  initialCreateValues?: ScreenRow;
  onInitialCreateApplied?: () => void;
  /** Actions supplémentaires par ligne (ex. déstockage). */
  extraRowActions?: (row: ScreenRow, reload: () => void) => ReactNode;
  /** Liste compacte : lignes plus basses, texte tronqué (modal au clic). */
  compactList?: boolean;
  /** Surcharge le clic ligne (ex. détail en modal). */
  listRowClick?: "detail" | "edit";
}

type FormMode = "create" | "edit" | "detail" | null;

const TACHE_ENTITY_KEY = "tache";

function recordLabelFromRow(row: ScreenRow, config: ScreenConfigFile): string | undefined {
  const lf = config.screen.label_field;
  const v = row[lf];
  if (v == null || v === "") return undefined;
  return String(v).trim() || undefined;
}

function crudSuccessAction(
  kind: "create" | "update" | "delete",
  lineCount: number,
  recordLabel?: string,
): EntitySuccessAction {
  if (kind === "delete") {
    return recordLabel ? "delete_named" : "delete";
  }
  if (kind === "create") {
    if (recordLabel) return "create_named";
    if (lineCount > 1) return "create_lines";
    return "create";
  }
  if (recordLabel) return "update_named";
  if (lineCount > 1) return "update_lines";
  return "update";
}

function compactListCell(content: ReactNode, fieldKey: string): ReactNode {
  const maxW =
    fieldKey === "description"
      ? "max-w-[11rem]"
      : fieldKey === "intitule"
        ? "max-w-[10rem]"
        : "max-w-[7rem]";
  if (content == null || content === false) return "—";
  if (typeof content === "string" || typeof content === "number") {
    const text = String(content);
    return (
      <span className={cn("block truncate", maxW)} title={text}>
        {text}
      </span>
    );
  }
  return <div className={cn("truncate", maxW)}>{content}</div>;
}

export function DataScreen({
  config,
  initialCreateValues,
  onInitialCreateApplied,
  extraRowActions,
  compactList = false,
  listRowClick,
}: DataScreenProps) {
  const screenKey = config.screen.key;
  const pk = config.screen.primaryKey;

  const hasRelations = useMemo(
    () =>
      config.fields.some(
        (f) =>
          f.type === "entity_ref" ||
          f.type === "entity_embed" ||
          f.type === "entity_embed_list",
      ),
    [config.fields],
  );

  const [rows, setRows] = useState<ScreenRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [filters, setFilters] = useState<Record<string, string>>({});
  const [formMode, setFormMode] = useState<FormMode>(null);
  const [formValues, setFormValues] = useState<ScreenRow>({});
  const [relationDetailId, setRelationDetailId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [formValidation, setFormValidation] = useState<{
    errors: ValidationIssue[];
    warnings: ValidationIssue[];
  }>({ errors: [], warnings: [] });
  const [filterValidation, setFilterValidation] = useState<{
    errors: ValidationIssue[];
    warnings: ValidationIssue[];
  }>({ errors: [], warnings: [] });
  const uploadDraftIdRef = useRef(crypto.randomUUID());
  const lineFormRef = useRef<EntityCreateLineFormHandle>(null);
  const initialCreateAppliedRef = useRef(false);
  const [entitySignatureTarget, setEntitySignatureTarget] = useState<{
    entityKey: string;
    recordId: string;
  } | null>(null);
  const { showSuccess, showError, showInfo } = useAlert();
  const { user } = useAuth();

  const isAutoSignatureTask = useCallback(
    (row: ScreenRow) => {
      if (screenKey !== "tache") return false;
      const type = String(row.type_tache ?? "");
      const entityKey = String(row.entite_a_signer ?? row.entite_a_valider ?? "").trim();
      const recordId = String(row.enregistrement_id ?? "").trim();
      return (
        (type === "signature" || type === "validation") &&
        entityKey !== "" &&
        recordId !== ""
      );
    },
    [screenKey],
  );

  const openEntitySignatureFromTask = useCallback((row: ScreenRow) => {
    const entityKey = String(row.entite_a_signer ?? row.entite_a_valider ?? "").trim();
    const recordId = String(row.enregistrement_id ?? "").trim();
    if (!entityKey || !recordId) return;
    setEntitySignatureTarget({ entityKey, recordId });
  }, []);

  const filterFields = useMemo(
    () => config.fields.filter((f) => f.filter?.enabled),
    [config.fields],
  );

  const listFields = useMemo(() => {
    const fromConfig = config.fields.filter((f) => f.list?.enabled && f.type !== "hidden");
    const createdAt = config.fields.find((f) => f.key === "created_at");
    if (createdAt && !fromConfig.some((f) => f.key === "created_at")) {
      return [createdAt, ...fromConfig];
    }
    return fromConfig;
  }, [config.fields]);

  const formFieldsForMode = useCallback(
    (mode: FormMode) =>
      config.fields.filter((f) => {
        if (f.type === "hidden" || f.type === "detail_link") return false;
        if (
          mode === "create" &&
          (f.key === "created_at" ||
            (f.form?.readOnly && !f.form?.autoGenerated))
        ) {
          return false;
        }
        return true;
      }),
    [config.fields],
  );

  const formFields = useMemo(
    () => formFieldsForMode(formMode),
    [formFieldsForMode, formMode],
  );

  const hasMultilineEmbeds = useMemo(
    () => embedHeaderFields(config.fields).length > 0,
    [config.fields],
  );

  const useLineTabsForm = useMemo(
    () =>
      hasMultilineEmbeds &&
      (formMode === "create" || formMode === "edit" || formMode === "detail"),
    [hasMultilineEmbeds, formMode],
  );

  const scalarFields = useMemo(
    () => (useLineTabsForm ? scalarFormFields(formFields) : formFields),
    [useLineTabsForm, formFields],
  );

  const embedFields = useMemo(
    () => (useLineTabsForm ? embedHeaderFields(formFields) : []),
    [useLineTabsForm, formFields],
  );


  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await invoke<ScreenRow[]>("dda_list", {
        payload: { screen_key: screenKey, filters },
      });
      setRows(data);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [screenKey, filters]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    const onSession = () => void load();
    window.addEventListener(BUSINESS_SESSION_CHANGED_EVENT, onSession);
    return () => window.removeEventListener(BUSINESS_SESSION_CHANGED_EVENT, onSession);
  }, [load]);

  useEffect(() => {
    const onImportOpen = (e: Event) => {
      const key = (e as CustomEvent<{ entityKey?: string }>).detail?.entityKey;
      if (key && key !== screenKey) return;
      setCsvImportOpen(true);
      showInfo(`Import CSV pour « ${config.screen.label} » — déposez votre fichier ci-dessous.`);
    };
    window.addEventListener(ENTITY_CSV_IMPORT_OPEN_EVENT, onImportOpen);
    return () => window.removeEventListener(ENTITY_CSV_IMPORT_OPEN_EVENT, onImportOpen);
  }, [screenKey, config.screen.label, showInfo]);

  const openRelationDetail = (row: ScreenRow) => {
    const id = String(row[pk] ?? "");
    if (id) setRelationDetailId(id);
  };

  const columns: Column<ListLineRow>[] = useMemo(
    () => {
      const base = listFields.map((f) => ({
        key: f.key,
        header: f.label,
        sortable: hasMultilineEmbeds ? false : f.list?.sortable,
        sharedAcrossLines: isSharedListColumn(f),
        render: (row: ListLineRow) => {
          if (f.type === "detail_link") {
            return (
              <Button
                size="sm"
                variant="secondary"
                onClick={(e) => {
                  e.stopPropagation();
                  openRelationDetail(row);
                }}
              >
                <ExternalLink className="h-3.5 w-3.5" />
                Détail
              </Button>
            );
          }
          const raw = row[f.key] ?? row[f.column];
          if (f.type === "image") {
            const path = raw != null ? String(raw) : "";
            return <TableImageCell relativePath={path || undefined} />;
          }
          if (f.type === "entity_ref") {
            return raw != null && String(raw).trim() ? String(raw) : "—";
          }
          if (f.type === "entity_embed_list") {
            const rows = parseEmbedListValue(raw);
            return rows.length > 0 ? `${rows.length} élément(s)` : "—";
          }
          const cell = formatCellValue(f, raw);
          return compactList ? compactListCell(cell, f.key) : cell;
        },
      }));
      if (hasMultilineEmbeds) {
        base.unshift({
          key: "_lineNum",
          header: "Ligne",
          sortable: false,
          sharedAcrossLines: false,
          render: (row: ListLineRow) =>
            row.__lineCount > 1 ? (
              <span className="text-muted">Ligne {row.__lineIndex + 1}</span>
            ) : (
              "—"
            ),
        });
      }
      return base;
    },
    [listFields, hasMultilineEmbeds, pk, compactList],
  );

  const runFormValidation = (values: ScreenRow) => {
    const report = validateScreenForm(config, values);
    setFormValidation({ errors: report.errors, warnings: report.warnings });
    return report;
  };

  const runFilterValidation = (filterValues: Record<string, string>) => {
    const report = validateScreenForm(config, filterValues, { filtersOnly: true });
    setFilterValidation({ errors: report.errors, warnings: report.warnings });
    return report;
  };

  const defaultTimeValue = (): string => {
    const now = new Date();
    return `${String(now.getHours()).padStart(2, "0")}:${String(now.getMinutes()).padStart(2, "0")}`;
  };

  const openCreateWith = useCallback(
    async (prefill?: ScreenRow) => {
      const fields = formFieldsForMode("create");
      const initial: ScreenRow = {};
      for (const f of fields) {
        if (f.default != null && f.default !== "") {
          initial[f.key] = f.default;
        } else if (f.type === "time" && f.required) {
          initial[f.key] = defaultTimeValue();
        }
      }
      try {
        const preview = await invoke<ScreenRow>("entity_compteur_preview", {
          payload: { entity_key: screenKey },
        });
        if (preview) {
          for (const [key, value] of Object.entries(preview)) {
            if (value !== null && value !== undefined && value !== "") {
              initial[key] = value;
            }
          }
        }
      } catch {
        /* aperçu matricule optionnel */
      }
      if (prefill) {
        for (const [key, value] of Object.entries(prefill)) {
          if (value !== null && value !== undefined && value !== "") {
            initial[key] = value;
          }
        }
      }
      setFormValues(initial);
      setFormValidation({ errors: [], warnings: [] });
      setFormMode("create");
    },
    [formFieldsForMode, screenKey],
  );

  const openCreate = () => {
    openCreateWith();
  };

  useEffect(() => {
    initialCreateAppliedRef.current = false;
  }, [screenKey]);

  useEffect(() => {
    if (!initialCreateValues || initialCreateAppliedRef.current) return;
    initialCreateAppliedRef.current = true;
    openCreateWith(initialCreateValues);
    onInitialCreateApplied?.();
  }, [initialCreateValues, openCreateWith, onInitialCreateApplied]);

  const displayRows = useMemo(
    () =>
      hasMultilineEmbeds
        ? expandRowsForLignes(rows, pk, config.fields)
        : rows.map((row) => ({
            ...row,
            __parentId: String(row[pk] ?? ""),
            __lineIndex: 0,
            __lineCount: 1,
            __isFirstLine: true,
          })),
    [hasMultilineEmbeds, rows, pk, config.fields],
  );

  const openRow = async (row: ListLineRow, mode: "edit" | "detail") => {
    const id = row.__parentId || String(row[pk] ?? "");
    const parentRow = rows.find((r) => String(r[pk] ?? "") === id) ?? row;
    if (isAutoSignatureTask(parentRow)) {
      openEntitySignatureFromTask(parentRow);
      return;
    }
    if (mode === "detail" && hasRelations) {
      openRelationDetail(parentRow);
      return;
    }
    try {
      const full = await invoke<ScreenRow>("dda_get", {
        payload: { screen_key: screenKey, id },
      });
      setFormValues(normalizeEditFormValues(full, config.fields));
      setFormValidation({ errors: [], warnings: [] });
      setFormMode(mode);
    } catch (e) {
      const msg = String(e);
      setError(msg);
      showError(msg);
    }
  };

  const closeForm = () => {
    setFormMode(null);
    setFormValues({});
    setFormValidation({ errors: [], warnings: [] });
  };

  const setField = (key: string, value: unknown) => {
    setFormValues((prev) => {
      const next = { ...prev, [key]: value };
      runFormValidation(next);
      return next;
    });
  };

  const submitForm = async () => {
    setError(null);
    let payload = formValues;
    if (useLineTabsForm && lineFormRef.current) {
      payload = { ...formValues, ...lineFormRef.current.flushForSubmit() };
    }
    if (isRecordSigned(payload) || !canCreatorEditRecord(payload, user?.id)) return;
    const report = runFormValidation(payload);
    if (!report.valid) return;
    try {
      const savedLineCount = parseParentLignes(payload, config.fields).length;
      const wasCreate = formMode === "create";
      if (formMode === "create") {
        await invoke("dda_create", {
          payload: { screen_key: screenKey, data: payload },
        });
      } else if (formMode === "edit") {
        const id = String(payload[pk] ?? "");
        await invoke("dda_update", {
          payload: { screen_key: screenKey, id, data: payload },
        });
      }
      closeForm();
      await load();
      const recordLabel =
        screenKey === TACHE_ENTITY_KEY
          ? String(payload.intitule ?? "").trim() || undefined
          : recordLabelFromRow(payload, config);
      if (screenKey === TACHE_ENTITY_KEY && !wasCreate) {
        clearTaskReminderKeys(String(payload[pk] ?? ""));
      }
      if (screenKey === TACHE_ENTITY_KEY) {
        window.dispatchEvent(new CustomEvent(TASK_REMINDERS_REFRESH_EVENT));
      }
      notifyEntitySuccess(showSuccess, screenKey, crudSuccessAction(wasCreate ? "create" : "update", savedLineCount, recordLabel), {
        line_count: savedLineCount,
        record_label: recordLabel,
      });
    } catch (e) {
      const parsed = parseValidationReportFromError(e);
      if (parsed) {
        setFormValidation({ errors: parsed.errors, warnings: parsed.warnings });
      } else {
        const msg = String(e);
        setError(msg);
        showError(msg);
      }
    }
  };

  const [printingId, setPrintingId] = useState<string | null>(null);
  const [printListOpen, setPrintListOpen] = useState(false);
  const [csvImportOpen, setCsvImportOpen] = useState(false);

  const printRow = async (row: ScreenRow) => {
    const id = String(row[pk] ?? "");
    if (!id) return;
    setPrintingId(id);
    setError(null);
    try {
      await printEntityRowPdf(screenKey, id);
      notifyEntitySuccess(showSuccess, screenKey, "export_pdf_row");
    } catch (e) {
      const msg = String(e);
      setError(msg);
      showError(`Échec de la génération PDF : ${msg}`);
    } finally {
      setPrintingId(null);
    }
  };

  const rowActions = (row: ScreenRow) => (
    <div className="flex gap-1 justify-end">
      {extraRowActions?.(row, () => void load())}
      {hasRelations && (
        <Guard privilege={config.screen.privileges.view}>
          <Button
            variant="ghost"
            size="sm"
            aria-label="Détail relations"
            title="Voir les liaisons embarquées"
            onClick={(e) => {
              e.stopPropagation();
              openRelationDetail(row);
            }}
          >
            <ExternalLink className="h-4 w-4" />
          </Button>
        </Guard>
      )}
      <Guard privilege={config.screen.privileges.view}>
        <Button
          variant="ghost"
          size="sm"
          aria-label="Imprimer PDF"
          title="Imprimer la fiche PDF"
          disabled={printingId === String(row[pk] ?? "")}
          onClick={(e) => {
            e.stopPropagation();
            void printRow(row);
          }}
        >
          <Printer className={`h-4 w-4 ${printingId === String(row[pk] ?? "") ? "animate-pulse" : ""}`} />
        </Button>
      </Guard>
      <Guard privilege={config.screen.privileges.update as Privilege}>
        {!isAutoSignatureTask(row) && (
          <Button
            variant="ghost"
            size="sm"
            aria-label={isSignatureRecordReadOnly(row, user?.id) ? "Consulter" : "Modifier"}
            title={
              isRecordSigned(row)
                ? "Objet signé — consultation seule"
                : isRecordRefused(row)
                  ? "Objet refusé — consultation seule"
                  : !canCreatorEditRecord(row, user?.id)
                    ? "Seul l'auteur peut modifier avant signature"
                    : undefined
            }
            onClick={(e) => {
              e.stopPropagation();
              openRow(row, "edit");
            }}
          >
            {isSignatureRecordReadOnly(row, user?.id) ? (
              <Eye className="h-4 w-4" />
            ) : (
              <Pencil className="h-4 w-4" />
            )}
          </Button>
        )}
      </Guard>
      <Guard privilege={config.screen.privileges.delete as Privilege}>
        {canCreatorEditRecord(row, user?.id) && (
          <Button
            variant="ghost"
            size="sm"
            aria-label="Supprimer"
            onClick={(e) => {
              e.stopPropagation();
              void deleteRow(row);
            }}
          >
            <Trash2 className="h-4 w-4 text-primary" />
          </Button>
        )}
      </Guard>
    </div>
  );

  const deleteRow = async (row: ScreenRow) => {
    const id = String(row[pk] ?? "");
    const confirmLabel =
      screenKey === TACHE_ENTITY_KEY
        ? "Supprimer cette tâche ?"
        : "Supprimer cet enregistrement ?";
    if (!id || !window.confirm(confirmLabel)) return;
    try {
      await invoke("dda_delete", { payload: { screen_key: screenKey, id } });
      await load();
      const recordLabel = recordLabelFromRow(row, config);
      if (screenKey === TACHE_ENTITY_KEY) {
        clearTaskReminderKeys(id);
        window.dispatchEvent(new CustomEvent(TASK_REMINDERS_REFRESH_EVENT));
      }
      notifyEntitySuccess(showSuccess, screenKey, crudSuccessAction("delete", 1, recordLabel), {
        record_label: recordLabel,
      });
    } catch (e) {
      const msg = String(e);
      setError(msg);
      if (screenKey === TACHE_ENTITY_KEY) {
        showError(`Impossible de supprimer la tâche : ${msg}`);
      } else {
        showError(msg);
      }
    }
  };

  const createLayout = config.layout.forms?.create;
  const editLayout = config.layout.forms?.edit;
  const detailLayout = config.layout.forms?.detail;
  const activeLayout =
    formMode === "create" ? createLayout : formMode === "edit" ? editLayout : detailLayout;
  const isSignedForm =
    (formMode === "edit" || formMode === "detail") && isRecordSigned(formValues);
  const isSignatureReadOnlyForm =
    (formMode === "edit" || formMode === "detail") &&
    isSignatureRecordReadOnly(formValues, user?.id);
  const formReadOnly =
    (formMode === "detail" && (detailLayout?.readOnly ?? true)) ||
    (formMode === "edit" && isAutoSignatureTask(formValues)) ||
    isSignedForm ||
    isSignatureReadOnlyForm;
  const formDisplayOnly =
    formMode === "detail" || isSignedForm || isSignatureReadOnlyForm;
  const excludeRecordId =
    formMode === "edit" && formValues[pk] != null ? String(formValues[pk]) : undefined;

  const formErrorsMap = issuesByField(formValidation.errors);
  const formWarningsMap = issuesByField(formValidation.warnings);
  const filterErrorsMap = issuesByField(filterValidation.errors);
  const filterWarningsMap = issuesByField(filterValidation.warnings);

  const renderFormField = (field: (typeof formFields)[number]) => (
    <FieldRenderer
      key={field.key}
      field={field}
      allFields={config.fields}
      fieldErrorsMap={formErrorsMap}
      fieldWarningsMap={formWarningsMap}
      values={formValues}
      onChange={setField}
      onBatchChange={(updates) => {
        setFormValues((prev) => {
          const next = { ...prev, ...updates };
          runFormValidation(next);
          return next;
        });
      }}
      onBlur={() => runFormValidation(formValues)}
      readOnly={formReadOnly}
      displayOnly={formDisplayOnly}
      fieldError={formErrorsMap[field.key]}
      fieldWarning={formWarningsMap[field.key]}
      screenKey={screenKey}
      uploadDraftId={uploadDraftIdRef.current}
      storageFolders={config.screen.storage?.folders}
      excludeRecordId={excludeRecordId}
    />
  );

  const objetIntitule = String(formValues.intitule ?? "").trim();

  const detailTextBody = (
    <div className="space-y-4">
      {isSignedForm && (
        <Alert
          variant="info"
          size="inline"
          message="Cet objet est signé et ne peut plus être modifié."
        />
      )}
      {!isSignedForm && isRecordRefused(formValues) && (
        <Alert
          variant="warning"
          size="inline"
          message="Cet objet a été refusé. Seul un signataire peut le réaccepter par signature."
        />
      )}
      {objetIntitule ? (
        <div className="rounded-lg border border-border bg-surface-elevated p-4">
          <p className="mb-2 text-xs font-medium uppercase tracking-wide text-muted">
            Objet concerné
          </p>
          <pre className="whitespace-pre-wrap font-sans text-sm leading-relaxed text-foreground">
            {objetIntitule}
          </pre>
        </div>
      ) : null}
      <dl className="grid gap-x-6 gap-y-4 sm:grid-cols-2">
        {formFields
          .filter((f) => isFieldVisible(f, formValues))
          .filter((f) => f.key !== "intitule" && f.column !== "intitule")
          .map((field) => (
            <FieldReadOnlyValue
              key={field.key}
              field={field}
              value={formValues[field.key] ?? formValues[field.column]}
              screenKey={screenKey}
              excludeRecordId={excludeRecordId}
            />
          ))}
      </dl>
    </div>
  );

  const formBody = (
    <div className="space-y-4">
      {isSignedForm && (
        <Alert
          variant="info"
          size="inline"
          message="Cet objet est signé et ne peut plus être modifié."
        />
      )}
      {!isSignedForm && isRecordRefused(formValues) && (
        <Alert
          variant="warning"
          size="inline"
          message="Cet objet a été refusé. Seul un signataire peut le réaccepter par signature."
        />
      )}
      {!isSignedForm &&
        !isRecordRefused(formValues) &&
        !canCreatorEditRecord(formValues, user?.id) &&
        hasSignatureWorkflow(formValues) && (
          <Alert
            variant="info"
            size="inline"
            message="Seul l'auteur de l'objet peut le modifier avant signature."
          />
        )}
      <ValidationBanner errors={formValidation.errors} warnings={formValidation.warnings} />
      {error && <Alert variant="danger" size="inline" message={error} />}
      <div className={formDisplayOnly ? "grid gap-4 sm:grid-cols-2" : "space-y-4"}>
        {scalarFields.map(renderFormField)}
      </div>
      {useLineTabsForm && (
        <EntityCreateLineForm
          ref={lineFormRef}
          key={`mother-lines-${formMode}-${String(formValues[pk] ?? "new")}`}
          entityLabel={config.screen.label}
          primaryKey={pk}
          allFields={config.fields}
          values={formValues}
          onChange={setField}
          onBatchChange={(updates) => {
            setFormValues((prev) => {
              const next = { ...prev, ...updates };
              runFormValidation(next);
              return next;
            });
          }}
          readOnly={formReadOnly}
          displayOnly={formDisplayOnly}
        >
          <div className={formDisplayOnly ? "grid gap-4 sm:grid-cols-2" : "space-y-4"}>
            {embedFields.map(renderFormField)}
          </div>
        </EntityCreateLineForm>
      )}
    </div>
  );

  return (
    <div className={compactList ? "p-2" : "p-8"}>
      <ScreenHeader
        layout={config.layout.list}
        privileges={config.screen.privileges}
        onCreate={openCreate}
        onRefresh={() => void load()}
        onPrintListPdf={() => setPrintListOpen(true)}
        onImportCsv={() => setCsvImportOpen(true)}
        onExportCsv={() => void exportEntityCsv(screenKey, config.screen.label)}
        loading={loading}
      />

      <PrintListPdfModal
        open={printListOpen}
        onClose={() => setPrintListOpen(false)}
        config={config}
      />
      <EntityCsvImportModal
        entityKey={screenKey}
        entityLabel={config.screen.label}
        open={csvImportOpen}
        onClose={() => setCsvImportOpen(false)}
        onImported={() => void load()}
      />

      <div className={cn("space-y-3", compactList ? "mb-3" : "mb-6")}>
        <FilterBar
          fields={filterFields}
          values={filters}
          fieldErrors={filterErrorsMap}
          fieldWarnings={filterWarningsMap}
          onChange={(key, value) => {
            const next = { ...filters, [key]: value };
            setFilters(next);
            runFilterValidation(next);
          }}
        />
        {(filterValidation.errors.length > 0 || filterValidation.warnings.length > 0) && (
          <ValidationBanner
            errors={filterValidation.errors}
            warnings={filterValidation.warnings}
          />
        )}
      </div>

      {error && !formMode && (
        <Alert variant="danger" size="inline" className="mb-4" message={error} />
      )}

      <Table
        dense={compactList}
        pageSize={config.layout.list.pagination?.pageSize}
        pageSizeOptions={config.layout.list.pagination?.pageSizeOptions}
        showPageSizeSelector={config.layout.list.pagination?.showPageSizeSelector}
        hideWhenSinglePage={config.layout.list.pagination?.hideWhenSinglePage}
        columns={[
          ...columns,
          {
            key: "_actions",
            header: "",
            className: compactList ? "w-28" : "w-36",
            sharedAcrossLines: true,
            render: (row: ListLineRow) =>
              row.__isFirstLine ? rowActions(rows.find((r) => String(r[pk] ?? "") === row.__parentId) ?? row) : null,
          } as Column<ListLineRow>,
        ]}
        data={displayRows}
        keyExtractor={(row) => `${row.__parentId}-${row.__lineIndex}`}
        lineCount={(row) => row.__lineCount}
        isFirstLine={(row) => row.__isFirstLine}
        defaultSortKey={hasMultilineEmbeds ? undefined : "created_at"}
        defaultSortDir="desc"
        emptyMessage={loading ? "Chargement…" : "Aucun enregistrement"}
        onRowClick={(row) => {
          const clickMode = listRowClick ?? config.layout.list.rowClick;
          if (clickMode === "detail") {
            void openRow(row, "detail");
          } else if (clickMode === "edit") {
            void openRow(row, "edit");
          }
        }}
      />

      {createLayout?.mode === "modal" && (
        <Modal
          open={formMode === "create"}
          onClose={closeForm}
          title={createLayout.title}
          size="lg"
          footer={
            <div className="flex justify-end gap-2">
              <Button variant="ghost" onClick={closeForm}>
                Annuler
              </Button>
              <Button onClick={() => void submitForm()}>
                {createLayout.submitLabel ?? "Enregistrer"}
              </Button>
            </div>
          }
        >
          {formBody}
        </Modal>
      )}

      {editLayout?.mode === "modal" && (
        <Modal
          open={formMode === "edit"}
          onClose={closeForm}
          title={
            isSignedForm
              ? `${config.screen.label} — fiche signée`
              : editLayout.title
          }
          size="lg"
          footer={
            isSignedForm || isAutoSignatureTask(formValues) ? (
              <div className="flex justify-end">
                <Button variant="ghost" onClick={closeForm}>
                  Fermer
                </Button>
              </div>
            ) : (
              <div className="flex justify-end gap-2">
                <Button variant="ghost" onClick={closeForm}>
                  Annuler
                </Button>
                <Button onClick={() => void submitForm()}>
                  {editLayout.submitLabel ?? "Enregistrer"}
                </Button>
              </div>
            )
          }
        >
          {formBody}
        </Modal>
      )}

      {detailLayout?.mode === "modal" && formMode === "detail" && !hasRelations && (
        <Modal
          open
          onClose={closeForm}
          title={detailLayout.title}
          size={compactList ? "xl" : "lg"}
          footer={
            <Button variant="ghost" onClick={closeForm}>
              Fermer
            </Button>
          }
        >
          {compactList ? detailTextBody : formBody}
        </Modal>
      )}

      {activeLayout?.mode === "offpanel" && formMode && formMode !== "create" && (
        <Offpanel
          open
          onClose={closeForm}
          title={activeLayout.title}
          width="lg"
          headerActions={
            formReadOnly ? (
              <Button size="sm" variant="ghost" onClick={closeForm}>
                Fermer
              </Button>
            ) : (
              <Button size="sm" onClick={() => void submitForm()}>
                {activeLayout.submitLabel ?? "Enregistrer"}
              </Button>
            )
          }
        >
          {formBody}
        </Offpanel>
      )}

      {relationDetailId && (
        <EntityRelationDetail
          screenKey={screenKey}
          recordId={relationDetailId}
          open={Boolean(relationDetailId)}
          onClose={() => setRelationDetailId(null)}
          title={detailLayout?.title ?? `Fiche — ${config.screen.label}`}
        />
      )}

      {entitySignatureTarget && (
        <EntitySignatureModal
          entityKey={entitySignatureTarget.entityKey}
          recordId={entitySignatureTarget.recordId}
          open
          onClose={() => setEntitySignatureTarget(null)}
          onSigned={() => {
            setEntitySignatureTarget(null);
            void load();
            notifyEntitySuccess(showSuccess, screenKey, "signature_ok");
          }}
          onRejected={() => {
            setEntitySignatureTarget(null);
            void load();
            notifyEntitySuccess(showSuccess, screenKey, "signature_refuse");
          }}
        />
      )}
    </div>
  );
}
