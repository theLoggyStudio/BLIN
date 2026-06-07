import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Plus, Trash2 } from "lucide-react";
import { Alert } from "@/items/Alert";
import { Button } from "@/items/Button";
import { Input } from "@/items/Input";
import { Modal } from "@/items/Modal";
import { Select } from "@/items/Select";
import { Table, type Column } from "@/items/Table";
import { Text } from "@/items/Text";
import { Textarea } from "@/items/Textarea";
import { EntityRegistryPromptButton } from "@/items/EntityRegistryPromptButton";
import { SyncProgressBar } from "@/items/SyncProgressBar";
import type { EntitySyncProgress } from "@/types/syncProgress";
import type {
  EntityAttribute,
  EntityDef,
  EntityRegistry,
  EntityRegistryResponse,
} from "@/types/entity";
import type { RoleRow } from "@/types/users";
import { useAlert } from "@/contexts/AlertContext";
import { isOrphanEntityKey } from "@/lib/orphanEntities";

const ATTR_TYPES: { value: string; label: string }[] = [
  { value: "boolean", label: "Booléen" },
  { value: "compteur", label: "Compteur auto (libellé + date + n°)" },
  { value: "date", label: "Date" },
  { value: "datetime", label: "Date/heure" },
  { value: "email", label: "E-mail" },
  { value: "entity", label: "Liaison entité" },
  { value: "enum", label: "Liste (enum — options ci-dessous)" },
  { value: "float", label: "Décimal (float)" },
  { value: "integer", label: "Entier (integer)" },
  { value: "matricule", label: "Matricule (manuel + date + n° quotidien)" },
  { value: "number", label: "Nombre (number)" },
  { value: "photo", label: "Photo (fichier image)" },
  { value: "stock", label: "Quantité (stock)" },
  { value: "string", label: "Texte (string)" },
  { value: "time", label: "Heure (time)" },
];

const EXAMPLE_JSON =
  'Collez le modèle « Gestion Scolaire » depuis src/constante/json/gestion-scolaire.registry.example.json';

function emptyEntity(): EntityDef {
  return {
    nom: "",
    label: "",
    description: "",
    ai_suggestions: true,
    requires_signature: false,
    signatory_role_ids: [],
    is_session: true,
    attributs: [],
  };
}

function emptyAttr(): EntityAttribute {
  return {
    nom: "",
    type: "string",
    label: "",
    required: false,
    relation_multiple: false,
    relation_exclusive_parent: true,
  };
}

function withRegistryMeta(
  base: EntityRegistry,
  patch: Partial<Pick<EntityRegistry, "ecosysteme" | "slogan" | "logo_url" | "logo" | "entities">>,
): EntityRegistry {
  return { ...base, ...patch };
}

function logoPreviewSrc(logo?: string): string | null {
  if (!logo?.trim()) return null;
  return logo.startsWith("data:") ? logo : `data:image/png;base64,${logo}`;
}

interface EntityPanelProps {
  onSaved?: () => void | Promise<void>;
}

