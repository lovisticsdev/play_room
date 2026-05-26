<script lang="ts">
  import type { PlayRoomSocket } from '../lib/websocket';
  import { appState, setConnected, setConnecting, setConnectionError, pushLog, applyServerResult } from '../lib/client-state';
  import { connectRequest } from '../lib/commands';

  export let socket: PlayRoomSocket;

  let serverUrl = 'ws://127.0.0.1:7878/ws';
  let displayName = 'alice';
  let reconnectToken = '';


  async function connect(): Promise<void> {
    const name = displayName.trim();
    if (!name) {
      setConnectionError('Display name is required');
      return;
    }

    setConnecting(true);
    try {
      await socket.connect(serverUrl.trim());
      setConnected(true);
      pushLog('success', `WebSocket open: ${serverUrl}`);
      const result = await socket.request(connectRequest(name, reconnectToken));
      applyServerResult(result);
      await socket.request({ type: 'list_rooms' }).then(applyServerResult);
    } catch (error) {
      setConnectionError(error instanceof Error ? error.message : 'Connection failed');
    }
  }

  function disconnect(): void {
    socket.close();
    setConnected(false);
    pushLog('warning', 'Disconnected');
  }
</script>

<div class="card glass-card mb-3">
  <div class="card-body">
    <div class="d-flex align-items-center justify-content-between gap-3 mb-3">
      <div>
        <h1 class="h4 mb-1">Play Room</h1>
        <div class="text-muted small">Svelte + TypeScript WebSocket client</div>
      </div>
      {#if $appState.connected}
        <span class="badge text-bg-success">connected</span>
      {:else if $appState.connecting}
        <span class="badge text-bg-warning">connecting</span>
      {:else}
        <span class="badge text-bg-secondary">offline</span>
      {/if}
    </div>

    <div class="row g-2 align-items-end">
      <div class="col-12 col-lg-5">
        <label class="form-label" for="server-url">Server URL</label>
        <input id="server-url" class="form-control" bind:value={serverUrl} disabled={$appState.connected || $appState.connecting} />
      </div>
      <div class="col-12 col-md-3 col-lg-2">
        <label class="form-label" for="display-name">Name</label>
        <input id="display-name" class="form-control" bind:value={displayName} disabled={$appState.connected || $appState.connecting} />
      </div>
      <div class="col-12 col-md-5 col-lg-3">
        <label class="form-label" for="reconnect-token">Reconnect token</label>
        <input id="reconnect-token" class="form-control" bind:value={reconnectToken} placeholder="optional" disabled={$appState.connected || $appState.connecting} />
      </div>
      <div class="col-12 col-md-4 col-lg-2 d-grid">
        {#if $appState.connected}
          <button class="btn btn-outline-danger" type="button" onclick={disconnect}>Disconnect</button>
        {:else}
          <button class="btn btn-primary" type="button" onclick={connect} disabled={$appState.connecting}>Connect</button>
        {/if}
      </div>
    </div>

    {#if $appState.welcome}
      <div class="mt-3 small">
        <div><span class="text-muted">Player:</span> <span class="room-id">{$appState.welcome.player_id}</span></div>
        <div><span class="text-muted">Token:</span> <span class="room-id">{$appState.welcome.reconnect_token}</span></div>
      </div>
    {/if}

    {#if $appState.lastError}
      <div class="alert alert-danger mt-3 mb-0 py-2">{$appState.lastError}</div>
    {/if}
  </div>
</div>
