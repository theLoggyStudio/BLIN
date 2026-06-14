/** Relâche le focus clavier (modale parente, autocomplete, etc.). */
export function blurActiveElement(): void {
  const el = document.activeElement;
  if (el instanceof HTMLElement) el.blur();
}
