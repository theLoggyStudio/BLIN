import { useEffect, useRef, useState } from "react";
import { Button } from "@/items/Button";
import type { PdfExportProgress } from "@/types/pdfExportProgress";

function formatEta(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return "—";
  if (seconds < 8) return "quelques secondes";
  const totalSec = Math.round(seconds);
  const m = Math.floor(totalSec / 60);
  const s = totalSec % 60;
  if (m === 0) return `${s} s`;
  if (s === 0) return `${m} min`;
  return `${m} min ${s} s`;
}

interface PdfExportProgressPanelProps {
  progress: PdfExportProgress | null;
  onCancel: () => void;
  cancelling?: boolean;
}

/** Barre de progression export PDF — compteur, estimation temps restant, annulation. */
export function PdfExportProgressPanel({
  progress,
  onCancel,
  cancelling = false,
}: PdfExportProgressPanelProps) {
  const startedAt = useRef(Date.now());
  const [eta, setEta] = useState<string>("—");

  useEffect(() => {
    if (!progress || progress.done) {
      setEta("—");
      return;
    }
    startedAt.current = Date.now();
  }, [progress?.phase, progress?.done]);

  useEffect(() => {
    if (!progress || progress.done || progress.current <= 0) {
      setEta("—");
      return;
    }

    const tick = () => {
      const elapsed = (Date.now() - startedAt.current) / 1000;
      const remaining = progress.total - progress.current;
      if (remaining <= 0) {
        setEta("quelques secondes");
        return;
      }
      const perUnit = elapsed / progress.current;
      setEta(formatEta(perUnit * remaining));
    };

    tick();
    const id = window.setInterval(tick, 400);
    return () => window.clearInterval(id);
  }, [progress]);

  const current = progress?.current ?? 0;
  const total = Math.max(progress?.total ?? 1, 1);
  const pct = progress?.done
    ? 100
    : Math.min(100, Math.round((current / total) * 100));
  const label = progress?.label ?? "Génération du PDF…";
  const remaining = Math.max(0, total - current);

  return (
    <div className="flex min-h-[min(40dvh,20rem)] flex-col items-center justify-center gap-6 px-2 py-8">
      <div
        className="w-full max-w-md rounded-xl border border-border bg-surface-elevated/40 p-5"
        role="progressbar"
        aria-valuenow={pct}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-label={label}
      >
        <div className="mb-3 flex items-start justify-between gap-3">
          <div className="min-w-0">
            <p className="text-sm font-medium text-foreground">{label}</p>
            {progress?.detail && (
              <p className="mt-1 text-xs text-muted">{progress.detail}</p>
            )}
          </div>
          <span className="shrink-0 font-mono text-xs text-secondary">
            {progress?.done ? "Terminé" : `${current} / ${total}`}
          </span>
        </div>

        <div className="h-2.5 overflow-hidden rounded-full bg-surface-elevated">
          <div
            className="h-full rounded-full bg-secondary transition-[width] duration-300 ease-out"
            style={{ width: `${pct}%` }}
          />
        </div>

        <div className="mt-3 flex flex-wrap items-center justify-between gap-2 text-xs text-muted">
          <span>
            {progress?.done
              ? "Fichier enregistré."
              : remaining > 0
                ? `${remaining} étape(s) restante(s)`
                : "Finalisation…"}
          </span>
          {!progress?.done && current > 0 && (
            <span>Temps restant estimé : {eta}</span>
          )}
        </div>
      </div>

      <Button
        type="button"
        variant="ghost"
        size="sm"
        onClick={onCancel}
        disabled={cancelling || progress?.done}
      >
        {cancelling ? "Annulation…" : "Annuler"}
      </Button>
    </div>
  );
}
