import { Input } from "@/items/Input";
import { Select } from "@/items/Select";
import { FieldMessages } from "@/items/FieldMessages";
import type { FieldDef, ValidationIssue } from "@/types/screen";

interface FilterBarProps {
  fields: FieldDef[];
  values: Record<string, string>;
  onChange: (key: string, value: string) => void;
  fieldErrors?: Record<string, ValidationIssue>;
  fieldWarnings?: Record<string, ValidationIssue>;
}

export function FilterBar({
  fields,
  values,
  onChange,
  fieldErrors = {},
  fieldWarnings = {},
}: FilterBarProps) {
  if (fields.length === 0) return null;

  return (
    <div className="flex flex-wrap gap-3 p-4 rounded-xl border border-border bg-surface">
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
  );
}
