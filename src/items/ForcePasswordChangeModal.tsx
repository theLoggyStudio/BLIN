import { useState, type FormEvent } from "react";
import { KeyRound } from "lucide-react";
import { useAuth } from "@/hooks/useAuth";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";

/** Bloque l'application tant que le mot de passe d'usine n'a pas été remplacé. */
export function ForcePasswordChangeModal() {
  const { changePassword } = useAuth();
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    if (password.length < 6) {
      setError("Le mot de passe doit contenir au moins 6 caractères.");
      return;
    }
    if (password !== confirm) {
      setError("Les mots de passe ne correspondent pas.");
      return;
    }
    setSubmitting(true);
    try {
      await changePassword(password, confirm);
    } catch (err) {
      setError(String(err));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <dialog
      open
      className="app-modal-dialog w-full max-w-md"
    >
      <div className="flex flex-col rounded-xl border border-border bg-card p-0 text-foreground shadow-2xl">
      <div className="border-b border-border px-6 py-4">
        <h2 className="text-lg font-semibold screen-title-gradient">Nouveau mot de passe</h2>
        <p className="mt-1 text-sm text-muted">
          Pour des raisons de sécurité, remplacez le mot de passe d&apos;usine avant de continuer.
        </p>
      </div>

      <form className="flex flex-col gap-4 px-6 py-5" onSubmit={(e) => void onSubmit(e)}>
        <Input
          label="Nouveau mot de passe"
          type="password"
          autoComplete="new-password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          required
          minLength={6}
        />
        <Input
          label="Confirmer le mot de passe"
          type="password"
          autoComplete="new-password"
          value={confirm}
          onChange={(e) => setConfirm(e.target.value)}
          required
          minLength={6}
        />

        {error && (
          <p className="rounded-lg border border-primary/40 bg-primary/10 px-3 py-2 text-sm text-primary">
            {error}
          </p>
        )}

        <Button type="submit" variant="secondary" size="lg" disabled={submitting} className="w-full">
          <KeyRound className="mr-2 h-4 w-4" />
          {submitting ? "Enregistrement…" : "Enregistrer et continuer"}
        </Button>
      </form>
      </div>
    </dialog>
  );
}
