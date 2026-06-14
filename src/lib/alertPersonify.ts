import { invoke } from "@tauri-apps/api/core";
import type { AlertVariant } from "@/items/Alert";

const MAX_PERSONIFY_LEN = 800;

function ensureSentence(text: string): string {
  const t = text.trim().replace(/\s+/g, " ");
  if (!t) return t;
  const lower = t.charAt(0).toLowerCase() + t.slice(1);
  return /[.!?…]$/.test(lower) ? lower : `${lower}.`;
}

/** Reformulation locale si le modèle IA est indisponible ou renvoie le texte brut. */
export function fallbackExpressiveAlert(message: string, variant: AlertVariant): string {
  const t = message.trim();
  if (!t) return message;

  if (/^PDF généré/i.test(t)) {
    return "C'est bon, j'ai généré le PDF pour toi. Tu peux le récupérer dans tes téléchargements.";
  }

  if (/^Enregistrement créé pour/i.test(t)) {
    return `Parfait, j'ai bien enregistré la nouvelle fiche. ${ensureSentence(t.replace(/^Enregistrement créé pour/i, "C'est en place pour"))}`;
  }
  if (/^Fiche « .+ » créée pour/i.test(t)) {
    return t.replace(
      /^Fiche « (.+) » créée pour « (.+) »\./,
      "Je viens de créer la fiche « $1 » dans $2. Tu peux la consulter dans la liste.",
    );
  }
  if (/^Fiche « .+ » mise à jour pour/i.test(t)) {
    return t.replace(
      /^Fiche « (.+) » mise à jour pour « (.+) »\./,
      "J'ai mis à jour la fiche « $1 » pour $2. Les changements sont bien enregistrés.",
    );
  }
  if (/^Enregistrement supprimé pour/i.test(t)) {
    return t.replace(
      /^Enregistrement supprimé pour « (.+) »\./,
      "C'est fait, j'ai supprimé l'enregistrement pour $1.",
    );
  }
  if (/^Export CSV terminé/i.test(t)) {
    return t.replace(
      /^Export CSV terminé pour « (.+) » \((.+)\)\./,
      "L'export CSV de $1 est prêt ($2). Tu peux ouvrir le fichier téléchargé.",
    );
  }
  if (/^PDF fiche généré/i.test(t)) {
    return "C'est bon, j'ai généré le PDF de la fiche. Tu peux le récupérer dans tes téléchargements.";
  }
  if (/^PDF liste généré/i.test(t)) {
    return t.replace(
      /^PDF liste généré pour « (.+) »\./,
      "J'ai généré le PDF de la liste « $1 ». Tu peux le télécharger tout de suite.",
    );
  }

  const importPartial = t.match(
    /^Import CSV partiel pour « (.+) » : (\d+) créé\(s\), (\d+) mis à jour, (\d+) erreur\(s\)\./,
  );
  if (importPartial) {
    const [, entity, created, updated, errors] = importPartial;
    return `L'import des fiches « ${entity} » est terminé avec des réserves : ${created} création(s), ${updated} mise(s) à jour, et ${errors} ligne(s) en erreur.`;
  }

  if (/^Import réussi/i.test(t) || /^Import CSV réussi/i.test(t)) {
    const importMatch = t.match(
      /^Import (?:CSV )?réussi pour « (.+) » : (\d+) créé\(s\), (\d+) mis à jour\./,
    );
    if (importMatch) {
      const [, entity, created, updated] = importMatch;
      const nCreated = Number(created);
      const nUpdated = Number(updated);
      const parts = [`J'ai terminé l'import des fiches « ${entity} ».`];
      if (nCreated > 0) {
        parts.push(
          `${nCreated} nouvel${nCreated > 1 ? "les" : ""} enregistrement${nCreated > 1 ? "s" : ""} ${nCreated > 1 ? "ont été créés" : "a été créé"}.`,
        );
      }
      if (nUpdated > 0) {
        parts.push(
          `${nUpdated} fiche${nUpdated > 1 ? "s" : ""} ${nUpdated > 1 ? "ont été mises" : "a été mise"} à jour.`,
        );
      } else if (nCreated > 0) {
        parts.push("Aucune mise à jour n'était nécessaire.");
      }
      return parts.join(" ");
    }
  }

  const taskQuoted = t.match(/^Tâche « (.+) » (créée|mise à jour|supprimée)/);
  if (taskQuoted) {
    const [, title, action] = taskQuoted;
    if (action === "créée") {
      return `Je viens de créer la tâche « ${title} ». Tu la retrouves dans ta liste quand tu veux.`;
    }
    if (action === "mise à jour") {
      return `J'ai mis à jour la tâche « ${title} ». Les changements sont enregistrés.`;
    }
    if (action === "supprimée") {
      return `J'ai supprimé la tâche « ${title} ». Elle n'apparaît plus dans ta liste.`;
    }
  }

  if (/^Tâche créée/i.test(t)) {
    return "Je viens de créer la tâche. Elle est prête dans ta liste, tu peux la consulter quand tu veux.";
  }
  if (/^Tâche.*mise à jour/i.test(t)) {
    return "J'ai mis à jour la tâche. Les modifications sont bien enregistrées.";
  }
  if (/^Tâche.*supprimée/i.test(t)) {
    return "J'ai supprimé la tâche. Elle n'apparaît plus dans ta liste.";
  }

  if (/^PDF liste généré/i.test(t)) {
    return t.replace(
      /^PDF liste généré pour « (.+) »\./,
      "J'ai généré le PDF de la liste « $1 ». Tu peux le télécharger tout de suite.",
    );
  }

  if (/^Échec/i.test(t) || /^Impossible/i.test(t)) {
    return `Je n'ai pas réussi à aller au bout : ${ensureSentence(t.replace(/^(Échec|Impossible)[^:]*:?\s*/i, ""))}`;
  }

  if (/^Identifiants invalides/i.test(t)) {
    return "Je n'ai pas reconnu cet e-mail ou ce mot de passe. Vérifie les identifiants et réessaie.";
  }

  if (/^Connexion —/i.test(t)) {
    return t.replace(/^Connexion —\s*/i, "").trim() || t;
  }

  switch (variant) {
    case "success":
      return `Parfait ! ${ensureSentence(t)}`;
    case "danger":
      return `Attention, il y a un souci : ${ensureSentence(t)}`;
    case "warning":
      return `Je te signale un point à surveiller : ${ensureSentence(t)}`;
    default:
      return ensureSentence(t.charAt(0).toUpperCase() + t.slice(1));
  }
}

