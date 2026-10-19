<script lang="ts">
  import { appState } from './lib/stores/appState.svelte';
  import Header from './components/Header.svelte';
  import ToastContainer from './components/ToastContainer.svelte';
  import BottomBar from './components/BottomBar.svelte';
  import QueueTab from './components/tabs/QueueTab.svelte';
  import ClipsTab from './components/tabs/ClipsTab.svelte';
  import SettingsTab from './components/tabs/SettingsTab.svelte';
</script>

<div class="h-screen w-screen flex flex-col bg-zinc-950 text-zinc-100 overflow-hidden font-sans">
  <!-- Top Navigation & System Status Badges -->
  <Header />

  <!-- Main View Area: Selected Tab View -->
  <main class="flex-1 overflow-hidden relative">
    {#if appState.activeTab === 'queue'}
      <QueueTab />
    {:else if appState.activeTab === 'clips'}
      <ClipsTab />
    {:else if appState.activeTab === 'settings'}
      <SettingsTab />
    {/if}
  </main>

  <!-- Bottom Bar: Status, Progress and Log Console Drawer -->
  <BottomBar />

  <!-- Floating Toast Notifications -->
  <ToastContainer />
</div>
