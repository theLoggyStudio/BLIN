import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Input } from "@/items/Input";
import {
  matriculeDisplayFromRow,
  matriculeLocalPreview,
  matriculePreviewPatchForField,
} from "@/lib/formatMatricule";
import type { MatriculeDef } from "@/types/entity";
import type { FieldDef, ScreenRow } from "@/types/screen";

interface MatriculeFieldProps {
  field: FieldDef;
  values: ScreenRow;
  screenKey: string;
  hint?: string;
  onBatchChange?: (updates: Record<string, unknown>) => void;
}

/** Champ matricule en lecture seule — affiche la valeur future (aperçu backend). */
export function MatriculeField({
  field,
  values,
  screenKey,
  hint,
  onBatchChange,
}: MatriculeFieldProps) {
  const [previewPatch, setPreviewPatch] = useState<Record<string, unknown>>({});
  const [catalogBase, setCatalogBase] = useState<string | undefined>();

  const mergedValues = useMemo(
    () => ({ ...values, ...previewPatch }),
    [values, previewPatch],
  );

  const display = useMemo(() => {
    const fromRow = matriculeDisplayFromRow(mergedValues, field.key);
    if (fromRow) return fromRow;
    const base = field.form?.matriculeBase?.trim() || catalogBase?.trim();
    if (base) return matriculeLocalPreview(base);
    return "";
  }, [mergedValues, field.key, field.form?.matriculeBase, catalogBase]);

  useEffect(() => {
    if (field.form?.matriculeBase?.trim() || matriculeDisplayFromRow(values, field.key)) {
      return;
    }
    let cancelled = false;
    void invoke<MatriculeDef[]>("entity_matricule_registry_list")
      .then((list) => {
        if (cancelled || list.length === 0) return;
        const base = list.length === 1 ? list[0].base : list[0]?.base;
        if (base?.trim()) setCatalogBase(base.trim());
      })
      .catch(() => {
        /* catalogue optionnel */
      });
    return () => {
      cancelled = true;
    };
  }, [field.form?.matriculeBase, field.key, values]);

  useEffect(() => {
    if (display) return;
    let cancelled = false;
    void invoke<ScreenRow>("entity_compteur_preview", {
      payload: { entity_key: screenKey },
    })
      .then((preview) => {
        if (cancelled || !preview) return;
        const patch = matriculePreviewPatchForField(preview, field.key);
        if (Object.keys(patch).length === 0) return;
        if (onBatchChange) {
          onBatchChange(patch);
        } else {
          setPreviewPatch(patch);
        }
      })
      .catch(() => {
        /* aperçu optionnel */
      });
    return () => {
      cancelled = true;
    };
  }, [display, screenKey, field.key, onBatchChange]);

  return (
    <Input
      label={field.label}
      disabled
      readOnly
      value={display}
      hint={hint ?? field.form?.placeholder}
    />
  );
}
