import { cn } from "@/lib/utils";
import { ChatLinkifiedText } from "@/items/ChatLinkifiedText";
import { ChatLoggyAttachments } from "@/items/ChatLoggyAttachments";
import type { ChatColsRequest, ChatDisplayBlock } from "@/types/ai";

export interface DashboardChatEntry {
  id: string;
  role: "user" | "assistant";
  content: string | null;
  displayBlocks?: ChatDisplayBlock[];
  colsRequest?: ChatColsRequest;
  /** Ouvre le sélecteur de colonnes à l'arrivée (réponse live). */
  autoOpenCols?: boolean;
  loading?: boolean;
  /** Spinner centré (ouverture écran entité). */
  entityLoader?: boolean;
}

interface DashboardChatThreadProps {
  entries: DashboardChatEntry[];
  className?: string;
  onOpenEntityFromChat?: (entityKey: string) => void;
  onChatFollowUp?: (text: string) => void;
}

/** Fil de discussion centré (bulles utilisateur + Loggy). */
export function DashboardChatThread({
  entries,
  className,
  onOpenEntityFromChat,
  onChatFollowUp,
}: DashboardChatThreadProps) {
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
              {(entry.displayBlocks?.length ?? 0) > 0 || entry.colsRequest ? (
                <ChatLoggyAttachments
                  displayBlocks={entry.displayBlocks}
                  colsRequest={entry.colsRequest}
                  autoOpenCols={entry.autoOpenCols}
                  onOpenEntity={onOpenEntityFromChat}
                  onColsConfirm={onChatFollowUp}
                />
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
