import { invoke } from "@tauri-apps/api/core";
import type { EntityAccessInfo } from "@/types/entity";

export async function fetchEntityAccess(entityKey: string): Promise<EntityAccessInfo> {
  return invoke<EntityAccessInfo>("entity_check_access", {
    payload: { entity_key: entityKey },
  });
}

export function buildEntityAccessDeniedFallback(info: EntityAccessInfo): string {
  if (info.contact_role_names.length === 0) {
    return `Je ne peux pas ouvrir « ${info.entity_label} » : vous n'avez pas les droits nécessaires. Aucun rôle n'est configuré pour cette entité — contactez votre administrateur.`;
  }
  if (info.contact_role_names.length === 1) {
    return `Je ne peux pas ouvrir « ${info.entity_label} » car vous n'avez pas les droits nécessaires. Contactez une personne ayant le rôle « ${info.contact_role_names[0]} ».`;
  }
  return `Je ne peux pas ouvrir « ${info.entity_label} » car vous n'avez pas les droits nécessaires. Contactez une personne ayant l'un de ces rôles : ${info.contact_role_names.join(", ")}.`;
}

export async function fetchEntityAccessDeniedMessage(
  entityKey: string,
  userMessage: string,
  info?: EntityAccessInfo,
): Promise<string> {
  const access = info ?? (await fetchEntityAccess(entityKey));
  try {
    return await invoke<string>("ai_entity_access_denied", {
      payload: {
        user_message: userMessage,
        entity_key: entityKey,
        entity_label: access.entity_label,
        contact_role_names: access.contact_role_names,
      },
    });
  } catch {
    return buildEntityAccessDeniedFallback(access);
  }
}
