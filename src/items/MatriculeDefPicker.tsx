import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/items/Button";
import { Input } from "@/items/Input";
import { Modal } from "@/items/Modal";
import { Select } from "@/items/Select";
import { Alert } from "@/items/Alert";
import type { MatriculeDef } from "@/types/entity";

interface MatriculeDefPickerProps {
  value?: string | null;
  onChange: (matriculeRef: string | undefined) => void;
}

/** Sélection d'une définition matricule (catalogue global) + création. */
export function MatriculeDefPicker({ value, onChange }: MatriculeDefPickerProps) {
  const [items, setItems] = useState<MatriculeDef[]>([]);
  const [loading, setLoading] = useState(false);
  const [createOpen, setCreateOpen] = useState(false);
  const [libelle, setLibelle] = useState("");
  const [base, setBase] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const list = await invoke<MatriculeDef[]>("entity_matricule_registry_list");
      setItems(list);
    } catch (e) {
      setItems([]);
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const handleCreate = async () => {
    setSaving(true);
    setError(null);
    try {
      const created = await invoke<MatriculeDef>("entity_matricule_registry_create", {
        payload: { libelle: libelle.trim(), base: base.trim() },
      });
      await load();
      onChange(created.id);
      setCreateOpen(false);
      setLibelle("");
      setBase("");
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const selected = items.find((m) => m.id === value);

  return (
    <div className="space-y-2 sm:col-span-2">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-end">
        <div className="min-w-0 flex-1">
          <Select
            label="Définition matricule"
            value={value ?? ""}
            disabled={loading}
            onChange={(e) => onChange(e.target.value || undefined)}
            options={[
              { value: "", label: loading ? "Chargement…" : "— Choisir un libellé —" },
              ...items.map((m) => ({
                value: m.id,
                label: `${m.libelle} (${m.base})`,
              })),
            ]}
          />
        </div>
        <Button size="sm" variant="secondary" onClick={() => setCreateOpen(true)}>
          Nouveau
        </Button>
      </div>
      {selected && (
        <p className="text-xs text-muted">
          Format à l&apos;écran :{" "}
          <span className="font-mono text-foreground">
            {selected.base}
            {"{jjmmaaaa}"}
            {"{n°}"}
          </span>{" "}
          — ex. {selected.base}1203202601
        </p>
      )}
      {error && !createOpen && <Alert variant="danger" size="field" message={error} />}

      <Modal
        open={createOpen}
        onClose={() => {
          setCreateOpen(false);
          setError(null);
        }}
        title="Nouvelle définition matricule"
        footer={
          <div className="flex justify-end gap-2">
            <Button variant="ghost" onClick={() => setCreateOpen(false)} disabled={saving}>
              Annuler
            </Button>
            <Button onClick={() => void handleCreate()} disabled={saving || !libelle.trim() || !base.trim()}>
              {saving ? "Création…" : "Créer"}
            </Button>
          </div>
        }
      >
        <div className="space-y-3">
          <Input
            label="Libellé"
            value={libelle}
            onChange={(e) => setLibelle(e.target.value)}
            hint="Nom affiché dans la liste — doit être unique."
          />
          <Input
            label="Base"
            value={base}
            onChange={(e) => setBase(e.target.value.toUpperCase())}
            hint="Préfixe du matricule (lettres/chiffres) — ex. MAT, CMD — doit être unique."
          />
          {error && createOpen && <Alert variant="danger" size="field" message={error} />}
        </div>
      </Modal>
    </div>
  );
}