function isRoboticEcho(original: string, rewritten: string): boolean {
  const a = original.trim().toLowerCase();
  const b = rewritten.trim().toLowerCase();
  return b === a || b.length < 20;
}

export async function personifyAlertMessage(
  message: string,
  variant: AlertVariant,
): Promise<string> {
  const trimmed = message.trim();
  if (!trimmed || trimmed.length > MAX_PERSONIFY_LEN) return message;
  try {
    const rewritten = await invoke<string>("ai_alert_personify", {
      payload: { message: trimmed, variant },
    });
    const out = rewritten.trim();
    if (!out || isRoboticEcho(trimmed, out)) {
      return fallbackExpressiveAlert(trimmed, variant);
    }
    return out;
  } catch {
    return fallbackExpressiveAlert(trimmed, variant);
  }
}

/** Tout texte affiché via Alert passe par Loggy (sauf vide ou trop long). */
export function shouldPersonifyAlertText(text: string | undefined): boolean {
  const t = text?.trim();
  return Boolean(t && t.length <= MAX_PERSONIFY_LEN);
}

/** Rappel de tâche planifiée — réécriture Loggy (2–3 phrases). */
export async function personifyTaskReminderMessage(message: string): Promise<string> {
  const trimmed = message.trim();
  if (!trimmed || trimmed.length > MAX_PERSONIFY_LEN) return message;
  try {
    const rewritten = await invoke<string>("ai_task_reminder_personify", {
      payload: { message: trimmed },
    });
    const out = rewritten.trim();
    if (!out || isRoboticEcho(trimmed, out)) {
      return fallbackExpressiveAlert(trimmed, "warning");
    }
    return out;
  } catch {
    return fallbackExpressiveAlert(trimmed, "warning");
  }
}
