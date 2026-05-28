<script lang="ts">
  import { connectionStore } from '../../lib/stores/connection';
  import { currentPlayer, currentRoomStore } from '../../lib/stores/current-room';
  import { sessionStore } from '../../lib/stores/session';
  import { uiStore } from '../../lib/stores/ui';
  import { playRoomClient } from '../../lib/client/play-room-client';

  let copied = false;

  $: status = $connectionStore.status;
  $: statusLabel = status === 'connected'
    ? 'Connected'
    : status === 'connecting'
      ? 'Connecting'
      : status === 'reconnecting'
        ? 'Reconnecting'
        : status === 'error'
          ? 'Connection Error'
          : 'Not Connected';
  $: playerRole = $currentPlayer?.role === 'spectator'
    ? 'Spectator'
    : $currentRoomStore.room?.host_id === $sessionStore.playerId
      ? 'Host'
      : $currentPlayer
        ? 'Player'
        : null;

  async function copyToken() {
    const token = $sessionStore.reconnectToken;
    if (!token) return;

    await navigator.clipboard.writeText(token);
    copied = true;
    setTimeout(() => {
      copied = false;
    }, 1400);
  }
</script>

<header class="top-bar">
  <button class="brand" type="button" onclick={() => uiStore.openRoomsModal('join')} aria-label="Open rooms">
    <div class="brand-mark">V</div>
    <div class="brand-text">
      <strong>PLAY</strong>
      <span>ROOM</span>
    </div>
  </button>

  <button class="nav-button" type="button" onclick={() => uiStore.openRoomsModal('join')} aria-label="Open rooms">
    <span class="nav-glyph">▦</span>
    <span>Rooms</span>
  </button>

  <div class="top-spacer"></div>

  <div class="connection-chip {status}">
    <span class="status-dot"></span>
    <span>{statusLabel}</span>
    {#if $connectionStore.latencyMs !== null && status === 'connected'}
      <em>{$connectionStore.latencyMs}ms</em>
    {:else}
      <em>-- ms</em>
    {/if}
  </div>

  <div class="identity-chip">
    <div class="identity-avatar">{$sessionStore.displayName?.[0]?.toUpperCase() ?? 'G'}</div>
    <span>{$sessionStore.displayName ?? 'Guest'}</span>
    {#if playerRole}<small class="identity-role">{playerRole}</small>{/if}
  </div>

  {#if $sessionStore.reconnectToken}
    <button class="token-button" type="button" onclick={copyToken}>{copied ? 'Copied' : 'Copy Reconnect Token'} <span>⧉</span></button>
  {/if}

  {#if status === 'connected'}
    <button class="icon-button" type="button" onclick={() => playRoomClient.disconnect()} title="Disconnect" aria-label="Disconnect">⏻</button>
  {/if}
</header>
