import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { KeyRound, Pencil, Plus, UserPlus } from "lucide-react";
import { Alert } from "@/items/Alert";
import { Guard } from "@/components/Guard";
import { Button } from "@/items/Button";
import { Input } from "@/items/Input";
import { Modal } from "@/items/Modal";
import { Select } from "@/items/Select";
import { Table, type Column } from "@/items/Table";
import { Text } from "@/items/Text";
import type { RoleRow, UserRow } from "@/types/users";

type UserForm = {
  id: string;
  nom: string;
  email: string;
  password: string;
  role_id: string;
  actif: boolean;
};

const emptyForm = (roleId = ""): UserForm => ({
  id: "",
  nom: "",
  email: "",
  password: "",
  role_id: roleId,
  actif: true,
});

/** Gestion des comptes utilisateurs (création / édition). */
export function UsersPanel() {
  const [users, setUsers] = useState<UserRow[]>([]);
  const [roles, setRoles] = useState<RoleRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [modalOpen, setModalOpen] = useState(false);
  const [form, setForm] = useState<UserForm>(emptyForm());

  const loadRoles = useCallback(async () => {
    try {
      const roleRows = await invoke<RoleRow[]>("users_list_roles");
      setRoles(roleRows);
    } catch {
      setRoles([]);
    }
  }, []);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const userRows = await invoke<UserRow[]>("users_list");
      setUsers(userRows);
      await loadRoles();
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [loadRoles]);

  useEffect(() => {
    void load();
  }, [load]);

  const roleOptions = useMemo(
    () => roles.map((r) => ({ value: r.id, label: r.nom })),
    [roles],
  );

  const openCreate = async () => {
    setError(null);
    try {
      const roleRows = await invoke<RoleRow[]>("users_list_roles");
      setRoles(roleRows);
      setForm(emptyForm(roleRows[0]?.id ?? ""));
      setModalOpen(true);
      setMessage(null);
    } catch (e) {
      setError(String(e));
    }
  };

  const openEdit = (u: UserRow) => {
    setForm({
      id: u.id,
      nom: u.nom,
      email: u.email,
      password: "",
      role_id: u.role_id,
      actif: u.actif,
    });
    setModalOpen(true);
    setMessage(null);
    setError(null);
  };

  const save = async () => {
    if (!form.nom.trim() || !form.email.trim() || !form.role_id) {
      setError("Nom, e-mail et rôle sont obligatoires.");
      return;
    }
    if (!form.id && form.password.length < 6) {
      setError("Mot de passe d'au moins 6 caractères pour un nouveau compte.");
      return;
    }
    setSaving(true);
    setError(null);
    setMessage(null);
    try {
      if (form.id) {
        await invoke<UserRow>("users_update", {
          payload: {
            id: form.id,
            nom: form.nom.trim(),
            email: form.email.trim(),
            role_id: form.role_id,
            actif: form.actif,
          },
        });
        setMessage("Utilisateur mis à jour.");
      } else {
        await invoke<UserRow>("users_create", {
          payload: {
            nom: form.nom.trim(),
            email: form.email.trim(),
            password: form.password,
            role_id: form.role_id,
          },
        });
        setMessage("Utilisateur créé.");
      }
      setModalOpen(false);
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const resetPassword = async (user: UserRow) => {
    const ok = window.confirm(
      `Réinitialiser le mot de passe de ${user.nom} à "user123" ?\nÀ sa prochaine connexion, l'utilisateur devra définir un nouveau mot de passe.`,
    );
    if (!ok) return;
    setSaving(true);
    setError(null);
    setMessage(null);
    try {
      await invoke<UserRow>("users_reset_password", { payload: { id: user.id } });
      setMessage(`Mot de passe réinitialisé pour ${user.nom} (temporaire : user123).`);
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const columns: Column<UserRow>[] = [
    { key: "nom", header: "Nom", sortable: true },
    { key: "email", header: "E-mail", sortable: true },
    { key: "role", header: "Rôle", sortable: true },
    {
      key: "actif",
      header: "Actif",
      render: (row) => (
        <span className={row.actif ? "text-emerald-400" : "text-muted"}>
          {row.actif ? "Oui" : "Non"}
        </span>
      ),
    },
    {
      key: "_actions",
      header: "",
      className: "w-28",
      render: (row) => (
        <Guard privilege="parametres:utilisateurs">
          <div className="flex gap-1">
            <Button
              variant="ghost"
              size="sm"
              aria-label="Modifier"
              title="Modifier"
              onClick={(e) => {
                e.stopPropagation();
                void loadRoles().then(() => openEdit(row));
              }}
            >
              <Pencil className="h-4 w-4" />
            </Button>
            <Button
              variant="ghost"
              size="sm"
              aria-label="Réinitialiser mot de passe"
              title='Réinitialiser à "user123"'
              onClick={(e) => {
                e.stopPropagation();
                void resetPassword(row);
              }}
            >
              <KeyRound className="h-4 w-4 text-primary" />
            </Button>
          </div>
        </Guard>
      ),
    },
  ];

  if (loading) {
    return <p className="text-sm text-muted">Chargement des utilisateurs…</p>;
  }

  return (
    <div className="space-y-4">
      <Text variant="muted" className="text-sm">
        Créez et modifiez les comptes. Les privilèges effectifs dépendent du rôle assigné.
      </Text>

      {(message || error) && (
        <Alert
          variant={error ? "danger" : "success"}
          size="box"
          role="status"
          message={error ?? message ?? ""}
        />
      )}

      <Guard privilege="parametres:utilisateurs">
        <div className="flex flex-wrap gap-2">
          <Button size="sm" onClick={() => void openCreate()} disabled={roles.length === 0}>
            <UserPlus className="h-4 w-4" />
            Nouvel utilisateur
          </Button>
        </div>
        {roles.length === 0 && (
          <Alert
            variant="warning"
            size="box"
            message="Créez d'abord un rôle dans le panneau Rôles."
          />
        )}
      </Guard>

      <Table
        columns={columns}
        data={users}
        keyExtractor={(r) => r.id}
        emptyMessage="Aucun utilisateur."
      />

      <Guard privilege="parametres:utilisateurs">
      <Modal
        open={modalOpen}
        onClose={() => setModalOpen(false)}
        title={form.id ? "Modifier l'utilisateur" : "Nouvel utilisateur"}
        size="md"
      >
        <div className="space-y-4">
          <Input
            label="Nom"
            value={form.nom}
            onChange={(e) => setForm({ ...form, nom: e.target.value })}
            required
          />
          <Input
            label="E-mail"
            type="email"
            value={form.email}
            onChange={(e) => setForm({ ...form, email: e.target.value })}
            required
          />
          {!form.id && (
            <Input
              label="Mot de passe"
              type="password"
              value={form.password}
              onChange={(e) => setForm({ ...form, password: e.target.value })}
              hint="Minimum 6 caractères"
              required
            />
          )}
          <Select
            label="Rôle"
            value={form.role_id}
            onChange={(e) => setForm({ ...form, role_id: e.target.value })}
            options={roleOptions}
          />
          <label className="flex cursor-pointer items-center gap-2 text-sm text-foreground">
            <input
              type="checkbox"
              checked={form.actif}
              onChange={(e) => setForm({ ...form, actif: e.target.checked })}
              className="h-4 w-4 rounded border-border accent-secondary"
            />
            Compte actif
          </label>
          <div className="flex gap-2 pt-2">
            <Button size="sm" disabled={saving} onClick={() => void save()}>
              <Plus className="h-4 w-4" />
              {saving ? "Enregistrement…" : form.id ? "Enregistrer" : "Créer"}
            </Button>
            <Button size="sm" variant="ghost" onClick={() => setModalOpen(false)}>
              Annuler
            </Button>
          </div>
        </div>
      </Modal>
      </Guard>
    </div>
  );
}
