<script lang="ts">
  import { DEFAULT_SERVER_URL, connectionStore } from '../../lib/stores/connection';
  import { playRoomClient } from '../../lib/client/play-room-client';
  import { loadDisplayName, loadReconnectToken, loadServerUrl } from '../../lib/storage/reconnect-token';
  import Button from '../ui/Button.svelte';
  import TextInput from '../ui/TextInput.svelte';

  let serverUrl = loadServerUrl(DEFAULT_SERVER_URL);
  let displayName = loadDisplayName();
  let reconnectToken = loadReconnectToken() ?? '';
  let formError: string | null = null;

  $: busy = $connectionStore.status === 'connecting' || $connectionStore.status === 'reconnecting';
  $: canConnect = displayName.trim().length > 0;
  $: canReconnect = reconnectToken.trim().length > 0;

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    formError = null;

    try {
      await playRoomClient.connect(serverUrl, displayName, null);
    } catch (error) {
      formError = error instanceof Error ? error.message : 'Connection failed';
    }
  }

  async function reconnect() {
    formError = null;

    try {
      await playRoomClient.connect(serverUrl, displayName, reconnectToken);
    } catch (error) {
      formError = error instanceof Error ? error.message : 'Reconnect failed';
    }
  }
</script>

<form class="connect-form" onsubmit={submit}>
  <div class="connect-grid">
    <TextInput id="display-name" label="Display name" bind:value={displayName} disabled={busy} placeholder="Enter your display name" />
    <Button type="submit" disabled={busy || !canConnect} full>{busy ? 'Connecting...' : 'Connect'}</Button>
  </div>

  <div class="connect-grid">
    <TextInput id="reconnect-token" label="Reconnect code (optional)" bind:value={reconnectToken} disabled={busy} placeholder="Enter reconnect code or token" />
    <Button variant="secondary" type="button" disabled={busy || !canReconnect} onclick={reconnect} full>
      Reconnect
    </Button>
  </div>

  <details class="advanced-server">
    <summary>Server connection</summary>
    <TextInput id="server-url" label="Server URL" bind:value={serverUrl} disabled={busy} required />
  </details>

  {#if formError || $connectionStore.error}
    <div class="form-error">{formError ?? $connectionStore.error}</div>
  {/if}
</form>
