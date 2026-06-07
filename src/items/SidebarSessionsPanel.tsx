import { useCallback, useEffect, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { ChevronDown, MessageSquare, Plus, Trash2 } from "lucide-react";
import {
  AI_CONVERSATION_NEW_EVENT,
  AI_CONVERSATION_SELECT_EVENT,
  AI_CONVERSATIONS_REFRESH_EVENT,
} from "@/constants/events";
import { cn } from "@/lib/utils";
import type { AiConversationSummary } from "@/types/ai";

import { formatDateTimeFr } from "@/lib/formatDateTime";
import { useAlert } from "@/contexts/AlertContext";

function formatSessionDate(iso: string): string {
  return formatDateTimeFr(iso);
}

interface SidebarSessionsPanelProps {
  collapsed?: boolean;
  activeConversationId?: string | null;
}

/** Panneau rétractable — historique des discussions Loggy. */
export function SidebarSessionsPanel({
  collapsed = false,
  activeConversationId = null,
}: SidebarSessionsPanelProps) {
  const { showError } = useAlert();
  const navigate = useNavigate();
  const [open, setOpen] = useState(true);
  const [sessions, setSessions] = useState<AiConversationSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameDraft, setRenameDraft] = useState("");
  const renameInputRef = useRef<HTMLInputElement>(null);

  const loadSessions = useCallback(async () => {
    setLoading(true);
    try {
      const rows = await invoke<AiConversationSummary[]>("ai_list_conversations");
      setSessions(rows);
    } catch {
      setSessions([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadSessions();
  }, [loadSessions]);

  useEffect(() => {
    const onRefresh = () => void loadSessions();
    window.addEventListener(AI_CONVERSATIONS_REFRESH_EVENT, onRefresh);
    return () => window.removeEventListener(AI_CONVERSATIONS_REFRESH_EVENT, onRefresh);
  }, [loadSessions]);

  useEffect(() => {
    if (renamingId) {
      renameInputRef.current?.focus();
      renameInputRef.current?.select();
    }
  }, [renamingId]);

  const startNew = () => {
    navigate("/");
    window.dispatchEvent(new CustomEvent(AI_CONVERSATION_NEW_EVENT));
  };

  const selectSession = (id: string) => {
    if (renamingId) return;
    navigate("/");
    window.dispatchEvent(
      new CustomEvent(AI_CONVERSATION_SELECT_EVENT, { detail: { conversationId: id } }),
    );
  };

  const startRename = (session: AiConversationSummary) => {
    setRenamingId(session.id);
    setRenameDraft(session.title?.trim() || "Sans titre");
  };

  const cancelRename = () => {
    setRenamingId(null);
    setRenameDraft("");
  };

  const commitRename = async (id: string) => {
    const next = renameDraft.trim();
    if (!next) {
      cancelRename();
      return;
    }
    const current = sessions.find((s) => s.id === id);
    if (current && (current.title?.trim() || "Sans titre") === next) {
      cancelRename();
      return;
    }
    try {
      await invoke("ai_rename_conversation", {
        payload: { conversation_id: id, title: next },
      });
      setSessions((prev) =>
        prev.map((s) => (s.id === id ? { ...s, title: next } : s)),
      );
      cancelRename();
    } catch (err) {
      showError(String(err));
    }
  };

  const deleteSession = async (e: React.MouseEvent, id: string, title: string) => {
    e.stopPropagation();
    const ok = window.confirm(
      `Supprimer la discussion « ${title} » ?\nCette action est irréversible.`,
    );
    if (!ok) return;
    try {
      await invoke("ai_delete_conversation", {
        payload: { conversation_id: id },
      });
      if (renamingId === id) cancelRename();
      if (activeConversationId === id) {
        window.dispatchEvent(new CustomEvent(AI_CONVERSATION_NEW_EVENT));
      }
      await loadSessions();
    } catch (err) {
      showError(String(err));
    }
  };

  if (collapsed) {
    return (
      <div className="sidebar-sessions-collapsed">
        <button
          type="button"
          className="sidebar-sessions-icon-btn"
          title="Nouvelle discussion"
          onClick={startNew}
        >
          <Plus className="h-4 w-4" />
        </button>
        <button
          type="button"
          className="sidebar-sessions-icon-btn"
          title="Sessions"
          onClick={() => {
            navigate("/");
            setOpen(true);
          }}
        >
          <MessageSquare className="h-4 w-4" />
        </button>
      </div>
    );
  }

  return (
    <section className="sidebar-sessions" aria-label="Sessions de discussion">
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
            <MessageSquare className="h-4 w-4" />
          </span>
          <span className="sidebar-sessions-header-label">Sessions</span>
          <span className="sidebar-sessions-badge">{sessions.length}</span>
        </button>
        <button
          type="button"
          className="sidebar-sessions-add-btn"
          title="Nouvelle discussion"
          aria-label="Nouvelle discussion"
          onClick={startNew}
        >
          <Plus className="h-4 w-4" />
        </button>
      </header>

      <div
        className={cn("sidebar-sessions-collapse", open && "sidebar-sessions-collapse--open")}
        aria-hidden={!open}
        inert={!open ? true : undefined}
        style={!open ? { pointerEvents: "none" } : undefined}
      >
        <div className="sidebar-sessions-collapse-inner">
          <div className="sidebar-sessions-body">
            {loading && sessions.length === 0 && (
              <p className="sidebar-sessions-empty">Chargement…</p>
            )}
            {!loading && sessions.length === 0 && (
              <p className="sidebar-sessions-empty">
                Aucune discussion.
                <br />
                <span className="text-muted">Posez une question sur l&apos;accueil.</span>
              </p>
            )}
            <ul className="sidebar-sessions-list" role="list">
              {sessions.map((s) => {
                const active = activeConversationId === s.id;
                const title = s.title?.trim() || "Sans titre";
                const isRenaming = renamingId === s.id;
                return (
                  <li key={s.id} className="sidebar-sessions-list-item">
                    <div
                      className={cn(
                        "sidebar-session-card",
                        active && "sidebar-session-card--active",
                        isRenaming && "sidebar-session-card--renaming",
                      )}
                    >
                      <div
                        role="button"
                        tabIndex={0}
                        className="sidebar-session-card-main"
                        onClick={() => selectSession(s.id)}
                        onKeyDown={(e) => {
                          if (isRenaming) return;
                          if (e.key === "Enter" || e.key === " ") {
                            e.preventDefault();
                            selectSession(s.id);
                          }
                        }}
                      >
                        {isRenaming ? (
                          <input
                            ref={renameInputRef}
                            type="text"
                            className="sidebar-session-card-rename-input"
                            value={renameDraft}
                            maxLength={200}
                            aria-label="Renommer la discussion"
                            onClick={(e) => e.stopPropagation()}
                            onChange={(e) => setRenameDraft(e.target.value)}
                            onKeyDown={(e) => {
                              e.stopPropagation();
                              if (e.key === "Enter") {
                                e.preventDefault();
                                void commitRename(s.id);
                              } else if (e.key === "Escape") {
                                e.preventDefault();
                                cancelRename();
                              }
                            }}
                            onBlur={() => void commitRename(s.id)}
                          />
                        ) : (
                          <span
                            className="sidebar-session-card-title"
                            title="Double-cliquez pour renommer"
                            onDoubleClick={(e) => {
                              e.stopPropagation();
                              e.preventDefault();
                              startRename(s);
                            }}
                          >
                            {title}
                          </span>
                        )}
                        <span className="sidebar-session-card-meta">
                          <time dateTime={s.updated_at}>{formatSessionDate(s.updated_at)}</time>
                          {s.message_count > 0 && (
                            <>
                              <span className="sidebar-session-card-sep" aria-hidden>
                                ·
                              </span>
                              <span>
                                {s.message_count} message{s.message_count > 1 ? "s" : ""}
                              </span>
                            </>
                          )}
                        </span>
                      </div>
                      <button
                        type="button"
                        className="sidebar-session-card-delete"
                        title="Supprimer cette discussion"
                        aria-label={`Supprimer ${title}`}
                        onClick={(e) => void deleteSession(e, s.id, title)}
                      >
                        <Trash2 className="h-4 w-4" />
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
