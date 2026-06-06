<script lang="ts">
  import { connectionStore } from '../../lib/stores/connection';
  import { sessionStore } from '../../lib/stores/session';
  import { uiStore } from '../../lib/stores/ui';
  import { playRoomClient } from '../../lib/client/play-room-client';
  import { copyText } from '../../lib/browser/clipboard';
  import { eventLogStore } from '../../lib/stores/event-log';

  let copied = false;

  $: status = $connectionStore.status;

  async function copyToken() {
    const token = $sessionStore.reconnectToken;
    if (!token) return;

    if (!(await copyText(token))) {
      eventLogStore.push('warning', 'Could not copy reconnect token');
      return;
    }
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
      <div class="identity-avatar">{$sessionStore.displayName?.[0]?.toUpperCase() ?? 'G'}</div>
      <span class="identity-name"><span class="status-dot"></span>{$sessionStore.displayName ?? 'Guest'}</span>
      <span class="identity-controls">
        {#if $sessionStore.reconnectToken}
          <button
            class="identity-icon-button"
            type="button"
            onclick={copyToken}
            title={copied ? 'Reconnect token copied' : 'Copy reconnect token'}
            aria-label={copied ? 'Reconnect token copied' : 'Copy reconnect token'}
          >
            {copied ? '✓' : '⧉'}
          </button>
        {/if}

        {#if status === 'connected'}
          <button
            class="identity-icon-button danger"
            type="button"
            onclick={() => playRoomClient.disconnect()}
            title="Disconnect"
            aria-label="Disconnect"
          >
            ⏻
          </button>
        {/if}
      </span>
    </div>
  </div>
</header>
