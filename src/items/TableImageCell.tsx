import { useMediaSrc } from "@/engine/mediaUtils";

interface TableImageCellProps {
  relativePath: string | undefined;
}

export function TableImageCell({ relativePath }: TableImageCellProps) {
  const src = useMediaSrc(relativePath);

  if (!relativePath?.trim()) {
    return <span className="text-muted text-xs">—</span>;
  }

  if (!src) {
    return (
      <div className="h-10 w-14 rounded bg-background/80 border border-border animate-pulse" />
    );
  }

  return (
    <img
      src={src}
      alt=""
      className="h-10 w-14 rounded object-cover border border-border"
    />
  );
}
