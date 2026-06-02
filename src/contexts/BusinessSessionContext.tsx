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
import { BUSINESS_SESSION_CHANGED_EVENT, ENTITY_REGISTRY_SYNCED_EVENT } from "@/constants/events";
import type {
  ActiveBusinessSession,
  EntityActiveSessionResponse,
  SessionEntityInfo,
} from "@/types/entity";

interface BusinessSessionContextValue {
  active: ActiveBusinessSession | null;
  sessionEntities: SessionEntityInfo[];
  loading: boolean;
  refresh: (screenKey?: string) => Promise<void>;
  setActive: (entityKey: string, recordId: string) => Promise<void>;
  clearActive: () => Promise<void>;
}

const BusinessSessionContext = createContext<BusinessSessionContextValue | null>(null);

export function BusinessSessionProvider({ children }: { children: ReactNode }) {
  const [active, setActiveState] = useState<ActiveBusinessSession | null>(null);
  const [sessionEntities, setSessionEntities] = useState<SessionEntityInfo[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async (screenKey?: string) => {
    setLoading(true);
    try {
      const res = await invoke<EntityActiveSessionResponse>("entity_active_session_get", {
        payload: { screen_key: screenKey ?? null },
      });
      setActiveState(res.active ?? null);
      setSessionEntities(res.session_entities ?? []);
    } catch {
      setActiveState(null);
      setSessionEntities([]);
    } finally {
      setLoading(false);
    }
  }, []);

  const setActive = useCallback(
    async (entityKey: string, recordId: string) => {
      const session = await invoke<ActiveBusinessSession>("entity_active_session_set", {
        payload: { entity_key: entityKey, record_id: recordId },
      });
      setActiveState(session);
      window.dispatchEvent(new CustomEvent(BUSINESS_SESSION_CHANGED_EVENT));
    },
    [],
  );

  const clearActive = useCallback(async () => {
    await invoke("entity_active_session_clear");
    setActiveState(null);
    window.dispatchEvent(new CustomEvent(BUSINESS_SESSION_CHANGED_EVENT));
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    const onRegistry = () => void refresh();
    window.addEventListener(ENTITY_REGISTRY_SYNCED_EVENT, onRegistry);
    return () => window.removeEventListener(ENTITY_REGISTRY_SYNCED_EVENT, onRegistry);
  }, [refresh]);

  const value = useMemo(
    () => ({
      active,
      sessionEntities,
      loading,
      refresh,
      setActive,
      clearActive,
    }),
    [active, sessionEntities, loading, refresh, setActive, clearActive],
  );

  return (
    <BusinessSessionContext.Provider value={value}>{children}</BusinessSessionContext.Provider>
  );
}

export function useBusinessSession(): BusinessSessionContextValue {
  const ctx = useContext(BusinessSessionContext);
  if (!ctx) {
    throw new Error("useBusinessSession doit être utilisé dans BusinessSessionProvider");
  }
  return ctx;
}

export function useBusinessSessionOptional(): BusinessSessionContextValue | null {
  return useContext(BusinessSessionContext);
}
