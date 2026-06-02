import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  AI_CONVERSATION_NEW_EVENT,
  AI_CONVERSATION_SELECT_EVENT,
  AI_CONVERSATIONS_REFRESH_EVENT,
  ENTITY_REGISTRY_SYNCED_EVENT,
} from "@/constants/events";
import { useDashboardChat } from "@/contexts/DashboardChatContext";
import { useTachesModal } from "@/contexts/TachesModalContext";
import { invoke } from "@tauri-apps/api/core";
import { EntityWorkspace } from "@/engine/EntityWorkspace";
import { CommandBar } from "@/items/CommandBar";
import {
  DashboardChatQueue,
  type PendingQuestion,
} from "@/items/DashboardChatQueue";
import {
  DashboardChatThread,
  type DashboardChatEntry,
} from "@/items/DashboardChatThread";
import { useEntityBranding } from "@/hooks/useEntityBranding";
import { randomDelayMs } from "@/lib/randomDelay";
import { sortEntitySuggestionsByPhrase } from "@/lib/entitySuggestions";
import type { AiChatReply, AiStoredMessage } from "@/types/ai";
import type { EntityCreateDraft, EntitySuggestion } from "@/types/entity";
import type { ScreenRow } from "@/types/screen";

const MAX_PENDING_QUESTIONS = 10;
const TACHE_ENTITY_KEY = "tache";

