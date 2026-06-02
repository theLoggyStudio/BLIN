import { useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ImagePlus, Loader2, Trash2 } from "lucide-react";
import { Button } from "@/items/Button";
import { FieldMessages } from "@/items/FieldMessages";
import {
  fileToBase64,
  parseImagesValue,
  useMediaSrc,
} from "@/engine/mediaUtils";
import type { ValidationIssue } from "@/types/screen";
import { cn } from "@/lib/utils";

interface ImagesFieldProps {
  label: string;
  value: unknown;
  onChange: (paths: string[]) => void;
  disabled?: boolean;
  screenKey: string;
  entityId: string;
  storageFolder: string;
  maxFiles?: number;
  accept?: string;
  fieldError?: ValidationIssue;
  fieldWarning?: ValidationIssue;
}

function GalleryThumb({ path, onRemove, disabled }: { path: string; onRemove: () => void; disabled?: boolean }) {
  const src = useMediaSrc(path);

  return (
    <div className="group relative h-24 w-24 overflow-hidden rounded-lg border border-border">
      {src ? (
        <img src={src} alt="" className="h-full w-full object-cover" />
      ) : (
        <div className="flex h-full items-center justify-center text-[10px] text-muted">…</div>
      )}
      {!disabled && (
        <button
          type="button"
          className="absolute right-1 top-1 rounded bg-background/90 p-1 opacity-0 transition group-hover:opacity-100"
          onClick={onRemove}
          aria-label="Supprimer"
        >
          <Trash2 className="h-3.5 w-3.5 text-primary" />
        </button>
      )}
    </div>
  );
}

export function ImagesField({
  label,
  value,
  onChange,
  disabled,
  screenKey,
  entityId,
  storageFolder,
  maxFiles = 12,
  accept = "image/*",
  fieldError,
  fieldWarning,
}: ImagesFieldProps) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [uploading, setUploading] = useState(false);
  const [localError, setLocalError] = useState<string | null>(null);
  const paths = parseImagesValue(value);
  const hasWarning = Boolean(fieldWarning && !fieldError);
  const atLimit = paths.length >= maxFiles;

  const upload = async (file: File) => {
    if (atLimit) return;
    setUploading(true);
    setLocalError(null);
    try {
      const dataBase64 = await fileToBase64(file);
      const relative = await invoke<string>("dda_media_upload", {
        payload: {
          screen_key: screenKey,
          entity_id: entityId,
          storage_folder: storageFolder,
          original_name: file.name,
          data_base64: dataBase64,
        },
      });
      onChange([...paths, relative]);
    } catch (e) {
      setLocalError(String(e));
    } finally {
      setUploading(false);
    }
  };

  const removeAt = async (index: number) => {
    const path = paths[index];
    if (!path) return;
    try {
      await invoke("dda_media_delete", {
        payload: { screen_key: screenKey, relative_path: path },
      });
      onChange(paths.filter((_, i) => i !== index));
    } catch (e) {
      setLocalError(String(e));
    }
  };

  return (
    <div
      className={cn(
        "flex flex-col gap-2",
        hasWarning && "rounded-lg ring-1 ring-amber-500/30 p-0.5 -m-0.5",
      )}
    >
      <div className="flex items-center justify-between gap-2">
        <span className="text-sm font-medium text-muted">{label}</span>
        <span className="text-xs text-muted">
          {paths.length} / {maxFiles}
        </span>
      </div>
      <div className="flex flex-wrap gap-2">
        {paths.map((path, index) => (
          <GalleryThumb
            key={`${path}-${index}`}
            path={path}
            disabled={disabled}
            onRemove={() => void removeAt(index)}
          />
        ))}
      </div>
      {!disabled && (
        <>
          <input
            ref={inputRef}
            type="file"
            accept={accept}
            multiple
            className="sr-only"
            onChange={(e) => {
              const files = Array.from(e.target.files ?? []);
              void (async () => {
                for (const file of files) {
                  if (paths.length >= maxFiles) break;
                  await upload(file);
                }
              })();
              e.target.value = "";
            }}
          />
          <Button
            type="button"
            variant="secondary"
            size="sm"
            disabled={uploading || atLimit}
            onClick={() => inputRef.current?.click()}
          >
            {uploading ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <ImagePlus className="h-4 w-4" />
            )}
            Ajouter à la galerie
          </Button>
        </>
      )}
      {localError && (
        <p className="text-xs text-primary" role="alert">
          {localError}
        </p>
      )}
      <FieldMessages error={fieldError} warning={fieldWarning} />
    </div>
  );
}
