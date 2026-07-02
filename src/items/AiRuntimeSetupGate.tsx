import { useCallback, useEffect, useState, type ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { FolderOpen, HardDriveDownload } from "lucide-react";
import { useAuth } from "@/hooks/useAuth";
import { Modal } from "@/components/ui/Modal";
import { Alert } from "@/items/Alert";
import { Button } from "@/items/Button";
import { Input } from "@/items/Input";
import { Text } from "@/items/Text";
import {
  AI_INSTALL_DONE_EVENT,
  AI_INSTALL_ERROR_EVENT,
  AI_INSTALL_PROGRESS_EVENT,
  fetchAiRuntimeStatus,
  pickAiInstallDirectory,
  startAiRuntimeInstall,
} from "@/lib/aiRuntime";
import { runAiStartupSequence } from "@/lib/aiStartup";
import type { AiInstallProgress, AiRuntimeStatus } from "@/types/ai";

/** Affiche l'UI tout de suite ; modal d'installation Loggy uniquement si le runtime IA manque. */
export function AiRuntimeSetupGate({ children }: { children: ReactNode }) {
  const { user } = useAuth();
  const [needsSetup, setNeedsSetup] = useState(false);
  const [status, setStatus] = useState<AiRuntimeStatus | null>(null);
  const [installDir, setInstallDir] = useState("");
  const [installing, setInstalling] = useState(false);
  const [progress, setProgress] = useState<AiInstallProgress | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refreshStatus = useCallback(async () => {
    setError(null);
    try {
      const s = await fetchAiRuntimeStatus();
      setStatus(s);
      setInstallDir(s.install_dir ?? s.default_install_dir);
      setNeedsSetup(!s.ready);
    } catch (e) {
      console.warn("[Blin] Statut runtime IA :", e);
      setNeedsSetup(false);
    }
  }, []);

  useEffect(() => {
    void refreshStatus();
  }, [refreshStatus]);

  useEffect(() => {
    if (!user || needsSetup || !status?.ready) return;
    void runAiStartupSequence();
  }, [user, needsSetup, status?.ready]);

  useEffect(() => {
    if (!installing) return;
    const unsubs: Array<Promise<() => void>> = [];

    unsubs.push(
      listen<AiInstallProgress>(AI_INSTALL_PROGRESS_EVENT, (ev) => {
        setProgress(ev.payload);
      }),
    );
    unsubs.push(
      listen(AI_INSTALL_DONE_EVENT, () => {
        setInstalling(false);
        setProgress(null);
        void refreshStatus();
      }),
    );
    unsubs.push(
      listen<string>(AI_INSTALL_ERROR_EVENT, (ev) => {
        setInstalling(false);
        setProgress(null);
        setError(typeof ev.payload === "string" ? ev.payload : "Installation echouee.");
      }),
    );

    return () => {
      void Promise.all(unsubs).then((fns) => fns.forEach((fn) => fn()));
    };
  }, [installing, refreshStatus]);

  const handleBrowse = async () => {
    const picked = await pickAiInstallDirectory(installDir || status?.default_install_dir);
    if (picked) setInstallDir(picked);
  };

  const handleInstall = async () => {
    const dir = installDir.trim();
    if (!dir) {
      setError("Indiquez un dossier d'installation.");
      return;
    }
    setError(null);
    setProgress({ phase: "prepare", percent: 0, message: "Demarrage…" });
    setInstalling(true);
    try {
      await startAiRuntimeInstall(dir);
    } catch (e) {
      setInstalling(false);
      setProgress(null);
      setError(String(e));
    }
  };

  return (
    <>
      {children}
      <Modal
        open={needsSetup}
        onClose={() => undefined}
        title="Installation de Loggy (IA locale)"
        size="lg"
        closeDisabled
        busy={installing}
        busyLabel={progress?.message ?? "Installation en cours…"}
        footer={
          <div className="flex flex-wrap justify-end gap-2">
            <Button
              type="button"
              variant="secondary"
              disabled={installing}
              onClick={() => void handleBrowse()}
            >
              <FolderOpen className="h-4 w-4" />
              Parcourir…
            </Button>
            <Button type="button" disabled={installing} onClick={() => void handleInstall()}>
              <HardDriveDownload className="h-4 w-4" />
              {installing ? "Installation…" : "Installer ici"}
            </Button>
          </div>
        }
      >
        <div className="space-y-4">
          <Text variant="muted" className="text-sm">
            Choisissez où installer l&apos;assistant Loggy sur ce PC : le serveur llama-server et le
            modele Ministral 8B (~5 Go) seront telecharges dans ce dossier, separement de
            l&apos;application Blin et de vos donnees metier.
          </Text>

          <Input
            label="Dossier d'installation IA"
            value={installDir}
            disabled={installing}
            onChange={(e) => setInstallDir(e.target.value)}
            placeholder={status?.default_install_dir ?? "C:\\…\\Blin\\Loggy"}
          />

          {progress && installing && (
            <div className="space-y-2">
              <div className="h-2 overflow-hidden rounded-full bg-surface-elevated">
                <div
                  className="h-full bg-secondary transition-all duration-300"
                  style={{ width: `${progress.percent}%` }}
                />
              </div>
              <Text variant="muted" className="text-xs">
                {progress.message} ({progress.percent} %)
              </Text>
            </div>
          )}

          {error && <Alert variant="danger" size="field" message={error} />}

          <Alert
            variant="info"
            size="field"
            message="Connexion Internet requise uniquement pour cette etape. Ensuite Loggy fonctionne hors ligne."
          />
        </div>
      </Modal>
    </>
  );
}
