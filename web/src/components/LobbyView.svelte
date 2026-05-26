<script lang="ts">
  import type { PlayRoomSocket } from '../lib/websocket';
  import { appState, applyServerResult, pushLog } from '../lib/client-state';
  import { createRoomRequest, joinRoomRequest, spectateRoomRequest } from '../lib/commands';
  import { phaseLabel } from '../lib/protocol';

  export let socket: PlayRoomSocket;

  let roomName = 'lobby';
  let roomTarget = '';

  async function refreshRooms(): Promise<void> {
    if (!$appState.connected) return;
    await socket.request({ type: 'list_rooms' }).then(applyServerResult).catch(reportError);
  }

  async function createRoom(): Promise<void> {
    if (!roomName.trim()) return;
    const result = await socket.request(createRoomRequest(roomName)).catch(reportError);
    if (result) applyServerResult(result);
    await refreshRooms();
  }

  async function joinRoom(target = roomTarget): Promise<void> {
    if (!target.trim()) return;
    const result = await socket.request(joinRoomRequest(target)).catch(reportError);
    if (result) applyServerResult(result);
    await refreshRooms();
  }

  async function spectateRoom(target = roomTarget): Promise<void> {
    if (!target.trim()) return;
    const result = await socket.request(spectateRoomRequest(target)).catch(reportError);
    if (result) applyServerResult(result);
    await refreshRooms();
  }

  function reportError(error: unknown): void {
    pushLog('error', error instanceof Error ? error.message : String(error));
  }
</script>

<div class="card glass-card mb-3">
  <div class="card-body">
    <div class="d-flex align-items-center justify-content-between gap-3 mb-3">
      <div>
        <h2 class="h5 mb-1">Lobby</h2>
        <div class="text-muted small">Create, join, or spectate rooms.</div>
      </div>
      <button class="btn btn-sm btn-outline-info" type="button" onclick={refreshRooms} disabled={!$appState.connected}>Refresh</button>
    </div>

    <div class="row g-2 mb-3">
      <div class="col-12 col-md-8">
        <input class="form-control" bind:value={roomName} placeholder="room name" disabled={!$appState.connected} />
      </div>
      <div class="col-12 col-md-4 d-grid">
        <button class="btn btn-primary" type="button" onclick={createRoom} disabled={!$appState.connected}>Create room</button>
      </div>
    </div>

    <div class="row g-2 mb-3">
      <div class="col-12 col-md-6">
        <input class="form-control" bind:value={roomTarget} placeholder="room id or unique name" disabled={!$appState.connected} />
      </div>
      <div class="col-6 col-md-3 d-grid">
        <button class="btn btn-outline-light" type="button" onclick={() => joinRoom()} disabled={!$appState.connected}>Join</button>
      </div>
      <div class="col-6 col-md-3 d-grid">
        <button class="btn btn-outline-warning" type="button" onclick={() => spectateRoom()} disabled={!$appState.connected}>Spectate</button>
      </div>
    </div>

    {#if $appState.rooms.length === 0}
      <div class="text-muted small">No active rooms.</div>
    {:else}
      <div class="list-group">
        {#each $appState.rooms as room}
          <div class="list-group-item">
            <div class="d-flex align-items-start justify-content-between gap-3">
              <div>
                <div class="fw-semibold">{room.name}</div>
                <div class="room-id">{room.id}</div>
                <div class="small text-muted mt-1">
                  {phaseLabel(room.phase)} · players {room.players}/{room.max_players} · spectators {room.spectators}
                </div>
              </div>
              <div class="d-flex gap-2">
                <button class="btn btn-sm btn-outline-light" type="button" onclick={() => joinRoom(room.id)}>Join</button>
                <button class="btn btn-sm btn-outline-warning" type="button" onclick={() => spectateRoom(room.id)}>Spectate</button>
              </div>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
