<script lang="ts">
  import { onMount } from 'svelte';
  import ConnectionPanel from './components/ConnectionPanel.svelte';
  import LobbyView from './components/LobbyView.svelte';
  import RoomView from './components/RoomView.svelte';
  import EventLog from './components/EventLog.svelte';
  import ReconnectPanel from './components/ReconnectPanel.svelte';
  import { PlayRoomSocket } from './lib/websocket';
  import { applyServerEvent, clearRuntimeState, pushLog, setConnected } from './lib/client-state';

  const socket = new PlayRoomSocket();

  onMount(() => {
    const unsubscribeEvent = socket.onEvent(applyServerEvent);
    const unsubscribeClose = socket.onClose((event) => {
      clearRuntimeState();
      pushLog('warning', `WebSocket closed (${event.code})`);
    });
    const unsubscribeError = socket.onError(() => {
      setConnected(false);
      pushLog('error', 'WebSocket error');
    });

    return () => {
      unsubscribeEvent();
      unsubscribeClose();
      unsubscribeError();
      socket.close();
    };
  });
</script>

<main class="app-shell">
  <ConnectionPanel {socket} />

  <div class="row g-3">
    <div class="col-12 col-xl-7">
      <LobbyView {socket} />
      <RoomView {socket} />
    </div>
    <div class="col-12 col-xl-5">
      <ReconnectPanel />
      <EventLog />
    </div>
  </div>
</main>
