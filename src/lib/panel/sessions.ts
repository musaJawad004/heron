// Ordering and wording for the panel's session rows. Kept apart from the
// components so the sort can be reasoned about (and tested) on its own.
import { needsAttention, type HookEvent, type Session } from '$lib/types';

export type PanelKind = 'attention' | 'busy' | 'idle';

const rank: Record<PanelKind, number> = { attention: 0, busy: 1, idle: 2 };

/** Same rule the store uses for `attentionCount`, so header and rows agree. */
export function panelKind(s: Session, attention: Record<string, HookEvent>): PanelKind {
  if (needsAttention(s) || s.id in attention) return 'attention';
  if (s.status === 'busy') return 'busy';
  return 'idle';
}

/** Attention first, then busy, then idle; stable inside each group. */
export function sortForPanel(sessions: Session[], attention: Record<string, HookEvent>): Session[] {
  return [...sessions].sort((a, b) => rank[panelKind(a, attention)] - rank[panelKind(b, attention)]);
}

export function statusWord(kind: PanelKind): string {
  switch (kind) {
    case 'attention':
      return 'Needs you';
    case 'busy':
      return 'Working';
    default:
      return 'Idle';
  }
}
