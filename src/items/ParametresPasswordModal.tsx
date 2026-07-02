import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Alert } from "@/items/Alert";
import { Button } from "@/items/Button";
import { Input } from "@/items/Input";
import { Modal } from "@/items/Modal";
import { Text } from "@/items/Text";

interface ParametresPasswordModalProps {
  open: boolean;
  onClose: () => void;
  onVerified: () => void;
}

/** Ré-authentification avant dépliage d'un panneau Paramètres. */
export function ParametresPasswordModal({
  open,
  onClose,
  onVerified,
}: ParametresPasswordModalProps) {
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (!open) return;
    setPassword("");
    setError(null);
    const t = window.setTimeout(() => inputRef.current?.focus(), 50);
    return () => window.clearTimeout(t);
  }, [open]);

  const submit = async () => {
    if (!password.trim()) {
      setError("Saisissez votre mot de passe.");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await invoke("auth_verify_password", { payload: { password } });
      onVerified();
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal open={open} onClose={onClose} title="Confirmer votre identité" size="sm">
      <Text variant="muted" className="mb-4 text-sm">
        Saisissez votre mot de passe pour déplier cette section des paramètres.
      </Text>
      {error && (
        <Alert variant="danger" size="inline" className="mb-3" message={error} />
      )}
      <form
        onSubmit={(e) => {
          e.preventDefault();
          void submit();
        }}
      >
        <Input
          ref={inputRef}
          label="Mot de passe"
          type="password"
          autoComplete="current-password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          disabled={busy}
        />
        <div className="mt-4 flex justify-end gap-2">
          <Button type="button" variant="ghost" onClick={onClose} disabled={busy}>
            Annuler
          </Button>
          <Button type="submit" disabled={busy}>
            {busy ? "Vérification…" : "Confirmer"}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
