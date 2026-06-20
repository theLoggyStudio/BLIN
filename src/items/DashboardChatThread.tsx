import { useEffect, useRef } from "react";
import { cn } from "@/lib/utils";
import { ChatLinkifiedText } from "@/items/ChatLinkifiedText";
import { ChatLoggyAttachments } from "@/items/ChatLoggyAttachments";
import { LoggySpeakButton } from "@/items/LoggySpeakButton";
import { isLoggyVoiceAutoEnabled, speakLoggy } from "@/lib/loggyVoice";
import type { ChatColsRequest, ChatDisplayBlock } from "@/types/ai";

export interface DashboardChatEntry {
  id: string;
  role: "user" | "assistant";
  content: string | null;
  /** Contenu utilisateur riche (remplace content pour les bulles user). */
  userContent?: React.ReactNode;
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
  // Lecture vocale automatique : lit la dernière réponse Loggy "live" sans
  // relire l'historique chargé au montage.
  const spokenRef = useRef<Set<string>>(new Set());
  const initializedRef = useRef(false);

  useEffect(() => {
    const ready = entries.filter(
      (e) => e.role === "assistant" && !e.loading && !!e.content,
    );
    if (!initializedRef.current) {
      for (const e of ready) spokenRef.current.add(e.id);
      initializedRef.current = true;
      return;
    }
    if (!isLoggyVoiceAutoEnabled()) return;
    const unspoken = ready.filter((e) => !spokenRef.current.has(e.id));
    for (const e of unspoken) spokenRef.current.add(e.id);
    const latest = unspoken[unspoken.length - 1];
    if (latest?.content) speakLoggy(latest.content);
  }, [entries]);

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
              {entry.userContent ?? (
                <p className="user-chat-text">{entry.content}</p>
              )}
            </div>
          );
        }

        const showAssistant = entry.loading || entry.content != null || entry.entityLoader;
        if (!showAssistant) return null;

        return (
          <div key={entry.id} className="flex w-full flex-col items-start gap-3">
            <div className="loggy-chat-bubble" aria-label="Message de Loggy" aria-busy={entry.loading}>
              <div className="loggy-chat-header">
                <span className="loggy-chat-author">Loggy</span>
                {entry.content ? <LoggySpeakButton text={entry.content} /> : null}
              </div>
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
