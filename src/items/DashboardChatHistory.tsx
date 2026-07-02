import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { History } from "lucide-react";
import { Button } from "@/items/Button";
import { Modal } from "@/items/Modal";
import { Text } from "@/items/Text";
import { formatDateTimeFr } from "@/lib/formatDateTime";
import type { AiConversationSummary } from "@/types/ai";

interface DashboardChatHistoryProps {
  onSelect: (conversationId: string) => void;
  refreshToken?: number;
}

/** Liste des discussions Loggy enregistrées (tableau de bord). */
export function DashboardChatHistory({ onSelect, refreshToken = 0 }: DashboardChatHistoryProps) {
  const [open, setOpen] = useState(false);
  const [rows, setRows] = useState<AiConversationSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const list = await invoke<AiConversationSummary[]>("ai_list_conversations");
      setRows(list);
    } catch (e) {
      setRows([]);
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (!open) return;
    void load();
  }, [open, load, refreshToken]);

  return (
    <>
      <Button
        type="button"
        size="sm"
        variant="ghost"
        className="self-start text-muted"
        onClick={() => setOpen(true)}
      >
        <History className="mr-1.5 h-3.5 w-3.5" />
        Historique
      </Button>

      <Modal
        open={open}
        onClose={() => setOpen(false)}
        title="Discussions Loggy"
        size="md"
      >
        <div className="space-y-3">
          {loading && <Text variant="muted">Chargement…</Text>}
          {error && <Text variant="muted">{error}</Text>}
          {!loading && !error && rows.length === 0 && (
            <Text variant="muted">Aucune discussion enregistrée.</Text>
          )}
          {!loading && rows.length > 0 && (
            <ul className="max-h-80 space-y-1 overflow-y-auto" role="list">
              {rows.map((row) => (
                <li key={row.id}>
                  <button
                    type="button"
                    className="w-full rounded-lg border border-border px-3 py-2.5 text-left transition-colors hover:bg-muted/20"
                    onClick={() => {
                      onSelect(row.id);
                      setOpen(false);
                    }}
                  >
                    <span className="block truncate text-sm text-foreground">
                      {row.title.trim() || "Discussion sans titre"}
                    </span>
                    <span className="mt-0.5 block text-xs text-muted">
                      {formatDateTimeFr(new Date(row.updated_at))} · {row.message_count} message
                      {row.message_count > 1 ? "s" : ""}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </Modal>
    </>
  );
}
