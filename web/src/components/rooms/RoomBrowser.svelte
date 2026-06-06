<script lang="ts">
  import { roomsStore } from '../../lib/stores/rooms';
  import { currentRoomStore } from '../../lib/stores/current-room';
  import { isPlayRoomRequestError, playRoomClient } from '../../lib/client/play-room-client';
  import { getRoomAction } from '../../lib/view/room-selectors';
  import Button from '../ui/Button.svelte';
  import RoomRow from './RoomRow.svelte';
  import TextInput from '../ui/TextInput.svelte';

  let query = '';
  let error: string | null = null;
  let suggestions: string[] = [];
  let suggestionKind: 'room' | 'player' = 'room';
  let suggestionBusy = false;

  $: currentRoomId = $currentRoomStore.room?.id ?? null;
  $: needle = query.trim().toLowerCase();
  $: filteredRooms = $roomsStore.rooms.filter((room) => {
    return !needle || room.name.toLowerCase().includes(needle) || room.id.toLowerCase().includes(needle);
  });

  async function joinByCode(event: SubmitEvent) {
    event.preventDefault();
    error = null;
    suggestions = [];
    suggestionKind = 'room';

    const code = query.trim();
    if (!code) return;

    try {
      await playRoomClient.joinOrSpectateRoom(code);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Join failed';
      if (isPlayRoomRequestError(err)) {
        suggestions = err.suggestions;
        suggestionKind = err.code === 'player_name_exists' ? 'player' : 'room';
      }
    }
  }

  async function retryWithDisplayName(suggestion: string) {
    const code = query.trim();
    if (!code || suggestionBusy) return;

    suggestionBusy = true;
    error = null;
    suggestions = [];

    try {
      await playRoomClient.updateDisplayName(suggestion);
      await playRoomClient.joinOrSpectateRoom(code);
    } catch (err) {
      error = err instanceof Error ? err.message : 'Join failed';
      if (isPlayRoomRequestError(err)) {
        suggestions = err.suggestions;
        suggestionKind = err.code === 'player_name_exists' ? 'player' : 'room';
      }
    } finally {
      suggestionBusy = false;
    }
  }
</script>

<div class="room-browser">
  <form class="browser-toolbar" onsubmit={joinByCode}>
    <TextInput id="room-search" bind:value={query} placeholder="Search rooms or enter code to join/watch" />
    <Button type="submit" variant="secondary" disabled={!query.trim()}>Join / Watch</Button>
  </form>

  <div class="browser-meta-row">
    <span>{filteredRooms.length} room{filteredRooms.length === 1 ? '' : 's'} visible</span>
    <button type="button" onclick={() => playRoomClient.refreshRooms()} disabled={$roomsStore.loading}>
      {$roomsStore.loading ? 'Refreshing...' : 'Refresh'}
    </button>
  </div>

  {#if error || $roomsStore.error}
    <div class="form-error">{error ?? $roomsStore.error}</div>
  {/if}

  {#if suggestions.length > 0}
    <div class="form-warning">
      {#if suggestionKind === 'player'}
        <span>Use a suggested display name:</span>
        <span class="suggestion-list">
          {#each suggestions as suggestion}
            <button type="button" class="suggestion-pill" disabled={suggestionBusy} onclick={() => retryWithDisplayName(suggestion)}>
              {suggestion}
            </button>
          {/each}
        </span>
      {:else}
        <span>Use a suggested room name:</span>
        <span class="suggestion-list">
          {#each suggestions as suggestion}
            <button type="button" class="suggestion-pill" onclick={() => (query = suggestion)}>
              {suggestion}
            </button>
          {/each}
        </span>
      {/if}
    </div>
  {/if}

  <div class="room-list">
    {#if filteredRooms.length === 0}
      <div class="empty-state">No rooms found. Create one or refresh the browser.</div>
    {:else}
      {#each filteredRooms as room (room.id)}
        <RoomRow {room} action={getRoomAction(room, currentRoomId)} />
      {/each}
    {/if}
  </div>
</div>
