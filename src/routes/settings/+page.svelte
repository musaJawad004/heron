<script lang="ts">
  import { onMount } from 'svelte';
  import { heron } from '$lib/stores.svelte';
  import { isMac } from '$lib/keys';
  import Tabs, { type TabItem } from '$lib/components/Tabs.svelte';
  import Bell from '$lib/components/icons/Bell.svelte';
  import Gear from '$lib/components/icons/Gear.svelte';
  import Info from '$lib/components/icons/Info.svelte';
  import Link from '$lib/components/icons/Link.svelte';
  import Panel from '$lib/components/icons/Panel.svelte';
  import AboutTab from '$lib/components/settings/AboutTab.svelte';
  import GeneralTab from '$lib/components/settings/GeneralTab.svelte';
  import HooksTab from '$lib/components/settings/HooksTab.svelte';
  import NotificationsTab from '$lib/components/settings/NotificationsTab.svelte';
  import PanelTab from '$lib/components/settings/PanelTab.svelte';

  const tabs = [
    { id: 'general', label: 'General', icon: Gear },
    { id: 'panel', label: 'Panel', icon: Panel },
    { id: 'notifications', label: 'Notifications', icon: Bell },
    { id: 'hooks', label: 'Hooks', icon: Link },
    { id: 'about', label: 'About', icon: Info },
  ] as const satisfies readonly TabItem[];
  type TabId = (typeof tabs)[number]['id'];
  const isTabId = (v: string | null): v is TabId => tabs.some((t) => t.id === v);

  let tab = $state<TabId>('general');
  const title = $derived(tabs.find((t) => t.id === tab)?.label ?? '');

  onMount(() => {
    heron.init();
    const wanted = new URLSearchParams(location.search).get('tab');
    if (isTabId(wanted)) tab = wanted;
  });

  function select(id: string) {
    if (isTabId(id)) tab = id;
  }
</script>

<div class="settings">
  <!-- On macOS this strip sits under the overlay title bar and doubles as the window title. -->
  <header class="titlebar" class:overlay={isMac} data-tauri-drag-region>
    <h1 data-tauri-drag-region>{title}</h1>
  </header>
  <div class="body">
    <nav class="sidebar" aria-label="Settings sections">
      <Tabs items={tabs} value={tab} onchange={select} />
    </nav>
    <div class="content" id="tabpanel-{tab}" role="tabpanel" aria-labelledby="tab-{tab}">
      {#if tab === 'general'}
        <GeneralTab />
      {:else if tab === 'panel'}
        <PanelTab />
      {:else if tab === 'notifications'}
        <NotificationsTab onShowHooks={() => (tab = 'hooks')} />
      {:else if tab === 'hooks'}
        <HooksTab />
      {:else}
        <AboutTab />
      {/if}
    </div>
  </div>
</div>

<style>
  .settings {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--surface-solid);
  }
  .titlebar {
    display: flex;
    align-items: center;
    justify-content: center;
    flex: none;
    height: 28px;
  }
  .overlay {
    /* Keep the title clear of the traffic lights on the left. */
    padding-left: 72px;
    padding-right: 72px;
  }
  h1 {
    margin: 0;
    font-size: var(--text-md);
    font-weight: 600;
    line-height: 16px;
  }
  .body {
    display: flex;
    flex: 1 1 auto;
    min-height: 0;
  }
  .sidebar {
    flex: none;
    width: 140px;
    padding: var(--space-2);
    border-right: 0.5px solid var(--separator);
  }
  .content {
    flex: 1 1 auto;
    min-width: 0;
    padding: var(--space-3) var(--space-5) var(--space-4);
    overflow-y: auto;
  }
  .content::-webkit-scrollbar {
    width: 6px;
  }
  .content::-webkit-scrollbar-thumb {
    border-radius: 3px;
    background: var(--text-tertiary);
  }
  :global(:focus-visible) {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
</style>
