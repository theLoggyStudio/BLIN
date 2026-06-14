/** Empilement global des modales (portails sur document.body). */
let modalStackDepth = 0;

export function pushModalStack(): number {
  modalStackDepth += 1;
  return modalStackDepth;
}

export function popModalStack(): void {
  modalStackDepth = Math.max(0, modalStackDepth - 1);
}

export function currentModalStackDepth(): number {
  return modalStackDepth;
}

export function modalZIndex(level: number): number {
  return 200 + level * 20;
}

/** z-index pour menus flottants (Select) : au-dessus du panneau courant, sous une modale empilée au-dessus. */
export function floatingMenuZIndex(): number {
  const depth = currentModalStackDepth();
  if (depth <= 0) return 100;
  if (depth === 1) return modalZIndex(1) + 5;
  return modalZIndex(depth - 1) + 15;
}
