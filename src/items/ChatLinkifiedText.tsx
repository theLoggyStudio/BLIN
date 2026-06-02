import { openUrl } from "@tauri-apps/plugin-opener";

const URL_PATTERN = /https?:\/\/[^\s<>\[\]()]+/g;

interface ChatLinkifiedTextProps {
  text: string;
  className?: string;
}

/** Texte de chat avec URLs http(s) cliquables (ouvre le navigateur système). */
export function ChatLinkifiedText({ text, className }: ChatLinkifiedTextProps) {
  const nodes: React.ReactNode[] = [];
  let lastIndex = 0;

  for (const match of text.matchAll(URL_PATTERN)) {
    const url = match[0];
    const index = match.index ?? 0;
    if (index > lastIndex) {
      nodes.push(text.slice(lastIndex, index));
    }
    nodes.push(
      <a
        key={`${index}-${url}`}
        href={url}
        className="chat-source-link"
        title={url}
        onClick={(e) => {
          e.preventDefault();
          void openUrl(url);
        }}
      >
        {url}
      </a>,
    );
    lastIndex = index + url.length;
  }

  if (lastIndex < text.length) {
    nodes.push(text.slice(lastIndex));
  }

  return <p className={className}>{nodes.length > 0 ? nodes : text}</p>;
}
