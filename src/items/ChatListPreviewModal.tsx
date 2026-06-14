import { ExternalLink } from "lucide-react";
import { Button } from "@/items/Button";
import { Modal } from "@/items/Modal";
import { PaginatedList } from "@/items/PaginatedList";
import { Table, type Column } from "@/items/Table";
import { LIST_PAGE_SIZE } from "@/constants/variable.constant";
import { formatCellValue } from "@/lib/chatDisplayParse";
import type { ChatDisplayBlock } from "@/types/ai";

interface ChatListPreviewModalProps {
  block: ChatDisplayBlock;
  open: boolean;
  onClose: () => void;
  onOpenEntity?: (entityKey: string) => void;
}

export function ChatListPreviewModal({
  block,
  open,
  onClose,
  onOpenEntity,
}: ChatListPreviewModalProps) {
  const rowCount = block.rows.length;
  const title =
    rowCount === 0
      ? "Liste — aucun résultat"
      : rowCount === 1
        ? "Liste — 1 enregistrement"
        : `Liste — ${rowCount} enregistrements`;

  const columns: Column<Record<string, unknown>>[] = block.columns.map((col) => ({
    key: col.key,
    header: col.label,
    sortable: true,
    render: (row) => formatCellValue(row[col.key]),
  }));

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={title}
      size="xl"
      footer={
        <div className="flex w-full flex-col-reverse gap-2 sm:flex-row sm:justify-end">
          <Button variant="ghost" onClick={onClose}>
            Fermer
          </Button>
          {block.entityKey && onOpenEntity && (
            <Button
              variant="secondary"
              onClick={() => {
                onClose();
                onOpenEntity(block.entityKey!);
              }}
            >
              <ExternalLink className="mr-1.5 h-3.5 w-3.5" />
              Ouvrir l&apos;écran entité
            </Button>
          )}
        </div>
      }
    >
      {block.kind === "list" && block.columns.length <= 1 ? (
        <PaginatedList
          items={block.rows}
          pageSize={LIST_PAGE_SIZE}
          empty={<p className="text-sm text-muted">Aucune ligne à afficher.</p>}
          className="max-h-[min(60vh,28rem)] overflow-y-auto text-sm text-foreground"
          renderItem={(row, ri) => {
            const col = block.columns[0];
            const val = col ? row[col.key] : Object.values(row)[0];
            return (
              <p key={ri} className="border-b border-border/40 py-2 last:border-b-0">
                {formatCellValue(val)}
              </p>
            );
          }}
        />
      ) : (
        <Table
          columns={columns}
          data={block.rows}
          keyExtractor={(row) => {
            for (const k of ["id", "reference", "reference_libelle", "created_at"]) {
              const v = row[k];
              if (v != null && v !== "") return String(v);
            }
            return JSON.stringify(row);
          }}
          emptyMessage="Aucune ligne à afficher."
          pageSize={15}
          pageSizeOptions={[10, 25, 50, 100]}
          dense
        />
      )}
    </Modal>
  );
}
