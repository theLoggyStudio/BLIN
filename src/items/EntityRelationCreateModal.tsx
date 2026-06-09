import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { FieldRenderer } from "@/engine/FieldRenderer";
import {
  issuesByField,
  parseValidationReportFromError,
  validateScreenForm,
} from "@/engine/validation";
import { Button } from "@/items/Button";
import { Modal } from "@/items/Modal";
import { Alert, ValidationBanner } from "@/items/Alert";
import type { ScreenConfigFile, ScreenRow, ValidationIssue } from "@/types/screen";

interface EntityRelationCreateModalProps {
  entityKey: string;
  open: boolean;
  onClose: () => void;
  onCreated: (row: ScreenRow) => void;
}

/** Création rapide d’un enregistrement cible pour une liaison entity_ref. */
export function EntityRelationCreateModal({
  entityKey,
  open,
  onClose,
  onCreated,
}: EntityRelationCreateModalProps) {
  const [config, setConfig] = useState<ScreenConfigFile | null>(null);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [values, setValues] = useState<ScreenRow>({});
  const [validation, setValidation] = useState<{
    errors: ValidationIssue[];
    warnings: ValidationIssue[];
  }>({ errors: [], warnings: [] });
  const uploadDraftIdRef = useRef(crypto.randomUUID());

  const formFields = useMemo(() => {
    if (!config) return [];
    return config.fields.filter(
      (f) =>
        f.type !== "hidden" &&
        f.type !== "detail_link" &&
        f.key !== "created_at" &&
        !f.form?.readOnly,
    );
  }, [config]);

  const loadConfig = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const cfg = await invoke<ScreenConfigFile>("entity_get_screen_config", {
        payload: { entity_key: entityKey },
      });
      setConfig(cfg);
    } catch (e) {
      setError(String(e));
      setConfig(null);
    } finally {
      setLoading(false);
    }
  }, [entityKey]);

  const buildInitial = useCallback((cfg: ScreenConfigFile): ScreenRow => {
    const initial: ScreenRow = {};
    const now = new Date();
    const defaultTime = `${String(now.getHours()).padStart(2, "0")}:${String(now.getMinutes()).padStart(2, "0")}`;
    for (const f of cfg.fields) {
      if (f.type === "hidden" || f.type === "detail_link") continue;
      if (f.default != null && f.default !== "") {
        initial[f.key] = f.default;
      } else if (f.type === "time" && f.required) {
        initial[f.key] = defaultTime;
      }
    }
    return initial;
  }, []);

  useEffect(() => {
    if (!open) {
      setConfig(null);
      setValues({});
      setValidation({ errors: [], warnings: [] });
      setError(null);
      return;
    }
    uploadDraftIdRef.current = crypto.randomUUID();
    void loadConfig();
  }, [open, loadConfig]);

  useEffect(() => {
    if (config && open) {
      const initial = buildInitial(config);
      setValues(initial);
      setValidation({ errors: [], warnings: [] });
    }
  }, [config, open, buildInitial]);

  const setField = (key: string, value: unknown) => {
    setValues((prev) => {
      const next = { ...prev, [key]: value };
      if (config) {
        const report = validateScreenForm(config, next);
        setValidation({ errors: report.errors, warnings: report.warnings });
      }
      return next;
    });
  };

  const submit = async () => {
    if (!config) return;
    setError(null);
    const report = validateScreenForm(config, values);
    setValidation({ errors: report.errors, warnings: report.warnings });
    if (!report.valid) return;
    setSaving(true);
    try {
      const row = await invoke<ScreenRow>("dda_create", {
        payload: { screen_key: entityKey, data: values },
      });
      onCreated(row);
      onClose();
    } catch (e) {
      const parsed = parseValidationReportFromError(e);
      if (parsed) {
        setValidation({ errors: parsed.errors, warnings: parsed.warnings });
      } else {
        setError(String(e));
      }
    } finally {
      setSaving(false);
    }
  };

  const errorsMap = issuesByField(validation.errors);
  const warningsMap = issuesByField(validation.warnings);
  const title = config?.screen.label
    ? `Créer — ${config.screen.label}`
    : "Créer un enregistrement";

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={title}
      size="lg"
      footer={
        <div className="flex justify-end gap-2">
          <Button variant="ghost" onClick={onClose} disabled={saving}>
            Annuler
          </Button>
          <Button onClick={() => void submit()} disabled={saving || loading || !config}>
            {saving ? "Enregistrement…" : "Créer"}
          </Button>
        </div>
      }
    >
      {loading && (
        <p className="py-8 text-center text-sm text-muted">Chargement du formulaire…</p>
      )}
      {!loading && config && (
        <div className="space-y-4 pr-1">
          <ValidationBanner errors={validation.errors} warnings={validation.warnings} />
          {error && <Alert variant="danger" size="box" message={error} />}
          {formFields.map((field) => (
            <FieldRenderer
              key={field.key}
              field={field}
              values={values}
              onChange={setField}
              screenKey={entityKey}
              uploadDraftId={uploadDraftIdRef.current}
              storageFolders={config.screen.storage?.folders}
              fieldError={errorsMap[field.key]}
              fieldWarning={warningsMap[field.key]}
            />
          ))}
        </div>
      )}
    </Modal>
  );
}
