import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Alert } from "@/items/Alert";
import { Button } from "@/items/Button";
import { Modal } from "@/items/Modal";
import { EntityDefFormTable } from "@/items/EntityDefFormTable";
import { SyncProgressBar } from "@/items/SyncProgressBar";
import { useAlert } from "@/contexts/AlertContext";
import { usePrivilege } from "@/hooks/usePrivilege";
import { normalizeEntityDefForSave } from "@/lib/entityDefForm";
import { ENTITY_REGISTRY_SYNCED_EVENT } from "@/constants/events";
import type { EntityDef } from "@/types/entity";
import type { EntitySyncProgress } from "@/types/syncProgress";
import type { RoleRow } from "@/types/users";

interface EntityDefLoggyModalProps {
  open: boolean;
  initialEntity: EntityDef | null;
  onClose: () => void;
  onSaved?: () => void;
}

/** Modal Loggy — création d'une définition d'entité (registre). */
export function EntityDefLoggyModal({
  open,
  initialEntity,
  onClose,
  onSaved,
}: EntityDefLoggyModalProps) {
  const hasFullRegistry = usePrivilege("parametres:entites");
  const hasCreateViaLoggy = usePrivilege("parametres:entites:creer");
  const canCreate = hasFullRegistry || hasCreateViaLoggy;
  const { showSuccess, showError, showWarning } = useAlert();
  const [entity, setEntity] = useState<EntityDef | null>(null);
  const [registryEntities, setRegistryEntities] = useState<EntityDef[]>([]);
  const [roles, setRoles] = useState<RoleRow[]>([]);
  const [saving, setSaving] = useState(false);
  const [syncProgress, setSyncProgress] = useState<EntitySyncProgress | null>(null);
  const [validationAttempted, setValidationAttempted] = useState(false);
  const [accessChecked, setAccessChecked] = useState(false);
  const [accessAllowed, setAccessAllowed] = useState(false);

  const verifyAccess = useCallback(async () => {
    try {
      const res = await invoke<{ allowed: boolean }>("entity_registry_create_access");
      setAccessAllowed(res.allowed);
    } catch {
      setAccessAllowed(false);
    } finally {
      setAccessChecked(true);
    }
  }, []);

  useEffect(() => {
    if (!open) {
      setAccessChecked(false);
      return;
    }
    void verifyAccess();
  }, [open, verifyAccess]);

  useEffect(() => {
    if (!open || !initialEntity) return;
    setEntity({
      ...initialEntity,
      attributs: [...initialEntity.attributs],
      signatory_role_ids: [...(initialEntity.signatory_role_ids ?? [])],
    });
    setValidationAttempted(false);
  }, [open, initialEntity]);

  useEffect(() => {
    if (!open || !accessAllowed) return;
    void invoke<{ nom: string; label?: string }[]>("entity_registry_list_brief")
      .then((rows) =>
        setRegistryEntities(
          rows.map((r) => ({
            nom: r.nom,
            label: r.label,
            attributs: [],
          })),
        ),
      )
      .catch(() => setRegistryEntities([]));
    void invoke<RoleRow[]>("users_list_roles")
      .then(setRoles)
      .catch(() => setRoles([]));
  }, [open, accessAllowed]);

  useEffect(() => {
    const unlisten = listen<EntitySyncProgress>("entity-sync-progress", (event) => {
      setSyncProgress(event.payload);
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  const save = async () => {
    if (!entity || !canCreate || !accessAllowed) return;
    setValidationAttempted(true);
    if (!entity.nom.trim()) {
      showWarning("Le nom de l'entité est obligatoire.");
      return;
    }
    if (
      entity.requires_signature &&
      (!entity.signatory_role_ids || entity.signatory_role_ids.length === 0)
    ) {
      showWarning("Sélectionnez au moins un rôle signataire.");
      return;
    }
    for (const attr of entity.attributs) {
      if (attr.type === "entity" && !attr.ref?.trim()) {
        showWarning(`L'attribut « ${attr.nom || "?"} » doit cibler une entité (ref).`);
        return;
      }
    }

    const normalized = normalizeEntityDefForSave(entity);
    setSaving(true);
    setSyncProgress({
      current: 0,
      total: 1,
      label: "Préparation…",
      step: "start",
      done: false,
    });
    try {
      const synced = await invoke<string[]>("entity_registry_append_entity", {
        payload: { entity: normalized },
      });
      const note =
        synced.length > 0
          ? `Entité « ${normalized.label} » créée. Synchronisé : ${synced.join(", ")}.`
          : `Entité « ${normalized.label} » créée et synchronisée.`;
      showSuccess(note);
      window.dispatchEvent(new CustomEvent(ENTITY_REGISTRY_SYNCED_EVENT));
      onSaved?.();
      onClose();
    } catch (e) {
      showError(String(e));
      setSyncProgress(null);
    } finally {
      setSaving(false);
      setSyncProgress((p) => (p?.done ? p : null));
    }
  };

  if (!open) return null;

  return (
    <Modal
      open={open}
      onClose={onClose}
      title="Nouvelle entité — registre métier"
      size="2xl"
      footer={
        <>
          <Button variant="ghost" onClick={onClose} disabled={saving}>
            Annuler
          </Button>
          <Button onClick={() => void save()} disabled={saving || !canCreate || !accessAllowed}>
            {saving ? "Enregistrement…" : "Créer l'entité"}
          </Button>
        </>
      }
    >
      {!accessChecked ? (
        <p className="py-8 text-center text-sm text-muted">Vérification des privilèges…</p>
      ) : !accessAllowed ? (
        <Alert
          variant="danger"
          size="box"
          message="Tu n'as pas le privilège parametres:entites:creer (ni parametres:entites). Contacte un administrateur."
        />
      ) : !entity ? (
        <p className="py-8 text-center text-sm text-muted">Chargement du formulaire…</p>
      ) : (
        <div className="space-y-4">
          <SyncProgressBar progress={syncProgress} active={saving} />
          <EntityDefFormTable
            value={entity}
            onChange={setEntity}
            existingEntities={registryEntities}
            roles={roles}
            validationAttempted={validationAttempted}
          />
        </div>
      )}
    </Modal>
  );
}
