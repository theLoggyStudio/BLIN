import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Modal } from "@/components/ui/Modal";
import { CommandBar } from "@/items/CommandBar";
import {
  DashboardChatThread,
  type DashboardChatEntry,
} from "@/items/DashboardChatThread";
import {
  askStatsLoggyQuestion,
  type StatsChatTurn,
  type StatsInterpretPayload,
} from "@/lib/statsInterpret";

const INITIAL_USER_ID = "stats-user-init";
const INITIAL_ASSISTANT_ID = "stats-assistant-init";

function newEntryId(): string {
  return `stats-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
}

interface StatsLoggyChatModalProps {
  open: boolean;
  onClose: () => void;
  interpretPayload: StatsInterpretPayload | null;
  interpretation: string;
  interpretLoading: boolean;
  /** Réinitialise le fil quand les données du graphique changent. */
  statsDataVersion: string;
}

/** Modale de discussion Loggy dédiée à l'analyse d'une courbe statistique. */
export function StatsLoggyChatModal({
  open,
  onClose,
  interpretPayload,
  interpretation,
  interpretLoading,
  statsDataVersion,
}: StatsLoggyChatModalProps) {
  const [thread, setThread] = useState<DashboardChatEntry[]>([]);
  const [question, setQuestion] = useState("");
  const [chatBusy, setChatBusy] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);
  const chatHistoryRef = useRef<StatsChatTurn[]>([]);

  const initialUserContent = useMemo(
    () => (
      <p className="user-chat-text">
        Analyse moi cette{" "}
        <button
          type="button"
          className="chat-source-link font-medium"
          onClick={onClose}
          title="Voir la Courbe"
        >
          courbe
        </button>
      </p>
    ),
    [onClose],
  );

  const buildInitialThread = useCallback((): DashboardChatEntry[] => {
    return [
      {
        id: INITIAL_USER_ID,
        role: "user",
        content: "Analyse moi cette courbe",
        userContent: initialUserContent,
      },
      {
        id: INITIAL_ASSISTANT_ID,
        role: "assistant",
        content: interpretation.trim() || null,
        loading: interpretLoading && !interpretation.trim(),
      },
    ];
  }, [initialUserContent, interpretation, interpretLoading]);

  useEffect(() => {
    if (!open) return;
    chatHistoryRef.current = [];
    setQuestion("");
    setChatBusy(false);
    setThread(buildInitialThread());
  }, [open, statsDataVersion, buildInitialThread]);

  useEffect(() => {
    if (!open) return;
    setThread((prev) =>
      prev.map((entry) =>
        entry.id === INITIAL_ASSISTANT_ID
          ? {
              ...entry,
              content: interpretation.trim() || null,
              loading: interpretLoading && !interpretation.trim(),
            }
          : entry,
      ),
    );
  }, [open, interpretation, interpretLoading]);

  useEffect(() => {
    if (!open) return;
    const el = scrollRef.current;
    if (el) {
      el.scrollTop = el.scrollHeight;
    }
  }, [open, thread, interpretation, interpretLoading]);

  const handleAsk = async () => {
    const text = question.trim();
    if (!text || !interpretPayload || chatBusy) return;

    const analysis =
      interpretation.trim() ||
      "Analyse en cours — les données du graphique sont chargées mais le commentaire détaillé n'est pas encore prêt.";

    const userId = newEntryId();
    const assistantId = newEntryId();

    setThread((prev) => [
      ...prev,
      { id: userId, role: "user", content: text },
      { id: assistantId, role: "assistant", content: null, loading: true },
    ]);
    setQuestion("");
    setChatBusy(true);

    try {
      const reply = await askStatsLoggyQuestion(
        interpretPayload,
        analysis,
        text,
        chatHistoryRef.current,
      );
      chatHistoryRef.current = [
        ...chatHistoryRef.current,
        { role: "user", content: text },
        { role: "assistant", content: reply },
      ];
      setThread((prev) =>
        prev.map((entry) =>
          entry.id === assistantId
            ? { ...entry, content: reply, loading: false }
            : entry,
        ),
      );
    } catch (e) {
      const err = String(e);
      setThread((prev) =>
        prev.map((entry) =>
          entry.id === assistantId
            ? { ...entry, content: err, loading: false }
            : entry,
        ),
      );
    } finally {
      setChatBusy(false);
    }
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      title="Avis de Loggy sur la courbe"
      size="2xl"
      busy={chatBusy}
      busyLabel="Loggy réfléchit…"
    >
      <div className="flex min-h-[min(60dvh,32rem)] max-h-[min(75dvh,40rem)] flex-col gap-3">
        <div
          ref={scrollRef}
          className="min-h-0 flex-1 overflow-y-auto rounded-lg border border-border bg-background/40 px-2 py-3 md:px-4"
        >
          <div className="mx-auto flex w-full max-w-2xl flex-col gap-3">
            <DashboardChatThread entries={thread} />
          </div>
        </div>

        <div className="shrink-0 border-t border-border pt-3">
          <div className="mx-auto w-full max-w-2xl">
            <CommandBar
              value={question}
              onChange={setQuestion}
              onSubmit={() => void handleAsk()}
              placeholder="Posez une question sur cette courbe…"
              sendDisabled={chatBusy || !interpretPayload}
              inputDisabled={chatBusy || !interpretPayload}
              suggestionsAbove={thread.length > 2}
            />
          </div>
        </div>
      </div>
    </Modal>
  );
}
