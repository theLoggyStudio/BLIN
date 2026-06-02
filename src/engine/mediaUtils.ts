import { useEffect, useState } from "react";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";

export function parseImagesValue(value: unknown): string[] {
  if (Array.isArray(value)) {
    return value.filter((x): x is string => typeof x === "string" && x.length > 0);
  }
  if (typeof value === "string" && value.trim()) {
    try {
      const parsed = JSON.parse(value) as unknown;
      if (Array.isArray(parsed)) {
        return parsed.filter((x): x is string => typeof x === "string" && x.length > 0);
      }
    } catch {
      return [value];
    }
  }
  return [];
}

export async function resolveMediaSrc(relativePath: string): Promise<string> {
  const absolute = await invoke<string>("dda_media_absolute_path", {
    payload: { relative_path: relativePath },
  });
  return convertFileSrc(absolute);
}

export function useMediaSrc(relativePath: string | undefined): string | null {
  const [src, setSrc] = useState<string | null>(null);

  useEffect(() => {
    if (!relativePath?.trim()) {
      setSrc(null);
      return;
    }
    let cancelled = false;
    void resolveMediaSrc(relativePath)
      .then((url) => {
        if (!cancelled) setSrc(url);
      })
      .catch(() => {
        if (!cancelled) setSrc(null);
      });
    return () => {
      cancelled = true;
    };
  }, [relativePath]);

  return src;
}

export function fileToBase64(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const result = reader.result;
      if (typeof result !== "string") {
        reject(new Error("Lecture fichier impossible"));
        return;
      }
      const base64 = result.includes(",") ? result.split(",")[1] : result;
      resolve(base64 ?? "");
    };
    reader.onerror = () => reject(reader.error ?? new Error("Lecture fichier impossible"));
    reader.readAsDataURL(file);
  });
}

export function defaultStorageFolder(
  fieldFolder: string | undefined,
  screenFolders: string[] | undefined,
): string {
  if (fieldFolder) return fieldFolder;
  const photo = screenFolders?.find((f) => f.includes("photo"));
  return photo ?? screenFolders?.[0] ?? "photos";
}

export function mediaEntityId(
  recordId: string | undefined,
  uploadDraftId: string,
): string {
  if (recordId?.trim()) return recordId;
  return `_draft/${uploadDraftId}`;
}
