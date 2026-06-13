import { useMemo } from "react";
import { CollapsiblePanel } from "@/items/CollapsiblePanel";
import { Input } from "@/items/Input";
import { Select } from "@/items/Select";
import { FieldMessages } from "@/items/Alert";
import type { FieldDef, ValidationIssue } from "@/types/screen";

interface FilterBarProps {
  fields: FieldDef[];
  values: Record<string, string>;
  onChange: (key: string, value: string) => void;
  fieldErrors?: Record<string, ValidationIssue>;
  fieldWarnings?: Record<string, ValidationIssue>;
  /** Panneau ouvert au chargement (défaut : replié). */
  defaultOpen?: boolean;
}

export function FilterBar({
  fields,
  values,
  onChange,
  fieldErrors = {},
  fieldWarnings = {},
  defaultOpen = false,
}: FilterBarProps) {
  const activeCount = useMemo(
    () => fields.filter((f) => (values[f.key] ?? "").trim().length > 0).length,
    [fields, values],
  );

  if (fields.length === 0) return null;

  const subtitle =
    activeCount > 0
      ? `${activeCount} filtre${activeCount > 1 ? "s" : ""} actif${activeCount > 1 ? "s" : ""}`
      : `${fields.length} champ${fields.length > 1 ? "s" : ""} disponible${fields.length > 1 ? "s" : ""}`;

  return (
    <CollapsiblePanel title="Filtres" subtitle={subtitle} defaultOpen={defaultOpen}>
      <div className="flex flex-wrap gap-3">
        {fields.map((field) => {
          const v = values[field.key] ?? "";
          const err = fieldErrors[field.key];
          const warn = fieldWarnings[field.key];
          if (field.type === "select" && field.options?.length) {
            return (
              <div key={field.key} className="min-w-[160px] flex-1">
                <Select
                  label={field.label}
                  value={v}
                  placeholder="Tous"
                  error={err?.message}
                  options={[{ value: "", label: "Tous" }, ...field.options]}
                  onChange={(e) => onChange(field.key, e.target.value)}
                />
                <FieldMessages error={err} warning={warn} />
              </div>
            );
          }
          return (
            <div key={field.key} className="min-w-[180px] flex-1">
              <Input
                label={field.label}
                value={v}
                placeholder="Filtrer…"
                error={err?.message}
                onChange={(e) => onChange(field.key, e.target.value)}
              />
              <FieldMessages error={err} warning={warn} />
            </div>
          );
        })}
      </div>
    </CollapsiblePanel>
  );
}
