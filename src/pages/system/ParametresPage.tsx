import { useCallback, useEffect, useMemo, useState } from "react";
import { Alert } from "@/items/Alert";
import { Guard } from "@/components/Guard";
import { invoke } from "@tauri-apps/api/core";
import { ChevronsDownUp, ChevronsUpDown, Cpu, Palette, RefreshCw, User } from "lucide-react";
import {
  allPanelsClosed,
  allPanelsOpen,
  loadParametresPanelsState,
  saveParametresPanelsState,
  type ParametresPanelId,
  type ParametresPanelsState,
} from "@/lib/parametresPanels";
import { usePrivilege } from "@/hooks/usePrivilege";
import { useParametresPanelUnlock } from "@/hooks/useParametresPanelUnlock";
import {
  privilegeForParametresPanel,
} from "@/lib/parametresPrivileges";
import { RolesPanel } from "@/items/RolesPanel";
import { UsersPanel } from "@/items/UsersPanel";
import { ImportExportPanel } from "@/items/ImportExportPanel";
import { EntityPanel } from "@/items/EntityPanel";
import { RegistryArchivePanel } from "@/items/RegistryArchivePanel";
import { PrintModelsPanel } from "@/items/PrintModelsPanel";
import { ThemePanel } from "@/items/ThemePanel";
import { useAuth } from "@/hooks/useAuth";
import { useEntityBranding } from "@/hooks/useEntityBranding";
import { PersonnalisationIaPanel } from "@/items/PersonnalisationIaPanel";
import { ParametresPasswordModal } from "@/items/ParametresPasswordModal";
import { Button } from "@/items/Button";
import { CollapsiblePanel } from "@/items/CollapsiblePanel";
import { Input } from "@/items/Input";
import { Select } from "@/items/Select";
import { Text } from "@/items/Text";
import type { AiStatus, AiVisionConfigPublic, AiWebSearchConfig } from "@/types/ai";

function llamaServerStatus(status: AiStatus): { value: string; ok?: boolean } {
  if (!status.model_present) {
    return { value: "Modèle absent — installez le GGUF", ok: false };
  }
  if (!status.llama_bin) {
    return { value: "Binaire llama-server introuvable", ok: false };
  }
  if (status.server_healthy) {
    return { value: "Actif", ok: true };
  }
  return {
    value: "Arrêté — démarre au 1er message ou via le bouton ci-dessous",
    ok: false,
  };
}

function StatusLine({ label, value, ok }: { label: string; value: string; ok?: boolean }) {
  return (
    <div className="flex flex-wrap items-baseline justify-between gap-2 py-2 border-b border-border last:border-0">
      <span className="text-sm text-muted">{label}</span>
      <span
        className={
          ok === undefined
            ? "text-sm text-foreground font-mono text-right break-all max-w-[70%]"
            : ok
              ? "text-sm text-emerald-400"
              : "text-sm text-amber-400"
        }
      >
        {value}
      </span>
    </div>
  );
}

function resolveSqliteFilePaths(status: AiStatus): string[] {
  if (status.db_paths?.length) return status.db_paths;
  if (status.db_path) return [status.db_path];
  const dir = status.db_dir.replace(/[/\\]+$/, "");
  return [`${dir}\\blin-gestion.sqlite`];
}

/** Écran système — compte, runtime Loggy et maintenance DDA. */
import { ENTITY_REGISTRY_SYNCED_EVENT } from "@/constants/events";

