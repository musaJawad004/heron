// Reports the capsule's rendered size to Rust so the window always fits it.
// The number sent is the border box from getBoundingClientRect() rounded up
// plus a 2 px safety margin; the CSS box-shadow is *not* included (the OS
// window shadow covers that).
//
// Reports are throttled to one per 30 ms with a leading and a trailing call,
// so a 150 ms slide moves the window along with the content instead of
// clipping it until the animation settles. Identical sizes are not re-sent.
import type { Attachment } from 'svelte/attachments';

const INTERVAL_MS = 30;

export function measure(
  report: (width: number, height: number) => void,
  safety = 2,
): Attachment<HTMLElement> {
  return (el) => {
    let timer: ReturnType<typeof setTimeout> | null = null;
    let last = '';

    const send = () => {
      const rect = el.getBoundingClientRect();
      const width = Math.ceil(rect.width) + safety;
      const height = Math.ceil(rect.height) + safety;
      const key = `${width}x${height}`;
      if (key === last) return;
      last = key;
      report(width, height);
    };
    const onChange = () => {
      if (timer !== null) return; // a trailing send is already scheduled
      send();
      timer = setTimeout(() => {
        timer = null;
        send();
      }, INTERVAL_MS);
    };

    const observer = new ResizeObserver(onChange);
    observer.observe(el);
    // System fonts can swap in after first paint and change the text width.
    document.fonts?.ready.then(onChange);

    return () => {
      observer.disconnect();
      if (timer !== null) clearTimeout(timer);
    };
  };
}
