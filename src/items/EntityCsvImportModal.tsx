import { useCallback, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Upload } from "lucide-react";
import { Button } from "@/items/Button";
import { Modal } from "@/items/Modal";
import { Text } from "@/items/Text";
import { useAlert } from "@/contexts/AlertContext";

interface EntityCsvImportModalProps {
  entityKey: string;
  entityLabel: string;
  open: boolean;
  onClose: () => void;
  onImported: () => void;
}

interface ImportResult {
  success: boolean;
  inserted: number;
  updated: number;
  error_count: number;
  errors: string[];
}

export function EntityCsvImportModal({
  entityKey,
  entityLabel,
  open,
  onClose,
  onImported,
}: EntityCsvImportModalProps) {
  const { showSuccess, showError, showWarning } = useAlert();
  const [fileName, setFileName] = useState<string | null>(null);
  const [csvText, setCsvText] = useState("");
  const [importing, setImporting] = useState(false);

  const reset = () => {
    setFileName(null);
    setCsvText("");
  };

  const handleClose = () => {
    reset();
    onClose();
  };

  const onFile = useCallback((file: File | undefined) => {
    if (!file) return;
    if (!file.name.toLowerCase().endsWith(".csv")) {
      showWarning("Seuls les fichiers CSV sont acceptés.");
      return;
    }
    const reader = new FileReader();
    reader.onload = () => {
      const text = typeof reader.result === "string" ? reader.result : "";
      setCsvText(text);
      setFileName(file.name);
    };
    reader.onerror = () => showError("Impossible de lire le fichier CSV.");
    reader.readAsText(file, "UTF-8");
  }, [showError, showWarning]);

  const handleImport = async () => {
    if (!csvText.trim()) {
      showWarning("Choisissez un fichier CSV à importer.");
      return;
    }
    setImporting(true);
    try {
      const res = await invoke<ImportResult>("entity_import_csv", {
        payload: { entity_key: entityKey, csv: csvText },
      });
      if (res.error_count > 0) {
        showWarning(
          `Import partiel pour « ${entityLabel} » : ${res.inserted} créé(s), ${res.updated} mis à jour, ${res.error_count} erreur(s).`,
        );
        if (res.errors[0]) showError(res.errors[0]);
      } else {
        showSuccess(
          `Import réussi pour « ${entityLabel} » : ${res.inserted} créé(s), ${res.updated} mis à jour.`,
        );
      }
      onImported();
      handleClose();
    } catch (e) {
      showError(String(e));
    } finally {
      setImporting(false);
    }
  };

  return (
    <Modal
      open={open}
      onClose={handleClose}
      title={`Importer CSV — ${entityLabel}`}
      size="md"
      footer={
        <div className="flex justify-end gap-2">
          <Button variant="ghost" onClick={handleClose} disabled={importing}>
            Annuler
          </Button>
          <Button onClick={() => void handleImport()} disabled={importing || !csvText.trim()}>
            {importing ? "Import…" : "Importer"}
          </Button>
        </div>
      }
    >
      <div className="space-y-4">
        <Text variant="muted" className="text-sm">
          Loggy accepte uniquement les fichiers CSV (séparateur <code>;</code>). La première ligne
          doit contenir les noms des champs de l&apos;entité.
        </Text>
        <label className="flex cursor-pointer flex-col items-center gap-3 rounded-xl border border-dashed border-border bg-surface-elevated/40 px-6 py-8 transition-colors hover:border-secondary/50">
          <Upload className="h-8 w-8 text-secondary" />
          <span className="text-sm text-foreground">
            {fileName ? fileName : "Glissez ou cliquez pour choisir un CSV"}
          </span>
          <input
            type="file"
            accept=".csv,text/csv"
            className="sr-only"
            onChange={(e) => onFile(e.target.files?.[0])}
          />
        </label>
      </div>
    </Modal>
  );
}
