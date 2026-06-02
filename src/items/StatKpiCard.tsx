import { Text } from "@/items/Text";
import { cn } from "@/lib/utils";

interface StatKpiCardProps {
  label: string;
  value: string | number;
  hint?: string;
  className?: string;
}

/** Carte indicateur — synthèse statistique (inspiré LoggPatient). */
export function StatKpiCard({ label, value, hint, className }: StatKpiCardProps) {
  return (
    <div className={cn("card-panel rounded-xl border border-border p-4", className)}>
      <Text variant="muted" className="uppercase tracking-wide text-[10px]">
        {label}
      </Text>
      <p className="mt-2 text-2xl font-semibold text-foreground tabular-nums">{value}</p>
      {hint && <Text variant="muted" className="mt-1">{hint}</Text>}
    </div>
  );
}
