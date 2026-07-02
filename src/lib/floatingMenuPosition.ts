export const FLOATING_MENU_GAP = 4;
export const FLOATING_MENU_VIEWPORT_PADDING = 8;
export const SELECT_MENU_MAX_HEIGHT = 240;
export const RELATION_AUTOCOMPLETE_MENU_MAX_HEIGHT = 320;

export type FloatingMenuPlacement = "above" | "below";

export interface FloatingMenuStyle {
  position: "fixed";
  top?: number;
  bottom?: number;
  left: number;
  width: number;
  maxHeight: number;
  zIndex: number;
}

export interface ComputeFloatingMenuOptions {
  preferredMaxHeight?: number;
  gap?: number;
  viewportPadding?: number;
  /** Hauteur minimale utile avant de basculer au-dessus. */
  minVisibleHeight?: number;
}

/**
 * Calcule la position d'un menu contextuel (portail fixed) selon l'espace
 * disponible au-dessus / en dessous de l'ancre.
 */
export function computeFloatingMenuStyle(
  anchor: DOMRect,
  zIndex: number,
  options: ComputeFloatingMenuOptions = {},
): FloatingMenuStyle & { placement: FloatingMenuPlacement } {
  const gap = options.gap ?? FLOATING_MENU_GAP;
  const pad = options.viewportPadding ?? FLOATING_MENU_VIEWPORT_PADDING;
  const preferredMax = options.preferredMaxHeight ?? SELECT_MENU_MAX_HEIGHT;
  const minVisible = options.minVisibleHeight ?? 96;

  const vw = window.innerWidth;
  const vh = window.innerHeight;

  const width = Math.min(anchor.width, vw - pad * 2);
  const left = Math.min(Math.max(pad, anchor.left), Math.max(pad, vw - width - pad));

  const spaceBelow = vh - anchor.bottom - pad;
  const spaceAbove = anchor.top - pad;

  const maxBelow = Math.min(preferredMax, Math.max(minVisible, spaceBelow - gap));
  const maxAbove = Math.min(preferredMax, Math.max(minVisible, spaceAbove - gap));

  const canFitBelow = spaceBelow - gap >= minVisible;
  const canFitAbove = spaceAbove - gap >= minVisible;

  const preferBelow =
    spaceBelow >= preferredMax
    || (canFitBelow && spaceBelow >= spaceAbove)
    || !canFitAbove;

  if (preferBelow && canFitBelow) {
    return {
      position: "fixed",
      top: anchor.bottom + gap,
      left,
      width,
      maxHeight: maxBelow,
      zIndex,
      placement: "below",
    };
  }

  if (canFitAbove) {
    return {
      position: "fixed",
      bottom: vh - anchor.top + gap,
      left,
      width,
      maxHeight: maxAbove,
      zIndex,
      placement: "above",
    };
  }

  if (spaceBelow >= spaceAbove) {
    return {
      position: "fixed",
      top: anchor.bottom + gap,
      left,
      width,
      maxHeight: maxBelow,
      zIndex,
      placement: "below",
    };
  }

  return {
    position: "fixed",
    bottom: vh - anchor.top + gap,
    left,
    width,
    maxHeight: maxAbove,
    zIndex,
    placement: "above",
  };
}

/** Hauteur max du menu liaison entité (20rem capé à 70vh). */
export function relationAutocompleteMenuMaxHeight(): number {
  return Math.min(RELATION_AUTOCOMPLETE_MENU_MAX_HEIGHT, window.innerHeight * 0.7);
}