/** Panneau Paramètres — registre des entités métier (JSON + éditeur structuré). */
export function EntityPanel({ onSaved }: EntityPanelProps) {
  const { showSuccess, showError, showWarning, showInfo } = useAlert();
  const [registry, setRegistry] = useState<EntityRegistry>({ entities: [] });
  const [jsonText, setJsonText] = useState(EXAMPLE_JSON);
  const [viewJson, setViewJson] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [syncProgress, setSyncProgress] = useState<EntitySyncProgress | null>(null);
  const [entityModal, setEntityModal] = useState<EntityDef | null>(null);
  const [entityIndex, setEntityIndex] = useState<number | null>(null);
  const [entityValidationAttempted, setEntityValidationAttempted] = useState(false);
  const [logoFileName, setLogoFileName] = useState<string | null>(null);
  const [roles, setRoles] = useState<RoleRow[]>([]);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const res = await invoke<EntityRegistryResponse>("entity_registry_get");
      setRegistry({
        ecosysteme: res.ecosysteme,
        slogan: res.slogan,
        logo_url: res.logo_url,
        logo: res.logo,
        entities: res.entities,
      });
      setJsonText(res.json || EXAMPLE_JSON);
    } catch (e) {
      showError(String(e));
    } finally {
      setLoading(false);
    }
  }, [showError]);

  useEffect(() => {
    void load();
  }, [load]);

  const loadRoles = useCallback(async () => {
    try {
      const rows = await invoke<RoleRow[]>("users_list_roles");
      setRoles(rows);
    } catch {
      setRoles([]);
    }
  }, []);

  useEffect(() => {
    if (entityModal !== null) {
      void loadRoles();
    }
  }, [entityModal, loadRoles]);

  useEffect(() => {
    const unlisten = listen<EntitySyncProgress>("entity-sync-progress", (event) => {
      setSyncProgress(event.payload);
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  const onLogoFile = (file: File | undefined) => {
    if (!file) return;
    if (file.size > 5 * 1024 * 1024) {
      showWarning("Image trop volumineuse (maximum 5 Mo).");
      return;
    }
    if (!file.type.startsWith("image/")) {
      showWarning("Choisissez un fichier image (PNG, JPEG, WebP, GIF).");
      return;
    }
    const reader = new FileReader();
    reader.onload = () => {
      const result = reader.result;
      if (typeof result !== "string") return;
      setRegistry((r) => withRegistryMeta(r, { logo: result, logo_url: undefined }));
      setLogoFileName(file.name);
      showInfo(
        "Logo chargé — cliquez sur « Enregistrer écosystème / logo » pour le conserver.",
      );
    };
    reader.onerror = () => showError("Impossible de lire le fichier image.");
    reader.readAsDataURL(file);
  };

  const persist = async (next: EntityRegistry) => {
    setSaving(true);
    setSyncProgress({
      current: 0,
      total: 1,
      label: "Préparation de la synchronisation…",
      step: "start",
      done: false,
    });
    try {
      const synced = await invoke<string[]>("entity_registry_save", {
        payload: { registry: next },
      });
      setRegistry(next);
      setJsonText(JSON.stringify(next, null, 2));
      const autoNote =
        synced.length > 0
          ? `Synchronisé : ${synced.join(", ")}. Les entités liées manquantes sont créées automatiquement.`
          : "Registre enregistré.";
      showSuccess(autoNote);
      await onSaved?.();
    } catch (e) {
      showError(String(e));
      setSyncProgress(null);
    } finally {
      setSaving(false);
      setSyncProgress((p) => (p?.done ? p : null));
    }
  };

  const saveFromJson = () => {
    try {
      const parsed = JSON.parse(jsonText) as EntityRegistry;
      if (!Array.isArray(parsed.entities)) {
        throw new Error('Le JSON doit contenir un tableau "entities".');
      }
      void persist(parsed);
    } catch (e) {
      showError(e instanceof Error ? e.message : String(e));
    }
  };

  const openCreateEntity = () => {
    setEntityIndex(null);
    setEntityValidationAttempted(false);
    setEntityModal(emptyEntity());
  };

  const openEditEntity = (index: number) => {
    setEntityIndex(index);
    setEntityValidationAttempted(false);
    setEntityModal({ ...registry.entities[index], attributs: [...registry.entities[index].attributs] });
  };

  const entityRefOptions = (currentEntityNom: string) => {
    const self = currentEntityNom.trim().toLowerCase().replace(/\s+/g, "_");
    return registry.entities
      .filter((e) => e.nom !== self && !isOrphanEntityKey(e.nom))
      .map((e) => ({ value: e.nom, label: e.label ?? e.nom }));
  };

  const saveEntityModal = () => {
    setEntityValidationAttempted(true);
    if (!entityModal?.nom.trim()) {
      showWarning("Le nom de l'entité est obligatoire.");
      return;
    }
    if (
      entityModal.requires_signature &&
      (!entityModal.signatory_role_ids || entityModal.signatory_role_ids.length === 0)
    ) {
      showWarning("Sélectionnez au moins un rôle signataire pour une entité à signer.");
      return;
    }
    if (
      entityModal.requires_signature &&
      !entityModal.attributs.some((a) => Boolean(a.required))
    ) {
      showWarning(
        "Une entité à signer doit comporter au moins un attribut « À remplir obligatoirement ».",
      );
      return;
    }
    for (const attr of entityModal.attributs) {
      if (attr.type === "entity" && !attr.ref?.trim()) {
        showWarning(
          `L'attribut « ${attr.nom || "?"} » de type liaison doit cibler une entité (ref).`,
        );
        return;
      }
    }
    const nom = entityModal.nom.trim().toLowerCase().replace(/\s+/g, "_");
    const draftEnt: EntityDef = {
      nom,
      label: entityModal.label?.trim() || entityModal.nom.trim(),
      description: entityModal.description?.trim() || undefined,
      ai_suggestions: Boolean(entityModal.ai_suggestions),
      requires_signature: Boolean(entityModal.requires_signature),
      signatory_role_ids: entityModal.requires_signature
        ? [...(entityModal.signatory_role_ids ?? [])]
        : [],
      is_session: true,
      attributs: entityModal.attributs
        .filter((a) => a.nom.trim())
        .map((a) => ({
          ...a,
          nom: a.nom.trim().toLowerCase().replace(/\s+/g, "_"),
          required: Boolean(a.required),
          relation_multiple: a.type === "entity" ? Boolean(a.relation_multiple) : undefined,
          ref:
            a.type === "entity"
              ? (a.ref?.trim().toLowerCase().replace(/\s+/g, "_") ?? undefined)
              : undefined,
          relation_exclusive_parent: a.type === "entity" ? true : undefined,
        })),
    };
    const ent: EntityDef = { ...draftEnt };
    const next = withRegistryMeta(registry, { entities: [...registry.entities] });
    if (entityIndex === null) {
      next.entities.push(ent);
    } else {
      next.entities[entityIndex] = ent;
    }
    setEntityModal(null);
    setEntityValidationAttempted(false);
    void persist(next);
  };

  const deleteEntity = (index: number) => {
    const next = withRegistryMeta(registry, {
      entities: registry.entities.filter((_, i) => i !== index),
    });
    void persist(next);
  };

  const columns: Column<EntityDef & { _index: number }>[] = [
    { key: "nom", header: "Nom (clé)", render: (r) => r.nom },
    { key: "label", header: "Libellé", render: (r) => r.label ?? r.nom },
    {
      key: "attributs",
      header: "Attributs",
      render: (r) => String(r.attributs.length),
    },
    {
      key: "ai_suggestions",
      header: "Suggestions IA",
      render: (r) => (r.ai_suggestions ? "Oui" : "—"),
    },
    {
      key: "actions",
      header: "",
      render: (r) => (
        <div className="flex gap-2 justify-end">
          <Button size="sm" variant="ghost" onClick={() => openEditEntity(r._index)}>
            Modifier
          </Button>
          <Button size="sm" variant="ghost" onClick={() => deleteEntity(r._index)}>
            <Trash2 className="h-4 w-4 text-primary" />
          </Button>
        </div>
      ),
    },
  ];

  const tableData = registry.entities
    .map((e, i) => ({ ...e, _index: i }))
    .filter((e) => e.nom !== "stock" && !isOrphanEntityKey(e.nom));

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <Text variant="label">Entités du projet</Text>
          <Text variant="muted" className="mt-1">
            {registry.entities.length} entité(s) — types étendus, liaisons « entité » avec création auto des
            cibles manquantes.
          </Text>
        </div>
        <div className="flex flex-wrap items-start justify-end gap-2">
          <EntityRegistryPromptButton
            ecosysteme={registry.ecosysteme}
            slogan={registry.slogan}
          />
          <Button size="sm" variant="ghost" onClick={() => setViewJson((v) => !v)}>
            {viewJson ? "Vue tableau" : "Vue JSON"}
          </Button>
          <Button size="sm" onClick={openCreateEntity}>
            <Plus className="h-4 w-4" />
            Ajouter une entité
          </Button>
        </div>
      </div>

      <div className="grid gap-3 rounded-xl border border-border p-4 sm:grid-cols-2">
        <Input
          label="Écosystème"
          value={registry.ecosysteme ?? ""}
          onChange={(e) =>
            setRegistry((r) => withRegistryMeta(r, { ecosysteme: e.target.value || undefined }))
          }
          hint="Titre affiché partout (sidebar, fenêtre, favicon, accueil)"
        />
        <Input
          label="Slogan"
          value={registry.slogan ?? ""}
          onChange={(e) =>
            setRegistry((r) => withRegistryMeta(r, { slogan: e.target.value || undefined }))
          }
          hint="Texte sous le nom — ex. Gestion scolaire simplifiée"
        />
        <div className="sm:col-span-2 space-y-2">
          <Text variant="label">Logo (fichier image)</Text>
          <input
            type="file"
            accept="image/png,image/jpeg,image/webp,image/gif"
            className="block w-full text-sm text-foreground file:mr-3 file:rounded-lg file:border-0 file:bg-secondary file:px-3 file:py-2 file:text-sm file:font-medium file:text-white hover:file:opacity-90"
            onChange={(e) => onLogoFile(e.target.files?.[0])}
          />
          <Text variant="muted">
            {logoFileName
              ? `Fichier sélectionné : ${logoFileName} — conversion base64 à l'enregistrement`
              : "PNG, JPEG, WebP ou GIF — max. 5 Mo"}
          </Text>
        </div>
        {logoPreviewSrc(registry.logo) && (
          <div className="sm:col-span-2 flex items-center gap-3">
            <img
              src={logoPreviewSrc(registry.logo)!}
              alt="Logo écosystème"
              className="h-12 w-12 rounded-lg border border-border object-contain bg-card"
            />
            <Text variant="muted">Aperçu du logo enregistré</Text>
          </div>
        )}
        <div className="sm:col-span-2">
          <Button
            size="sm"
            variant="secondary"
            disabled={saving}
            onClick={() => void persist(registry)}
          >
            Enregistrer écosystème / logo
          </Button>
        </div>
      </div>

      <SyncProgressBar progress={syncProgress} active={saving} />

      {loading ? (
        <p className="text-sm text-muted">Chargement…</p>
      ) : viewJson ? (
        <div className="space-y-3">
          <Textarea
            label="Registre JSON"
            hint='Entité : ai_suggestions (bool). Attribut : required (bool), type, ref, default. Modèle : gestion-scolaire.registry.example.json'
            value={jsonText}
            onChange={(e) => setJsonText(e.target.value)}
            className="min-h-[280px]"
          />
          <Button size="sm" disabled={saving} onClick={saveFromJson}>
            {saving ? "Enregistrement…" : "Enregistrer et synchroniser"}
          </Button>
        </div>
      ) : (
        <Table
          columns={columns}
          data={tableData}
          keyExtractor={(r) => r.nom}
          emptyMessage="Aucune entité — ajoutez-en une."
        />
      )}

      <Modal
        open={entityModal !== null}
        onClose={() => {
          setEntityValidationAttempted(false);
          setEntityModal(null);
        }}
        title={entityIndex === null ? "Nouvelle entité" : "Modifier l'entité"}
        size="lg"
      >
        {entityModal && (
          <div className="max-h-[70vh] space-y-4 overflow-y-auto pr-1">
            <Input
              label="Nom (clé technique)"
              value={entityModal.nom}
              onChange={(e) => setEntityModal({ ...entityModal, nom: e.target.value })}
              hint="ex. users, clients, tache"
            />
            <Input
              label="Libellé affiché"
              value={entityModal.label ?? ""}
              onChange={(e) => setEntityModal({ ...entityModal, label: e.target.value })}
            />
            <Textarea
              label="Description"
              value={entityModal.description ?? ""}
              onChange={(e) => setEntityModal({ ...entityModal, description: e.target.value })}
              className="min-h-[72px] sm:col-span-2"
            />
            <label className="flex cursor-pointer items-center gap-3 sm:col-span-2">
              <input
                type="checkbox"
                checked={Boolean(entityModal.requires_signature)}
                onChange={(e) =>
                  setEntityModal({
                    ...entityModal,
                    requires_signature: e.target.checked,
                    signatory_role_ids: e.target.checked
                      ? entityModal.signatory_role_ids ?? []
                      : [],
                  })
                }
                className="h-4 w-4 rounded border-border accent-secondary"
              />
              <span className="text-sm text-foreground">Entité à signer</span>
            </label>
            <label className="flex cursor-pointer items-center gap-3 sm:col-span-2">
              <input
                type="checkbox"
                checked={Boolean(entityModal.ai_suggestions)}
                onChange={(e) =>
                  setEntityModal({
                    ...entityModal,
                    ai_suggestions: e.target.checked,
                  })
                }
                className="h-4 w-4 rounded border-border accent-secondary"
              />
              <span className="text-sm text-foreground">Ajouter à la Suggestions IA</span>
            </label>
            {entityModal.requires_signature && (
              <div
                className={`space-y-2 rounded-lg border p-3 sm:col-span-2 ${
                  entityValidationAttempted &&
                  (!entityModal.signatory_role_ids || entityModal.signatory_role_ids.length === 0)
                    ? "border-primary bg-primary/10"
                    : "border-border"
                }`}
              >
                <Text variant="label">Rôles signataires</Text>
                <Text variant="muted" className="text-xs">
                  Trigger automatique : à chaque création d&apos;un objet, une tâche « signature »
                  privée est générée pour chaque rôle sélectionné (entité Tâche requise). Un seul
                  signataire agréé suffit pour signer l&apos;objet.
                </Text>
                {roles.length === 0 ? (
                  <p className="text-sm text-muted">Aucun rôle — créez des rôles dans Paramètres.</p>
                ) : (
                  <div className="flex flex-col gap-2">
                    {roles.map((role) => {
                      const checked = (entityModal.signatory_role_ids ?? []).includes(role.id);
                      return (
                        <label
                          key={role.id}
                          className="flex cursor-pointer items-center gap-3"
                        >
                          <input
                            type="checkbox"
                            checked={checked}
                            onChange={(e) => {
                              const current = entityModal.signatory_role_ids ?? [];
                              const next = e.target.checked
                                ? [...current, role.id]
                                : current.filter((id) => id !== role.id);
                              setEntityModal({
                                ...entityModal,
                                signatory_role_ids: next,
                              });
                            }}
                            className="h-4 w-4 rounded border-border accent-secondary"
                          />
                          <span className="text-sm text-foreground">{role.nom}</span>
                          <span className="text-xs text-muted">({role.id})</span>
                        </label>
                      );
                    })}
                  </div>
                )}
                {entityValidationAttempted &&
                  (!entityModal.signatory_role_ids ||
                    entityModal.signatory_role_ids.length === 0) && (
                    <Alert
                      variant="danger"
                      size="field"
                      message="Sélection obligatoire : cochez au moins un rôle signataire."
                    />
                  )}
              </div>
            )}
            <Text variant="label" className="sm:col-span-2">
              Attributs
            </Text>
            {entityModal.attributs.map((attr, idx) => (
              <div key={idx} className="grid gap-2 rounded-lg border border-border p-3 sm:grid-cols-2">
                <Input
                  label="Nom"
                  value={attr.nom}
                  onChange={(e) => {
                    const attributs = [...entityModal.attributs];
                    attributs[idx] = { ...attr, nom: e.target.value };
                    setEntityModal({ ...entityModal, attributs });
                  }}
                />
                <Select
                  label="Type"
                  value={String(attr.type).startsWith("enum[") ? "enum" : String(attr.type)}
                  onChange={(e) => {
                    const attributs = [...entityModal.attributs];
                    const type = e.target.value;
                    attributs[idx] = {
                      ...attr,
                      type,
                      ref: type === "entity" ? attr.ref ?? "" : undefined,
                      relation_multiple:
                        type === "entity" ? Boolean(attr.relation_multiple) : undefined,
                      relation_exclusive_parent:
                        type === "entity" ? true : undefined,
                      enum_options: type === "enum" ? attr.enum_options ?? [] : undefined,
                    };
                    setEntityModal({ ...entityModal, attributs });
                  }}
                  options={ATTR_TYPES.map((t) => ({ value: t.value, label: t.label }))}
                />
                {(attr.type === "enum" || String(attr.type).startsWith("enum")) && (
                  <Input
                    label="Options enum (virgules)"
                    value={(attr.enum_options ?? []).join(", ")}
                    onChange={(e) => {
                      const attributs = [...entityModal.attributs];
                      attributs[idx] = {
                        ...attr,
                        type: "enum",
                        enum_options: e.target.value
                          .split(",")
                          .map((s) => s.trim())
                          .filter(Boolean),
                      };
                      setEntityModal({ ...entityModal, attributs });
                    }}
                    hint="ex. lundi,mardi,mercredi"
                  />
                )}
                {attr.type === "entity" && (
                  <div className="space-y-2 sm:col-span-2">
                    <Select
                      label="Entité liée (ref)"
                      value={attr.ref ?? ""}
                      onChange={(e) => {
                        const attributs = [...entityModal.attributs];
                        attributs[idx] = { ...attr, ref: e.target.value };
                        setEntityModal({ ...entityModal, attributs });
                      }}
                      options={[
                        { value: "", label: "— Choisir —" },
                        ...entityRefOptions(entityModal.nom),
                      ]}
                    />
                    <label className="flex cursor-pointer items-center gap-3">
                      <input
                        type="checkbox"
                        checked={Boolean(attr.relation_multiple)}
                        onChange={(e) => {
                          const attributs = [...entityModal.attributs];
                          attributs[idx] = {
                            ...attr,
                            relation_multiple: e.target.checked,
                            relation_exclusive_parent: true,
                          };
                          setEntityModal({ ...entityModal, attributs });
                        }}
                        className="h-4 w-4 rounded border-border accent-secondary"
                      />
                      <span className="text-sm text-foreground">
                        Liste de {attr.ref?.trim() || "l'entité liée"}
                      </span>
                    </label>
                    <p className="text-xs text-muted sm:col-span-2">
                      Coché : plusieurs copies embarquées (one-to-many). Décoché : une seule copie
                      des champs fille dans cette fiche (one-to-one). Les valeurs sont dupliquées —
                      pas de liaison par identifiant.
                    </p>
                  </div>
                )}
                <Input
                  label="Libellé"
                  value={attr.label ?? ""}
                  onChange={(e) => {
                    const attributs = [...entityModal.attributs];
                    attributs[idx] = { ...attr, label: e.target.value };
                    setEntityModal({ ...entityModal, attributs });
                  }}
                />
                <Input
                  label="Valeur par défaut"
                  value={attr.default != null ? String(attr.default) : ""}
                  onChange={(e) => {
                    const attributs = [...entityModal.attributs];
                    const raw = e.target.value;
                    let def: EntityAttribute["default"] = raw;
                    if (raw === "") def = undefined;
                    else if (attr.type === "number" || attr.type === "integer" || attr.type === "float") {
                      def = Number(raw);
                    } else if (attr.type === "boolean") {
                      def = raw === "true" || raw === "1";
                    }
                    attributs[idx] = { ...attr, default: def };
                    setEntityModal({ ...entityModal, attributs });
                  }}
                />
                {attr.type !== "compteur" && (
                <label className="flex cursor-pointer items-center gap-3 sm:col-span-2">
                  <input
                    type="checkbox"
                    checked={Boolean(attr.required)}
                    onChange={(e) => {
                      const attributs = [...entityModal.attributs];
                      attributs[idx] = { ...attr, required: e.target.checked };
                      setEntityModal({ ...entityModal, attributs });
                    }}
                    className="h-4 w-4 rounded border-border accent-secondary"
                  />
                  <span className="text-sm text-foreground">À remplir obligatoirement</span>
                </label>
                )}
                {(attr.type === "compteur" || attr.type === "matricule") && (
                  <p className="text-xs text-muted sm:col-span-2">
                    {attr.type === "matricule"
                      ? "Matricule : saisie manuelle + date du jour (jjmmaaaa) + numéro quotidien auto."
                      : "Compteur : libellé, date du jour (jjmmaaaa) et numéro générés automatiquement à la création (champs visibles en lecture seule)."}
                  </p>
                )}
                <div className="flex items-end">
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => {
                      const attributs = entityModal.attributs.filter((_, i) => i !== idx);
                      setEntityModal({ ...entityModal, attributs });
                    }}
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>
              </div>
            ))}
            <Button
              size="sm"
              variant="secondary"
              onClick={() =>
                setEntityModal({
                  ...entityModal,
                  attributs: [...entityModal.attributs, emptyAttr()],
                })
              }
            >
              <Plus className="h-4 w-4" />
              Attribut
            </Button>
            <Button size="sm" className="w-full" disabled={saving} onClick={saveEntityModal}>
              {saving ? "Synchronisation…" : "Enregistrer l'entité"}
            </Button>
          </div>
        )}
      </Modal>
    </div>
  );
}
