import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { ENTITY_REGISTRY_SYNCED_EVENT } from "@/constants/events";
import { applyAppBranding } from "@/lib/applyBranding";
import { useAuth } from "@/hooks/useAuth";

export const DEFAULT_BRAND_TITLE = "Blin";
export const DEFAULT_BRAND_SLOGAN = "Gestion par entités";
const DEFAULT_LOGO = "/logo.png";

function logoSrcFromData(logo?: string): string {
  if (!logo?.trim()) return DEFAULT_LOGO;
  return logo.startsWith("data:") ? logo : `data:image/png;base64,${logo}`;
}

interface EntityBrandingContextValue {
  title: string;
  slogan: string;
  logoSrc: string;
}

const EntityBrandingContext = createContext<EntityBrandingContextValue | null>(null);

interface BrandingGetResponse {
  ecosysteme?: string;
  slogan?: string;
  logo?: string;
}

export function EntityBrandingProvider({ children }: { children: ReactNode }) {
  const { user } = useAuth();
  const [title, setTitle] = useState(DEFAULT_BRAND_TITLE);
  const [slogan, setSlogan] = useState(DEFAULT_BRAND_SLOGAN);
  const [logoSrc, setLogoSrc] = useState(DEFAULT_LOGO);

  const refresh = useCallback(async () => {
    try {
      const res = await invoke<BrandingGetResponse>("entity_branding_get");
      setTitle(res.ecosysteme?.trim() || DEFAULT_BRAND_TITLE);
      setSlogan(res.slogan?.trim() || DEFAULT_BRAND_SLOGAN);
      setLogoSrc(logoSrcFromData(res.logo));
    } catch {
      setTitle(DEFAULT_BRAND_TITLE);
      setSlogan(DEFAULT_BRAND_SLOGAN);
      setLogoSrc(DEFAULT_LOGO);
    }
  }, []);

  useEffect(() => {
    void refresh();
    const onSync = () => void refresh();
    window.addEventListener(ENTITY_REGISTRY_SYNCED_EVENT, onSync);
    return () => window.removeEventListener(ENTITY_REGISTRY_SYNCED_EVENT, onSync);
  }, [refresh, user?.id]);

  useEffect(() => {
    void applyAppBranding({ title, slogan, logoSrc });
  }, [title, slogan, logoSrc]);

  const value = useMemo(() => ({ title, slogan, logoSrc }), [title, slogan, logoSrc]);

  return (
    <EntityBrandingContext.Provider value={value}>{children}</EntityBrandingContext.Provider>
  );
}

export function useEntityBranding(): EntityBrandingContextValue {
  const ctx = useContext(EntityBrandingContext);
  if (!ctx) {
    throw new Error("useEntityBranding doit être utilisé dans EntityBrandingProvider");
  }
  return ctx;
}
