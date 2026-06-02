/** Thème UI — variables CSS alignées sur `src/index.css` (@theme). */

export interface UiThemeColors {
  primary: string;
  primaryHover: string;
  secondary: string;
  secondaryHover: string;
  accent: string;
  accentHover: string;
  emerald: string;
  emeraldHover: string;
  background: string;
  surface: string;
  surfaceElevated: string;
  card: string;
  cardPanel: string;
  teal: string;
  tealHover: string;
  border: string;
  muted: string;
  foreground: string;
}

export const DEFAULT_UI_THEME: UiThemeColors = {
  primary: "#dc2626",
  primaryHover: "#b91c1c",
  secondary: "#06b6d4",
  secondaryHover: "#0891b2",
  accent: "#2563eb",
  accentHover: "#1d4ed8",
  emerald: "#10b981",
  emeraldHover: "#059669",
  background: "#0a0a0a",
  surface: "#000000",
  surfaceElevated: "#1a1a1a",
  card: "#1e1e1e",
  cardPanel: "#1e1e1e",
  teal: "#4db6ac",
  tealHover: "#26a69a",
  border: "#333333",
  muted: "#a3a3a3",
  foreground: "#fafafa",
};

const STORAGE_KEY = "blin:ui-theme";

const CSS_VAR_MAP: { key: keyof UiThemeColors; var: string }[] = [
  { key: "primary", var: "--color-primary" },
  { key: "primaryHover", var: "--color-primary-hover" },
  { key: "secondary", var: "--color-secondary" },
  { key: "secondaryHover", var: "--color-secondary-hover" },
  { key: "accent", var: "--color-accent" },
  { key: "accentHover", var: "--color-accent-hover" },
  { key: "emerald", var: "--color-emerald" },
  { key: "emeraldHover", var: "--color-emerald-hover" },
  { key: "background", var: "--color-background" },
  { key: "surface", var: "--color-surface" },
  { key: "surfaceElevated", var: "--color-surface-elevated" },
  { key: "card", var: "--color-card" },
  { key: "cardPanel", var: "--color-card-panel" },
  { key: "teal", var: "--color-teal" },
  { key: "tealHover", var: "--color-teal-hover" },
  { key: "border", var: "--color-border" },
  { key: "muted", var: "--color-muted" },
  { key: "foreground", var: "--color-foreground" },
];

export const UI_THEME_FIELDS: {
  key: keyof UiThemeColors;
  label: string;
  group: "accents" | "fond" | "sidebar" | "texte";
}[] = [
  { key: "primary", label: "Primaire (titres dégradé)", group: "accents" },
  { key: "primaryHover", label: "Primaire — survol", group: "accents" },
  { key: "secondary", label: "Secondaire (focus, liens)", group: "accents" },
  { key: "secondaryHover", label: "Secondaire — survol", group: "accents" },
  { key: "accent", label: "Accent (dégradés)", group: "accents" },
  { key: "accentHover", label: "Accent — survol", group: "accents" },
  { key: "emerald", label: "Succès / validation", group: "accents" },
  { key: "emeraldHover", label: "Succès — survol", group: "accents" },
  { key: "background", label: "Fond principal", group: "fond" },
  { key: "surface", label: "Sidebar", group: "fond" },
  { key: "surfaceElevated", label: "Surfaces surélevées", group: "fond" },
  { key: "card", label: "Cartes", group: "fond" },
  { key: "cardPanel", label: "Panneaux / modals", group: "fond" },
  { key: "border", label: "Bordures", group: "fond" },
  { key: "teal", label: "Pilules menu (teal)", group: "sidebar" },
  { key: "tealHover", label: "Pilules menu — survol", group: "sidebar" },
  { key: "foreground", label: "Texte principal", group: "texte" },
  { key: "muted", label: "Texte secondaire", group: "texte" },
];

export interface UiThemePreset {
  id: string;
  label: string;
  colors: UiThemeColors;
}

export const UI_THEME_PRESETS: UiThemePreset[] = [
  { id: "blin", label: "Blin (défaut)", colors: DEFAULT_UI_THEME },
  {
    id: "ocean",
    label: "Océan",
    colors: {
      ...DEFAULT_UI_THEME,
      primary: "#0ea5e9",
      primaryHover: "#0284c7",
      secondary: "#22d3ee",
      secondaryHover: "#06b6d4",
      accent: "#6366f1",
      accentHover: "#4f46e5",
      teal: "#2dd4bf",
      tealHover: "#14b8a6",
    },
  },
  {
    id: "forest",
    label: "Forêt",
    colors: {
      ...DEFAULT_UI_THEME,
      primary: "#16a34a",
      primaryHover: "#15803d",
      secondary: "#84cc16",
      secondaryHover: "#65a30d",
      accent: "#059669",
      accentHover: "#047857",
      teal: "#4ade80",
      tealHover: "#22c55e",
    },
  },
  {
    id: "sunset",
    label: "Coucher de soleil",
    colors: {
      ...DEFAULT_UI_THEME,
      primary: "#f97316",
      primaryHover: "#ea580c",
      secondary: "#f43f5e",
      secondaryHover: "#e11d48",
      accent: "#a855f7",
      accentHover: "#9333ea",
      teal: "#fb923c",
      tealHover: "#f97316",
    },
  },
  {
    id: "violet",
    label: "Violet",
    colors: {
      ...DEFAULT_UI_THEME,
      primary: "#8b5cf6",
      primaryHover: "#7c3aed",
      secondary: "#c084fc",
      secondaryHover: "#a855f7",
      accent: "#ec4899",
      accentHover: "#db2777",
      teal: "#a78bfa",
      tealHover: "#8b5cf6",
    },
  },
];

const HEX_RE = /^#([0-9a-fA-F]{6}|[0-9a-fA-F]{3})$/;

export function normalizeHexColor(raw: string, fallback: string): string {
  const t = raw.trim();
  if (!HEX_RE.test(t)) return fallback;
  if (t.length === 4) {
    const r = t[1];
    const g = t[2];
    const b = t[3];
    return `#${r}${r}${g}${g}${b}${b}`.toLowerCase();
  }
  return t.toLowerCase();
}

function mergeTheme(partial: Partial<UiThemeColors>): UiThemeColors {
  return { ...DEFAULT_UI_THEME, ...partial };
}

export function loadUiTheme(): UiThemeColors {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULT_UI_THEME };
    const parsed = JSON.parse(raw) as Partial<UiThemeColors>;
    const merged = mergeTheme(parsed);
    for (const { key } of CSS_VAR_MAP) {
      merged[key] = normalizeHexColor(merged[key], DEFAULT_UI_THEME[key]);
    }
    return merged;
  } catch {
    return { ...DEFAULT_UI_THEME };
  }
}

export function saveUiTheme(theme: UiThemeColors): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(theme));
  } catch {
    /* quota */
  }
}

export function applyUiTheme(theme: UiThemeColors): void {
  const root = document.documentElement;
  for (const { key, var: cssVar } of CSS_VAR_MAP) {
    root.style.setProperty(cssVar, theme[key]);
  }
}

export function resetUiTheme(): UiThemeColors {
  const theme = { ...DEFAULT_UI_THEME };
  saveUiTheme(theme);
  applyUiTheme(theme);
  return theme;
}
