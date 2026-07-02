import type { AiRuntimeStatus } from "@/types/ai";

export const AI_INSTALL_PROGRESS_EVENT = "ai-install-progress";
export const AI_INSTALL_DONE_EVENT = "ai-install-done";
export const AI_INSTALL_ERROR_EVENT = "ai-install-error";

export async function fetchAiRuntimeStatus(): Promise<AiRuntimeStatus> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<AiRuntimeStatus>("ai_runtime_status");
}

export async function startAiRuntimeInstall(installDir: string): Promise<void> {
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("ai_runtime_install", { payload: { install_dir: installDir } });
}

export async function pickAiInstallDirectory(defaultPath?: string): Promise<string | null> {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({
      directory: true,
      multiple: false,
      defaultPath: defaultPath,
      title: "Dossier d'installation de Loggy (IA locale)",
    });
    if (selected === null || Array.isArray(selected)) return null;
    return selected;
  } catch {
    return null;
  }
}
