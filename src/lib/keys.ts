// Keyboard helpers: platform-aware modifier labels and shortcut matching.

export const isMac: boolean =
  typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);

/** "⌘" on macOS, "Ctrl" elsewhere. */
export const modLabel: string = isMac ? '⌘' : 'Ctrl';

/** Human-readable shortcut for tooltips: `shortcut("N")` → "⌘N" or "Ctrl+N". */
export function shortcut(key: string): string {
  return isMac ? `${modLabel}${key}` : `${modLabel}+${key}`;
}

/** True when the platform's primary modifier (⌘ or Ctrl) is held and no other modifier is. */
export function isMod(e: KeyboardEvent): boolean {
  const primary = isMac ? e.metaKey : e.ctrlKey;
  const other = isMac ? e.ctrlKey : e.metaKey;
  return primary && !other && !e.altKey && !e.shiftKey;
}

/** True when the event comes from a text control, where list shortcuts must not fire. */
export function isTyping(e: KeyboardEvent): boolean {
  const el = e.target;
  if (!(el instanceof HTMLElement)) return false;
  return (
    el.isContentEditable || el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.tagName === 'SELECT'
  );
}

/** Moves an index by `delta`, clamping to the list (no wrap, like native lists). */
export function stepIndex(current: number, delta: number, length: number): number {
  if (length === 0) return -1;
  if (current < 0) return delta > 0 ? 0 : length - 1;
  return Math.min(length - 1, Math.max(0, current + delta));
}

/** Focuses the element `delta` places away from the focused one in `items`. Returns true if it moved. */
export function focusSibling(items: HTMLElement[], delta: number): boolean {
  const active = document.activeElement;
  const current = items.findIndex((el) => el === active || el.contains(active));
  const next = items[stepIndex(current, delta, items.length)];
  if (!next) return false;
  next.focus();
  return true;
}
