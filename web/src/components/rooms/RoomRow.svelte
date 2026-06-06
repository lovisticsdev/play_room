<script lang="ts">
  import type { RoomSummary } from '../../lib/protocol/types';
  import { roomPhaseTag, roomSummaryMeta, type RoomAction } from '../../lib/view/room-selectors';
  import { isPlayRoomRequestError, playRoomClient } from '../../lib/client/play-room-client';
  import Button from '../ui/Button.svelte';

  export let room: RoomSummary;
  export let action: RoomAction;

  let busy = false;
  let error: string | null = null;
  let warning = false;
  let suggestions: string[] = [];

  $: actionLabel = action === 'current' ? 'Current' : action === 'join' ? 'Join' : 'Watch';

  async function performRoomAction() {
    if (action === 'join') {
      await playRoomClient.joinOrSpectateRoom(room.id);
      return;
    }
    await playRoomClient.spectateRoom(room.id);
  }

  async function act() {
    if (action === 'current' || busy) return;

    busy = true;
    error = null;
    warning = false;
    suggestions = [];

    try {
      await performRoomAction();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Room action failed';
      if (isPlayRoomRequestError(err)) {
        warning = err.code === 'player_name_exists';
        suggestions = err.suggestions;
      }
    } finally {
      busy = false;
    }
  }

  async function retryWithDisplayName(suggestion: string) {
    if (action === 'current' || busy) return;

    busy = true;
    error = null;
    warning = false;
    suggestions = [];

    try {
      await playRoomClient.updateDisplayName(suggestion);
      await performRoomAction();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Room action failed';
      if (isPlayRoomRequestError(err)) {
        warning = err.code === 'player_name_exists';
        suggestions = err.suggestions;
      }
    } finally {
      busy = false;
    }
  }
</script>

<div class="room-row-wrap">
  <div class="room-row" class:current={action === 'current'}>
    <div class="room-row-main">
      <strong>{room.name}</strong>
      <span>{roomPhaseTag(room.phase)}</span>
    </div>
    <div class="room-row-meta">
      <span>Players {room.players} / {room.max_players}</span>
      <span>Watchers {room.spectators}</span>
      <span>{roomSummaryMeta(room)}</span>
    </div>
    <Button variant={action === 'watch' ? 'secondary' : 'primary'} disabled={action === 'current' || busy} onclick={act}>
      {busy ? 'Working...' : actionLabel}
    </Button>
  </div>

  {#if error}
    <div class={warning ? 'form-warning room-row-message' : 'form-error room-row-message'}>
      {error}
      {#if suggestions.length > 0}
        <span class="room-row-suggestions">
          <span>Use a suggested display name:</span>
          <span class="suggestion-list">
            {#each suggestions as suggestion}
              <button type="button" class="suggestion-pill" disabled={busy} onclick={() => retryWithDisplayName(suggestion)}>
                {suggestion}
              </button>
            {/each}
          </span>
        </span>
      {/if}
    </div>
  {/if}
</div>
