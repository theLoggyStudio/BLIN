import { useMemo } from "react";
import { RotateCcw } from "lucide-react";
import { useUiTheme } from "@/contexts/UiThemeContext";
import { Button } from "@/items/Button";
import { Input } from "@/items/Input";
import { Text } from "@/items/Text";
import {
  UI_THEME_FIELDS,
  UI_THEME_PRESETS,
  type UiThemeColors,
} from "@/lib/uiTheme";

const GROUP_LABELS: Record<(typeof UI_THEME_FIELDS)[number]["group"], string> = {
  accents: "Accents & dégradés",
  fond: "Fonds & cartes",
  sidebar: "Menu latéral",
  texte: "Textes",
};

function ColorRow({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (hex: string) => void;
}) {
  return (
    <div className="grid gap-2 sm:grid-cols-[minmax(0,1fr)_auto_7rem] sm:items-center">
      <span className="text-sm text-foreground">{label}</span>
      <input
        type="color"
        value={value}
        aria-label={label}
        className="h-10 w-full max-w-[4.5rem] cursor-pointer rounded-lg border border-border bg-card-panel p-1"
        onChange={(e) => onChange(e.target.value)}
      />
      <Input
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="font-mono text-xs"
        aria-label={`${label} (code hexadécimal)`}
      />
    </div>
  );
}

function ThemePreview({ theme }: { theme: UiThemeColors }) {
  return (
    <div
      className="overflow-hidden rounded-xl border border-border"
      style={{ background: theme.background }}
    >
      <div className="flex gap-0">
        <div
          className="w-24 shrink-0 p-3 space-y-2"
          style={{ background: theme.surface }}
        >
          <div
            className="rounded-full px-3 py-1.5 text-xs font-semibold text-white"
            style={{ background: theme.teal }}
          >
            Menu
          </div>
          <div
            className="rounded-full px-3 py-1 text-[10px] opacity-80"
            style={{ background: `${theme.teal}33`, color: theme.foreground }}
          >
            Inactif
          </div>
        </div>
        <div className="min-w-0 flex-1 p-4 space-y-3">
          <p
            className="text-lg font-bold"
            style={{
              background: `linear-gradient(90deg, ${theme.primary}, ${theme.accent})`,
              WebkitBackgroundClip: "text",
              WebkitTextFillColor: "transparent",
              backgroundClip: "text",
            }}
          >
            Titre écran
          </p>
          <div
            className="rounded-lg border p-3 text-sm"
            style={{
              background: theme.cardPanel,
              borderColor: theme.border,
              color: theme.foreground,
            }}
          >
            <span style={{ color: theme.muted }}>Texte secondaire — </span>
            contenu carte
          </div>
          <div className="flex flex-wrap gap-2">
            <span
              className="rounded-md px-3 py-1 text-xs font-medium text-white"
              style={{ background: theme.secondary }}
            >
              Action
            </span>
            <span
              className="rounded-md px-3 py-1 text-xs font-medium text-white"
              style={{ background: theme.emerald }}
            >
              OK
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}

/** Panneau Paramètres — personnalisation du thème de couleurs. */
export function ThemePanel() {
  const { theme, updateColor, applyPreset, resetToDefault, isCustomized } =
    useUiTheme();

  const groups = useMemo(() => {
    const map = new Map<
      (typeof UI_THEME_FIELDS)[number]["group"],
      typeof UI_THEME_FIELDS
    >();
    for (const f of UI_THEME_FIELDS) {
      const list = map.get(f.group) ?? [];
      list.push(f);
      map.set(f.group, list);
    }
    return map;
  }, []);

  return (
    <div className="space-y-6">
      <Text variant="muted" className="text-sm">
        Personnalisez les couleurs de l&apos;interface (sidebar, cartes, dégradés des
        titres, boutons). Les changements sont appliqués immédiatement et enregistrés sur
        ce poste.
      </Text>

      <div className="sticky top-0 z-10 -mx-1 bg-card px-1 pb-3 pt-1">
        <ThemePreview theme={theme} />
      </div>

      <div>
        <Text variant="label" className="mb-2">
          Modèles prédéfinis
        </Text>
        <div className="flex flex-wrap gap-2">
          {UI_THEME_PRESETS.map((preset) => (
            <Button
              key={preset.id}
              type="button"
              size="sm"
              variant="secondary"
              onClick={() => applyPreset(preset)}
            >
              {preset.label}
            </Button>
          ))}
        </div>
      </div>

      {(["accents", "fond", "sidebar", "texte"] as const).map((group) => {
        const fields = groups.get(group);
        if (!fields?.length) return null;
        return (
          <div
            key={group}
            className="space-y-3 rounded-lg border border-border bg-surface-elevated/30 p-4"
          >
            <Text variant="label">{GROUP_LABELS[group]}</Text>
            <div className="space-y-3">
              {fields.map((f) => (
                <ColorRow
                  key={f.key}
                  label={f.label}
                  value={theme[f.key]}
                  onChange={(hex) => updateColor(f.key, hex)}
                />
              ))}
            </div>
          </div>
        );
      })}

      <div className="flex flex-wrap gap-2 pt-1">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={resetToDefault}
          disabled={!isCustomized}
        >
          <RotateCcw className="h-4 w-4" />
          Réinitialiser Blin
        </Button>
      </div>
    </div>
  );
}
