import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import {
  applyUiTheme,
  DEFAULT_UI_THEME,
  loadUiTheme,
  normalizeHexColor,
  resetUiTheme,
  saveUiTheme,
  type UiThemeColors,
  type UiThemePreset,
} from "@/lib/uiTheme";

interface UiThemeContextValue {
  theme: UiThemeColors;
  setTheme: (next: UiThemeColors) => void;
  updateColor: (key: keyof UiThemeColors, value: string) => void;
  applyPreset: (preset: UiThemePreset) => void;
  resetToDefault: () => void;
  isCustomized: boolean;
}

const UiThemeContext = createContext<UiThemeContextValue | null>(null);

function themesEqual(a: UiThemeColors, b: UiThemeColors): boolean {
  return (Object.keys(DEFAULT_UI_THEME) as (keyof UiThemeColors)[]).every(
    (k) => a[k] === b[k],
  );
}

export function UiThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setThemeState] = useState<UiThemeColors>(() => loadUiTheme());

  const commit = useCallback((next: UiThemeColors) => {
    setThemeState(next);
    applyUiTheme(next);
    saveUiTheme(next);
  }, []);

  const updateColor = useCallback((key: keyof UiThemeColors, value: string) => {
    setThemeState((prev) => {
      const next = {
        ...prev,
        [key]: normalizeHexColor(value, prev[key]),
      };
      applyUiTheme(next);
      saveUiTheme(next);
      return next;
    });
  }, []);

  const applyPreset = useCallback(
    (preset: UiThemePreset) => {
      commit({ ...preset.colors });
    },
    [commit],
  );

  const resetToDefault = useCallback(() => {
    commit(resetUiTheme());
  }, [commit]);

  const value = useMemo(
    (): UiThemeContextValue => ({
      theme,
      setTheme: commit,
      updateColor,
      applyPreset,
      resetToDefault,
      isCustomized: !themesEqual(theme, DEFAULT_UI_THEME),
    }),
    [theme, commit, updateColor, applyPreset, resetToDefault],
  );

  return (
    <UiThemeContext.Provider value={value}>{children}</UiThemeContext.Provider>
  );
}

export function useUiTheme(): UiThemeContextValue {
  const ctx = useContext(UiThemeContext);
  if (!ctx) {
    throw new Error("useUiTheme doit être utilisé dans UiThemeProvider");
  }
  return ctx;
}
