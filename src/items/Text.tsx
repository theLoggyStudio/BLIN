import { cn } from "@/lib/utils";

interface TextProps {
  variant?: "title" | "subtitle" | "body" | "muted" | "label";
  as?: "h1" | "h2" | "p" | "span";
  children: React.ReactNode;
  className?: string;
}

const variants = {
  title: "text-2xl font-bold text-foreground",
  subtitle: "text-sm text-muted",
  body: "text-sm text-foreground",
  muted: "text-xs text-muted",
  label: "text-sm font-medium text-foreground",
};

export function Text({
  variant = "body",
  as: Tag = "p",
  children,
  className,
}: TextProps) {
  return <Tag className={cn(variants[variant], className)}>{children}</Tag>;
}
