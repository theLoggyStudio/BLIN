import { invoke } from "@tauri-apps/api/core";

export type EntitySuccessAction =
  | "create"
  | "create_named"
  | "create_lines"
  | "update"
  | "update_named"
  | "update_lines"
  | "delete"
  | "delete_named"
  | "import_ok"
  | "import_partial"
  | "export_csv"
  | "export_pdf_row"
  | "export_pdf_list"
  | "signature_ok"
  | "signature_refuse";

export type EntitySuccessParams = Record<string, string | number | undefined>;

function stringifyParams(params?: EntitySuccessParams): Record<string, string> {
  if (!params) return {};
  const out: Record<string, string> = {};
  for (const [k, v] of Object.entries(params)) {
    if (v !== undefined && v !== null) out[k] = String(v);
  }
  return out;
}

/** Résout le message succès depuis le catalogue trigger (`dda/success/{entity}.json`). */
export async function resolveEntitySuccessMessage(
  entityKey: string,
  action: EntitySuccessAction,
  params?: EntitySuccessParams,
): Promise<string> {
  const res = await invoke<{ message: string }>("entity_success_message", {
    payload: {
      entity_key: entityKey,
      action,
      params: stringifyParams(params),
    },
  });
  return res.message;
}

/** Affiche un toast succès Loggy (personnification IA incluse via AlertContext). */
export function notifyEntitySuccess(
  showSuccess: (message: string) => void,
  entityKey: string,
  action: EntitySuccessAction,
  params?: EntitySuccessParams,
): void {
  void resolveEntitySuccessMessage(entityKey, action, params)
    .then(showSuccess)
    .catch(() => {
      showSuccess(`Opération terminée pour « ${entityKey} ».`);
    });
}
