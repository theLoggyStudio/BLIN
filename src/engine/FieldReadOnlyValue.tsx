import { useCallback, useEffect, useState } from "react";
import { formatCellValue } from "@/engine/screenUtils";
import { parseImagesValue } from "@/engine/mediaUtils";
import { fetchRelationLabels } from "@/items/EntityRelationAutocomplete";
import { TableImageCell } from "@/items/TableImageCell";
import type { FieldDef } from "@/types/screen";

interface FieldReadOnlyValueProps {
  field: FieldDef;
  value: unknown;
  screenKey: string;
  excludeRecordId?: string;
}

function parseEntityRefListValue(value: unknown): string[] {
  if (Array.isArray(value)) {
    return value.map((v) => String(v ?? "").trim()).filter(Boolean);
  }
  if (typeof value === "string") {
    const trimmed = value.trim();
    if (!trimmed) return [];
    try {
      const parsed = JSON.parse(trimmed);
      if (Array.isArray(parsed)) {
        return parsed.map((v) => String(v ?? "").trim()).filter(Boolean);
      }
    } catch {
      return trimmed
        .split(",")
        .map((s) => s.trim())
        .filter(Boolean);
    }
  }
  return [];
}

function resolveSelectLabel(field: FieldDef, val: unknown): string {
  const strVal = String(val ?? "");
  if (!strVal) return "—";
  if (field.type === "boolean") {
    return val === true || val === 1 || val === "1" || val === "true" ? "Oui" : "Non";
  }
  if (field.type === "select" && field.options?.length) {
    const opt = field.options.find((o) => o.value === strVal);
    if (opt) return opt.label;
  }
  return formatCellValue(field, val);
}

function EntityRefReadOnlyLabel({
  field,
  value,
  screenKey,
  excludeRecordId,
}: FieldReadOnlyValueProps) {
  const strVal = String(value ?? "").trim();
  const [label, setLabel] = useState(strVal || "—");

  const load = useCallback(async () => {
    if (!strVal) {
      setLabel("—");
      return;
    }
    try {
      const map = await fetchRelationLabels(screenKey, field.key, [strVal], excludeRecordId);
      setLabel(map.get(strVal) ?? strVal);
    } catch {
      setLabel(strVal);
    }
  }, [strVal, screenKey, field.key, excludeRecordId]);

  useEffect(() => {
    void load();
  }, [load]);

  return <>{label}</>;
}

function EntityRefListReadOnly({
  field,
  value,
  screenKey,
  excludeRecordId,
}: FieldReadOnlyValueProps) {
  const ids = parseEntityRefListValue(value);
  const [labels, setLabels] = useState<string[]>([]);

  useEffect(() => {
    if (ids.length === 0) {
      setLabels([]);
      return;
    }
    let cancelled = false;
    void fetchRelationLabels(screenKey, field.key, ids, excludeRecordId)
      .then((map) => {
        if (cancelled) return;
        setLabels(ids.map((id) => map.get(id) ?? id));
      })
      .catch(() => {
        if (!cancelled) setLabels(ids);
      });
    return () => {
      cancelled = true;
    };
  }, [ids.join("|"), screenKey, field.key, excludeRecordId]);

  if (ids.length === 0) return <>—</>;
  return <>{labels.join(", ")}</>;
}

export function FieldReadOnlyValue({
  field,
  value,
  screenKey,
  excludeRecordId,
}: FieldReadOnlyValueProps) {
  if (field.type === "image") {
    const path = value != null ? String(value).trim() : "";
    return (
      <div>
        <dt className="text-xs font-medium uppercase tracking-wide text-muted">{field.label}</dt>
        <dd className="mt-1">
          {path ? (
            <TableImageCell relativePath={path} />
          ) : (
            <span className="text-sm text-foreground">—</span>
          )}
        </dd>
      </div>
    );
  }

  if (field.type === "images") {
    const paths = parseImagesValue(value);
    return (
      <div>
        <dt className="text-xs font-medium uppercase tracking-wide text-muted">{field.label}</dt>
        <dd className="mt-1 text-sm text-foreground">
          {paths.length > 0 ? `${paths.length} photo${paths.length > 1 ? "s" : ""}` : "—"}
        </dd>
      </div>
    );
  }

  if (field.type === "entity_ref") {
    return (
      <div>
        <dt className="text-xs font-medium uppercase tracking-wide text-muted">{field.label}</dt>
        <dd className="mt-0.5 text-sm text-foreground">
          <EntityRefReadOnlyLabel
            field={field}
            value={value}
            screenKey={screenKey}
            excludeRecordId={excludeRecordId}
          />
        </dd>
      </div>
    );
  }

  if (field.type === "entity_ref_list") {
    return (
      <div>
        <dt className="text-xs font-medium uppercase tracking-wide text-muted">{field.label}</dt>
        <dd className="mt-0.5 text-sm text-foreground">
          <EntityRefListReadOnly
            field={field}
            value={value}
            screenKey={screenKey}
            excludeRecordId={excludeRecordId}
          />
        </dd>
      </div>
    );
  }

  return (
    <div>
      <dt className="text-xs font-medium uppercase tracking-wide text-muted">{field.label}</dt>
      <dd className="mt-0.5 whitespace-pre-wrap break-words text-sm text-foreground">
        {resolveSelectLabel(field, value)}
      </dd>
    </div>
  );
}
