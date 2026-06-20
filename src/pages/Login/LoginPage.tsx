import { useCallback, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Copy, Lock, QrCode } from "lucide-react";
import { useAuth } from "@/hooks/useAuth";
import { useEntityBranding } from "@/hooks/useEntityBranding";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { Alert } from "@/items/Alert";
import { Modal } from "@/items/Modal";
import { QRCodeSVG } from "qrcode.react";
import { getInvalidCredentialsMessage } from "@/lib/loginMessagesCache";
import { DEFAULT_ADMIN_EMAIL } from "@/constants/variable.constant";

interface RemoteConnectionResponse {
  ip?: string;
  url?: string;
  frontUrl?: string;
  success?: boolean;
}

export function LoginPage() {
  const { login } = useAuth();
  const { title, slogan, logoSrc } = useEntityBranding();
  const [email, setEmail] = useState(DEFAULT_ADMIN_EMAIL);
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [qrOpen, setQrOpen] = useState(false);
  const [copied, setCopied] = useState(false);
  const [scanUrl, setScanUrl] = useState("");
  const [scanLoading, setScanLoading] = useState(false);
  const [scanError, setScanError] = useState<string | null>(null);

  const loadScanUrl = useCallback(async () => {
    setScanLoading(true);
    setScanError(null);
    try {
      const data = await invoke<RemoteConnectionResponse>("remote_connection_get");
      if (data?.success && (data.frontUrl || data.url)) {
        setScanUrl(data.frontUrl || data.url || "");
      } else {
        setScanUrl("");
        setScanError("Impossible d'obtenir l'adresse réseau. Vérifiez votre connexion.");
      }
    } catch {
      setScanUrl("");
      setScanError("Impossible d'obtenir l'adresse réseau du PC.");
    } finally {
      setScanLoading(false);
    }
  }, []);

  const openQrModal = () => {
    setQrOpen(true);
    void loadScanUrl();
  };

  const closeQrModal = () => {
    setQrOpen(false);
    setScanUrl("");
    setScanError(null);
  };

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    setSubmitting(true);
    try {
      await login(email.trim(), password);
    } catch (err) {
      const raw = String(err);
      const isInvalid =
        raw.includes("Identifiants invalides") ||
        raw.toLowerCase().includes("mot de passe") ||
        raw.toLowerCase().includes("e-mail");
      setError(isInvalid ? getInvalidCredentialsMessage(raw) : raw);
    } finally {
      setSubmitting(false);
    }
  };

  const sendMailTo = () => {
    const subject = "Connexion Blin depuis mobile";
    const body = `Scannez ou ouvrez ce lien depuis votre mobile :\n${scanUrl}\n\nAttention : l'URL peut changer après redémarrage de l'application.`;
    window.location.href = `mailto:?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`;
  };

  const copyUrl = async () => {
    if (!scanUrl) return;
    try {
      await navigator.clipboard.writeText(scanUrl);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      setCopied(false);
    }
  };

  return (
    <div className="login-page flex min-h-screen items-center justify-center px-4 py-10">
      <div className="relative w-full max-w-md rounded-2xl border border-border bg-card p-8 shadow-2xl">
        <button
          type="button"
          onClick={openQrModal}
          className="absolute right-4 top-4 z-10 flex h-9 w-9 items-center justify-center rounded-full bg-secondary text-white shadow-lg transition hover:scale-105 hover:opacity-95"
          aria-label="Scannez pour vous connecter"
          title="Scannez pour vous connecter"
        >
          <QrCode className="h-5 w-5" />
        </button>
        <div className="mb-8 flex flex-col items-center gap-4 text-center">
          <img src={logoSrc} alt={title} className="h-32 w-32 object-contain" />
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

          {error && <Alert variant="danger" size="box" message={error} />}

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

      <Modal
        open={qrOpen}
        onClose={closeQrModal}
        title="Scannez pour vous connecter"
        size="sm"
      >
        <div className="flex flex-col items-center gap-4">
          {scanLoading ? (
            <div className="flex h-[190px] w-[190px] items-center justify-center rounded-lg border border-border bg-surface-elevated text-sm text-muted">
              Chargement...
            </div>
          ) : scanUrl ? (
            <div className="rounded-lg bg-white p-3">
              <QRCodeSVG value={scanUrl} size={190} />
            </div>
          ) : (
            <div className="flex h-[190px] w-[190px] items-center justify-center rounded-lg border border-border bg-surface-elevated px-4 text-center text-sm text-muted">
              {scanError || "Adresse indisponible"}
            </div>
          )}
          {scanError && (
            <Alert variant="danger" size="box" centered className="text-xs" message={scanError} />
          )}
          <p className="text-xs text-muted text-center">
            Utilisez l&apos;adresse IP locale du PC (même réseau Wi‑Fi). Le jeton QR expire après 15 minutes.
          </p>
          <div className="w-full rounded-lg border border-border bg-background px-3 py-2 text-xs text-muted break-all">
            {scanUrl || "—"}
          </div>
          <div className="flex w-full gap-2">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="flex-1"
              onClick={copyUrl}
              disabled={!scanUrl}
            >
              <Copy className="mr-2 h-4 w-4" />
              {copied ? "Copié" : "Copier l'URL"}
            </Button>
            <Button
              type="button"
              variant="secondary"
              size="sm"
              className="flex-1"
              onClick={sendMailTo}
              disabled={!scanUrl}
            >
              Envoyer par e-mail
            </Button>
          </div>
        </div>
      </Modal>
    </div>
  );
}
