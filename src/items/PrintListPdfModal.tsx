import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Modal } from "@/items/Modal";
import { Button } from "@/items/Button";
import { Input } from "@/items/Input";
import { Select } from "@/items/Select";
import { Text } from "@/items/Text";
import {
  defaultListPdfSubtitle,
  defaultListPdfTitle,
  printEntityListPdf,
} from "@/lib/print/listPrint";
import { formatTableBlockToken } from "@/lib/print/templateVariables";
import type { ScreenConfigFile, ScreenRow } from "@/types/screen";
import { useAlert } from "@/contexts/AlertContext";

interface PrintListPdfModalProps {
  open: boolean;
  onClose: () => void;
  config: ScreenConfigFile;
}

const STOCK_KEY = "stock";

export function PrintListPdfModal({ open, onClose, config }: PrintListPdfModalProps) {
  const { showSuccess, showError } = useAlert();
  const screenKey = config.screen.key;
  const listFields = useMemo(
    () =>
      config.fields.filter(
        (f) =>
          f.list?.enabled !== false &&
          f.type !== "hidden" &&
          f.type !== "detail_link",
      ),
    [config.fields],
  );

  const dateFields = useMemo(
    () => config.fields.filter((f) => f.type === "date" || f.type === "datetime"),
    [config.fields],
  );

  const [rows, setRows] = useState<ScreenRow[]>([]);
  const [loading, setLoading] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [visibleColumns, setVisibleColumns] = useState<Set<string>>(new Set());
  const [columnFilters, setColumnFilters] = useState<Record<string, string>>({});
  const [dateField, setDateField] = useState("");
  const [dateFrom, setDateFrom] = useState("");
  const [dateTo, setDateTo] = useState("");
  const [entitySourceFilter, setEntitySourceFilter] = useState("");
  const [titre, setTitre] = useState("");
  const [sousTitre, setSousTitre] = useState("");

  const loadRows = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await invoke<ScreenRow[]>("dda_list", {
        payload: { screen_key: screenKey, filters: {} },
      });
      setRows(data);
    } catch (e) {
      setError(String(e));
      setRows([]);
    } finally {
      setLoading(false);
    }
  }, [screenKey]);

  useEffect(() => {
    if (!open) return;
    setTitre(defaultListPdfTitle(config.screen.label));
    setSousTitre("");
    const keys = new Set(listFields.map((f) => f.key));
    setVisibleColumns(keys);
    setColumnFilters({});
    setDateField(dateFields[0]?.key ?? "");
    setDateFrom("");
    setDateTo("");
    setEntitySourceFilter("");
    void loadRows();
  }, [open, config.screen.label, listFields, dateFields, loadRows]);

  const entitySources = useMemo(() => {
    if (screenKey !== STOCK_KEY) return [];
    const set = new Set<string>();
    for (const r of rows) {
      const v = r.entite_source;
      if (v != null && String(v).trim()) set.add(String(v).trim());
    }
    return Array.from(set).sort((a, b) => a.localeCompare(b, "fr"));
  }, [rows, screenKey]);

  const filteredPreviewCount = useMemo(() => {
    return rows.filter((row) => {
      if (entitySourceFilter.trim() && screenKey === STOCK_KEY) {
        if (String(row.entite_source ?? "").trim() !== entitySourceFilter.trim()) {
          return false;
        }
      }
      if (dateField && (dateFrom || dateTo)) {
        const raw = String(row[dateField] ?? "").slice(0, 10);
        if (raw) {
          if (dateFrom && raw < dateFrom) return false;
          if (dateTo && raw > dateTo) return false;
        }
      }
      for (const [key, val] of Object.entries(columnFilters)) {
        if (!val.trim()) continue;
        const hay = String(row[key] ?? "").toLowerCase();
        if (!hay.includes(val.trim().toLowerCase())) return false;
      }
      return true;
    }).length;
  }, [rows, columnFilters, dateField, dateFrom, dateTo, entitySourceFilter, screenKey]);

  const toggleColumn = (key: string) => {
    setVisibleColumns((prev) => {
      const next = new Set(prev);
      if (next.has(key)) {
        if (next.size > 1) next.delete(key);
      } else {
        next.add(key);
      }
      return next;
    });
  };

  const handleExport = async () => {
    setExporting(true);
    setError(null);
    try {
      const cols = listFields.map((f) => f.key).filter((k) => visibleColumns.has(k));
      await printEntityListPdf({
        screenKey,
        visibleColumns: cols,
        filters: columnFilters,
        dateField: dateField || undefined,
        dateFrom: dateFrom || undefined,
        dateTo: dateTo || undefined,
        entitySourceFilter:
          screenKey === STOCK_KEY && entitySourceFilter.trim()
            ? entitySourceFilter.trim()
            : undefined,
        titre: titre.trim() || undefined,
        sousTitre:
          sousTitre.trim() ||
          defaultListPdfSubtitle(screenKey, filteredPreviewCount),
      });
      showSuccess(`PDF liste généré pour « ${config.screen.label} ».`);
      onClose();
    } catch (e) {
      const msg = String(e);
      setError(msg);
      showError(`Échec PDF liste : ${msg}`);
    } finally {
      setExporting(false);
    }
  };

  const tableToken = formatTableBlockToken(screenKey);

  return (
    <Modal
      open={open}
      onClose={onClose}
      title="Exporter la liste en PDF"
      size="lg"
      footer={
        <div className="flex justify-end gap-2">
          <Button variant="ghost" onClick={onClose} disabled={exporting}>
            Annuler
          </Button>
          <Button onClick={() => void handleExport()} disabled={exporting || loading}>
            {exporting ? "Génération…" : "Générer le PDF"}
          </Button>
        </div>
      }
    >
      <div className="max-h-[70vh] space-y-5 overflow-y-auto pr-1">
        <Text variant="muted" className="text-sm">
          Le modèle « Liste » insère un tableau HTML pleine largeur via la variable{" "}
          <code className="text-secondary">{tableToken}</code> (colonnes et filtres ci-dessous).
        </Text>

        <div className="grid gap-3 sm:grid-cols-2">
          <Input
            label="Titre du document"
            value={titre}
            onChange={(e) => setTitre(e.target.value)}
          />
          <Input
            label="Sous-titre (optionnel)"
            value={sousTitre}
            placeholder="Laisser vide pour un libellé automatique"
            onChange={(e) => setSousTitre(e.target.value)}
          />
        </div>

        {screenKey === STOCK_KEY && entitySources.length > 0 && (
          <Select
            label="Filtrer par entité source"
            value={entitySourceFilter}
            onChange={(e) => setEntitySourceFilter(e.target.value)}
            options={[
              { value: "", label: "Toutes les entités" },
              ...entitySources.map((s) => ({ value: s, label: s })),
            ]}
          />
        )}

        {dateFields.length > 0 && (
          <div className="rounded-lg border border-border bg-surface-elevated/40 p-3 space-y-3">
            <Text variant="label">Filtre par date</Text>
            <Select
              label="Champ date"
              value={dateField}
              onChange={(e) => setDateField(e.target.value)}
              options={[
                { value: "", label: "Aucun filtre date" },
                ...dateFields.map((f) => ({ value: f.key, label: f.label })),
              ]}
            />
            <div className="grid gap-3 sm:grid-cols-2">
              <Input
                label="Du"
                type="date"
                value={dateFrom}
                disabled={!dateField}
                onChange={(e) => setDateFrom(e.target.value)}
              />
              <Input
                label="Au"
                type="date"
                value={dateTo}
                disabled={!dateField}
                onChange={(e) => setDateTo(e.target.value)}
              />
            </div>
          </div>
        )}

        <div className="rounded-lg border border-border bg-surface-elevated/40 p-3">
          <Text variant="label" className="mb-2">
            Colonnes visibles dans le PDF
          </Text>
          <div className="flex flex-wrap gap-2">
            {listFields.map((f) => (
              <label
                key={f.key}
                className="flex cursor-pointer items-center gap-2 rounded-md border border-border px-2 py-1 text-sm"
              >
                <input
                  type="checkbox"
                  checked={visibleColumns.has(f.key)}
                  onChange={() => toggleColumn(f.key)}
                />
                {f.label}
              </label>
            ))}
          </div>
        </div>

        <div className="rounded-lg border border-border bg-surface-elevated/40 p-3 space-y-2">
          <Text variant="label">Filtre par colonne (contient)</Text>
          {listFields.map((f) => (
            <Input
              key={f.key}
              label={f.label}
              value={columnFilters[f.key] ?? ""}
              placeholder="Tous si vide"
              onChange={(e) =>
                setColumnFilters((prev) => ({ ...prev, [f.key]: e.target.value }))
              }
            />
          ))}
        </div>

        {loading && (
          <p className="text-sm text-muted">Chargement des données…</p>
        )}
        {!loading && (
          <p className="text-sm text-secondary">
            Aperçu : {filteredPreviewCount} ligne(s) sur {rows.length} seront incluses dans{" "}
            {tableToken}.
          </p>
        )}
        {error && (
          <p className="text-sm text-primary" role="alert">
            {error}
          </p>
        )}
      </div>
    </Modal>
  );
}
