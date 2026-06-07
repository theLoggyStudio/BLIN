import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ClipboardCopy } from "lucide-react";
import { Button } from "@/items/Button";
import { Input } from "@/items/Input";
import { Modal } from "@/items/Modal";
import { Text } from "@/items/Text";
import { useIsAppAdmin } from "@/hooks/useIsAppAdmin";
import { buildEntityRegistryAiPrompt } from "@/lib/entityRegistryAiPrompt";
import type { RoleRow } from "@/types/users";

interface EntityRegistryPromptButtonProps {
  ecosysteme?: string;
  slogan?: string;
}

/** Copie un prompt IA pour générer un registry.json adapté au domaine saisi (admin uniquement). */
export function EntityRegistryPromptButton({
  ecosysteme,
  slogan,
}: EntityRegistryPromptButtonProps) {
  const isAdmin = useIsAppAdmin();
  const [open, setOpen] = useState(false);
  const [domain, setDomain] = useState("");
  const [feedback, setFeedback] = useState<string | null>(null);
  const [copying, setCopying] = useState(false);

  if (!isAdmin) {
    return null;
  }

  const openModal = () => {
    setFeedback(null);
    setOpen(true);
  };

  const copyPrompt = async () => {
    const domainHint = domain.trim();
    if (!domainHint) {
      setFeedback("Indiquez le domaine métier visé.");
      return;
    }
    setFeedback(null);
    setCopying(true);
    try {
      let roles: RoleRow[] = [];
      try {
        roles = await invoke<RoleRow[]>("users_list_roles");
      } catch {
        roles = [];
      }
      const text = buildEntityRegistryAiPrompt({
        currentEcosystem: ecosysteme,
        currentSlogan: slogan,
        domainHint,
        roles,
      });
      await navigator.clipboard.writeText(text);
      setFeedback("Prompt copié — collez-le dans votre IA.");
      window.setTimeout(() => {
        setOpen(false);
        setFeedback(null);
      }, 1200);
    } catch {
      setFeedback("Impossible de copier — autorisez le presse-papiers.");
    } finally {
      setCopying(false);
    }
  };

  return (
    <>
      <div className="flex flex-col items-end gap-1">
        <Button
          size="sm"
          variant="outline"
          type="button"
          title="Générer un prompt IA pour créer le JSON d'écosystème (domaine au choix)"
          onClick={openModal}
        >
          <ClipboardCopy className="h-4 w-4" />
          Prompt IA
        </Button>
      </div>

      <Modal
        open={open}
        onClose={() => setOpen(false)}
        title="Prompt IA — nouvel écosystème"
        size="md"
        footer={
          <div className="flex w-full flex-wrap items-center justify-between gap-2">
            {feedback && (
              <span className="text-xs text-secondary" role="status">
                {feedback}
              </span>
            )}
            <div className="ml-auto flex gap-2">
              <Button variant="ghost" type="button" onClick={() => setOpen(false)}>
                Annuler
              </Button>
              <Button
                type="button"
                disabled={!domain.trim() || copying}
                onClick={() => void copyPrompt()}
              >
                <ClipboardCopy className="h-4 w-4" />
                Copier le prompt
              </Button>
            </div>
          </div>
        }
      >
        <div className="space-y-4">
          <Text variant="muted">
            Blin s&apos;adapte à <strong className="text-foreground">n&apos;importe quel métier</strong>.
            Indiquez le domaine visé : le prompt demandera à l&apos;IA un{" "}
            <code className="text-secondary">registry.json</code> entièrement adapté (entités, champs,
            liaisons).
          </Text>

          <Input
            label="Domaine métier visé"
            value={domain}
            onChange={(e) => setDomain(e.target.value)}
            placeholder="ex. cabinet vétérinaire, club de tennis, atelier mécanique, ONG humanitaire…"
            hint="Obligatoire — c'est la base pour nommer l'écosystème et concevoir les entités."
            autoFocus
          />

          {(ecosysteme?.trim() || slogan?.trim()) && (
            <div className="rounded-lg border border-border bg-background px-3 py-2 text-sm text-muted">
              <p className="font-medium text-foreground">Valeurs actuelles (reprises si utile)</p>
              {ecosysteme?.trim() && (
                <p>
                  Écosystème : <span className="text-foreground">{ecosysteme}</span>
                </p>
              )}
              {slogan?.trim() && (
                <p>
                  Slogan : <span className="text-foreground">{slogan}</span>
                </p>
              )}
            </div>
          )}

          <Text variant="muted" className="text-xs">
            Après copie : collez le prompt dans ChatGPT, Claude, etc., puis importez le JSON dans{" "}
            <strong className="text-foreground">Vue JSON</strong> et enregistrez.
          </Text>
        </div>
      </Modal>
    </>
  );
}
