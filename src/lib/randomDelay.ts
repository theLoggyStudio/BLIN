/** Délai aléatoire entre min et max (millisecondes, inclus). */
export function randomDelayMs(minMs: number, maxMs: number): Promise<void> {
  const lo = Math.min(minMs, maxMs);
  const hi = Math.max(minMs, maxMs);
  const ms = lo + Math.floor(Math.random() * (hi - lo + 1));
  return new Promise((resolve) => setTimeout(resolve, ms));
}
