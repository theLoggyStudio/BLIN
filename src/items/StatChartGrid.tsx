import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

interface StatChartGridProps {
  children: ReactNode;
  columns?: 1 | 2 | 3;
  className?: string;
}

/** Grille responsive pour cartes KPI et graphiques. */
export function StatChartGrid({ children, columns = 2, className }: StatChartGridProps) {
  const cols =
    columns === 1
      ? "grid-cols-1"
      : columns === 3
        ? "grid-cols-1 lg:grid-cols-3"
        : "grid-cols-1 md:grid-cols-2";
  return <div className={cn("grid gap-4", cols, className)}>{children}</div>;
}
