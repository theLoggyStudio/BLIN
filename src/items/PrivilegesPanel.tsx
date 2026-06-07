import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Save, Shield } from "lucide-react";
import { Alert } from "@/items/Alert";
import { Button } from "@/items/Button";
import { Input } from "@/items/Input";
import { Select } from "@/items/Select";
import { Text } from "@/items/Text";
import {
  isPrivilegeChecked,
  mergePrivilegeCatalog,
  normalizePrivilegesForSave,
  togglePrivilege,
} from "@/lib/rolePrivileges";
import type { RoleWithPrivileges } from "@/types/privileges";

interface PrivilegesPanelProps {
  onClose?: () => void;
}

/** Affectation des privilèges existants par rôle (catalogue alimenté uniquement par les triggers). */
export function PrivilegesPanel({ onClose }: PrivilegesPanelProps) {
  const [roles, setRoles] = useState<RoleWithPrivileges[]>([]);
  const [catalog, setCatalog] = useState<string[]>([]);
  const [roleId, setRoleId] = useState("");
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [filter, setFilter] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [roleRows, privCatalog] = await Promise.all([
        invoke<RoleWithPrivileges[]>("roles_list_with_privileges"),
        invoke<string[]>("privileges_list_catalog"),
      ]);
      setRoles(roleRows);
      setCatalog(privCatalog);
      if (roleRows.length > 0 && !roleId) {
        setRoleId(roleRows[0].id);
        setSelected(new Set(roleRows[0].privileges));
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [roleId]);

  useEffect(() => {
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps -- chargement initial
  }, []);

  useEffect(() => {
    const role = roles.find((r) => r.id === roleId);
    if (role) {
      setSelected(new Set(role.privileges));
    }
  }, [roleId, roles]);

  const allPrivileges = useMemo(
    () => mergePrivilegeCatalog(catalog, selected),
    [catalog, selected],
  );

  const filteredPrivileges = useMemo(() => {
    const q = filter.trim().toLowerCase();
    if (!q) return allPrivileges;
    return allPrivileges.filter((p) => p.toLowerCase().includes(q));
  }, [allPrivileges, filter]);

  const toggle = (priv: string) => {
    setSelected((prev) => togglePrivilege(prev, priv, allPrivileges));
  };

  const save = async () => {
    if (!roleId) return;
    setSaving(true);
    setMessage(null);
    setError(null);
    try {
      await invoke("roles_update_privileges", {
        payload: {
          role_id: roleId,
          privileges: normalizePrivilegesForSave(selected, catalog),
        },
      });
      setMessage("Privilèges enregistrés pour ce rôle.");
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const currentRole = roles.find((r) => r.id === roleId);

  if (loading) {
    return <p className="text-sm text-muted py-4">Chargement des rôles et privilèges…</p>;
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2 text-secondary">
        <Shield className="h-5 w-5" />
        <Text variant="label">Privilèges par rôle</Text>
      </div>

      {(message || error) && (
        <Alert
          variant={error ? "danger" : "info"}
          size="inline"
          role="status"
          message={error ?? message ?? ""}
        />
      )}

      <Select
        label="Rôle"
        value={roleId}
        onChange={(e) => setRoleId(e.target.value)}
        options={roles.map((r) => ({ value: r.id, label: r.nom }))}
      />

      {currentRole && (
        <Text variant="muted">
          {selected.size} privilège(s) actif(s) pour « {currentRole.nom} ». Les droits listés sont créés
          automatiquement (entités, écrans système) — pas de saisie manuelle.
        </Text>
      )}

      <Input
        label="Filtrer"
        value={filter}
        onChange={(e) => setFilter(e.target.value)}
        placeholder="ex. tache, users, ai"
      />

      <div className="max-h-64 overflow-y-auto rounded-lg border border-border divide-y divide-border">
        {filteredPrivileges.length === 0 ? (
          <p className="p-4 text-sm text-muted">Aucun privilège.</p>
        ) : (
          filteredPrivileges.map((priv) => (
            <label
              key={priv}
              className="flex cursor-pointer items-center gap-3 px-4 py-2.5 hover:bg-surface-elevated/50"
            >
              <input
                type="checkbox"
                checked={isPrivilegeChecked(selected, priv)}
                onChange={() => toggle(priv)}
                className="h-4 w-4 rounded border-border accent-secondary"
              />
              <span className="font-mono text-sm text-foreground">{priv}</span>
            </label>
          ))
        )}
      </div>

      <div className="flex flex-wrap gap-2 pt-2">
        <Button size="sm" onClick={() => void save()} disabled={saving || !roleId}>
          <Save className="h-4 w-4" />
          {saving ? "Enregistrement…" : "Enregistrer"}
        </Button>
        {onClose && (
          <Button size="sm" variant="ghost" onClick={onClose}>
            Fermer
          </Button>
        )}
      </div>
    </div>
  );
}
