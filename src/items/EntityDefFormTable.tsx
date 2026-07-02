import type { ReactNode } from "react";
import { Plus, Trash2 } from "lucide-react";
import { Button } from "@/items/Button";
import { Input } from "@/components/ui/Input";
import { Select } from "@/components/ui/Select";
import { Textarea } from "@/items/Textarea";
import { Text } from "@/items/Text";
import {
  ENTITY_ATTR_TYPES,
  emptyEntityAttribute,
} from "@/lib/entityDefForm";
import { isOrphanEntityKey } from "@/lib/orphanEntities";
import type { EntityDef } from "@/types/entity";
import type { RoleRow } from "@/types/users";

interface EntityDefFormTableProps {
  value: EntityDef;
  onChange: (next: EntityDef) => void;
  existingEntities: EntityDef[];
  roles: RoleRow[];
  validationAttempted?: boolean;
}

function FormRow({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <div className="grid gap-2 border-b border-border/60 py-3 md:grid-cols-[34%_1fr] md:items-start md:gap-4">
      <div className="text-sm font-medium text-foreground">
        {label}
        {hint ? <span className="mt-1 block text-xs font-normal text-muted">{hint}</span> : null}
      </div>
      <div className="min-w-0">{children}</div>
    </div>
  );
}

/** Formulaire entité — label à gauche, contrôle à droite. */
export function EntityDefFormTable({
  value,
  onChange,
  existingEntities,
  roles,
  validationAttempted,
}: EntityDefFormTableProps) {
  const entityRefOptions = existingEntities
    .filter((e) => e.nom !== value.nom.trim().toLowerCase() && !isOrphanEntityKey(e.nom))
    .map((e) => ({ value: e.nom, label: e.label ?? e.nom }));

  return (
    <div className="space-y-4">
      <div className="w-full">
          <FormRow label="Nom (clé technique)" hint="ex. fournisseur, clients">
            <Input
              value={value.nom}
              onChange={(e) => onChange({ ...value, nom: e.target.value })}
              placeholder="fournisseur"
            />
          </FormRow>
          <FormRow label="Libellé affiché">
            <Input
              value={value.label ?? ""}
              onChange={(e) => onChange({ ...value, label: e.target.value })}
            />
          </FormRow>
          <FormRow label="Description">
            <Textarea
              value={value.description ?? ""}
              onChange={(e) => onChange({ ...value, description: e.target.value })}
              className="min-h-[72px]"
            />
          </FormRow>
          <FormRow label="Entité à signer">
            <label className="flex min-h-11 cursor-pointer items-center gap-3 py-1">
              <input
                type="checkbox"
                checked={Boolean(value.requires_signature)}
                onChange={(e) =>
                  onChange({
                    ...value,
                    requires_signature: e.target.checked,
                    signatory_role_ids: e.target.checked
                      ? (value.signatory_role_ids ?? [])
                      : [],
                  })
                }
                className="h-5 w-5 shrink-0 rounded border-border accent-secondary md:h-4 md:w-4"
              />
              <span className="text-sm text-foreground">Créer des tâches de signature automatiques</span>
            </label>
          </FormRow>
          <FormRow label="Suggestions IA (barre Loggy)">
            <label className="flex min-h-11 cursor-pointer items-center gap-3 py-1">
              <input
                type="checkbox"
                checked={Boolean(value.ai_suggestions)}
                onChange={(e) => onChange({ ...value, ai_suggestions: e.target.checked })}
                className="h-5 w-5 shrink-0 rounded border-border accent-secondary md:h-4 md:w-4"
              />
              <span className="text-sm text-foreground">Proposer « Gérer {value.label || "…"} »</span>
            </label>
          </FormRow>
      </div>

      {value.requires_signature && (
        <div className="rounded-lg border border-border p-3">
          <Text variant="label" className="mb-2 block">
            Rôles signataires
          </Text>
          {roles.length === 0 ? (
            <p className="text-sm text-muted">Aucun rôle disponible.</p>
          ) : (
            <div className="flex flex-col gap-2">
              {roles.map((role) => {
                const checked = (value.signatory_role_ids ?? []).includes(role.id);
                return (
                  <label key={role.id} className="flex cursor-pointer items-center gap-3">
                    <input
                      type="checkbox"
                      checked={checked}
                      onChange={(e) => {
                        const current = value.signatory_role_ids ?? [];
                        const next = e.target.checked
                          ? [...current, role.id]
                          : current.filter((id) => id !== role.id);
                        onChange({ ...value, signatory_role_ids: next });
                      }}
                      className="h-4 w-4 rounded border-border accent-secondary"
                    />
                    <span className="text-sm">{role.nom}</span>
                  </label>
                );
              })}
            </div>
          )}
          {validationAttempted &&
            (!value.signatory_role_ids || value.signatory_role_ids.length === 0) && (
              <p className="mt-2 text-xs text-primary">Sélectionnez au moins un rôle signataire.</p>
            )}
        </div>
      )}

      <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
        <Text variant="label">Attributs</Text>
        <Button
          size="sm"
          variant="secondary"
          className="w-full sm:w-auto"
          onClick={() =>
            onChange({
              ...value,
              attributs: [...value.attributs, emptyEntityAttribute()],
            })
          }
        >
          <Plus className="mr-1 h-3.5 w-3.5" />
          Ajouter
        </Button>
      </div>

      {value.attributs.length === 0 ? (
        <p className="text-sm text-muted">Ajoutez au moins un attribut métier (en plus de id si besoin).</p>
      ) : (
        <div className="space-y-2">
          <div className="hidden gap-2 px-1 text-xs font-medium text-muted md:grid md:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_minmax(0,1fr)_auto_auto] md:items-center">
            <span>Nom</span>
            <span>Libellé</span>
            <span>Type</span>
            <span className="text-center">Obligatoire</span>
            <span className="w-9" />
          </div>
          {value.attributs.map((attr, idx) => (
            <div
              key={idx}
              className="space-y-2 rounded-lg border border-border p-3 md:space-y-0 md:border-0 md:p-0"
            >
              <div className="grid gap-2 md:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_minmax(0,1fr)_auto_auto] md:items-center md:gap-2 md:rounded-lg md:border md:border-border md:p-2">
                <Input
                  value={attr.nom}
                  onChange={(e) => {
                    const attributs = [...value.attributs];
                    attributs[idx] = { ...attr, nom: e.target.value };
                    onChange({ ...value, attributs });
                  }}
                  placeholder="nom_champ"
                />
                <Input
                  value={attr.label ?? ""}
                  onChange={(e) => {
                    const attributs = [...value.attributs];
                    attributs[idx] = { ...attr, label: e.target.value };
                    onChange({ ...value, attributs });
                  }}
                  placeholder="Libellé"
                />
                <Select
                  value={String(attr.type).startsWith("enum[") ? "enum" : String(attr.type)}
                  onChange={(e) => {
                    const type = e.target.value;
                    const attributs = [...value.attributs];
                    attributs[idx] = {
                      ...attr,
                      type,
                      ref: type === "entity" ? (attr.ref ?? "") : undefined,
                      enum_options: type === "enum" ? (attr.enum_options ?? []) : undefined,
                    };
                    onChange({ ...value, attributs });
                  }}
                  options={ENTITY_ATTR_TYPES.map((t) => ({ value: t.value, label: t.label }))}
                />
                {attr.type !== "compteur" && attr.type !== "matricule" ? (
                  <label className="flex cursor-pointer items-center justify-center gap-2 px-1">
                    <input
                      type="checkbox"
                      checked={Boolean(attr.required)}
                      onChange={(e) => {
                        const attributs = [...value.attributs];
                        attributs[idx] = { ...attr, required: e.target.checked };
                        onChange({ ...value, attributs });
                      }}
                      className="h-4 w-4 rounded border-border accent-secondary"
                    />
                    <span className="text-sm md:sr-only">Obligatoire</span>
                  </label>
                ) : (
                  <span className="text-center text-xs text-muted md:px-2">—</span>
                )}
                <Button
                  size="sm"
                  variant="ghost"
                  className="w-9 shrink-0 px-0 md:justify-self-end"
                  onClick={() => {
                    const attributs = value.attributs.filter((_, i) => i !== idx);
                    onChange({ ...value, attributs });
                  }}
                  aria-label="Retirer l'attribut"
                >
                  <Trash2 className="h-4 w-4 text-primary" />
                </Button>
              </div>
              {(attr.type === "enum" || String(attr.type).startsWith("enum")) && (
                <FormRow label="Options (enum)" hint="Séparées par des virgules">
                  <Input
                    value={(attr.enum_options ?? []).join(", ")}
                    onChange={(e) => {
                      const attributs = [...value.attributs];
                      attributs[idx] = {
                        ...attr,
                        type: "enum",
                        enum_options: e.target.value
                          .split(",")
                          .map((s) => s.trim())
                          .filter(Boolean),
                      };
                      onChange({ ...value, attributs });
                    }}
                  />
                </FormRow>
              )}
              {attr.type === "entity" && (
                <div className="space-y-2 md:pl-2">
                  <FormRow label="Entité liée (ref)">
                    <Select
                      value={attr.ref ?? ""}
                      onChange={(e) => {
                        const attributs = [...value.attributs];
                        attributs[idx] = { ...attr, ref: e.target.value };
                        onChange({ ...value, attributs });
                      }}
                      options={[{ value: "", label: "— Choisir —" }, ...entityRefOptions]}
                    />
                  </FormRow>
                  <FormRow label="Liste multiple">
                    <label className="flex cursor-pointer items-center gap-3">
                      <input
                        type="checkbox"
                        checked={Boolean(attr.relation_multiple)}
                        onChange={(e) => {
                          const attributs = [...value.attributs];
                          attributs[idx] = {
                            ...attr,
                            relation_multiple: e.target.checked,
                          };
                          onChange({ ...value, attributs });
                        }}
                        className="h-4 w-4 rounded border-border accent-secondary"
                      />
                      <span className="text-sm">Plusieurs enregistrements liés</span>
                    </label>
                  </FormRow>
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
