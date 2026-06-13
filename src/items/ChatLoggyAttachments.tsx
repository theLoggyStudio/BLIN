import { useEffect, useState } from "react";
import { Columns3, Table2 } from "lucide-react";
import { Button } from "@/items/Button";
import { ChatColsPickerModal } from "@/items/ChatColsPickerModal";
import { ChatListPreviewModal } from "@/items/ChatListPreviewModal";
import type { ChatColsRequest, ChatDisplayBlock } from "@/types/ai";

interface ChatLoggyAttachmentsProps {
  displayBlocks?: ChatDisplayBlock[];
  colsRequest?: ChatColsRequest;
  /** Ouvre automatiquement le sélecteur de colonnes (réponse live). */
  autoOpenCols?: boolean;
  onOpenEntity?: (entityKey: string) => void;
  onColsConfirm?: (message: string) => void;
}

function previewButtonLabel(block: ChatDisplayBlock): string {
  const n = block.rows.length;
  if (n === 0) return "Voir le résultat (vide)";
  if (n === 1) return "Voir la liste (1 ligne)";
  return `Voir la liste (${n} lignes)`;
}

/**
 * Contenu structuré Loggy (listes, sélecteurs…) : bouton centré dans la bulle → modal.
 * La bulle ne contient que le texte court ; jamais de tableau ni liste de colonnes inline.
 */
export function ChatLoggyAttachments({
  displayBlocks = [],
  colsRequest,
  autoOpenCols = false,
  onOpenEntity,
  onColsConfirm,
}: ChatLoggyAttachmentsProps) {
  const [listOpenIndex, setListOpenIndex] = useState<number | null>(null);
  const [colsOpen, setColsOpen] = useState(false);

  useEffect(() => {
    if (autoOpenCols && colsRequest) {
      setColsOpen(true);
    }
  }, [autoOpenCols, colsRequest?.entityKey]);

  const hasLists = displayBlocks.length > 0;
  const hasCols = !!colsRequest && colsRequest.available.length > 0;

  if (!hasLists && !hasCols) return null;

  return (
    <>
      <div className="mt-4 flex w-full flex-col items-center gap-2 pointer-events-auto">
        {displayBlocks.map((block, bi) => (
          <Button
            key={`list-${bi}`}
            type="button"
            size="sm"
            variant="secondary"
            className="min-w-[12rem] justify-center"
            onClick={() => setListOpenIndex(bi)}
          >
            <Table2 className="mr-1.5 h-4 w-4 shrink-0" />
            {previewButtonLabel(block)}
          </Button>
        ))}
        {hasCols && colsRequest && (
          <Button
            type="button"
            size="sm"
            variant="secondary"
            className="min-w-[12rem] justify-center"
            onClick={() => setColsOpen(true)}
          >
            <Columns3 className="mr-1.5 h-4 w-4 shrink-0" />
            Choisir les colonnes ({colsRequest.available.length})
          </Button>
        )}
      </div>

      {listOpenIndex !== null && displayBlocks[listOpenIndex] && (
        <ChatListPreviewModal
          block={displayBlocks[listOpenIndex]}
          open
          onClose={() => setListOpenIndex(null)}
          onOpenEntity={onOpenEntity}
        />
      )}

      {hasCols && colsRequest && (
        <ChatColsPickerModal
          request={colsRequest}
          open={colsOpen}
          onClose={() => setColsOpen(false)}
          onConfirm={(msg) => onColsConfirm?.(msg)}
        />
      )}
    </>
  );
}
