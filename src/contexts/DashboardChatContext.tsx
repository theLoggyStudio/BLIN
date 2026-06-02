import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from "react";

interface DashboardChatContextValue {
  conversationId: string | null;
  setConversationId: (id: string | null) => void;
}

const DashboardChatContext = createContext<DashboardChatContextValue | null>(null);

export function DashboardChatProvider({ children }: { children: ReactNode }) {
  const [conversationId, setConversationId] = useState<string | null>(null);
  const value = useMemo(
    () => ({ conversationId, setConversationId }),
    [conversationId],
  );
  return (
    <DashboardChatContext.Provider value={value}>{children}</DashboardChatContext.Provider>
  );
}

export function useDashboardChat() {
  const ctx = useContext(DashboardChatContext);
  if (!ctx) {
    throw new Error("useDashboardChat doit être utilisé dans DashboardChatProvider");
  }
  return ctx;
}

/** Accès optionnel (sidebar hors provider strict). */
export function useDashboardChatOptional() {
  return useContext(DashboardChatContext);
}
