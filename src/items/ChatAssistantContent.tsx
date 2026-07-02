import { useState } from "react";
import { ClipboardCopy, Check } from "lucide-react";
import { ChatLinkifiedText } from "@/items/ChatLinkifiedText";
import { Button } from "@/items/Button";
import { cn } from "@/lib/utils";

interface ChatAssistantContentProps {
  text: string;
  className?: string;
}

type Segment =
  | { kind: "text"; value: string }
  | { kind: "code"; lang: string; value: string };

function splitMarkdownCode(text: string): Segment[] {
  const segments: Segment[] = [];
  const re = /```(\w*)\n?([\s\S]*?)```/g;
  let last = 0;
  for (const match of text.matchAll(re)) {
    const index = match.index ?? 0;
    if (index > last) {
      segments.push({ kind: "text", value: text.slice(last, index) });
    }
    segments.push({
      kind: "code",
      lang: match[1] || "json",
      value: match[2].trim(),
    });
    last = index + match[0].length;
  }
  if (last < text.length) {
    segments.push({ kind: "text", value: text.slice(last) });
  }
  if (segments.length === 0) {
    segments.push({ kind: "text", value: text });
  }
  return segments;
}

function CopyCodeButton({ code }: { code: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <Button
      type="button"
      size="sm"
      variant="ghost"
      className="h-7 gap-1 px-2 text-xs"
      onClick={() => {
        void navigator.clipboard.writeText(code).then(() => {
          setCopied(true);
          window.setTimeout(() => setCopied(false), 1600);
        });
      }}
    >
      {copied ? <Check className="h-3.5 w-3.5" /> : <ClipboardCopy className="h-3.5 w-3.5" />}
      {copied ? "Copié" : "Copier le JSON"}
    </Button>
  );
}

/** Réponse assistant avec blocs de code copiables (analyse image, etc.). */
export function ChatAssistantContent({ text, className }: ChatAssistantContentProps) {
  const segments = splitMarkdownCode(text);
  return (
    <div className={cn("space-y-3", className)}>
      {segments.map((seg, i) => {
        if (seg.kind === "text") {
          const trimmed = seg.value.trim();
          if (!trimmed) return null;
          return (
            <ChatLinkifiedText
              key={`t-${i}`}
              text={trimmed}
              className="loggy-chat-text whitespace-pre-wrap"
            />
          );
        }
        return (
          <div key={`c-${i}`} className="chat-code-block-wrap">
            <div className="chat-code-block-header">
              <span className="text-xs uppercase tracking-wide text-muted">{seg.lang || "code"}</span>
              <CopyCodeButton code={seg.value} />
            </div>
            <pre className="chat-code-block">
              <code>{seg.value}</code>
            </pre>
          </div>
        );
      })}
    </div>
  );
}
