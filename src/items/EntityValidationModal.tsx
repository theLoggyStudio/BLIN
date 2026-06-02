import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/items/Button";
import { Modal } from "@/items/Modal";
import { Text } from "@/items/Text";
import type { RecordValidationDetail } from "@/types/entity";

interface EntityValidationModalProps {
  entityKey: string;
  recordId: string;
  open: boolean;
  onClose: () => void;
  onValidated?: () => void;
}

/** Fiche en lecture seule + validation pour enregistrements « non validés ». */
export function EntityValidationModal({
  entityKey,
  recordId,
  open,
  onClose,
  onValidated,
}: EntityValidationModalProps) {
  const [detail, setDetail] = useState<RecordValidationDetail | null>(null);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!open || !recordId) return;
    setLoading(true);
    setError(null);
    try {
      const d = await invoke<RecordValidationDetail>("entity_record_validation_detail", {
        payload: { entity_key: entityKey, record_id: recordId },
      });
      setDetail(d);
    } catch (e) {
      setDetail(null);
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [open, entityKey, recordId]);

  useEffect(() => {
    if (open) {
      void load();
    } else {
      setDetail(null);
      setError(null);
    }
  }, [open, load]);

  const handleValidate = async () => {
    setSaving(true);
    setError(null);
    try {
      await invoke("entity_record_validate", {
        payload: { entity_key: entityKey, record_id: recordId },
      });
      onValidated?.();
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  if (!open) return null;

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={
        detail
          ? `${detail.entityLabel} — ${detail.validated ? "Fiche" : "À valider"}`
          : "Validation"
      }
      size="lg"
      footer={
        <div className="flex justify-end gap-2">
          <Button variant="ghost" onClick={onClose} disabled={saving}>
            Fermer
          </Button>
          {detail?.canValidate && !detail.validated && (
            <Button onClick={() => void handleValidate()} disabled={saving || loading}>
              {saving ? "Validation…" : "Valider"}
            </Button>
          )}
        </div>
      }
    >
      {loading && (
        <p className="py-8 text-center text-sm text-muted">Chargement de la fiche…</p>
      )}
      {!loading && error && (
        <p className="text-sm text-primary" role="alert">
          {error}
        </p>
      )}
      {!loading && detail && (
        <div className="max-h-[65vh] space-y-4 overflow-y-auto pr-1">
          {!detail.validated && (
            <div className="rounded-lg border border-amber-500/40 bg-amber-500/10 px-3 py-2 text-sm text-amber-200">
              Cet enregistrement n&apos;est pas encore validé. Il ne peut pas être utilisé dans
              une liaison tant qu&apos;un valideur ne l&apos;a pas approuvé.
            </div>
          )}
          {detail.validated && (
            <div className="rounded-lg border border-emerald-500/30 bg-emerald-500/10 px-3 py-2 text-sm text-emerald-300">
              Enregistrement validé.
            </div>
          )}
          <dl className="space-y-3">
            {detail.fields.map((f) => (
              <div
                key={f.key}
                className="grid gap-1 border-b border-border pb-3 last:border-0 sm:grid-cols-[minmax(8rem,30%)_1fr]"
              >
                <dt className="text-sm font-medium text-muted">{f.label}</dt>
                <dd className="text-sm text-foreground whitespace-pre-wrap break-words">
                  {f.value || "—"}
                </dd>
              </div>
            ))}
          </dl>
          {!detail.canValidate && !detail.validated && detail.canView && (
            <Text variant="muted" className="text-xs">
              Consultation seule — seuls les rôles valideurs peuvent approuver cet enregistrement.
            </Text>
          )}
        </div>
      )}
    </Modal>
  );
}
