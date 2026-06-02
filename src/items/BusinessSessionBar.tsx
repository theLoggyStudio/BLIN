import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { CalendarRange, X } from "lucide-react";
import { useBusinessSession } from "@/contexts/BusinessSessionContext";
import { Button } from "@/items/Button";
import { Modal } from "@/items/Modal";
import { Select } from "@/items/Select";
import { Text } from "@/items/Text";
import type { ScreenRow } from "@/types/screen";

interface BusinessSessionBarProps {
  collapsed?: boolean;
}

/** Sélection de la session métier active (entités `is_session` du registre). */
export function BusinessSessionBar({ collapsed = false }: BusinessSessionBarProps) {
  const { active, sessionEntities, loading, setActive, clearActive } = useBusinessSession();
  const [pickerOpen, setPickerOpen] = useState(false);
  const [entityKey, setEntityKey] = useState("");
  const [recordId, setRecordId] = useState("");
  const [options, setOptions] = useState<{ value: string; label: string }[]>([]);
  const [optionsLoading, setOptionsLoading] = useState(false);

  useEffect(() => {
    if (sessionEntities.length === 0) return;
    if (!entityKey || !sessionEntities.some((e) => e.key === entityKey)) {
      setEntityKey(sessionEntities[0].key);
    }
  }, [sessionEntities, entityKey]);

  const loadOptions = useCallback(async (key: string) => {
    if (!key) return;
    setOptionsLoading(true);
    try {
      const rows = await invoke<ScreenRow[]>("dda_list", {
        payload: { screen_key: key, filters: {} },
      });
      setOptions(
        rows.map((row) => {
          const id = String(row.id ?? "");
          const label =
            String(row.libelle ?? row.nom ?? row.titre ?? row.reference ?? id).trim() || id;
          return { value: id, label };
        }),
      );
      setRecordId((prev) => (prev && rows.some((r) => String(r.id) === prev) ? prev : ""));
    } catch {
      setOptions([]);
    } finally {
      setOptionsLoading(false);
    }
  }, []);

  useEffect(() => {
    if (!pickerOpen || !entityKey) return;
    void loadOptions(entityKey);
  }, [pickerOpen, entityKey, loadOptions]);

  if (loading || sessionEntities.length === 0) {
    return null;
  }

  if (collapsed) {
    return (
      <button
        type="button"
        className="sidebar-sessions-icon-btn"
        title={active?.label ?? "Session métier"}
        onClick={() => setPickerOpen(true)}
      >
        <CalendarRange className="h-4 w-4" />
      </button>
    );
  }

  const sessionLabel = active?.label?.trim() || "Aucune session active";

  return (
    <>
      <section className="rounded-lg border border-border bg-surface-elevated/80 p-2" aria-label="Session métier">
        <div className="flex items-start gap-2">
          <CalendarRange className="mt-0.5 h-4 w-4 shrink-0 text-secondary" aria-hidden />
          <div className="min-w-0 flex-1">
            <p className="text-[10px] font-medium uppercase tracking-wide text-muted">Session</p>
            <p className="truncate text-xs text-foreground" title={sessionLabel}>
              {sessionLabel}
            </p>
          </div>
          {active && (
            <button
              type="button"
              className="rounded p-1 text-muted hover:bg-background hover:text-foreground"
              title="Effacer la session active"
              aria-label="Effacer la session active"
              onClick={() => void clearActive()}
            >
              <X className="h-3.5 w-3.5" />
            </button>
          )}
        </div>
        <Button
          size="sm"
          variant="secondary"
          className="mt-2 w-full text-xs"
          onClick={() => setPickerOpen(true)}
        >
          {active ? "Changer" : "Choisir une session"}
        </Button>
      </section>

      <Modal
        open={pickerOpen}
        onClose={() => setPickerOpen(false)}
        title="Session métier active"
      >
        <div className="flex flex-col gap-4">
          <Text variant="muted" className="text-sm">
            Les enregistrements liés à cette session seront filtrés et préremplis à la création.
          </Text>
          {sessionEntities.length > 1 && (
            <Select
              label="Type de session"
              value={entityKey}
              onChange={(e) => setEntityKey(e.target.value)}
              options={sessionEntities.map((e) => ({ value: e.key, label: e.label }))}
            />
          )}
          <Select
            label="Enregistrement"
            value={recordId}
            onChange={(e) => setRecordId(e.target.value)}
            disabled={optionsLoading || options.length === 0}
            options={[
              { value: "", label: optionsLoading ? "Chargement…" : "— Choisir —" },
              ...options,
            ]}
          />
          <div className="flex justify-end gap-2">
            <Button variant="ghost" onClick={() => setPickerOpen(false)}>
              Annuler
            </Button>
            <Button
              disabled={!recordId}
              onClick={async () => {
                await setActive(entityKey, recordId);
                setPickerOpen(false);
              }}
            >
              Activer
            </Button>
          </div>
        </div>
      </Modal>
    </>
  );
}
