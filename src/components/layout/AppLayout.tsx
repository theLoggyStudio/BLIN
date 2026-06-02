import { useCallback, useEffect, useState } from "react";
import { Outlet } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import {
  ChevronLeft,
  ChevronRight,
  LayoutDashboard,
  ListTodo,
  LogOut,
  Package,
  Settings,
} from "lucide-react";
import { useAuth } from "@/hooks/useAuth";
import { useEntityBranding } from "@/hooks/useEntityBranding";
import { Guard } from "@/components/Guard";
import { Button } from "@/components/ui/Button";
import { MenuItem } from "@/items/Menu";
import { BusinessSessionBar } from "@/items/BusinessSessionBar";
import { SidebarSessionsPanel } from "@/items/SidebarSessionsPanel";
import { useDashboardChatOptional } from "@/contexts/DashboardChatContext";
import { useTachesModal } from "@/contexts/TachesModalContext";
import { useStockModal } from "@/contexts/StockModalContext";
import { ENTITY_REGISTRY_SYNCED_EVENT } from "@/constants/events";
import { cn } from "@/lib/utils";

export function AppLayout() {
  const { user, logout } = useAuth();
  const { title, slogan, logoSrc } = useEntityBranding();
  const [collapsed, setCollapsed] = useState(false);
  const dashboardChat = useDashboardChatOptional();
  const { openTaches } = useTachesModal();
  const { openStock } = useStockModal();
  const [stockEnabled, setStockEnabled] = useState(false);

  const refreshStockStatus = useCallback(() => {
    void invoke<{ enabled: boolean }>("entity_stock_status")
      .then((s) => setStockEnabled(s.enabled))
      .catch(() => setStockEnabled(false));
  }, []);

  useEffect(() => {
    refreshStockStatus();
    window.addEventListener(ENTITY_REGISTRY_SYNCED_EVENT, refreshStockStatus);
    return () =>
      window.removeEventListener(ENTITY_REGISTRY_SYNCED_EVENT, refreshStockStatus);
  }, [refreshStockStatus]);

  return (
    <div className="app-shell">
      <aside
        className={cn(
          "flex h-svh max-h-svh shrink-0 flex-col overflow-hidden border-r border-border bg-surface transition-all duration-200",
          collapsed ? "w-16" : "w-64",
        )}
      >
        <div className="flex shrink-0 items-center justify-between border-b border-border p-4">
          {!collapsed ? (
            <div className="flex min-w-0 items-center gap-3">
              <img src={logoSrc} alt={title} className="h-10 w-10 shrink-0 object-contain" />
              <div className="min-w-0">
                <p className="truncate text-lg font-bold leading-tight text-gradient-brand">{title}</p>
                <p className="truncate text-xs text-muted">{slogan}</p>
              </div>
            </div>
          ) : (
            <img src={logoSrc} alt={title} className="mx-auto h-8 w-8 object-contain" />
          )}
          <button
            type="button"
            onClick={() => setCollapsed((c) => !c)}
            className="rounded-lg p-1.5 text-muted hover:bg-surface-elevated hover:text-foreground"
            aria-label={collapsed ? "Déplier le menu" : "Replier le menu"}
          >
            {collapsed ? <ChevronRight className="h-4 w-4" /> : <ChevronLeft className="h-4 w-4" />}
          </button>
        </div>

        <nav className="flex min-h-0 flex-1 flex-col gap-2 overflow-hidden p-3">
          <div className="shrink-0 space-y-2">
            <MenuItem to="/" label="Tableau de bord" icon={LayoutDashboard} end collapsed={collapsed} />
            <Guard privilege="tache:voir">
              <button
                type="button"
                title={collapsed ? "Tâches" : undefined}
                onClick={() => openTaches()}
                className={cn("nav-app-btn w-full", collapsed && "justify-center px-2")}
              >
                <ListTodo className="h-4 w-4 shrink-0" />
                {!collapsed && <span className="truncate">Tâches</span>}
              </button>
            </Guard>
            {stockEnabled && (
              <Guard privilege="stock:voir">
                <button
                  type="button"
                  title={collapsed ? "Stock" : undefined}
                  onClick={() => openStock()}
                  className={cn("nav-app-btn w-full", collapsed && "justify-center px-2")}
                >
                  <Package className="h-4 w-4 shrink-0" />
                  {!collapsed && <span className="truncate">Stock</span>}
                </button>
              </Guard>
            )}
            <MenuItem to="/parametres" label="Paramètres" icon={Settings} collapsed={collapsed} />
            <BusinessSessionBar collapsed={collapsed} />
          </div>
          <SidebarSessionsPanel
            collapsed={collapsed}
            activeConversationId={dashboardChat?.conversationId ?? null}
          />
        </nav>

        <div className="shrink-0 space-y-3 border-t border-border p-4">
          {!collapsed && (
            <div className="px-1">
              <p className="truncate text-sm font-medium">{user?.nom}</p>
              <p className="truncate text-xs text-muted">{user?.role}</p>
            </div>
          )}
          <Button
            variant="ghost"
            size="sm"
            className={cn("w-full", collapsed ? "justify-center px-2" : "justify-start")}
            onClick={() => void logout()}
            title="Déconnexion"
          >
            <LogOut className="h-4 w-4" />
            {!collapsed && "Déconnexion"}
          </Button>
        </div>
      </aside>

      <main className="app-main">
        <Outlet />
      </main>
    </div>
  );
}
