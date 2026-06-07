import { useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ImagePlus, Loader2, Trash2 } from "lucide-react";
import { Button } from "@/items/Button";
import { Alert, FieldMessages } from "@/items/Alert";
import {
  fileToBase64,
  useMediaSrc,
} from "@/engine/mediaUtils";
import type { ValidationIssue } from "@/types/screen";
import { cn } from "@/lib/utils";

interface ImageFieldProps {
  label: string;
  value: string;
  onChange: (path: string) => void;
  disabled?: boolean;
  screenKey: string;
  entityId: string;
  storageFolder: string;
  accept?: string;
  fieldError?: ValidationIssue;
  fieldWarning?: ValidationIssue;
}

export function ImageField({
  label,
  value,
  onChange,
  disabled,
  screenKey,
  entityId,
  storageFolder,
  accept = "image/*",
  fieldError,
  fieldWarning,
}: ImageFieldProps) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [uploading, setUploading] = useState(false);
  const [localError, setLocalError] = useState<string | null>(null);
  const previewSrc = useMediaSrc(value || undefined);
  const hasWarning = Boolean(fieldWarning && !fieldError);

  const upload = async (file: File) => {
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
      if (value && value !== relative) {
        await invoke("dda_media_delete", {
          payload: { screen_key: screenKey, relative_path: value },
        }).catch(() => undefined);
      }
      onChange(relative);
    } catch (e) {
      setLocalError(String(e));
    } finally {
      setUploading(false);
    }
  };

  const remove = async () => {
    if (!value) return;
    setLocalError(null);
    try {
      await invoke("dda_media_delete", {
        payload: { screen_key: screenKey, relative_path: value },
      });
      onChange("");
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
      <span className="text-sm font-medium text-muted">{label}</span>
      <div className="flex flex-wrap items-start gap-4">
        <div
          className={cn(
            "relative h-36 w-48 shrink-0 overflow-hidden rounded-lg border border-border bg-background/60",
            fieldError && "border-primary",
          )}
        >
          {previewSrc ? (
            <img src={previewSrc} alt="" className="h-full w-full object-cover" />
          ) : (
            <div className="flex h-full items-center justify-center text-xs text-muted">
              Aucune image
            </div>
          )}
          {uploading && (
            <div className="absolute inset-0 flex items-center justify-center bg-background/80">
              <Loader2 className="h-6 w-6 animate-spin text-secondary" />
            </div>
          )}
        </div>
        {!disabled && (
          <div className="flex flex-col gap-2">
            <input
              ref={inputRef}
              type="file"
              accept={accept}
              className="sr-only"
              onChange={(e) => {
                const file = e.target.files?.[0];
                if (file) void upload(file);
                e.target.value = "";
              }}
            />
            <Button
              type="button"
              variant="secondary"
              size="sm"
              disabled={uploading}
              onClick={() => inputRef.current?.click()}
            >
              <ImagePlus className="h-4 w-4" />
              {value ? "Remplacer" : "Ajouter une photo"}
            </Button>
            {value && (
              <Button
                type="button"
                variant="ghost"
                size="sm"
                disabled={uploading}
                onClick={() => void remove()}
              >
                <Trash2 className="h-4 w-4 text-primary" />
                Supprimer
              </Button>
            )}
          </div>
        )}
      </div>
      {localError && <Alert variant="danger" size="field" message={localError} />}
      <FieldMessages error={fieldError} warning={fieldWarning} />
    </div>
  );
}
