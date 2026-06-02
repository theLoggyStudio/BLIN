import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { FileText, Plus, Save, Trash2 } from "lucide-react";
import { Button } from "@/items/Button";
import { Input } from "@/items/Input";
import { Select } from "@/items/Select";
import { Text } from "@/items/Text";
import { PrintVariablesTab } from "@/items/PrintVariablesTab";
import { TemplateVariableTextarea } from "@/items/TemplateVariableTextarea";
import { DEFAULT_FICHE_CSS } from "@/lib/print/defaultCss";
import { buildPrintPreviewSrcDoc } from "@/lib/print/previewDoc";
import { buildVariableCatalog } from "@/lib/print/templateVariables";
import type { EntityDef, EntityRegistryResponse } from "@/types/entity";
import type { PrintModelDetail, PrintModelRow, PrintTemplateDefaults } from "@/types/print";

type EditorTab = "html" | "css" | "variables" | "preview";

const emptyDraft = (): PrintModelDetail => ({
  id: "",
  name: "",
  description: "",
  screen_key: null,
  created_at: "",
  updated_at: "",
  html_content: "",
  css_content: DEFAULT_FICHE_CSS,
});

/** Studio modèles d'impression HTML/CSS (logique inspirée de Loma Admin + aperçu). */
export function PrintModelsPanel() {
  const [models, setModels] = useState<PrintModelRow[]>([]);
  const [entityDefs, setEntityDefs] = useState<EntityDef[]>([]);
  const [draft, setDraft] = useState<PrintModelDetail>(emptyDraft);
  const [tab, setTab] = useState<EditorTab>("html");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [list, reg] = await Promise.all([
        invoke<PrintModelRow[]>("print_models_list"),
        invoke<EntityRegistryResponse>("entity_registry_get"),
      ]);
      setModels(list);
      setEntityDefs(reg.entities);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const entityOptions = useMemo(
    () =>
      entityDefs.map((e) => ({
        value: e.nom,
        label: e.label?.trim() || e.nom,
      })),
    [entityDefs],
  );

  const variableCatalog = useMemo(() => buildVariableCatalog(entityDefs), [entityDefs]);

  const previewSrc = useMemo(
    () => buildPrintPreviewSrcDoc(draft.html_content, draft.css_content),
    [draft.html_content, draft.css_content],
  );

  const selectModel = async (id: string) => {
    if (!id) {
      setDraft(emptyDraft());
      return;
    }
    setError(null);
    try {
      const detail = await invoke<PrintModelDetail>("print_models_get", {
        payload: { id },
      });
      setDraft(detail);
      setTab("preview");
    } catch (e) {
      setError(String(e));
    }
  };

  const loadDefaultsForEntity = async (entityKey: string) => {
    if (!entityKey) return;
    setError(null);
    try {
      const defaults = await invoke<PrintTemplateDefaults>("print_models_defaults", {
        payload: { entity_key: entityKey },
      });
      setDraft((d) => ({
        ...d,
        screen_key: entityKey,
        html_content: defaults.html,
        css_content: defaults.css,
        name:
          d.name ||
          `Fiche ${entityOptions.find((x) => x.value === entityKey)?.label ?? entityKey}`,
      }));
      setMessage("Modèle auto-généré à partir des attributs de l'entité.");
    } catch (e) {
      setError(String(e));
    }
  };

  const save = async () => {
    if (!draft.name.trim()) {
      setError("Nom du modèle requis.");
      return;
    }
    setSaving(true);
    setMessage(null);
    setError(null);
    try {
      const saved = await invoke<PrintModelDetail>("print_models_upsert", {
        payload: {
          id: draft.id || null,
          name: draft.name.trim(),
          description: draft.description?.trim() ?? "",
          html_content: draft.html_content,
          css_content: draft.css_content,
          screen_key: draft.screen_key,
        },
      });
      setDraft(saved);
      await load();
      setMessage("Modèle enregistré.");
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const remove = async () => {
    if (!draft.id || !window.confirm("Supprimer ce modèle ?")) return;
    setSaving(true);
    setError(null);
    try {
      await invoke("print_models_delete", { payload: { id: draft.id } });
      setDraft(emptyDraft());
      await load();
      setMessage("Modèle supprimé.");
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return <p className="text-sm text-muted">Chargement des modèles…</p>;
  }

  return (
    <div className="space-y-4">
      <Text variant="muted" className="text-sm">
        Créez des modèles HTML/CSS par entité. Utilisez{" "}
        <code className="text-secondary">{`{{nomTable.nomVariable}}`}</code> (ex.{" "}
        <code className="text-secondary">{`{{ecole.nom}}`}</code>) — remplacées à l&apos;impression
        PDF sur chaque ligne.
      </Text>

      {(message || error) && (
        <p className={`text-sm ${error ? "text-primary" : "text-secondary"}`} role="status">
          {error ?? message}
        </p>
      )}

      <div className="flex flex-wrap gap-2 items-end">
        <div className="min-w-[200px] flex-1">
          <Select
            label="Modèle existant"
            value={draft.id}
            onChange={(e) => void selectModel(e.target.value)}
            options={[
              { value: "", label: "— Nouveau modèle —" },
              ...models.map((m) => ({
                value: m.id,
                label: `${m.name}${m.screen_key ? ` (${m.screen_key})` : ""}`,
              })),
            ]}
          />
        </div>
        <Button size="sm" variant="secondary" onClick={() => setDraft(emptyDraft())}>
          <Plus className="h-4 w-4" />
          Nouveau
        </Button>
      </div>

      <div className="space-y-4">
        <div className="grid gap-4 rounded-lg border border-border p-4 sm:grid-cols-2">
          <Input
            label="Nom du modèle"
            value={draft.name}
            onChange={(e) => setDraft({ ...draft, name: e.target.value })}
          />
          <Select
            label="Entité liée (screen_key)"
            value={draft.screen_key ?? ""}
            onChange={(e) => {
              const v = e.target.value;
              setDraft({ ...draft, screen_key: v || null });
            }}
            options={[{ value: "", label: "— Aucune —" }, ...entityOptions]}
          />
          <div className="sm:col-span-2">
            <Input
              label="Description"
              value={draft.description}
              onChange={(e) => setDraft({ ...draft, description: e.target.value })}
            />
          </div>
          <div className="sm:col-span-2 flex flex-wrap gap-2">
            <Button
              size="sm"
              variant="ghost"
              disabled={!draft.screen_key}
              onClick={() => void loadDefaultsForEntity(draft.screen_key ?? "")}
            >
              <FileText className="h-4 w-4" />
              Générer HTML depuis les attributs
            </Button>
          </div>
        </div>

        <div className="flex gap-2 border-b border-border">
          {(["html", "css", "variables", "preview"] as EditorTab[]).map((t) => (
            <button
              key={t}
              type="button"
              className={`px-3 py-2 text-sm capitalize ${
                tab === t
                  ? "border-b-2 border-secondary text-foreground"
                  : "text-muted hover:text-foreground"
              }`}
              onClick={() => setTab(t)}
            >
              {t === "html"
                ? "HTML"
                : t === "css"
                  ? "CSS"
                  : t === "variables"
                    ? "Variables"
                    : "Aperçu"}
            </button>
          ))}
        </div>

        {tab === "html" && (
          <TemplateVariableTextarea
            label="HTML du modèle"
            hint="Tapez {{ pour suggérer une table, puis un champ après le point"
            value={draft.html_content}
            onChange={(html_content) => setDraft({ ...draft, html_content })}
            catalog={variableCatalog}
            className="min-h-[220px] font-mono text-xs"
          />
        )}
        {tab === "css" && (
          <TemplateVariableTextarea
            label="CSS du modèle"
            hint="Variables possibles dans le CSS (ex. content: '{{date.aujourdhui}}')"
            value={draft.css_content}
            onChange={(css_content) => setDraft({ ...draft, css_content })}
            catalog={variableCatalog}
            className="min-h-[220px] font-mono text-xs"
          />
        )}
        {tab === "variables" && (
          <PrintVariablesTab entities={entityDefs} primaryTableKey={draft.screen_key} />
        )}
        {tab === "preview" && (
          <div className="rounded-lg border border-border bg-white overflow-hidden">
            <iframe
              title="Aperçu modèle"
              srcDoc={previewSrc}
              className="h-[480px] w-full border-0"
              sandbox="allow-same-origin"
            />
          </div>
        )}

        <div className="flex flex-wrap gap-2">
          <Button size="sm" disabled={saving} onClick={() => void save()}>
            <Save className="h-4 w-4" />
            {saving ? "Enregistrement…" : "Enregistrer le modèle"}
          </Button>
          {draft.id && (
            <Button size="sm" variant="ghost" disabled={saving} onClick={() => void remove()}>
              <Trash2 className="h-4 w-4 text-primary" />
              Supprimer
            </Button>
          )}
        </div>
      </div>

      {models.length === 0 && (
        <p className="text-sm text-muted">
          Aucun modèle. Enregistrez le registre des entités pour créer automatiquement une fiche
          par entité, ou cliquez sur « Nouveau » pour en créer un.
        </p>
      )}
    </div>
  );
}