export function ParametresPage() {
  const { title } = useEntityBranding();
  const { user, syncSessionPrivileges } = useAuth();
  const canParamAssistant = usePrivilege("parametres:assistant");
  const canParamPersonnalisation = usePrivilege("parametres:personnalisation_ia");
  const canParamCompte = usePrivilege("parametres:compte");
  const canParamTheme = usePrivilege("parametres:theme");
  const canParamImpression = usePrivilege("parametres:impression");
  const canParamEntites = usePrivilege("parametres:entites");
  const canParamEntitesCreer = usePrivilege("parametres:entites:creer");
  const canParamArchives = usePrivilege("parametres:archives");
  const canParamImportsExports = usePrivilege("parametres:imports_exports");
  const canParamRoles = usePrivilege("parametres:roles");
  const canParamUtilisateurs = usePrivilege("parametres:utilisateurs");
  const {
    passwordModalOpen,
    requestUnlock,
    closePasswordModal,
    onPasswordVerified,
  } = useParametresPanelUnlock();
  const [panelsOpen, setPanelsOpen] = useState<ParametresPanelsState>(loadParametresPanelsState);
  const [status, setStatus] = useState<AiStatus | null>(null);
  const [loadingStatus, setLoadingStatus] = useState(true);
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [webSearch, setWebSearch] = useState(true);
  const [visionConfig, setVisionConfig] = useState<AiVisionConfigPublic | null>(null);
  const [visionProvider, setVisionProvider] = useState("openrouter");
  const [visionApiKey, setVisionApiKey] = useState("");
  const [visionModel, setVisionModel] = useState("");

  const loadStatus = useCallback(async () => {
    setLoadingStatus(true);
    setError(null);
    try {
      const [s, web, vision] = await Promise.all([
        invoke<AiStatus>("ai_status"),
        invoke<AiWebSearchConfig>("ai_web_search_get_config"),
        invoke<AiVisionConfigPublic>("ai_vision_get_config"),
      ]);
      setStatus(s);
      setWebSearch(web.enabled);
      setVisionConfig(vision);
      setVisionProvider(vision.provider || "openrouter");
      setVisionModel(vision.model || "");
    } catch (e) {
      setStatus(null);
      setError(String(e));
    } finally {
      setLoadingStatus(false);
    }
  }, []);

  useEffect(() => {
    if (!canParamAssistant) return;
    void loadStatus();
  }, [loadStatus, canParamAssistant]);

  const runAction = async (key: string, fn: () => Promise<string>) => {
    setBusy(key);
    setMessage(null);
    setError(null);
    try {
      const msg = await fn();
      setMessage(msg);
      await loadStatus();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  };

  const modelReady = status?.model_present && status?.llama_bin;

  const visiblePanelIds = useMemo((): ParametresPanelId[] => {
    const ids: ParametresPanelId[] = [];
    if (canParamAssistant) ids.push("assistant");
    if (canParamPersonnalisation) ids.push("personnalisation_ia");
    if (canParamCompte) ids.push("compte");
    if (canParamTheme) ids.push("theme");
    if (canParamImpression) ids.push("impression");
    if (canParamEntites || canParamEntitesCreer) ids.push("entites");
    if (canParamArchives) ids.push("archives");
    if (canParamImportsExports) ids.push("imports_exports");
    if (canParamRoles) ids.push("roles");
    if (canParamUtilisateurs) ids.push("utilisateurs");
    return ids;
  }, [
    canParamAssistant,
    canParamPersonnalisation,
    canParamCompte,
    canParamTheme,
    canParamImpression,
    canParamEntites,
    canParamEntitesCreer,
    canParamArchives,
    canParamImportsExports,
    canParamRoles,
    canParamUtilisateurs,
  ]);

  const setPanelOpen = useCallback((id: ParametresPanelId, open: boolean) => {
    setPanelsOpen((prev) => {
      const next = { ...prev, [id]: open };
      saveParametresPanelsState(next);
      return next;
    });
  }, []);

  const handlePanelOpenChange = useCallback(
    (id: ParametresPanelId, open: boolean) => {
      if (!open) {
        setPanelOpen(id, false);
        return;
      }
      void requestUnlock().then((ok) => {
        if (ok) setPanelOpen(id, true);
      });
    },
    [requestUnlock, setPanelOpen],
  );

  const expandAllPanels = () => {
    void requestUnlock().then((ok) => {
      if (!ok) return;
      setPanelsOpen((prev) => {
        const next = { ...prev };
        for (const id of visiblePanelIds) next[id] = true;
        saveParametresPanelsState(next);
        return next;
      });
    });
  };

  const collapseAllPanels = () => {
    setPanelsOpen((prev) => {
      const next = { ...prev };
      for (const id of visiblePanelIds) next[id] = false;
      saveParametresPanelsState(next);
      return next;
    });
  };

  const panelOpen = (id: ParametresPanelId) => panelsOpen[id] ?? false;

  return (
    <div className="h-full min-h-0 overflow-y-auto">
      <div className="mx-auto max-w-3xl px-6 py-8">
      <header className="mb-8">
        <Text variant="title" as="h1" className="screen-title-gradient text-3xl">
          Paramètres
        </Text>
        <div className="mt-2 rounded-lg border border-border bg-card px-4 py-2 text-sm text-muted">
          Compte, registre des entités métier et identité de l&apos;écosystème ({title}).
        </div>
      </header>

      {(message || error) && (
        <Alert
          variant={error ? "danger" : "info"}
          size="box"
          className="mb-6 px-4 py-3"
          role="status"
          message={error ?? message ?? ""}
        />
      )}

      <div className="space-y-6">
        {visiblePanelIds.length === 0 ? (
          <Alert
            variant="info"
            size="box"
            className="px-4 py-3"
            message="Aucune section Paramètres n'est accessible avec votre rôle actuel."
          />
        ) : (
          <>
        <div className="parametres-panels-toolbar" role="toolbar" aria-label="Panneaux Paramètres">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={expandAllPanels}
            disabled={allPanelsOpen(panelsOpen, visiblePanelIds)}
          >
            <ChevronsUpDown className="h-4 w-4" />
            Tout déplier
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={collapseAllPanels}
            disabled={allPanelsClosed(panelsOpen, visiblePanelIds)}
          >
            <ChevronsDownUp className="h-4 w-4" />
            Tout replier
          </Button>
        </div>

        <Guard privilege={privilegeForParametresPanel("assistant")}>
          <CollapsiblePanel
            title={`Assistant — ${title}`}
            subtitle="Modèle local, recherche Internet optionnelle"
            open={panelOpen("assistant")}
            onOpenChange={(open) => handlePanelOpenChange("assistant", open)}
            headerAction={
              <Button
                variant="ghost"
                size="sm"
                onClick={() => void loadStatus()}
                disabled={loadingStatus}
              >
                <RefreshCw className={`h-4 w-4 ${loadingStatus ? "animate-spin" : ""}`} />
              </Button>
            }
          >
            {loadingStatus && !status ? (
              <p className="text-sm text-muted">Chargement…</p>
            ) : status ? (
              <div>
                <StatusLine
                  label="Modèle GGUF"
                  value={
                    status.model_present
                      ? `${status.model_name} — présent`
                      : `${status.model_name} — absent`
                  }
                  ok={status.model_present}
                />
                <StatusLine
                  label="Binaire llama-server"
                  value={status.llama_bin ? "Prêt" : "Introuvable"}
                  ok={status.llama_bin}
                />
                <StatusLine
                  label="Dossier IA (Loggy)"
                  value={status.install_dir ?? "Non configuré — installez au premier lancement"}
                  ok={!!status.install_dir && status.model_present}
                />
                <StatusLine label="Serveur IA" {...llamaServerStatus(status)} />
                <StatusLine label="Backend" value={status.backend} />
                <StatusLine
                  label="GPU"
                  value={
                    status.gpu_enabled
                      ? `${status.gpu_layers} calques`
                      : "CPU uniquement"
                  }
                />
                <StatusLine
                  label="Contexte / threads"
                  value={`${status.ctx_size} tokens · ${status.threads} threads`}
                />
                <StatusLine
                  label="Profilage"
                  value={status.profiled ? status.profile_summary : "Non profilé"}
                  ok={status.profiled}
                />
                <StatusLine
                  label="Expérience locale"
                  value={`${status.experience_entries} entrée(s)`}
                />
                <StatusLine
                  label="Recherche Internet"
                  value={status.web_search_enabled ? "Activée (DuckDuckGo)" : "Désactivée"}
                  ok={status.web_search_enabled}
                />
                <StatusLine label="Dossier données" value={status.db_dir} />
                <div className="border-b border-border py-2 last:border-0">
                  <span className="text-sm text-muted">Fichiers base de données (.sqlite)</span>
                  <p className="mt-1 text-xs text-muted">
                    Blin utilise l&apos;extension <span className="font-mono">.sqlite</span>, pas{" "}
                    <span className="font-mono">.db</span>.
                  </p>
                  <ul className="mt-2 space-y-1">
                    {resolveSqliteFilePaths(status).map((p) => (
                      <li key={p} className="break-all font-mono text-xs text-foreground">
                        {p}
                      </li>
                    ))}
                  </ul>
                </div>
                {!modelReady && (
                  <p className="mt-4 text-xs text-muted break-all">
                    Chemin modèle : <span className="font-mono">{status.model_path}</span>
                  </p>
                )}
              </div>
            ) : (
              <p className="text-sm text-muted">Statut indisponible.</p>
            )}

            <label className="mt-4 flex cursor-pointer items-center gap-3 rounded-lg border border-border px-3 py-2.5">
              <input
                type="checkbox"
                checked={webSearch}
                disabled={!!busy}
                className="h-4 w-4 rounded border-border accent-secondary"
                onChange={(e) => {
                  const enabled = e.target.checked;
                  setWebSearch(enabled);
                  void runAction("web", async () => {
                    await invoke<AiWebSearchConfig>("ai_web_search_set_config", {
                      payload: { enabled },
                    });
                    return enabled
                      ? "Recherche Internet activée pour Loggy."
                      : "Recherche Internet désactivée.";
                  });
                }}
              />
              <span className="text-sm text-foreground">
                Autoriser Loggy à rechercher sur Internet (questions pratiques, actualités…)
              </span>
            </label>

            <div className="mt-4 space-y-3 rounded-lg border border-border px-3 py-3">
              <p className="text-sm font-medium text-foreground">Analyse d&apos;image (tableau de bord)</p>
              <Text variant="muted" className="text-xs">
                Pour lire une capture (facture, formulaire…) et produire un JSON à coller manuellement.
                OpenRouter est gratuit (sans carte bancaire) — recommandé si vous n&apos;avez pas Gemini.
              </Text>
              <Select
                label="Fournisseur"
                value={visionProvider}
                onChange={(e) => {
                  const p = e.target.value;
                  setVisionProvider(p);
                  if (!visionModel.trim() || visionModel === visionConfig?.model) {
                    setVisionModel(p === "gemini" ? "gemini-2.0-flash" : "openrouter/free");
                  }
                }}
                options={[
                  { value: "openrouter", label: "OpenRouter (gratuit, recommandé)" },
                  { value: "gemini", label: "Google Gemini (gratuit via AI Studio)" },
                ]}
              />
              <Text variant="muted" className="text-xs">
                {visionProvider === "openrouter" ? (
                  <>
                    Clé sur{" "}
                    <a
                      href="https://openrouter.ai/keys"
                      className="text-secondary underline"
                      onClick={(e) => {
                        e.preventDefault();
                        void import("@tauri-apps/plugin-opener").then(({ openUrl }) =>
                          openUrl("https://openrouter.ai/keys"),
                        );
                      }}
                    >
                      openrouter.ai/keys
                    </a>
                  </>
                ) : (
                  <>
                    Clé sur{" "}
                    <a
                      href="https://aistudio.google.com/apikey"
                      className="text-secondary underline"
                      onClick={(e) => {
                        e.preventDefault();
                        void import("@tauri-apps/plugin-opener").then(({ openUrl }) =>
                          openUrl("https://aistudio.google.com/apikey"),
                        );
                      }}
                    >
                      Google AI Studio
                    </a>
                  </>
                )}
              </Text>
              {visionConfig?.configured && visionConfig.keyHint && (
                <p className="text-xs text-muted">
                  Clé enregistrée {visionConfig.keyHint} · {visionConfig.providerLabel} · modèle{" "}
                  {visionConfig.model}
                </p>
              )}
              <Input
                label="Clé API"
                type="password"
                value={visionApiKey}
                onChange={(e) => setVisionApiKey(e.target.value)}
                placeholder={
                  visionConfig?.configured
                    ? "Nouvelle clé (laisser vide pour conserver l'actuelle)"
                    : visionProvider === "openrouter"
                      ? "sk-or-…"
                      : "AIza…"
                }
                autoComplete="off"
              />
              <Input
                label="Modèle (optionnel)"
                value={visionModel}
                onChange={(e) => setVisionModel(e.target.value)}
                placeholder={
                  visionProvider === "gemini" ? "gemini-2.0-flash" : "qwen/qwen2.5-vl-72b-instruct:free"
                }
                hint={
                  visionProvider === "openrouter"
                    ? "Ex. qwen/qwen2.5-vl-72b-instruct:free, openrouter/free, google/gemini-2.0-flash-exp:free"
                    : "Ex. gemini-2.0-flash"
                }
              />
              <Button
                size="sm"
                variant="secondary"
                disabled={!!busy || (!visionApiKey.trim() && !visionConfig?.configured)}
                onClick={() =>
                  void runAction("vision", async () => {
                    const updated = await invoke<AiVisionConfigPublic>("ai_vision_set_config", {
                      payload: {
                        provider: visionProvider,
                        api_key: visionApiKey.trim() || undefined,
                        model: visionModel.trim() || undefined,
                      },
                    });
                    setVisionConfig(updated);
                    setVisionApiKey("");
                    return updated.configured
                      ? "Configuration vision enregistrée — analyse d'image disponible sur le tableau de bord."
                      : "Indiquez une clé API pour activer l'analyse d'image.";
                  })
                }
              >
                Enregistrer l&apos;analyse d&apos;image
              </Button>
            </div>

            <div className="mt-5 flex flex-wrap gap-2">
              <Button
                size="sm"
                variant="secondary"
                disabled={!!busy || !modelReady || status?.server_healthy}
                onClick={() =>
                  void runAction("start-server", () => invoke<string>("ai_start_server"))
                }
              >
                {busy === "start-server" ? "Démarrage…" : "Démarrer le serveur IA"}
              </Button>
              <Button
                size="sm"
                disabled={!!busy || !status?.model_present}
                onClick={() =>
                  void runAction("profile", () =>
                    invoke<string>("ai_profile_runtime", { payload: { force: false } }),
                  )
                }
              >
                <Cpu className="h-4 w-4" />
                {busy === "profile" ? "Profilage…" : "Profiler le matériel"}
              </Button>
              <Button
                size="sm"
                variant="secondary"
                disabled={!!busy || !status?.model_present}
                onClick={() =>
                  void runAction("profile-force", () =>
                    invoke<string>("ai_profile_runtime", { payload: { force: true } }),
                  )
                }
              >
                {busy === "profile-force" ? "Relance…" : "Forcer le profilage"}
              </Button>
              <Button
                size="sm"
                variant="ghost"
                disabled={!!busy}
                onClick={() =>
                  void runAction("reindex", async () => {
                    const n = await invoke<number>("ai_reindex");
                    return `Index IA mis à jour (${n} fichier(s) mémoire).`;
                  })
                }
              >
                {busy === "reindex" ? "Indexation…" : "Réindexer la mémoire IA"}
              </Button>
            </div>
          </CollapsiblePanel>
        </Guard>

        <Guard privilege={privilegeForParametresPanel("personnalisation_ia")}>
          <CollapsiblePanel
            title="Personnalisation IA"
            subtitle="Voix de Loggy — activation, réglages et voix personnelles"
            open={panelOpen("personnalisation_ia")}
            onOpenChange={(open) => handlePanelOpenChange("personnalisation_ia", open)}
            overflowVisibleWhenOpen
          >
            <PersonnalisationIaPanel />
          </CollapsiblePanel>
        </Guard>

        <Guard privilege={privilegeForParametresPanel("compte")}>
        <CollapsiblePanel
          title="Compte"
          subtitle="Session active sur ce poste"
          open={panelOpen("compte")}
          onOpenChange={(open) => handlePanelOpenChange("compte", open)}
        >
          <div className="flex items-start gap-3">
            <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-surface-elevated text-muted">
              <User className="h-5 w-5" />
            </div>
            <div className="min-w-0 flex-1 space-y-1">
              <p className="font-medium text-foreground">{user?.nom ?? "—"}</p>
              <p className="text-sm text-muted">{user?.email ?? "—"}</p>
              <p className="text-xs text-muted">Rôle : {user?.role ?? "—"}</p>
            </div>
          </div>
        </CollapsiblePanel>
        </Guard>

        <Guard privilege={privilegeForParametresPanel("theme")}>
        <CollapsiblePanel
          title="Thème de couleurs"
          subtitle="Apparence de l'interface — enregistré sur ce poste"
          open={panelOpen("theme")}
          onOpenChange={(open) => handlePanelOpenChange("theme", open)}
          headerAction={<Palette className="h-4 w-4 text-muted" aria-hidden />}
          overflowVisibleWhenOpen
        >
          <ThemePanel />
        </CollapsiblePanel>
        </Guard>

        <Guard privilege={privilegeForParametresPanel("impression")}>
          <CollapsiblePanel
            title="Création de modèles d'impression"
            subtitle="Éditeur HTML/CSS — fiche PDF par ligne de tableau"
            open={panelOpen("impression")}
            onOpenChange={(open) => handlePanelOpenChange("impression", open)}
          >
            <PrintModelsPanel />
          </CollapsiblePanel>
        </Guard>

        <Guard anyOf={["parametres:entites", "parametres:entites:creer"]}>
          <CollapsiblePanel
            title="Entités métier"
            subtitle="Source de vérité — tables SQLite et formulaires auto"
            open={panelOpen("entites")}
            onOpenChange={(open) => handlePanelOpenChange("entites", open)}
          >
            <EntityPanel
              onSaved={async () => {
                await syncSessionPrivileges();
                window.dispatchEvent(new Event(ENTITY_REGISTRY_SYNCED_EVENT));
              }}
            />
          </CollapsiblePanel>
        </Guard>

        <Guard privilege={privilegeForParametresPanel("archives")}>
          <CollapsiblePanel
            title="Archives du registre"
            subtitle="5 dernières versions avant synchronisation — copie JSON"
            open={panelOpen("archives")}
            onOpenChange={(open) => handlePanelOpenChange("archives", open)}
          >
            <RegistryArchivePanel />
          </CollapsiblePanel>
        </Guard>

        <Guard privilege={privilegeForParametresPanel("roles")}>
          <CollapsiblePanel
            title="Rôles et Privilèges"
            subtitle="Création des rôles et affectation des privilèges"
            open={panelOpen("roles")}
            onOpenChange={(open) => handlePanelOpenChange("roles", open)}
          >
            <RolesPanel />
          </CollapsiblePanel>
        </Guard>

        <Guard privilege={privilegeForParametresPanel("utilisateurs")}>
          <CollapsiblePanel
            title="Utilisateurs"
            subtitle="Comptes, e-mail et affectation de rôle"
            open={panelOpen("utilisateurs")}
            onOpenChange={(open) => handlePanelOpenChange("utilisateurs", open)}
          >
            <UsersPanel />
          </CollapsiblePanel>
        </Guard>

        <Guard privilege={privilegeForParametresPanel("imports_exports")}>
          <CollapsiblePanel
            title="Imports / Exports"
            subtitle="Journal des importations et exportations par utilisateur"
            open={panelOpen("imports_exports")}
            onOpenChange={(open) => handlePanelOpenChange("imports_exports", open)}
          >
            <ImportExportPanel />
          </CollapsiblePanel>
        </Guard>
          </>
        )}
      </div>

      <ParametresPasswordModal
        open={passwordModalOpen}
        onClose={closePasswordModal}
        onVerified={onPasswordVerified}
      />
    </div>
    </div>
  );
}
