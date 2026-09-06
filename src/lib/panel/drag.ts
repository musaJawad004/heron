// Drag-end detection for a window that never takes focus. Tauri starts the
// native move on mousedown over a `data-tauri-drag-region` element; the
// webview does not always see the matching mouseup while the OS owns the
// drag, so the first mouse event that arrives with no button held (mouseup,
// or the next mousemove) counts as the end. `onEnd` fires only when the
// pointer actually travelled, so a plain click on the header stays a click
// and does not overwrite the saved origin.
import type { Attachment } from 'svelte/attachments';

export function dragEnd(onEnd: () => void, threshold = 3): Attachment<HTMLElement> {
  return (el) => {
    let start: { x: number; y: number } | null = null;

    const down = (e: MouseEvent) => {
      if (e.button !== 0) return;
      const target = e.target;
      // Tauri only honours the attribute on the exact element under the cursor.
      if (!(target instanceof Element) || !target.hasAttribute('data-tauri-drag-region')) return;
      start = { x: e.screenX, y: e.screenY };
    };
    const finish = (e: MouseEvent) => {
      if (!start) return;
      const moved = Math.abs(e.screenX - start.x) + Math.abs(e.screenY - start.y) >= threshold;
      start = null;
      if (moved) onEnd();
    };
    const move = (e: MouseEvent) => {
      if (start && e.buttons === 0) finish(e);
    };

    el.addEventListener('mousedown', down);
    window.addEventListener('mouseup', finish, true);
    window.addEventListener('mousemove', move, true);
    return () => {
      el.removeEventListener('mousedown', down);
      window.removeEventListener('mouseup', finish, true);
      window.removeEventListener('mousemove', move, true);
    };
  };
}
