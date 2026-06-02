import { useState, type FormEvent } from "react";
import { Lock } from "lucide-react";
import { useAuth } from "@/hooks/useAuth";
import { useEntityBranding } from "@/hooks/useEntityBranding";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";

const DEFAULT_ADMIN_EMAIL = "admin@blin.local";

export function LoginPage() {
  const { login } = useAuth();
  const { title, slogan, logoSrc } = useEntityBranding();
  const [email, setEmail] = useState(DEFAULT_ADMIN_EMAIL);
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      await login(email.trim(), password);
    } catch (err) {
      setError(String(err));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="login-page flex min-h-screen items-center justify-center px-4 py-10">
      <div className="card-panel w-full max-w-md rounded-2xl border border-border p-8 shadow-2xl">
        <div className="mb-8 flex flex-col items-center gap-4 text-center">
          <img src={logoSrc} alt={title} className="h-16 w-16 object-contain" />
          <div>
            <h1 className="font-brand-serif text-3xl font-normal tracking-tight screen-title-gradient">
              {title}
            </h1>
            {slogan.trim() && (
              <p className="mt-2 text-sm text-muted">{slogan}</p>
            )}
          </div>
        </div>

        <form className="flex flex-col gap-5" onSubmit={(e) => void onSubmit(e)}>
          <Input
            label="Adresse e-mail"
            type="email"
            autoComplete="username"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder={DEFAULT_ADMIN_EMAIL}
            required
          />
          <Input
            label="Mot de passe"
            type="password"
            autoComplete="current-password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
          />

          {error && (
            <p className="rounded-lg border border-primary/40 bg-primary/10 px-3 py-2 text-sm text-primary">
              {error}
            </p>
          )}

          <Button type="submit" variant="secondary" size="lg" disabled={submitting} className="w-full">
            <Lock className="mr-2 h-4 w-4" />
            {submitting ? "Connexion…" : "Se connecter"}
          </Button>
        </form>

        <p className="mt-6 text-center text-xs leading-relaxed text-muted">
          Première installation : connectez-vous avec{" "}
          <span className="text-secondary">{DEFAULT_ADMIN_EMAIL}</span> et le mot de passe{" "}
          <span className="text-secondary">admin1234</span>. Vous devrez le modifier immédiatement.
        </p>
      </div>
    </div>
  );
}
