import { cn } from "@/lib/utils";
import type { VisionAnalyzeEntityOptions } from "@/types/ai";

export interface PendingQuestion {
  id: string;
  text: string;
  imageDataUrl?: string;
  visionEntityOptions?: VisionAnalyzeEntityOptions;
}

interface DashboardChatQueueProps {
  items: PendingQuestion[];
  maxItems: number;
  className?: string;
}

/** File d'attente des questions pendant que Loggy réfléchit. */
export function DashboardChatQueue({ items, maxItems, className }: DashboardChatQueueProps) {
  if (items.length === 0) return null;

  return (
    <div
      className={cn("dashboard-chat-queue", className)}
      role="status"
      aria-live="polite"
      aria-label="Questions en attente"
    >
      <p className="dashboard-chat-queue-title">
        En attente ({items.length}/{maxItems})
      </p>
      <ol className="dashboard-chat-queue-list">
        {items.map((item, index) => (
          <li key={item.id} className="dashboard-chat-queue-item">
            <span className="dashboard-chat-queue-index">{index + 1}.</span>
            <span className="dashboard-chat-queue-text">
              {item.text}
              {item.imageDataUrl ? " 🖼" : ""}
            </span>
          </li>
        ))}
      </ol>
    </div>
  );
}
