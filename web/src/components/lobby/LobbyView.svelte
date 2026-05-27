<script lang="ts">
  import { connectionStore } from '../../lib/stores/connection';
  import { lobbyStore } from '../../lib/stores/lobby';
  import { roomStore } from '../../lib/stores/room';
  import { dispatch } from '../../lib/network/dispatcher';
  import { joinRoomRequest, spectateRoomRequest } from '../../lib/protocol/commands';
  import { phaseLabel } from '../../lib/protocol/rules';
  import CreateRoomForm from './CreateRoomForm.svelte';
  import GlassCard from '../ui/GlassCard.svelte';
  import type { RoomId } from '../../lib/protocol/types';

  function join(id: RoomId) {
    dispatch('NETWORK_REQUEST', joinRoomRequest(id));
  }

  function spectate(id: RoomId) {
    dispatch('NETWORK_REQUEST', spectateRoomRequest(id));
  }
</script>

<GlassCard>
  <div class="d-flex justify-content-between align-items-center mb-3">
    <div>
      <h2 class="h5 mb-0">Rooms</h2>
      <div class="text-muted small">Create a room or choose one from the active list.</div>
    </div>
    <button class="btn btn-sm btn-outline-light" onclick={() => dispatch('SYSTEM_REFRESH_ROOMS')} disabled={!$connectionStore.connected}>Refresh</button>
  </div>

  <CreateRoomForm />

  {#if $lobbyStore.rooms.length === 0}
    <div class="text-muted small">No active rooms.</div>
  {:else}
    <div class="list-group">
      {#each $lobbyStore.rooms as room (room.id)}
        <div class="list-group-item">
          <div class="d-flex align-items-start justify-content-between gap-3">
            <div>
              <div class="fw-semibold">{room.name}</div>
              <div class="room-id">{room.id}</div>
              <div class="small text-muted mt-1">
                {phaseLabel(room.phase)} · players {room.players}/{room.max_players} · spectators {room.spectators}
              </div>
            </div>
            <div class="d-flex gap-2 flex-wrap justify-content-end">
              {#if $roomStore.currentRoom?.id === room.id}
                <span class="badge badge-soft align-self-center">Current room</span>
              {:else}
                <button class="btn btn-sm btn-outline-light" type="button" onclick={() => join(room.id)} disabled={!$connectionStore.connected}>Join</button>
                <button class="btn btn-sm btn-outline-warning" type="button" onclick={() => spectate(room.id)} disabled={!$connectionStore.connected}>Spectate</button>
              {/if}
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</GlassCard>
