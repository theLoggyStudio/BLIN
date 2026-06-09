import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Pencil, Plus, Save, Shield, Trash2 } from "lucide-react";
import { Alert } from "@/items/Alert";
import { Button } from "@/items/Button";
import { Input } from "@/items/Input";
import { Modal } from "@/items/Modal";
import { Select } from "@/items/Select";
import { Table, type Column } from "@/items/Table";
import { Text } from "@/items/Text";
import {
  isPrivilegeChecked,
  mergePrivilegeCatalog,
  normalizePrivilegesForSave,
  togglePrivilege,
} from "@/lib/rolePrivileges";
import type { RoleWithPrivileges } from "@/types/privileges";
import type { RoleRow } from "@/types/users";

const PROTECTED_ROLES = new Set([
  "role-admin",
  "role-agent",
  "role-directeur",
  "role-tech",
  "role-compta",
]);

/** Gestion des rôles et de leurs privilèges. */
export function RolesPanel() {
  const [roles, setRoles] = useState<RoleWithPrivileges[]>([]);
  const [catalog, setCatalog] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [roleId, setRoleId] = useState("");
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [privFilter, setPrivFilter] = useState("");
  const [createOpen, setCreateOpen] = useState(false);
  const [newRoleName, setNewRoleName] = useState("");
  const [renameOpen, setRenameOpen] = useState(false);
  const [renameName, setRenameName] = useState("");

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
      if (roleRows.length > 0 && !roleRows.some((r) => r.id === roleId)) {
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
    if (role) setSelected(new Set(role.privileges));
  }, [roleId, roles]);

  const currentRole = roles.find((r) => r.id === roleId);

  const allPrivileges = useMemo(
    () => mergePrivilegeCatalog(catalog, selected),
    [catalog, selected],
  );

  const filteredPrivileges = useMemo(() => {
    const q = privFilter.trim().toLowerCase();
    if (!q) return allPrivileges;
    return allPrivileges.filter((p) => p.toLowerCase().includes(q));
  }, [allPrivileges, privFilter]);

  const toggle = (priv: string) => {
    setSelected((prev) => togglePrivilege(prev, priv, allPrivileges));
  };

  const savePrivileges = async () => {
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
      setMessage("Privilèges enregistrés.");
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const createRole = async () => {
    const nom = newRoleName.trim();
    if (!nom) {
      setError("Nom du rôle requis.");
      return;
    }
    setSaving(true);
    setError(null);
    try {
      const created = await invoke<RoleRow>("roles_create", { payload: { nom } });
      setMessage(`Rôle « ${created.nom} » créé.`);
      setCreateOpen(false);
      setNewRoleName("");
      setRoleId(created.id);
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const renameRole = async () => {
    if (!roleId) return;
    const nom = renameName.trim();
    if (!nom) return;
    setSaving(true);
    setError(null);
    try {
      await invoke<RoleRow>("roles_update", { payload: { id: roleId, nom } });
      setMessage("Rôle renommé.");
      setRenameOpen(false);
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const deleteRole = async (id: string) => {
    if (PROTECTED_ROLES.has(id)) return;
    if (!window.confirm("Supprimer ce rôle ?")) return;
    setSaving(true);
    setError(null);
    try {
      await invoke("roles_delete", { payload: { id } });
      setMessage("Rôle supprimé.");
      setRoleId("");
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const tableColumns: Column<RoleWithPrivileges>[] = [
    { key: "nom", header: "Rôle", sortable: true },
    {
      key: "privileges",
      header: "Privilèges",
      render: (row) => (
        <span className="text-muted text-sm">{row.privileges.length} droit(s)</span>
      ),
    },
    {
      key: "_actions",
      header: "",
      className: "w-28",
      render: (row) => (
        <div className="flex justify-end gap-1">
          <Button
            variant="ghost"
            size="sm"
            aria-label="Configurer"
            onClick={(e) => {
              e.stopPropagation();
              setRoleId(row.id);
            }}
          >
            <Shield className="h-4 w-4" />
          </Button>
          <Button
            variant="ghost"
            size="sm"
            aria-label="Renommer"
            onClick={(e) => {
              e.stopPropagation();
              setRoleId(row.id);
              setRenameName(row.nom);
              setRenameOpen(true);
            }}
          >
            <Pencil className="h-4 w-4" />
          </Button>
          {!PROTECTED_ROLES.has(row.id) && (
            <Button
              variant="ghost"
              size="sm"
              aria-label="Supprimer"
              onClick={(e) => {
                e.stopPropagation();
                void deleteRole(row.id);
              }}
            >
              <Trash2 className="h-4 w-4 text-primary" />
            </Button>
          )}
        </div>
      ),
    },
  ];

  if (loading) {
    return <p className="text-sm text-muted">Chargement des rôles…</p>;
  }

  return (
    <div className="space-y-6">
      <Text variant="muted" className="text-sm">
        Créez des rôles métier et affectez les privilèges générés automatiquement (entités, écrans).
      </Text>

      {(message || error) && (
        <Alert
          variant={error ? "danger" : "success"}
          size="box"
          role="status"
          message={error ?? message ?? ""}
        />
      )}

      <div className="flex flex-wrap gap-2">
        <Button size="sm" onClick={() => setCreateOpen(true)}>
          <Plus className="h-4 w-4" />
          Nouveau rôle
        </Button>
      </div>

      <Table
        columns={tableColumns}
        data={roles}
        keyExtractor={(r) => r.id}
        emptyMessage="Aucun rôle."
        onRowClick={(row) => setRoleId(row.id)}
      />

      <div className="rounded-lg border border-border p-4 space-y-4">
        <div className="flex items-center gap-2 text-secondary">
          <Shield className="h-5 w-5" />
          <Text variant="label">Privilèges du rôle sélectionné</Text>
        </div>

        <Select
          label="Rôle"
          value={roleId}
          onChange={(e) => setRoleId(e.target.value)}
          options={roles.map((r) => ({ value: r.id, label: r.nom }))}
        />

        {currentRole && (
          <Text variant="muted">
            {selected.size} privilège(s) pour « {currentRole.nom} ».
          </Text>
        )}

        <Input
          label="Filtrer les privilèges"
          value={privFilter}
          onChange={(e) => setPrivFilter(e.target.value)}
          placeholder="ex. ecole, users, documents"
        />

        <div className="max-h-56 overflow-y-auto rounded-lg border border-border divide-y divide-border">
          {filteredPrivileges.length === 0 ? (
            <p className="p-4 text-sm text-muted">Aucun privilège (synchronisez les entités d&apos;abord).</p>
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

        <Button size="sm" disabled={saving || !roleId} onClick={() => void savePrivileges()}>
          <Save className="h-4 w-4" />
          {saving ? "Enregistrement…" : "Enregistrer les privilèges"}
        </Button>
      </div>

      <Modal open={createOpen} onClose={() => setCreateOpen(false)} title="Nouveau rôle" size="sm">
        <div className="space-y-4">
          <Input
            label="Nom du rôle"
            value={newRoleName}
            onChange={(e) => setNewRoleName(e.target.value)}
            hint="ex. Secrétaire, Comptable métier"
            required
          />
          <div className="flex gap-2">
            <Button size="sm" disabled={saving} onClick={() => void createRole()}>
              Créer
            </Button>
            <Button size="sm" variant="ghost" onClick={() => setCreateOpen(false)}>
              Annuler
            </Button>
          </div>
        </div>
      </Modal>

      <Modal open={renameOpen} onClose={() => setRenameOpen(false)} title="Renommer le rôle" size="sm">
        <div className="space-y-4">
          <Input
            label="Nom"
            value={renameName}
            onChange={(e) => setRenameName(e.target.value)}
            required
          />
          <div className="flex gap-2">
            <Button size="sm" disabled={saving} onClick={() => void renameRole()}>
              Enregistrer
            </Button>
            <Button size="sm" variant="ghost" onClick={() => setRenameOpen(false)}>
              Annuler
            </Button>
          </div>
        </div>
      </Modal>
    </div>
  );
}
