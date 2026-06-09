import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { EntityDefLoggyModal } from "@/items/EntityDefLoggyModal";
import type { EntityDef } from "@/types/entity";

interface EntityDefLoggyModalContextValue {
  openRegistryEntityCreate: (entity: EntityDef) => Promise<boolean>;
}

const EntityDefLoggyModalContext = createContext<EntityDefLoggyModalContextValue | null>(null);

export function EntityDefLoggyModalProvider({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState(false);
  const [initialEntity, setInitialEntity] = useState<EntityDef | null>(null);

  const openRegistryEntityCreate = useCallback(async (entity: EntityDef) => {
    try {
      const access = await invoke<{ allowed: boolean }>("entity_registry_create_access");
      if (!access.allowed) return false;
    } catch {
      return false;
    }
    setInitialEntity({
      ...entity,
      attributs: [...entity.attributs],
      signatory_role_ids: [...(entity.signatory_role_ids ?? [])],
    });
    setOpen(true);
    return true;
  }, []);

  const close = useCallback(() => {
    setOpen(false);
    setInitialEntity(null);
  }, []);

  const value = useMemo(
    () => ({ openRegistryEntityCreate }),
    [openRegistryEntityCreate],
  );

  return (
    <EntityDefLoggyModalContext.Provider value={value}>
      {children}
      <EntityDefLoggyModal open={open} initialEntity={initialEntity} onClose={close} />
    </EntityDefLoggyModalContext.Provider>
  );
}

export function useEntityDefLoggyModal(): EntityDefLoggyModalContextValue {
  const ctx = useContext(EntityDefLoggyModalContext);
  if (!ctx) {
    throw new Error("useEntityDefLoggyModal doit être utilisé dans EntityDefLoggyModalProvider");
  }
  return ctx;
}
