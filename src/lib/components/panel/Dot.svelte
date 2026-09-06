<script lang="ts">
  import type { PanelKind } from '$lib/panel/sessions';

  let { kind, size = 8, pulse = false }: { kind: PanelKind; size?: number; pulse?: boolean } = $props();
</script>

<span class="dot {kind}" class:pulse style:--size="{size}px" aria-hidden="true"></span>

<style>
  .dot {
    display: inline-block;
    flex: none;
    width: var(--size);
    height: var(--size);
    border-radius: 50%;
    background: var(--status-idle);
    pointer-events: none;
  }
  .busy {
    background: var(--status-busy);
  }
  .attention {
    background: var(--status-attention);
  }
  .pulse {
    animation: pulse 1.2s ease-in-out infinite;
  }
  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
      transform: scale(1);
    }
    50% {
      opacity: 0.45;
      transform: scale(0.8);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .pulse {
      animation: none;
    }
  }
</style>
