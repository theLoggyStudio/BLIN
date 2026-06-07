import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Alert } from "@/items/Alert";
import { Button } from "@/items/Button";
import { Modal } from "@/items/Modal";
import type { RecordSignatureDetail, SignatoryContact } from "@/types/entity";

interface EntitySignatureModalProps {
  entityKey: string;
  recordId: string;
  open: boolean;
  onClose: () => void;
  onSigned?: () => void;
  onRejected?: () => void;
}

function SignatoryContactsList({ contacts }: { contacts: SignatoryContact[] }) {
  if (contacts.length === 0) {
    return (
      <p className="text-sm text-muted">
        Aucun utilisateur actif avec un rôle signataire n&apos;a été trouvé. Contactez un
        administrateur.
      </p>
    );
  }
  return (
    <ul className="mt-2 space-y-1.5 text-sm text-foreground">
      {contacts.map((c) => (
        <li key={c.userId} className="rounded-md border border-border bg-background px-3 py-2">
          <span className="font-medium">{c.nom}</span>
          <span className="text-muted"> — {c.roleNom}</span>
          {c.email.trim() && (
            <span className="block text-xs text-muted">{c.email}</span>
          )}
        </li>
      ))}
    </ul>
  );
}

/** Fiche en lecture seule + signature / refus pour objets « non signés ». */
export function EntitySignatureModal({
  entityKey,
  recordId,
  open,
  onClose,
  onSigned,
  onRejected,
}: EntitySignatureModalProps) {
  const [detail, setDetail] = useState<RecordSignatureDetail | null>(null);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [rejectMode, setRejectMode] = useState(false);
  const [rejectReason, setRejectReason] = useState("");
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    if (!open || !recordId) return;
    setLoading(true);
    setError(null);
    try {
      const d = await invoke<RecordSignatureDetail>("entity_record_signature_detail", {
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
      setRejectMode(false);
      setRejectReason("");
    }
  }, [open, load]);

  const handleSign = async () => {
    setSaving(true);
    setError(null);
    try {
      await invoke("entity_record_sign", {
        payload: { entity_key: entityKey, record_id: recordId },
      });
      onSigned?.();
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleReject = async () => {
    setSaving(true);
    setError(null);
    try {
      await invoke("entity_record_reject", {
        payload: {
          entity_key: entityKey,
          record_id: recordId,
          reason: rejectReason.trim() || null,
        },
      });
      onRejected?.();
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  if (!open) return null;

  const pending = detail && !detail.signed && !detail.rejected;
  const showPendingNotice = pending && !detail.canSign;
  const showSignActions = detail && !detail.signed && detail.canSign;

  const titleSuffix = detail?.signed
    ? "Fiche signée"
    : detail?.rejected
      ? "Signature refusée"
      : "Non signé";

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={detail ? `${detail.entityLabel} — ${titleSuffix}` : "Signature"}
      size="lg"
      footer={
        <div className="flex justify-end">
          <Button variant="ghost" onClick={onClose} disabled={saving}>
            Fermer
          </Button>
        </div>
      }
    >
      {loading && (
        <p className="py-8 text-center text-sm text-muted">Chargement de la fiche…</p>
      )}
      {!loading && error && <Alert variant="danger" size="inline" message={error} />}
      {!loading && detail && (
        <div className="flex max-h-[70vh] flex-col gap-4">
          <div className="min-h-0 flex-1 space-y-4 overflow-y-auto pr-1">
            {showPendingNotice && (
              <Alert variant="warning" size="box" className="py-3 text-amber-100">
                <p>
                  <strong className="text-amber-50">{detail.entityLabel}</strong> non signé.
                  Veuillez demander aux personnes suivantes :
                </p>
                <SignatoryContactsList contacts={detail.signatoryContacts} />
              </Alert>
            )}

            {showSignActions && !rejectMode && (
              <Alert
                variant="info"
                size="box"
                className="text-foreground"
                message="Cet objet doit être signé avant d'être utilisable dans une liaison. Contrôlez la fiche ci-dessous puis signez ou refusez (une seule décision suffit)."
              />
            )}

            {detail.signed && (
              <Alert
                variant="success"
                size="box"
                className="text-emerald-300"
                message="Objet signé — modifications interdites."
              />
            )}

            {detail.rejected && (
              <Alert variant="danger" size="box" className="text-red-200">
                <p>
                  Signature refusée
                  {detail.refusedBy ? ` par ${detail.refusedBy}` : ""}.
                </p>
                {detail.refusalReason?.trim() && (
                  <p className="mt-2 text-sm">
                    <span className="font-medium">Motif :</span> {detail.refusalReason}
                  </p>
                )}
                {detail.canSign ? (
                  <p className="mt-2 text-sm">
                    Vous pouvez réaccepter cet objet en le signant ci-dessous.
                  </p>
                ) : (
                  <p className="mt-2 text-sm">
                    Un signataire peut réaccepter cet objet par signature.
                  </p>
                )}
              </Alert>
            )}

            {detail.canView && detail.fields.length > 0 && (
              <dl className="space-y-3">
                {detail.fields.map((f) => (
                  <div
                    key={f.key}
                    className="grid gap-1 border-b border-border pb-3 last:border-0 sm:grid-cols-[minmax(8rem,30%)_1fr]"
                  >
                    <dt className="text-sm font-medium text-muted">{f.label}</dt>
                    <dd className="whitespace-pre-wrap break-words text-sm text-foreground">
                      {f.value || "—"}
                    </dd>
                  </div>
                ))}
              </dl>
            )}

            {!detail.canView && pending && !showPendingNotice && (
              <p className="text-sm text-muted">
                Vous n&apos;avez pas accès au détail de cette fiche.
              </p>
            )}
          </div>

          {showSignActions && (
            <div className="shrink-0 space-y-3 border-t border-border pt-4">
              {rejectMode ? (
                <>
                  <label className="block text-sm font-medium text-foreground">
                    Motif du refus (optionnel)
                    <textarea
                      className="mt-1.5 w-full rounded-md border border-border bg-background px-3 py-2 text-sm text-foreground"
                      rows={3}
                      value={rejectReason}
                      onChange={(e) => setRejectReason(e.target.value)}
                      placeholder="Indiquez la raison du refus…"
                      disabled={saving}
                    />
                  </label>
                  <div className="flex flex-col gap-2 sm:flex-row">
                    <Button
                      variant="ghost"
                      className="sm:flex-1"
                      onClick={() => {
                        setRejectMode(false);
                        setRejectReason("");
                      }}
                      disabled={saving}
                    >
                      Annuler
                    </Button>
                    <Button
                      variant="danger"
                      className="sm:flex-1"
                      onClick={() => void handleReject()}
                      disabled={saving || loading}
                    >
                      {saving ? "Refus…" : "Confirmer le refus"}
                    </Button>
                  </div>
                </>
              ) : (
                <div className="flex flex-col gap-2 sm:flex-row">
                  {detail.canReject && (
                    <Button
                      variant="danger"
                      className="sm:flex-1"
                      onClick={() => setRejectMode(true)}
                      disabled={saving || loading}
                    >
                      REFUSER
                    </Button>
                  )}
                  <Button
                    className="sm:flex-1"
                    size="lg"
                    onClick={() => void handleSign()}
                    disabled={saving || loading}
                  >
                    {saving
                      ? "Signature…"
                      : detail.rejected
                        ? "RÉACCEPTER (SIGNER)"
                        : "SIGNER"}
                  </Button>
                </div>
              )}
            </div>
          )}
        </div>
      )}
    </Modal>
  );
}

/** @deprecated Utiliser EntitySignatureModal */
export { EntitySignatureModal as EntityValidationModal };
