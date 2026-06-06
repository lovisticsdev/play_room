<script lang="ts">
  import { connectionStore } from '../../lib/stores/connection';
  import { sessionStore } from '../../lib/stores/session';
  import { uiStore } from '../../lib/stores/ui';
  import { playRoomClient } from '../../lib/client/play-room-client';

  let copied = false;

  $: status = $connectionStore.status;

  async function copyToken() {
    const token = $sessionStore.reconnectToken;
    if (!token) return;

    await navigator.clipboard.writeText(token);
    copied = true;
    setTimeout(() => {
      copied = false;
    }, 1400);
  }

  function openRooms() {
    uiStore.openRoomsModal('join');
    void playRoomClient.refreshRooms({ silent: true });
  }
</script>

<header class="top-bar">
  <button class="brand" type="button" onclick={openRooms} aria-label="Open rooms">
    <span class="brand-logo" aria-hidden="true"></span>
  </button>

  <div class="top-actions">
    <div class="identity-chip {status}">
      <span class="status-dot"></span>
      <div class="identity-avatar">{$sessionStore.displayName?.[0]?.toUpperCase() ?? 'G'}</div>
      <span>{$sessionStore.displayName ?? 'Guest'}</span>
    </div>

    {#if $sessionStore.reconnectToken}
      <button class="token-button" type="button" onclick={copyToken}>{copied ? 'Copied' : 'Copy Reconnect Token'} <span>⧉</span></button>
    {/if}

    {#if status === 'connected'}
      <button class="icon-button" type="button" onclick={() => playRoomClient.disconnect()} title="Disconnect" aria-label="Disconnect">⏻</button>
    {/if}
  </div>
</header>
