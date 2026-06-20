import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Alert } from "@/items/Alert";
import { Modal } from "@/items/Modal";
import { Button } from "@/items/Button";
import { Input } from "@/items/Input";
import { Select } from "@/items/Select";
import { Text } from "@/items/Text";
import { PdfExportProgressPanel } from "@/items/PdfExportProgressPanel";
import {
  defaultListPdfSubtitle,
  defaultListPdfTitle,
  printEntityListPdf,
} from "@/lib/print/listPrint";
import { PdfExportCancelled } from "@/lib/print/pdfExport";
import { fetchDdaListCount, fetchDdaListPage } from "@/lib/ddaList";
import { formatTableBlockToken } from "@/lib/print/templateAttributes";
import type { ScreenConfigFile } from "@/types/screen";
import type { PdfExportProgress } from "@/types/pdfExportProgress";
import { useAlert } from "@/contexts/AlertContext";
import { notifyEntitySuccess } from "@/lib/entitySuccessAlert";

interface PrintListPdfModalProps {
  open: boolean;
  onClose: () => void;
  config: ScreenConfigFile;
}

const STOCK_KEY = "stock";
const STOCK_SOURCES_SAMPLE_SIZE = 100;

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

  const [listTotal, setListTotal] = useState(0);
  const [entitySources, setEntitySources] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [cancelling, setCancelling] = useState(false);
  const [pdfProgress, setPdfProgress] = useState<PdfExportProgress | null>(null);
  const abortRef = useRef<AbortController | null>(null);
  const [error, setError] = useState<string | null>(null);

  const [visibleColumns, setVisibleColumns] = useState<Set<string>>(new Set());
  const [columnFilters, setColumnFilters] = useState<Record<string, string>>({});
  const [dateField, setDateField] = useState("");
  const [dateFrom, setDateFrom] = useState("");
  const [dateTo, setDateTo] = useState("");
  const [entitySourceFilter, setEntitySourceFilter] = useState("");
  const [titre, setTitre] = useState("");
  const [sousTitre, setSousTitre] = useState("");

  const serverFilters = useMemo(() => {
    const out: Record<string, string> = {};
    for (const f of listFields) {
      const val = columnFilters[f.key]?.trim();
      if (val && f.filter?.enabled) out[f.key] = val;
    }
    return out;
  }, [columnFilters, listFields]);

  const hasClientOnlyFilters = useMemo(() => {
    if (entitySourceFilter.trim() && screenKey === STOCK_KEY) return true;
    if (dateField && (dateFrom || dateTo)) return true;
    return listFields.some(
      (f) => columnFilters[f.key]?.trim() && !f.filter?.enabled,
    );
  }, [columnFilters, listFields, dateField, dateFrom, dateTo, entitySourceFilter, screenKey]);

  const [serverFilteredTotal, setServerFilteredTotal] = useState<number | null>(null);

  const loadMeta = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const total = await fetchDdaListCount(screenKey);
      setListTotal(total);

      if (screenKey === STOCK_KEY) {
        const sample = await fetchDdaListPage(screenKey, {
          page: 0,
          pageSize: STOCK_SOURCES_SAMPLE_SIZE,
        });
        const set = new Set<string>();
        for (const r of sample.rows) {
          const v = r.entite_source;
          if (v != null && String(v).trim()) set.add(String(v).trim());
        }
        setEntitySources(Array.from(set).sort((a, b) => a.localeCompare(b, "fr")));
      } else {
        setEntitySources([]);
      }
    } catch (e) {
      setError(String(e));
      setListTotal(0);
      setEntitySources([]);
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
    setExporting(false);
    setCancelling(false);
    setPdfProgress(null);
    abortRef.current = null;
    setError(null);
    void loadMeta();
  }, [open, config.screen.label, listFields, dateFields, loadMeta]);

  useEffect(() => {
    if (!open) return;
    const keys = Object.keys(serverFilters);
    if (keys.length === 0) {
      setServerFilteredTotal(null);
      return;
    }
    let cancelled = false;
    void fetchDdaListCount(screenKey, serverFilters).then((n) => {
      if (!cancelled) setServerFilteredTotal(n);
    });
    return () => {
      cancelled = true;
    };
  }, [open, screenKey, serverFilters]);

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

  const previewCount =
    serverFilteredTotal != null && !hasClientOnlyFilters
      ? serverFilteredTotal
      : listTotal;

  const handleCancelExport = () => {
    if (!exporting || cancelling) return;
    setCancelling(true);
    abortRef.current?.abort();
  };

  const handleExport = async () => {
    const controller = new AbortController();
    abortRef.current = controller;
    setExporting(true);
    setCancelling(false);
    setPdfProgress(null);
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
          defaultListPdfSubtitle(screenKey, previewCount),
        onProgress: setPdfProgress,
        signal: controller.signal,
      });
      setPdfProgress((prev) =>
        prev ? { ...prev, current: prev.total, done: true, label: "Export terminé" } : prev,
      );
      notifyEntitySuccess(showSuccess, screenKey, "export_pdf_list");
      onClose();
    } catch (e) {
      if (e instanceof PdfExportCancelled) {
        setError("Export annulé.");
        showError("Export PDF annulé.");
        return;
      }
      const msg = String(e);
      setError(msg);
      showError(`Échec PDF liste : ${msg}`);
    } finally {
      setExporting(false);
      setCancelling(false);
      abortRef.current = null;
    }
  };

  const handleModalClose = () => {
    if (exporting) return;
    onClose();
  };

  const tableToken = formatTableBlockToken(screenKey);

  return (
    <Modal
      open={open}
      onClose={handleModalClose}
      closeDisabled={exporting}
      title={exporting ? "Génération du PDF" : "Exporter la liste en PDF"}
      size="lg"
      footer={
        exporting ? undefined : (
          <div className="flex justify-end gap-2">
            <Button variant="ghost" onClick={handleModalClose}>
              Annuler
            </Button>
            <Button onClick={() => void handleExport()} disabled={loading}>
              Générer le PDF
            </Button>
          </div>
        )
      }
    >
      {exporting ? (
        <PdfExportProgressPanel
          progress={pdfProgress}
          onCancel={handleCancelExport}
          cancelling={cancelling}
        />
      ) : (
      <div className="max-h-[70vh] space-y-5 overflow-y-auto pr-1">
        <Text variant="muted" className="text-sm">
          Le modèle « Liste » insère un tableau HTML pleine largeur via l&apos;attribut{" "}
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
          <p className="text-sm text-muted">Chargement…</p>
        )}
        {!loading && (
          <p className="text-sm text-secondary">
            {hasClientOnlyFilters ? (
              <>
                Filtres date / colonnes appliqués à l&apos;export — base : {listTotal} ligne(s) au
                total.
              </>
            ) : serverFilteredTotal != null ? (
              <>
                Aperçu : {serverFilteredTotal} ligne(s) filtrée(s) sur {listTotal} — tableau{" "}
                {tableToken}.
              </>
            ) : (
              <>
                Aperçu : {listTotal} ligne(s) seront incluses dans {tableToken}.
              </>
            )}
          </p>
        )}
        {error && <Alert variant="danger" size="box" message={error} />}
      </div>
      )}
    </Modal>
  );
}
