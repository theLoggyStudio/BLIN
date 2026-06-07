import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/items/Button";
import { Modal } from "@/items/Modal";
import type { RecordSignatureDetail, SignatoryContact } from "@/types/entity";

interface EntitySignatureModalProps {
  entityKey: string;
  recordId: string;
  open: boolean;
  onClose: () => void;
  onSigned?: () => void;
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

/** Fiche en lecture seule + signature pour objets « non signés ». */
export function EntitySignatureModal({
  entityKey,
  recordId,
  open,
  onClose,
  onSigned,
}: EntitySignatureModalProps) {
  const [detail, setDetail] = useState<RecordSignatureDetail | null>(null);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
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

  if (!open) return null;

  const showPendingNotice = detail && !detail.signed && !detail.canSign;
  const showSignAction = detail?.canSign && !detail.signed;

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={
        detail
          ? `${detail.entityLabel} — ${detail.signed ? "Fiche signée" : "Non signé"}`
          : "Signature"
      }
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
      {!loading && error && (
        <p className="text-sm text-primary" role="alert">
          {error}
        </p>
      )}
      {!loading && detail && (
        <div className="flex max-h-[70vh] flex-col gap-4">
          <div className="min-h-0 flex-1 space-y-4 overflow-y-auto pr-1">
            {showPendingNotice && (
              <div className="rounded-lg border border-amber-500/40 bg-amber-500/10 px-3 py-3 text-sm text-amber-100">
                <p>
                  <strong className="text-amber-50">{detail.entityLabel}</strong> non signé.
                  Veuillez demander aux personnes suivantes :
                </p>
                <SignatoryContactsList contacts={detail.signatoryContacts} />
              </div>
            )}

            {detail.canSign && !detail.signed && (
              <div className="rounded-lg border border-secondary/40 bg-secondary/10 px-3 py-2 text-sm text-foreground">
                Cet objet doit être signé avant d&apos;être utilisable dans une liaison. Contrôlez
                la fiche ci-dessous puis signez (une seule signature suffit).
              </div>
            )}

            {detail.signed && (
              <div className="rounded-lg border border-emerald-500/30 bg-emerald-500/10 px-3 py-2 text-sm text-emerald-300">
                Objet signé — modifications interdites.
              </div>
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

            {!detail.canView && !detail.signed && !showPendingNotice && (
              <p className="text-sm text-muted">
                Vous n&apos;avez pas accès au détail de cette fiche.
              </p>
            )}
          </div>

          {showSignAction && (
            <div className="shrink-0 border-t border-border pt-4">
              <Button
                className="w-full"
                size="lg"
                onClick={() => void handleSign()}
                disabled={saving || loading}
              >
                {saving ? "Signature…" : "SIGNER"}
              </Button>
            </div>
          )}
        </div>
      )}
    </Modal>
  );
}

/** @deprecated Utiliser EntitySignatureModal */
export { EntitySignatureModal as EntityValidationModal };
