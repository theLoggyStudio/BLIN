import {
  forwardRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { EntityLineTabs } from "@/items/EntityLineTabs";
import {
  CREATE_LINES_FORM_KEY,
  embedSnapshotToFormUpdates,
  emptyEmbedSnapshot,
  mergeActiveLineSnapshots,
  multilineFormPatch,
  parseParentLignes,
  readEmbedSnapshot,
} from "@/lib/createFormLines";
import type { FieldDef, ScreenRow } from "@/types/screen";

export interface EntityCreateLineFormHandle {
  /** Réaligne ligne 1 + __create_lines__ avant dda_create / dda_update. */
  flushForSubmit: () => ScreenRow;
}

interface EntityCreateLineFormProps {
  entityLabel: string;
  primaryKey?: string;
  allFields: FieldDef[];
  values: ScreenRow;
  onChange: (key: string, value: unknown) => void;
  onBatchChange?: (updates: Record<string, unknown>) => void;
  readOnly?: boolean;
  displayOnly?: boolean;
  children: ReactNode;
}

/**
 * Onglets « Ligne N » au niveau de l'entité mère.
 * Les champs embarqués passent par onChange ; __create_lines__ est resynchronisé à chaque saisie.
 */
export const EntityCreateLineForm = forwardRef<
  EntityCreateLineFormHandle,
  EntityCreateLineFormProps
>(function EntityCreateLineForm(
  {
    entityLabel,
    primaryKey = "id",
    allFields,
    values,
    onChange,
    onBatchChange,
    readOnly,
    displayOnly,
    children,
  },
  ref,
) {
  const recordKey = String(values[primaryKey] ?? "new");
  const [activeIndex, setActiveIndex] = useState(0);
  const [lines, setLines] = useState<Record<string, unknown>[]>(() =>
    parseParentLignes(values, allFields),
  );
  const syncingRef = useRef(false);
  const valuesRef = useRef(values);
  valuesRef.current = values;
  const linesRef = useRef(lines);
  linesRef.current = lines;
  const activeIndexRef = useRef(activeIndex);
  activeIndexRef.current = activeIndex;

  const pushLinesToForm = useCallback(
    (nextLines: Record<string, unknown>[], active: number) => {
      syncingRef.current = true;
      const activeSnap = nextLines[active] ?? emptyEmbedSnapshot(allFields);
      const updates = embedSnapshotToFormUpdates(activeSnap, allFields);
      if (onBatchChange) onBatchChange(updates);
      else {
        for (const [k, v] of Object.entries(updates)) onChange(k, v);
      }
      const extras = nextLines.slice(1);
      onChange(
        CREATE_LINES_FORM_KEY,
        extras.length > 0 ? JSON.stringify(extras) : "",
      );
      queueMicrotask(() => {
        syncingRef.current = false;
      });
    },
    [allFields, onBatchChange, onChange],
  );

  useImperativeHandle(
    ref,
    () => ({
      flushForSubmit: () =>
        multilineFormPatch(
          valuesRef.current,
          allFields,
          linesRef.current,
          activeIndexRef.current,
        ),
    }),
    [allFields],
  );

  useEffect(() => {
    syncingRef.current = true;
    const parsed = parseParentLignes(valuesRef.current, allFields);
    setLines(parsed);
    setActiveIndex(0);
    queueMicrotask(() => {
      syncingRef.current = false;
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps -- values lues via ref au changement d'ID
  }, [recordKey, allFields]);

  /** Chaque onChange de champ embarqué → met à jour lines puis __create_lines__. */
  useEffect(() => {
    if (syncingRef.current) return;

    const snap = readEmbedSnapshot(values, allFields);
    let nextLines: Record<string, unknown>[] = linesRef.current;

    setLines((prev) => {
      nextLines = prev.map((line, i) => (i === activeIndex ? { ...line, ...snap } : line));
      linesRef.current = nextLines;
      return nextLines;
    });

    queueMicrotask(() => {
      if (syncingRef.current) return;
      const currentLines = linesRef.current;
      if (currentLines.length <= 1) {
        const extra = valuesRef.current[CREATE_LINES_FORM_KEY];
        if (extra != null && String(extra).trim() !== "") {
          syncingRef.current = true;
          onChange(CREATE_LINES_FORM_KEY, "");
          queueMicrotask(() => {
            syncingRef.current = false;
          });
        }
        return;
      }
      const merged = mergeActiveLineSnapshots(
        valuesRef.current,
        allFields,
        currentLines,
        activeIndexRef.current,
      );
      const serialized = JSON.stringify(merged.slice(1));
      const currentRaw = valuesRef.current[CREATE_LINES_FORM_KEY];
      const currentSerialized =
        typeof currentRaw === "string"
          ? currentRaw
          : currentRaw != null
            ? JSON.stringify(currentRaw)
            : "";
      if (currentSerialized !== serialized) {
        syncingRef.current = true;
        onChange(CREATE_LINES_FORM_KEY, serialized);
        queueMicrotask(() => {
          syncingRef.current = false;
        });
      }
    });
  }, [values, allFields, activeIndex, onChange]);

  const switchToLine = (targetIndex: number) => {
    if (targetIndex === activeIndex) return;
    setLines((prev) => {
      const currentSnap = readOnly
        ? (prev[activeIndex] ?? emptyEmbedSnapshot(allFields))
        : readEmbedSnapshot(values, allFields);
      const next = prev.map((line, i) => (i === activeIndex ? currentSnap : line));
      pushLinesToForm(next, targetIndex);
      setActiveIndex(targetIndex);
      return next;
    });
  };

  const addLine = () => {
    if (readOnly) return;
    setLines((prev) => {
      const currentSnap = readEmbedSnapshot(values, allFields);
      const next = prev.map((line, i) => (i === activeIndex ? currentSnap : line));
      next.push(emptyEmbedSnapshot(allFields));
      const newIndex = next.length - 1;
      pushLinesToForm(next, newIndex);
      setActiveIndex(newIndex);
      return next;
    });
  };

  const removeLine = (index: number) => {
    if (readOnly) return;
    setLines((prev) => {
      if (prev.length <= 1) return prev;
      const currentSnap = readEmbedSnapshot(values, allFields);
      const withCurrent = prev.map((line, i) => (i === activeIndex ? currentSnap : line));
      const next = withCurrent.filter((_, i) => i !== index);
      let newActive = activeIndex;
      if (index === activeIndex) {
        newActive = Math.min(activeIndex, next.length - 1);
      } else if (index < activeIndex) {
        newActive = activeIndex - 1;
      }
      pushLinesToForm(next, newActive);
      setActiveIndex(newActive);
      return next;
    });
  };

  return (
    <EntityLineTabs
      entityLabel={entityLabel}
      lineCount={lines.length}
      activeIndex={activeIndex}
      onSelect={switchToLine}
      onAdd={readOnly ? undefined : addLine}
      onRemove={readOnly ? undefined : removeLine}
      readOnly={readOnly}
      displayOnly={displayOnly}
    >
      <div key={`mother-line-panel-${activeIndex}`}>{children}</div>
    </EntityLineTabs>
  );
});
