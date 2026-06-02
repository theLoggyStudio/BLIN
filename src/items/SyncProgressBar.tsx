import type { EntitySyncProgress } from "@/types/syncProgress";

interface SyncProgressBarProps {
  progress: EntitySyncProgress | null;
  active?: boolean;
}

/** Barre de progression pendant la chaîne de triggers (séquentielle). */
export function SyncProgressBar({ progress, active }: SyncProgressBarProps) {
  if (!active && !progress) return null;

  const current = progress?.current ?? 0;
  const total = Math.max(progress?.total ?? 1, 1);
  const pct = progress?.done ? 100 : Math.min(100, Math.round((current / total) * 100));
  const label = progress?.label ?? "Synchronisation des entités…";

  return (
    <div
      className="rounded-lg border border-border bg-card px-4 py-3"
      role="progressbar"
      aria-valuenow={pct}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-label={label}
    >
      <div className="mb-2 flex items-center justify-between gap-2 text-sm">
        <span className="text-foreground">{label}</span>
        <span className="shrink-0 font-mono text-xs text-muted">
          {progress?.done ? "Terminé" : `${current} / ${total}`}
        </span>
      </div>
      <div className="h-2 overflow-hidden rounded-full bg-surface-elevated">
        <div
          className="h-full rounded-full bg-teal transition-[width] duration-300 ease-out"
          style={{ width: `${pct}%` }}
        />
      </div>
      {!progress?.done && (
        <p className="mt-2 text-xs text-muted">
          Exécution séquentielle des triggers (schéma, privilèges, validations, mémoire IA…).
        </p>
      )}
    </div>
  );
}
