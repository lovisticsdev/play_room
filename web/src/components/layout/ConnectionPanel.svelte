<script lang="ts">
  import { connectionStore } from '../../lib/stores/connection';
  import { dispatch } from '../../lib/network/dispatcher';
  import { connectRequest } from '../../lib/protocol/commands';
  import GlassCard from '../ui/GlassCard.svelte';

  let serverUrl = 'ws://127.0.0.1:7878/ws';
  let displayName = 'alice';
  let reconnectToken = '';

  $: isBusy = $connectionStore.connected || $connectionStore.connecting;

  function handleConnect(event: Event) {
    event.preventDefault();
    if (!displayName.trim()) return;
    // Dispatcher handles the socket init, promise chains, and store updates natively
    dispatch('SYSTEM_CONNECT', { url: serverUrl.trim(), request: connectRequest(displayName, reconnectToken) });
  }

  function handleDisconnect() {
    dispatch('SYSTEM_DISCONNECT');
  }
</script>

<GlassCard>
  <div class="d-flex align-items-center justify-content-between gap-3 mb-3">
    <div>
      <h1 class="h4 mb-0">Play Room</h1>
      <div class="text-muted small">
        {#if $connectionStore.connected}
          Connected as <span class="fw-semibold text-light">{displayName}</span>
        {:else if $connectionStore.connecting}
          Connecting...
        {:else}
          Disconnected
        {/if}
      </div>
    </div>
    {#if $connectionStore.error}
      <div class="text-danger small fw-semibold">{$connectionStore.error}</div>
    {/if}
  </div>

  <form onsubmit={handleConnect} class="row g-2 align-items-end">
    <div class="col-12 col-md-4 col-lg-5">
      <label class="form-label" for="server-url">Server URL</label>
      <input id="server-url" class="form-control" bind:value={serverUrl} disabled={isBusy} required />
    </div>
    <div class="col-12 col-md-3 col-lg-2">
      <label class="form-label" for="display-name">Name</label>
      <input id="display-name" class="form-control" bind:value={displayName} disabled={isBusy} required />
    </div>
    <div class="col-12 col-md-5 col-lg-3">
      <label class="form-label" for="reconnect-token">Reconnect token</label>
      <input id="reconnect-token" class="form-control" bind:value={reconnectToken} placeholder="optional" disabled={isBusy} />
    </div>
    <div class="col-12 col-md-4 col-lg-2 d-grid">
      {#if $connectionStore.connected}
        <button class="btn btn-outline-danger" type="button" onclick={handleDisconnect}>Disconnect</button>
      {:else}
        <button class="btn btn-primary" type="submit" disabled={$connectionStore.connecting}>Connect</button>
      {/if}
    </div>
  </form>

  {#if $connectionStore.welcome}
    <div class="mt-3 small">
      <div><span class="text-muted">Player:</span> <span class="room-id">{$connectionStore.welcome.player_id}</span></div>
      <div><span class="text-muted">Token:</span> <span class="room-id">{$connectionStore.welcome.reconnect_token}</span></div>
    </div>
  {/if}
</GlassCard>