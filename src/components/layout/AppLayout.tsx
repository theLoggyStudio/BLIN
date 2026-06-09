import { useCallback, useEffect, useState } from "react";
import { NavLink, Outlet, useLocation } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import type { LucideIcon } from "lucide-react";
import {
  ChevronLeft,
  ChevronRight,
  LayoutDashboard,
  ListTodo,
  LogOut,
  Menu,
  Package,
  Settings,
  X,
} from "lucide-react";
import { useAuth } from "@/hooks/useAuth";
import { useIsMobile } from "@/hooks/useMediaQuery";
import { useTaskReminders } from "@/hooks/useTaskReminders";
import { useEntityBranding } from "@/hooks/useEntityBranding";
import { Guard } from "@/components/Guard";
import { Button } from "@/components/ui/Button";
import { MenuItem } from "@/items/Menu";
import { SidebarWindowsPanel } from "@/items/SidebarWindowsPanel";
import { AiStartupHost } from "@/items/AiStartupHost";
import { useTachesModal } from "@/contexts/TachesModalContext";
import { useStockModal } from "@/contexts/StockModalContext";
import { ENTITY_REGISTRY_SYNCED_EVENT } from "@/constants/events";
import { cn } from "@/lib/utils";

export function AppLayout() {
  const { user, logout } = useAuth();
  const { title, slogan, logoSrc } = useEntityBranding();
  const location = useLocation();
  const isMobile = useIsMobile();
  const [collapsed, setCollapsed] = useState(false);
  const [mobileNavOpen, setMobileNavOpen] = useState(false);
  const { openTaches } = useTachesModal();
  const { openStock } = useStockModal();
  const [stockEnabled, setStockEnabled] = useState(false);

  useTaskReminders();

  useEffect(() => {
    setMobileNavOpen(false);
  }, [location.pathname]);

  useEffect(() => {
    if (!isMobile) setMobileNavOpen(false);
  }, [isMobile]);

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

  const sidebarCollapsed = isMobile ? false : collapsed;

  return (
    <div className="app-shell">
      <AiStartupHost />

      {isMobile && mobileNavOpen && (
        <button
          type="button"
          className="fixed inset-0 z-40 bg-black/60 md:hidden"
          aria-label="Fermer le menu"
          onClick={() => setMobileNavOpen(false)}
        />
      )}

      <aside
        className={cn(
          "flex h-svh max-h-svh shrink-0 flex-col overflow-hidden border-r border-border bg-surface transition-all duration-200",
          isMobile
            ? cn(
                "fixed inset-y-0 left-0 z-50 w-[min(18rem,88vw)] shadow-2xl",
                mobileNavOpen ? "translate-x-0" : "-translate-x-full",
              )
            : collapsed
              ? "w-16"
              : "w-64",
        )}
      >
        <div className="flex shrink-0 items-center justify-between border-b border-border p-4">
          {!sidebarCollapsed ? (
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
          {isMobile ? (
            <button
              type="button"
              onClick={() => setMobileNavOpen(false)}
              className="rounded-lg p-1.5 text-muted hover:bg-surface-elevated hover:text-foreground"
              aria-label="Fermer le menu"
            >
              <X className="h-4 w-4" />
            </button>
          ) : (
            <button
              type="button"
              onClick={() => setCollapsed((c) => !c)}
              className="rounded-lg p-1.5 text-muted hover:bg-surface-elevated hover:text-foreground"
              aria-label={collapsed ? "Déplier le menu" : "Replier le menu"}
            >
              {collapsed ? <ChevronRight className="h-4 w-4" /> : <ChevronLeft className="h-4 w-4" />}
            </button>
          )}
        </div>

        <nav className="flex min-h-0 flex-1 flex-col gap-2 overflow-hidden p-3">
          <div className="shrink-0 space-y-2">
            <MenuItem
              to="/"
              label="Tableau de bord"
              icon={LayoutDashboard}
              end
              collapsed={sidebarCollapsed}
            />
            <Guard privilege="tache:voir">
              <button
                type="button"
                title={sidebarCollapsed ? "Tâches" : undefined}
                onClick={() => {
                  setMobileNavOpen(false);
                  openTaches();
                }}
                className={cn("nav-app-btn w-full", sidebarCollapsed && "justify-center px-2")}
              >
                <ListTodo className="h-4 w-4 shrink-0" />
                {!sidebarCollapsed && <span className="truncate">Tâches</span>}
              </button>
            </Guard>
            {stockEnabled && (
              <Guard privilege="stock:voir">
                <button
                  type="button"
                  title={sidebarCollapsed ? "Stock" : undefined}
                  onClick={() => {
                    setMobileNavOpen(false);
                    openStock();
                  }}
                  className={cn("nav-app-btn w-full", sidebarCollapsed && "justify-center px-2")}
                >
                  <Package className="h-4 w-4 shrink-0" />
                  {!sidebarCollapsed && <span className="truncate">Stock</span>}
                </button>
              </Guard>
            )}
            <MenuItem
              to="/parametres"
              label="Paramètres"
              icon={Settings}
              collapsed={sidebarCollapsed}
            />
          </div>
          <SidebarWindowsPanel collapsed={sidebarCollapsed} />
        </nav>

        <div className="shrink-0 space-y-3 border-t border-border p-4 pb-[max(1rem,env(safe-area-inset-bottom))]">
          {!sidebarCollapsed && (
            <div className="px-1">
              <p className="truncate text-sm font-medium">{user?.nom}</p>
              <p className="truncate text-xs text-muted">{user?.role}</p>
            </div>
          )}
          <Button
            variant="ghost"
            size="sm"
            className={cn("w-full", sidebarCollapsed ? "justify-center px-2" : "justify-start")}
            onClick={() => void logout()}
            title="Déconnexion"
          >
            <LogOut className="h-4 w-4" />
            {!sidebarCollapsed && "Déconnexion"}
          </Button>
        </div>
      </aside>

      <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
        {isMobile && (
          <header className="flex shrink-0 items-center gap-3 border-b border-border bg-surface px-3 py-2.5 pt-[max(0.625rem,env(safe-area-inset-top))]">
            <button
              type="button"
              onClick={() => setMobileNavOpen(true)}
              className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg text-muted hover:bg-surface-elevated hover:text-foreground"
              aria-label="Ouvrir le menu"
            >
              <Menu className="h-5 w-5" />
            </button>
            <img src={logoSrc} alt="" className="h-8 w-8 shrink-0 object-contain" aria-hidden />
            <div className="min-w-0 flex-1">
              <p className="truncate text-sm font-bold text-gradient-brand">{title}</p>
              <p className="truncate text-xs text-muted">{user?.nom}</p>
            </div>
          </header>
        )}

        <main className="app-main">
          <Outlet />
        </main>

        {isMobile && (
          <nav
            className="flex shrink-0 items-stretch justify-around border-t border-border bg-surface px-1 pb-[max(0.35rem,env(safe-area-inset-bottom))] pt-1"
            aria-label="Navigation principale"
          >
            <NavLinkBtn to="/" label="Accueil" icon={LayoutDashboard} end />
            <Guard privilege="tache:voir">
              <button
                type="button"
                className="mobile-nav-btn"
                onClick={() => openTaches()}
                aria-label="Tâches"
              >
                <ListTodo className="h-5 w-5" />
                <span>Tâches</span>
              </button>
            </Guard>
            {stockEnabled && (
              <Guard privilege="stock:voir">
                <button
                  type="button"
                  className="mobile-nav-btn"
                  onClick={() => openStock()}
                  aria-label="Stock"
                >
                  <Package className="h-5 w-5" />
                  <span>Stock</span>
                </button>
              </Guard>
            )}
            <NavLinkBtn to="/parametres" label="Réglages" icon={Settings} />
          </nav>
        )}
      </div>
    </div>
  );
}

function NavLinkBtn({
  to,
  label,
  icon: Icon,
  end,
}: {
  to: string;
  label: string;
  icon: LucideIcon;
  end?: boolean;
}) {
  return (
    <NavLink
      to={to}
      end={end}
      className={({ isActive }) => cn("mobile-nav-btn", isActive && "mobile-nav-btn--active")}
      aria-label={label}
    >
      <Icon className="h-5 w-5" />
      <span>{label}</span>
    </NavLink>
  );
}
