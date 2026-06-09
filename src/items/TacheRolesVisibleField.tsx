import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Alert } from "@/items/Alert";
import { Text } from "@/items/Text";
import type { RoleRow } from "@/types/users";

const VIS_PERSONNALISEE = "personnalisee";

interface TacheRolesVisibleFieldProps {
  label: string;
  visibilite: unknown;
  rolesCsv: string;
  onChange: (csv: string) => void;
  error?: string;
  readOnly?: boolean;
}

/** Cases à cocher des rôles pour visibilité « personnalisée » sur l'entité tache. */
export function TacheRolesVisibleField({
  label,
  visibilite,
  rolesCsv,
  onChange,
  error,
  readOnly,
}: TacheRolesVisibleFieldProps) {
  const [roles, setRoles] = useState<RoleRow[]>([]);
  const [loading, setLoading] = useState(true);
  const selected = parseRolesCsv(rolesCsv);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const list = await invoke<RoleRow[]>("entity_list_roles");
      setRoles(list);
    } catch {
      setRoles([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  if (String(visibilite ?? "") !== VIS_PERSONNALISEE) {
    return null;
  }

  const toggle = (roleId: string, checked: boolean) => {
    if (readOnly) return;
    const next = new Set(selected);
    if (checked) {
      next.add(roleId);
    } else {
      next.delete(roleId);
    }
    onChange(encodeRolesCsv(Array.from(next)));
  };

  return (
    <div className="flex flex-col gap-2">
      <Text variant="label">{label}</Text>
      {loading && <p className="text-sm text-muted">Chargement des rôles…</p>}
      {!loading && roles.length === 0 && (
        <p className="text-sm text-muted">Aucun rôle défini dans Paramètres.</p>
      )}
      <div className="flex flex-col gap-2 rounded-lg border border-border p-3">
        {roles.map((role) => (
          <label key={role.id} className="flex cursor-pointer items-center gap-3">
            <input
              type="checkbox"
              checked={selected.includes(role.id)}
              disabled={readOnly}
              onChange={(e) => toggle(role.id, e.target.checked)}
              className="h-4 w-4 rounded border-border accent-secondary"
            />
            <span className="text-sm text-foreground">{role.nom}</span>
            <span className="text-xs text-muted">({role.id})</span>
          </label>
        ))}
      </div>
      {error && <Alert variant="danger" size="box" message={error} />}
    </div>
  );
}

function parseRolesCsv(raw: string): string[] {
  if (!raw.trim()) return [];
  return raw
    .trim()
    .replace(/^,|,$/g, "")
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean);
}

function encodeRolesCsv(ids: string[]): string {
  if (ids.length === 0) return "";
  const sorted = [...ids].sort();
  return `,${sorted.join(",")},`;
}
