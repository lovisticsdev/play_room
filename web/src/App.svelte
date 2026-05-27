<script lang="ts">
  import { onMount } from 'svelte';
  import { initDispatcher } from './lib/network/dispatcher';
  import { roomStore } from './lib/stores/room';

  import ConnectionPanel from './components/layout/ConnectionPanel.svelte';
  import ReconnectPanel from './components/layout/ReconnectPanel.svelte';
  import EventLog from './components/layout/EventLog.svelte';
  import LobbyView from './components/lobby/LobbyView.svelte';
  import RoomView from './components/room/RoomView.svelte';

  onMount(() => {
    const cleanupDispatcher = initDispatcher();

    return () => {
      cleanupDispatcher();
    };
  });
</script>

<main class="app-shell">
  <ConnectionPanel />

  <div class="row g-3">
    <div class="col-12 col-xl-8">
      {#if $roomStore.currentRoom}
        <RoomView />
      {/if}
      <LobbyView />
    </div>
    <div class="col-12 col-xl-4">
      <ReconnectPanel />
      <EventLog />
    </div>
  </div>
</main>
