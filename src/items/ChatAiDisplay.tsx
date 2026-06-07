import { ExternalLink } from "lucide-react";
import { Button } from "@/items/Button";
import type { ChatDisplayBlock } from "@/types/ai";
import { formatCellValue } from "@/lib/chatDisplayParse";

interface ChatAiDisplayProps {
  blocks: ChatDisplayBlock[];
  onOpenEntity?: (entityKey: string) => void;
}

export function ChatAiDisplay({ blocks, onOpenEntity }: ChatAiDisplayProps) {
  if (blocks.length === 0) return null;

  return (
    <div className="mt-3 space-y-3 pointer-events-none select-text">
      {blocks.map((block, bi) => (
        <div key={bi} className="relative rounded-lg border border-border bg-background/60 p-2">
          {block.kind === "list" ? (
            <ul className="list-disc space-y-1 pl-5 text-sm text-foreground">
              {block.rows.map((row, ri) => {
                const col = block.columns[0];
                const val = col ? row[col.key] : Object.values(row)[0];
                return <li key={ri}>{formatCellValue(val)}</li>;
              })}
            </ul>
          ) : (
            <div className="max-h-64 overflow-auto">
              <table className="w-full text-left text-xs">
                <thead>
                  <tr className="border-b border-border">
                    {block.columns.map((col) => (
                      <th key={col.key} className="px-2 py-1.5 font-medium text-muted">
                        {col.label}
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {block.rows.length === 0 ? (
                    <tr>
                      <td
                        colSpan={Math.max(1, block.columns.length)}
                        className="px-2 py-3 text-center text-muted"
                      >
                        Aucune ligne
                      </td>
                    </tr>
                  ) : (
                    block.rows.map((row, ri) => (
                      <tr key={ri} className="border-b border-border/50 last:border-0">
                        {block.columns.map((col) => (
                          <td key={col.key} className="px-2 py-1.5 text-foreground">
                            {formatCellValue(row[col.key])}
                          </td>
                        ))}
                      </tr>
                    ))
                  )}
                </tbody>
              </table>
            </div>
          )}
          {block.entityKey && onOpenEntity && (
            <div className="pointer-events-auto mt-2 flex justify-end">
              <Button
                type="button"
                size="sm"
                variant="secondary"
                onClick={() => onOpenEntity(block.entityKey!)}
              >
                <ExternalLink className="mr-1.5 h-3.5 w-3.5" />
                Ouvrir l&apos;écran
              </Button>
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
