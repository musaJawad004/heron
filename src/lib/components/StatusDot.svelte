<script lang="ts">
  import type { SessionStatus } from '$lib/types';

  interface Props {
    status: SessionStatus;
    /** Forces the attention colour and pulse (a hook event is waiting on the human). */
    attention?: boolean;
    size?: number;
  }
  let { status, attention = false, size = 8 }: Props = $props();

  const tone = $derived(
    attention || status === 'needs_input' || status === 'waiting'
      ? 'attention'
      : status === 'busy'
        ? 'busy'
        : 'idle',
  );
</script>

<span class="dot {tone}" style="--size: {size}px" aria-hidden="true"></span>

<style>
  .dot {
    flex: none;
    width: var(--size);
    height: var(--size);
    border-radius: 50%;
    background: var(--status-idle);
  }
  .busy {
    background: var(--status-busy);
  }
  .attention {
    background: var(--status-attention);
    animation: pulse 1.2s ease-out infinite;
  }
  @keyframes pulse {
    0% {
      box-shadow: 0 0 0 0 color-mix(in srgb, var(--status-attention) 55%, transparent);
    }
    100% {
      box-shadow: 0 0 0 5px color-mix(in srgb, var(--status-attention) 0%, transparent);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .attention {
      animation: none;
    }
  }
</style>
