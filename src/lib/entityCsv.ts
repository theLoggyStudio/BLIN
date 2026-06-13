import { invoke } from "@tauri-apps/api/core";
import { pushLoggyAlert } from "@/contexts/AlertContext";
import { resolveEntitySuccessMessage } from "@/lib/entitySuccessAlert";

interface ExportResponse {
  csv: string;
  file_name: string;
}

export async function exportEntityCsv(entityKey: string, entityLabel?: string): Promise<void> {
  const res = await invoke<ExportResponse>("entity_export_csv", {
    payload: { entity_key: entityKey },
  });
  const blob = new Blob(["\uFEFF", res.csv], { type: "text/csv;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = res.file_name;
  a.click();
  URL.revokeObjectURL(url);
  const msg = await resolveEntitySuccessMessage(entityKey, "export_csv", {
    file_name: res.file_name,
  });
  pushLoggyAlert(msg, "success");
}

export async function exportMultipleEntitiesCsv(
  entityKeys: string[],
  labels?: Record<string, string>,
): Promise<void> {
  for (const key of entityKeys) {
    await exportEntityCsv(key, labels?.[key] ?? key);
  }
}