function newEntryId(): string {
  return `m-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

function bumpConversationsList() {
  window.dispatchEvent(new CustomEvent(AI_CONVERSATIONS_REFRESH_EVENT));
}

function messagesToThread(messages: AiStoredMessage[]): DashboardChatEntry[] {
  return messages
    .filter((m) => m.role === "user" || m.role === "assistant")
    .map((m, i) => ({
      id: `hist-${i}-${m.role}`,
      role: m.role as "user" | "assistant",
      content: m.content,
    }));
}

/** Tableau de bord — barre de commande + gestion dynamique des entités (sans fenêtre IA flottante). */
export function DashboardPage() {
  const { title, slogan } = useEntityBranding();
  const [command, setCommand] = useState("");
  const [activeEntity, setActiveEntity] = useState<string | null>(null);
  const [entityCreateDraft, setEntityCreateDraft] = useState<ScreenRow | null>(null);
  const [chatStarted, setChatStarted] = useState(false);
  const { conversationId, setConversationId } = useDashboardChat();
  const { openTaches } = useTachesModal();
  const [thread, setThread] = useState<DashboardChatEntry[]>([]);
  const [pendingQueue, setPendingQueue] = useState<PendingQuestion[]>([]);
  const [suggestionsRefresh, setSuggestionsRefresh] = useState(0);
  const [knownSuggestions, setKnownSuggestions] = useState<EntitySuggestion[]>([]);
  const conversationIdRef = useRef<string | null>(null);
  const pendingQueueRef = useRef<PendingQuestion[]>([]);

  const chatBusy = thread.some((e) => e.role === "assistant" && e.loading);

  useEffect(() => {
    conversationIdRef.current = conversationId;
  }, [conversationId]);

  const commandInputHistory = useMemo(
    () =>
      thread
        .filter(
          (e): e is DashboardChatEntry & { role: "user"; content: string } =>
            e.role === "user" && typeof e.content === "string" && e.content.trim().length > 0,
        )
        .map((e) => e.content.trim()),
    [thread],
  );

  const commandResponseHistory = useMemo(
    () =>
      thread
        .filter(
          (e): e is DashboardChatEntry & { role: "assistant"; content: string } =>
            e.role === "assistant" &&
            !e.loading &&
            typeof e.content === "string" &&
            e.content.trim().length > 0,
        )
        .map((e) => e.content.trim()),
    [thread],
  );

  const sendDisabled =
    chatBusy && pendingQueue.length >= MAX_PENDING_QUESTIONS;

  const loadSuggestions = useCallback(async () => {
    try {
      const rows = await invoke<EntitySuggestion[]>("entity_list_manageable");
      setKnownSuggestions(sortEntitySuggestionsByPhrase(rows));
    } catch {
      setKnownSuggestions([]);
    }
  }, []);

  useEffect(() => {
    void loadSuggestions();
  }, [loadSuggestions, suggestionsRefresh]);

  useEffect(() => {
    const refresh = () => {
      setSuggestionsRefresh((n) => n + 1);
      void loadSuggestions();
    };
    window.addEventListener(ENTITY_REGISTRY_SYNCED_EVENT, refresh);
    return () => window.removeEventListener(ENTITY_REGISTRY_SYNCED_EVENT, refresh);
  }, [loadSuggestions]);

  const resetChat = useCallback(() => {
    setConversationId(null);
    conversationIdRef.current = null;
    setThread([]);
    setChatStarted(false);
    setPendingQueue([]);
    pendingQueueRef.current = [];
    setCommand("");
  }, [setConversationId]);

  const loadConversation = useCallback(
    async (id: string) => {
      try {
        const messages = await invoke<AiStoredMessage[]>("ai_conversation_messages", {
          payload: { conversation_id: id },
        });
        setConversationId(id);
        conversationIdRef.current = id;
        setThread(messagesToThread(messages));
        setChatStarted(messages.length > 0);
        setPendingQueue([]);
        pendingQueueRef.current = [];
        setCommand("");
        setActiveEntity(null);
      } catch (e) {
        window.alert(String(e));
      }
    },
    [setConversationId],
  );

  useEffect(() => {
    const onSelect = (e: Event) => {
      const id = (e as CustomEvent<{ conversationId: string }>).detail?.conversationId;
      if (id) void loadConversation(id);
    };
    const onNew = () => resetChat();
    window.addEventListener(AI_CONVERSATION_SELECT_EVENT, onSelect);
    window.addEventListener(AI_CONVERSATION_NEW_EVENT, onNew);
    return () => {
      window.removeEventListener(AI_CONVERSATION_SELECT_EVENT, onSelect);
      window.removeEventListener(AI_CONVERSATION_NEW_EVENT, onNew);
    };
  }, [loadConversation, resetChat]);

  const patchAssistant = useCallback((assistantId: string, patch: Partial<DashboardChatEntry>) => {
    setThread((prev) =>
      prev.map((e) => (e.id === assistantId && e.role === "assistant" ? { ...e, ...patch } : e)),
    );
  }, []);

  const openEntityWorkspace = useCallback(
    (entityKey: string, draft?: ScreenRow) => {
      if (entityKey === TACHE_ENTITY_KEY) {
        setActiveEntity(null);
        setEntityCreateDraft(null);
        openTaches(draft);
        return;
      }
      setEntityCreateDraft(draft ?? null);
      setActiveEntity(entityKey);
    },
    [openTaches],
  );

  const openEntityWithTransition = useCallback(
    async (entityKey: string, userMessage: string) => {
      if (entityKey === TACHE_ENTITY_KEY) {
        setCommand("");
        openTaches();
        return;
      }
      setChatStarted(true);
      setCommand("");
      setPendingQueue([]);
      const assistantId = newEntryId();
      setThread([
        { id: newEntryId(), role: "user", content: userMessage },
        {
          id: assistantId,
          role: "assistant",
          content: null,
          loading: true,
          entityLoader: false,
        },
      ]);

      void invoke<string>("ai_dashboard_transition", {
        payload: { user_message: userMessage, entity_key: entityKey },
      })
        .then((phrase) => {
          if (phrase.trim()) {
            patchAssistant(assistantId, {
              content: phrase.trim(),
              loading: false,
              entityLoader: true,
            });
          } else {
            patchAssistant(assistantId, { loading: false, entityLoader: true });
          }
        })
        .catch(() => {
          patchAssistant(assistantId, { loading: false, entityLoader: true });
        });

      await randomDelayMs(1000, 3000);

      setThread([]);
      setChatStarted(false);
      setActiveEntity(entityKey);
    },
    [patchAssistant, openTaches],
  );

  const openEntityCreate = useCallback(
    (draft: EntityCreateDraft, userMessage: string) => {
      setChatStarted(true);
      setCommand("");
      setPendingQueue([]);
      setThread([
        { id: newEntryId(), role: "user", content: userMessage },
        { id: newEntryId(), role: "assistant", content: draft.assistant_message },
      ]);
      openEntityWorkspace(draft.entity_key, draft.initial_data as ScreenRow);
    },
    [openEntityWorkspace],
  );

  const applyCreateActionFromReply = useCallback(
    (reply: AiChatReply) => {
      if (!reply.open_entity_create) return false;
      const { entity_key, initial_data } = reply.open_entity_create;
      openEntityWorkspace(entity_key, initial_data as ScreenRow);
      return true;
    },
    [openEntityWorkspace],
  );

  useEffect(() => {
    if (activeEntity === TACHE_ENTITY_KEY) {
      openTaches(entityCreateDraft ?? undefined);
      setActiveEntity(null);
      setEntityCreateDraft(null);
    }
  }, [activeEntity, entityCreateDraft, openTaches]);

  const askLoggyPractical = useCallback(async (text: string) => {
    setChatStarted(true);
    const assistantId = newEntryId();
    setThread((prev) => [
      ...prev,
      { id: newEntryId(), role: "user", content: text },
      { id: assistantId, role: "assistant", content: null, loading: true },
    ]);

    try {
      const reply = await invoke<AiChatReply>("ai_dashboard_answer", {
        payload: {
          message: text,
          conversation_id: conversationIdRef.current,
        },
      });
      conversationIdRef.current = reply.conversation_id;
      setConversationId(reply.conversation_id);
      patchAssistant(assistantId, {
        content: reply.message.trim(),
        loading: false,
      });
      applyCreateActionFromReply(reply);
      bumpConversationsList();
    } catch (e) {
      patchAssistant(assistantId, {
        content: String(e),
        loading: false,
      });
    }
  }, [applyCreateActionFromReply, patchAssistant, setConversationId]);

  const enqueueQuestion = useCallback((text: string) => {
    setPendingQueue((prev) => {
      if (prev.length >= MAX_PENDING_QUESTIONS) return prev;
      const next = [...prev, { id: newEntryId(), text }];
      pendingQueueRef.current = next;
      return next;
    });
  }, []);

  useEffect(() => {
    pendingQueueRef.current = pendingQueue;
  }, [pendingQueue]);

  useEffect(() => {
    if (chatBusy) return;
    if (pendingQueueRef.current.length === 0) return;

    const [head, ...rest] = pendingQueueRef.current;
    pendingQueueRef.current = rest;
    setPendingQueue(rest);
    void askLoggyPractical(head.text);
  }, [chatBusy, askLoggyPractical]);

  const resolveEntityKey = useCallback(
    async (text: string): Promise<string | null> => {
      const lower = text.trim().toLowerCase();
      for (const s of knownSuggestions) {
        if (lower === s.phrase.toLowerCase()) {
          return s.key;
        }
      }
      try {
        return await invoke<string | null>("entity_match_intent", {
          payload: { message: text },
        });
      } catch {
        return null;
      }
    },
    [knownSuggestions],
  );

  const submitCommand = async () => {
    const text = command.trim();
    if (!text || sendDisabled) return;

    const mustQueue = chatBusy || pendingQueue.length > 0;

    if (!mustQueue) {
      try {
        const draft = await invoke<EntityCreateDraft | null>("entity_match_create_draft", {
          payload: { message: text },
        });
        if (draft) {
          setCommand("");
          openEntityCreate(draft, text);
          return;
        }
      } catch {
        /* fallback intent / Loggy */
      }
    }

    const matched = await resolveEntityKey(text);
    if (matched && !mustQueue) {
      await openEntityWithTransition(matched, text);
      return;
    }

    if (mustQueue) {
      if (pendingQueue.length >= MAX_PENDING_QUESTIONS) return;
      enqueueQuestion(text);
      setCommand("");
      return;
    }

    setCommand("");
    await askLoggyPractical(text);
  };

  const commandBar = (
    <CommandBar
      value={command}
      onChange={setCommand}
      onSubmit={() => void submitCommand()}
      inputHistory={chatStarted ? commandInputHistory : []}
      responseHistory={chatStarted ? commandResponseHistory : []}
      suggestionsRefreshToken={suggestionsRefresh}
      sendDisabled={sendDisabled}
      inputDisabled={false}
      suggestionsAbove={chatStarted}
      placeholder={chatStarted ? "Poser une question" : "Que souhaitez-vous faire ?"}
      onSuggestionSelect={(key, phrase) => {
        void openEntityWithTransition(key, phrase);
      }}
    />
  );

  const showHome = !activeEntity;

  return (
    <div className="dashboard-page relative">
      {showHome && !chatStarted && (
        <div className="dashboard-home-hero">
          <div className="flex w-full max-w-2xl flex-col items-center gap-8 text-center">
            <div className="flex w-full flex-col items-center gap-3">
              <h1 className="font-brand-serif max-w-full text-4xl font-normal tracking-tight text-gradient-brand md:text-6xl">
                {title}
              </h1>
              {slogan.trim() && (
                <p className="max-w-lg text-sm text-muted">{slogan}</p>
              )}
            </div>
            {commandBar}
          </div>
        </div>
      )}

      {showHome && chatStarted && (
        <div className="dashboard-chat-shell">
          <div className="dashboard-chat-messages">
            <div className="dashboard-chat-messages-inner">
              <DashboardChatThread entries={thread} />
            </div>
          </div>
          <footer className="dashboard-chat-footer">
            <div className="dashboard-chat-footer-inner">
              <DashboardChatQueue items={pendingQueue} maxItems={MAX_PENDING_QUESTIONS} />
              {commandBar}
            </div>
          </footer>
        </div>
      )}

      {activeEntity && (
        <div className="h-full min-h-0 overflow-y-auto">
          <EntityWorkspace
            entityKey={activeEntity}
            initialCreateValues={entityCreateDraft ?? undefined}
            onCreateDraftConsumed={() => setEntityCreateDraft(null)}
            onClose={() => {
              setActiveEntity(null);
              setEntityCreateDraft(null);
              setSuggestionsRefresh((n) => n + 1);
            }}
          />
        </div>
      )}
    </div>
  );
}
