import { useEffect, useState } from "react";
import { Plus, Trash2 } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { Alert } from "@/items/Alert";
import { Button } from "@/items/Button";
import { Input } from "@/items/Input";
import { Modal } from "@/items/Modal";
import { Text } from "@/items/Text";
import { cn } from "@/lib/utils";
import type { VisionAnalyzeEntityOptions, VisionAttributeHint } from "@/types/ai";
import type { RoleRow } from "@/types/users";

export interface VisionAnalyzeOptionsModalProps {
  open: boolean;
  onClose: () => void;
  onConfirm: (options: VisionAnalyzeEntityOptions) => void;
}

const defaultOptions = (): VisionAnalyzeEntityOptions => ({
  requires_signature: false,
  ai_suggestions: true,
  signatory_role_ids: [],
  attribute_hints: [],
});

/** Options entité (signature, suggestions, signataires) avant analyse vision. */
export function VisionAnalyzeOptionsModal({
  open,
  onClose,
  onConfirm,
}: VisionAnalyzeOptionsModalProps) {
  const [roles, setRoles] = useState<RoleRow[]>([]);
  const [options, setOptions] = useState<VisionAnalyzeEntityOptions>(defaultOptions);
  const [loadingRoles, setLoadingRoles] = useState(false);

  useEffect(() => {
    if (!open) return;
    setOptions(defaultOptions());
    setLoadingRoles(true);
    void invoke<RoleRow[]>("users_list_roles")
      .then(setRoles)
      .catch(() => setRoles([]))
      .finally(() => setLoadingRoles(false));
  }, [open]);

  const toggleRole = (roleId: string) => {
    setOptions((prev) => {
      const set = new Set(prev.signatory_role_ids);
      if (set.has(roleId)) set.delete(roleId);
      else set.add(roleId);
      return { ...prev, signatory_role_ids: [...set] };
    });
  };

  const updateAttributeHint = (idx: number, patch: Partial<VisionAttributeHint>) => {
    setOptions((prev) => {
      const hints = [...prev.attribute_hints];
      hints[idx] = { ...hints[idx], ...patch };
      return { ...prev, attribute_hints: hints };
    });
  };

  const addAttributeHint = () => {
    setOptions((prev) => ({
      ...prev,
      attribute_hints: [...prev.attribute_hints, { nom: "", required: true }],
    }));
  };

  const removeAttributeHint = (idx: number) => {
    setOptions((prev) => ({
      ...prev,
      attribute_hints: prev.attribute_hints.filter((_, i) => i !== idx),
    }));
  };

  const needsSignatories =
    options.requires_signature &&
    options.signatory_role_ids.length === 0 &&
    roles.length > 0;

  return (
    <Modal
      open={open}
      onClose={onClose}
      title="Entité depuis l'image"
      size="lg"
      footer={
        <div className="flex w-full justify-end gap-2">
          <Button type="button" variant="ghost" onClick={onClose}>
            Annuler
          </Button>
          <Button
            type="button"
            disabled={needsSignatories}
            onClick={() => onConfirm(options)}
          >
            Analyser l&apos;image
          </Button>
        </div>
      }
    >
      <div className="space-y-4">
        <Text variant="muted" className="text-sm">
          Loggy complétera le registre existant. Indiquez les options pour la principale entité
          créée ou étendue (ex. facture, commande).
        </Text>

        <div className="flex flex-wrap items-center gap-x-6 gap-y-3">
          <label className="flex cursor-pointer items-center gap-2">
            <input
              type="checkbox"
              checked={options.requires_signature}
              onChange={(e) =>
                setOptions((prev) => ({
                  ...prev,
                  requires_signature: e.target.checked,
                  signatory_role_ids: e.target.checked ? prev.signatory_role_ids : [],
                }))
              }
              className="h-4 w-4 rounded border-border accent-secondary"
            />
            <span className="text-sm text-foreground">Entité à signer</span>
          </label>
          <label className="flex cursor-pointer items-center gap-2">
            <input
              type="checkbox"
              checked={options.ai_suggestions}
              onChange={(e) =>
                setOptions((prev) => ({ ...prev, ai_suggestions: e.target.checked }))
              }
              className="h-4 w-4 rounded border-border accent-secondary"
            />
            <span className="text-sm text-foreground">Proposer dans les suggestions IA</span>
          </label>
        </div>

        {options.requires_signature && (
          <div className="space-y-2 rounded-lg border border-border p-3">
            <Text variant="label">Rôles signataires</Text>
            {loadingRoles && <Text variant="muted">Chargement des rôles…</Text>}
            {!loadingRoles && roles.length === 0 && (
              <Text variant="muted">Aucun rôle — créez-en dans Paramètres → Rôles.</Text>
            )}
            {!loadingRoles && roles.length > 0 && (
              <div className="flex flex-wrap items-center gap-x-5 gap-y-2">
                {roles.map((role) => {
                  const checked = options.signatory_role_ids.includes(role.id);
                  return (
                    <label
                      key={role.id}
                      className={cn(
                        "flex cursor-pointer items-center gap-2 rounded-md px-1 py-0.5",
                        checked && "text-secondary",
                      )}
                    >
                      <input
                        type="checkbox"
                        checked={checked}
                        onChange={() => toggleRole(role.id)}
                        className="h-4 w-4 rounded border-border accent-secondary"
                      />
                      <span className="text-sm">{role.nom}</span>
                    </label>
                  );
                })}
              </div>
            )}
            {needsSignatories && (
              <Alert
                variant="warning"
                size="inline"
                message="Cochez au moins un rôle signataire."
              />
            )}
          </div>
        )}

        <div className="space-y-2 rounded-lg border border-border p-3">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <Text variant="label">Colonnes / attributs</Text>
            <Button type="button" size="sm" variant="secondary" onClick={addAttributeHint}>
              <Plus className="mr-1 h-3.5 w-3.5" />
              Ajouter
            </Button>
          </div>
          <Text variant="muted" className="text-xs">
            Indiquez les colonnes visibles dans l&apos;image et cochez « Obligatoire » si le champ
            doit être requis. Laissez vide pour laisser Loggy déduire depuis l&apos;image.
          </Text>
          {options.attribute_hints.length === 0 ? (
            <Text variant="muted" className="text-sm">
              Aucune colonne — Loggy déterminera les attributs et leur obligation.
            </Text>
          ) : (
            <div className="space-y-2">
              <div className="hidden gap-3 px-1 text-xs font-medium text-muted sm:grid sm:grid-cols-[1fr_auto_auto]">
                <span>Nom (snake_case)</span>
                <span className="text-center">Obligatoire</span>
                <span className="w-9" />
              </div>
              {options.attribute_hints.map((hint, idx) => (
                <div
                  key={idx}
                  className="flex flex-wrap items-center gap-x-4 gap-y-2 rounded-md border border-border/60 p-2 sm:grid sm:grid-cols-[1fr_auto_auto] sm:items-center sm:gap-3"
                >
                  <Input
                    value={hint.nom}
                    onChange={(e) => updateAttributeHint(idx, { nom: e.target.value })}
                    placeholder="ex. date_vente, prix_total"
                    className="min-w-[140px] flex-1"
                  />
                  <label className="flex shrink-0 cursor-pointer items-center gap-2 sm:justify-center">
                    <input
                      type="checkbox"
                      checked={hint.required}
                      onChange={(e) =>
                        updateAttributeHint(idx, { required: e.target.checked })
                      }
                      className="h-4 w-4 rounded border-border accent-secondary"
                    />
                    <span className="text-sm text-foreground">Obligatoire</span>
                  </label>
                  <Button
                    type="button"
                    size="sm"
                    variant="ghost"
                    className="w-9 shrink-0 px-0 sm:justify-self-end"
                    onClick={() => removeAttributeHint(idx)}
                    aria-label="Retirer la colonne"
                  >
                    <Trash2 className="h-4 w-4 text-primary" />
                  </Button>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </Modal>
  );
}
