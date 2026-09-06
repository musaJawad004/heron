// Optimistic settings writes: mutate the local snapshot so controls respond
// instantly, then persist through Rust (which pushes the authoritative snapshot).
import { heron } from '$lib/stores.svelte';
import type { Settings } from '$lib/types';

export function setSetting<K extends keyof Settings>(key: K, value: Settings[K]): Promise<void> {
  heron.snapshot.settings[key] = value;
  const patch: Partial<Settings> = {};
  patch[key] = value;
  return heron.update(patch);
}
