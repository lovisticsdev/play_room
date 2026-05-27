<script lang="ts">
  import { roomStore } from '../../lib/stores/room';
  import GlassCard from '../ui/GlassCard.svelte';
  import PlayerList from './PlayerList.svelte';
  import GameBoard from './GameBoard.svelte';
  import Scoreboard from './Scoreboard.svelte';
</script>

{#if $roomStore.currentRoom}
  <div class="row g-3 align-items-start">
    <div class="col-12 col-lg-7">
      <GameBoard />
    </div>
    <div class="col-12 col-lg-5">
      <Scoreboard scores={$roomStore.currentRoom.scoreboard} />
    </div>
  </div>

  <GlassCard>
    <div class="d-flex align-items-start justify-content-between gap-3 mb-3">
      <div>
        <h2 class="h5 mb-1">{$roomStore.currentRoom.name}</h2>
        <div class="room-id">{$roomStore.currentRoom.id}</div>
      </div>
      <span class="badge badge-soft">round {$roomStore.currentRoom.round}</span>
    </div>
    <PlayerList players={$roomStore.currentRoom.players} hostId={$roomStore.currentRoom.host_id} />
  </GlassCard>
{/if}
