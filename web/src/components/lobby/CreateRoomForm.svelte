<script lang="ts">
  import { connectionStore } from '../../lib/stores/connection';
  import { lobbyStore } from '../../lib/stores/lobby';
  import { dispatch } from '../../lib/network/dispatcher';
  import { createRoomRequest } from '../../lib/protocol/commands';

  let roomName = 'lobby';

  $: trimmedRoomName = roomName.trim();
  $: duplicateRoomName = trimmedRoomName.length > 0
    && $lobbyStore.rooms.some((room) => room.name.trim().toLowerCase() === trimmedRoomName.toLowerCase());

  function handleCreate(event: Event) {
    event.preventDefault();
    if (!trimmedRoomName || duplicateRoomName) return;
    dispatch('NETWORK_REQUEST', createRoomRequest(trimmedRoomName));
  }
</script>

<form onsubmit={handleCreate} class="row g-2 mb-3">
  <div class="col-12 col-md-6">
    <input class="form-control" bind:value={roomName} placeholder="Room name" disabled={!$connectionStore.connected} required />
    {#if duplicateRoomName}
      <div class="small text-warning mt-1">Room name already exists.</div>
    {/if}
  </div>
  <div class="col-12 col-md-6 d-grid">
    <button class="btn btn-primary" type="submit" disabled={!$connectionStore.connected || duplicateRoomName}>Create Room</button>
  </div>
</form>
