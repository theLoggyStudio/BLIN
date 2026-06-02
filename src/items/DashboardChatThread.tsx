import { cn } from "@/lib/utils";
import { ChatLinkifiedText } from "@/items/ChatLinkifiedText";

export interface DashboardChatEntry {
  id: string;
  role: "user" | "assistant";
  content: string | null;
  loading?: boolean;
  /** Spinner centré (ouverture écran entité). */
  entityLoader?: boolean;
}

interface DashboardChatThreadProps {
  entries: DashboardChatEntry[];
  className?: string;
}

/** Fil de discussion centré (bulles utilisateur + Loggy). */
export function DashboardChatThread({ entries, className }: DashboardChatThreadProps) {
  if (entries.length === 0) {
    return (
      <div className={cn("dashboard-chat-empty", className)} aria-hidden>
        <div className="dashboard-chat-center-loader" />
      </div>
    );
  }

  return (
    <div className={cn("loggy-chat-thread w-full", className)} role="log" aria-live="polite">
      {entries.map((entry) => {
        if (entry.role === "user") {
          return (
            <div key={entry.id} className="user-chat-bubble" aria-label="Votre message">
              <span className="user-chat-author">Vous</span>
              <p className="user-chat-text">{entry.content}</p>
            </div>
          );
        }

        const showAssistant = entry.loading || entry.content != null || entry.entityLoader;
        if (!showAssistant) return null;

        return (
          <div key={entry.id} className="flex w-full flex-col items-start gap-3">
            <div className="loggy-chat-bubble" aria-label="Message de Loggy" aria-busy={entry.loading}>
              <span className="loggy-chat-author">Loggy</span>
              {entry.content ? (
                <ChatLinkifiedText
                  text={entry.content}
                  className="loggy-chat-text whitespace-pre-wrap"
                />
              ) : entry.loading ? (
                <p className="loggy-chat-text loggy-chat-typing" aria-hidden>
                  <span className="loggy-chat-dot" />
                  <span className="loggy-chat-dot" />
                  <span className="loggy-chat-dot" />
                </p>
              ) : null}
            </div>
            {entry.entityLoader && (
              <div className="dashboard-chat-center-loader mx-auto" aria-hidden />
            )}
          </div>
        );
      })}
    </div>
  );
}
