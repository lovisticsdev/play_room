<script lang="ts">
  import type { PlayRoomSocket } from '../lib/websocket';
  import { appState } from '../lib/client-state';
  import PlayerList from './PlayerList.svelte';
  import GameBoard from './GameBoard.svelte';
  import Scoreboard from './Scoreboard.svelte';

  export let socket: PlayRoomSocket;
</script>

{#if $appState.currentRoom}
  <div class="card glass-card mb-3">
    <div class="card-body">
      <div class="d-flex align-items-start justify-content-between gap-3 mb-3">
        <div>
          <h2 class="h5 mb-1">{$appState.currentRoom.name}</h2>
          <div class="room-id">{$appState.currentRoom.id}</div>
        </div>
        <span class="badge badge-soft">round {$appState.currentRoom.round}</span>
      </div>
      <PlayerList players={$appState.currentRoom.players} hostId={$appState.currentRoom.host_id} />
    </div>
  </div>

  <GameBoard socket={socket} room={$appState.currentRoom} />
  <Scoreboard scores={$appState.currentRoom.scoreboard} />
{:else}
  <div class="card glass-card mb-3">
    <div class="card-body text-muted">No room selected. Create, join, or spectate a room.</div>
  </div>
{/if}
