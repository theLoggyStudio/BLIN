import type { MouseEvent } from "react";
import { formatDateFr, formatDateTimeFr, formatJjmmaaaaFr, formatTimeFr, parseToDate } from "@/lib/formatDateTime";
import { cn } from "@/lib/utils";
import type { RelationSelectOption } from "@/types/entity";

function parseDetailSegments(detail: string): { label: string; value: string }[] {
  return detail
    .split(" · ")
    .map((part) => {
      const sep = part.indexOf(" : ");
      if (sep === -1) {
        return { label: "", value: part.trim() };
      }
      return {
        label: part.slice(0, sep).trim(),
        value: part.slice(sep + 3).trim(),
      };
    })
    .filter((s) => s.value.length > 0);
}

function formatSegmentValue(value: string, label: string): string {
  const labelLc = label.toLowerCase();
  if (/^\d+(\.\d+)?$/.test(value)) {
    const n = Number(value);
    if (Number.isFinite(n) && Number.isInteger(n)) return String(Math.trunc(n));
    if (Number.isFinite(n) && n === Math.trunc(n)) return String(Math.trunc(n));
  }
  if (labelLc.includes("jjmmaaaa") || labelLc.endsWith("— date")) {
    return formatJjmmaaaaFr(value);
  }
  if (parseToDate(value)) {
    if (labelLc.includes("heure") && !labelLc.includes("date")) return formatTimeFr(value);
    if (labelLc.includes("date") && !labelLc.includes("heure")) return formatDateFr(value);
    return formatDateTimeFr(value);
  }
  return value;
}

/** Attributs d'un résultat liaison entité — titre + lignes pleine largeur. */
export function RelationOptionLine({ option }: { option: RelationSelectOption }) {
  const detail = option.detail?.trim() || option.label;
  const title = option.label.trim() || option.value;
  const segments = parseDetailSegments(detail).filter(
    (seg) => !(seg.label.toLowerCase().includes("référence") && seg.value === title),
  );

  return (
    <div className="w-full min-w-0">
      <div className="mb-2 border-b border-border/50 pb-2">
        <span className="block truncate text-sm font-semibold leading-snug text-foreground">
          {title}
        </span>
      </div>
      {segments.length > 0 && (
        <dl className="w-full overflow-hidden rounded-md border border-border/40">
          {segments.map((seg, i) => (
            <div
              key={`${seg.label}-${i}`}
              className={cn(
                "grid w-full grid-cols-1 items-start gap-x-3 gap-y-0.5 px-2.5 py-2 sm:grid-cols-[minmax(7rem,36%)_1fr]",
                i % 2 === 0 ? "bg-background/40" : "bg-surface-elevated/35",
              )}
            >
              {seg.label ? (
                <dt className="text-[0.65rem] font-semibold uppercase leading-snug tracking-wide text-muted">
                  {seg.label}
                </dt>
              ) : (
                <dt className="sr-only">Attribut</dt>
              )}
              <dd className="min-w-0 text-xs leading-snug text-foreground/95 break-words">
                <small>{formatSegmentValue(seg.value, seg.label)}</small>
              </dd>
            </div>
          ))}
        </dl>
      )}
    </div>
  );
}

interface RelationOptionRowProps {
  option: RelationSelectOption;
  active?: boolean;
  selected?: boolean;
  className?: string;
  onClick?: () => void;
  onMouseDown?: (e: MouseEvent<HTMLButtonElement>) => void;
  onMouseEnter?: () => void;
}

/** Ligne cliquable pleine largeur pour un résultat liaison entité. */
export function RelationOptionRow({
  option,
  active,
  selected,
  className,
  onClick,
  onMouseDown,
  onMouseEnter,
}: RelationOptionRowProps) {
  return (
    <button
      type="button"
      className={cn(
        "flex w-full min-w-0 border-b border-border/50 px-3 py-3 text-left transition-colors last:border-b-0 sm:px-4",
        active ? "bg-surface-elevated" : "hover:bg-surface-elevated/70",
        selected && "ring-1 ring-inset ring-secondary/40",
        className,
      )}
      onClick={onClick}
      onMouseDown={onMouseDown}
      onMouseEnter={onMouseEnter}
    >
      <RelationOptionLine option={option} />
    </button>
  );
}
