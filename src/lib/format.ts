// Pure formatting helpers shared by every window. No DOM, no Tauri.

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/** Compact relative time: "now", "2 m", "3 h", "yesterday", "4 d", "12 Aug". */
export function relativeTime(timestampMs: number, now: number = Date.now()): string {
  const diff = Math.max(0, now - timestampMs);
  if (diff < MINUTE) return 'now';
  if (diff < HOUR) return `${Math.floor(diff / MINUTE)} m`;
  if (diff < DAY) return `${Math.floor(diff / HOUR)} h`;
  if (isYesterday(timestampMs, now)) return 'yesterday';
  if (diff < 7 * DAY) return `${Math.floor(diff / DAY)} d`;
  return new Date(timestampMs).toLocaleDateString(undefined, { day: 'numeric', month: 'short' });
}

function isYesterday(timestampMs: number, now: number): boolean {
  const then = new Date(timestampMs);
  const yesterday = new Date(now);
  yesterday.setDate(yesterday.getDate() - 1);
  return (
    then.getFullYear() === yesterday.getFullYear() &&
    then.getMonth() === yesterday.getMonth() &&
    then.getDate() === yesterday.getDate()
  );
}

const HOME_PREFIX = /^(?:\/Users\/[^/]+|\/home\/[^/]+|[A-Za-z]:\\Users\\[^\\]+)(?=[\\/]|$)/;

/** Replaces the user's home directory prefix with `~` (macOS, Linux and Windows layouts). */
export function shortenHome(path: string): string {
  return path.replace(HOME_PREFIX, '~');
}

/** Truncates the middle of a string with an ellipsis so both ends stay readable. */
export function middleEllipsis(text: string, max: number): string {
  if (max < 3 || text.length <= max) return text;
  const keep = max - 1;
  const head = Math.ceil(keep / 2);
  const tail = keep - head;
  return `${text.slice(0, head)}…${text.slice(text.length - tail)}`;
}

/**
 * Splits a path so the last segment can stay visible while CSS ellipsises the
 * leading part (`head` keeps its trailing separator).
 */
export function splitPath(path: string): { head: string; tail: string } {
  const trimmed = path.replace(/[\\/]+$/, '');
  const at = Math.max(trimmed.lastIndexOf('/'), trimmed.lastIndexOf('\\'));
  if (at < 0) return { head: '', tail: trimmed };
  return { head: trimmed.slice(0, at + 1), tail: trimmed.slice(at + 1) };
}

/** Plural helper: `count(2, "session")` → "2 sessions". */
export function count(n: number, noun: string, plural: string = `${noun}s`): string {
  return `${n} ${n === 1 ? noun : plural}`;
}
