import { useCallback, useState } from "react";
import { useNavigate } from "react-router-dom";
import { ChevronDown, LayoutGrid, Plus, X } from "lucide-react";
import { useOpenWindows } from "@/contexts/OpenWindowsContext";
import { usePrivilege } from "@/hooks/usePrivilege";
import { FOCUS_AI_WINDOW_EVENT } from "@/constants/events";
import { cn } from "@/lib/utils";
import { formatDateTimeFr } from "@/lib/formatDateTime";

interface SidebarWindowsPanelProps {
  collapsed?: boolean;
}

const AI_WINDOW_ID = "ai";

function formatWindowDate(ts: number): string {
  return formatDateTimeFr(new Date(ts));
}

/** Panneau rétractable — fenêtres ouvertes (IA, entités, tâches, stock…). */
export function SidebarWindowsPanel({ collapsed = false }: SidebarWindowsPanelProps) {
  const navigate = useNavigate();
  const { windows, activeWindowId, focusWindow, closeWindow, openWindow } = useOpenWindows();
  const [open, setOpen] = useState(true);
  const canAi = usePrivilege("ai:utiliser");

  const openNewAiWindow = useCallback(() => {
    navigate("/");
    openWindow({ id: AI_WINDOW_ID, kind: "ai", title: "Discussion Loggy" });
    window.dispatchEvent(
      new CustomEvent(FOCUS_AI_WINDOW_EVENT, { detail: { isNew: true } }),
    );
  }, [navigate, openWindow]);

  const selectWindow = (id: string) => {
    navigate("/");
    focusWindow(id);
  };

  if (collapsed) {
    return (
      <div className="sidebar-sessions-collapsed">
        <button
          type="button"
          className="sidebar-sessions-icon-btn"
          title="Fenêtres ouvertes"
          onClick={() => {
            navigate("/");
            setOpen(true);
          }}
        >
          <LayoutGrid className="h-4 w-4" />
        </button>
        {canAi && (
          <button
            type="button"
            className="sidebar-sessions-add-btn sidebar-sessions-add-btn--collapsed"
            title="Nouvelle discussion Loggy"
            aria-label="Nouvelle discussion Loggy"
            onClick={openNewAiWindow}
          >
            <Plus className="h-4 w-4" />
          </button>
        )}
      </div>
    );
  }

  return (
    <section className="sidebar-sessions" aria-label="Fenêtres ouvertes">
      <header className="sidebar-sessions-header">
        <button
          type="button"
          className="sidebar-sessions-header-toggle"
          onClick={() => setOpen((o) => !o)}
          aria-expanded={open}
        >
          <ChevronDown
            className={cn("sidebar-sessions-chevron", open && "sidebar-sessions-chevron--open")}
          />
          <span className="sidebar-sessions-header-icon">
            <LayoutGrid className="h-4 w-4" />
          </span>
          <span className="sidebar-sessions-header-label">Fenêtres</span>
          <span className="sidebar-sessions-badge">{windows.length}</span>
        </button>
        {canAi && (
          <button
            type="button"
            className="sidebar-sessions-add-btn"
            title="Nouvelle discussion Loggy"
            aria-label="Nouvelle discussion Loggy"
            onClick={openNewAiWindow}
          >
            <Plus className="h-4 w-4" />
          </button>
        )}
      </header>

      <div
        className={cn("sidebar-sessions-collapse", open && "sidebar-sessions-collapse--open")}
        aria-hidden={!open}
        inert={!open ? true : undefined}
        style={!open ? { pointerEvents: "none" } : undefined}
      >
        <div className="sidebar-sessions-collapse-inner">
          <div className="sidebar-sessions-body">
            {windows.length === 0 && (
              <p className="sidebar-sessions-empty">
                Aucune fenêtre ouverte.
                <br />
                <span className="text-muted">
                  Utilisez <strong>+</strong> pour démarrer une discussion avec Loggy.
                </span>
              </p>
            )}
            <ul className="sidebar-sessions-list" role="list">
              {windows.map((w) => {
                const active = activeWindowId === w.id;
                return (
                  <li key={w.id} className="sidebar-sessions-list-item">
                    <div
                      className={cn(
                        "sidebar-session-card",
                        active && "sidebar-session-card--active",
                      )}
                    >
                      <div
                        role="button"
                        tabIndex={0}
                        className="sidebar-session-card-main"
                        onClick={() => selectWindow(w.id)}
                        onKeyDown={(e) => {
                          if (e.key === "Enter" || e.key === " ") {
                            e.preventDefault();
                            selectWindow(w.id);
                          }
                        }}
                      >
                        <span className="sidebar-session-card-title">{w.title}</span>
                        <span className="sidebar-session-card-meta">
                          <time dateTime={new Date(w.openedAt).toISOString()}>
                            {formatWindowDate(w.openedAt)}
                          </time>
                        </span>
                      </div>
                      <button
                        type="button"
                        className="sidebar-session-card-delete"
                        title="Fermer cette fenêtre"
                        aria-label={`Fermer ${w.title}`}
                        onClick={(e) => {
                          e.stopPropagation();
                          closeWindow(w.id);
                        }}
                      >
                        <X className="h-4 w-4" />
                      </button>
                    </div>
                  </li>
                );
              })}
            </ul>
          </div>
        </div>
      </div>
    </section>
  );
}
