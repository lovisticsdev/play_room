<script lang="ts">
  import { roomStore, isParticipant, isReady, activeRound, allowedMoves } from '../../lib/stores/room';
  import { dispatch } from '../../lib/network/dispatcher';
  import { setReadyRequest, setSpectatorRequest, submitMoveRequest } from '../../lib/protocol/commands';
  import { phaseLabel } from '../../lib/protocol/rules';
  import GlassCard from '../ui/GlassCard.svelte';
  import type { Move } from '../../lib/protocol/types';

  function ready(state: boolean) {
    dispatch('NETWORK_REQUEST', setReadyRequest(state));
  }

  function spectate(state: boolean) {
    dispatch('NETWORK_REQUEST', setSpectatorRequest(state));
  }

  function move(m: Move) {
    dispatch('NETWORK_REQUEST', submitMoveRequest(m));
  }
</script>

<GlassCard>
  <div class="d-flex align-items-start justify-content-between gap-3 mb-3">
    <div>
      <h2 class="h5 mb-1">Game Board</h2>
      <div class="small text-muted">
        {$roomStore.currentRoom?.rules.game === 'rock_paper_scissors' ? 'Rock Paper Scissors' : 'Rock Paper Scissors Lizard Spock'}
        · {$roomStore.currentRoom?.rules.target_score} to win
      </div>
    </div>
    {#if $activeRound}
      <span class="badge text-bg-warning">round timer active</span>
    {:else if $roomStore.currentRoom}
      <span class="badge text-bg-secondary">{phaseLabel($roomStore.currentRoom.phase)}</span>
    {/if}
  </div>

  <div class="d-flex flex-wrap gap-2 mb-3">
    {#if $isParticipant}
      {#if $isReady}
        <button class="btn btn-outline-warning" type="button" onclick={() => ready(false)}>Unready</button>
      {:else}
        <button class="btn btn-success" type="button" onclick={() => ready(true)}>Ready</button>
      {/if}
      <button class="btn btn-outline-warning" type="button" onclick={() => spectate(true)}>Switch to spectator</button>
    {:else}
      <button class="btn btn-outline-info" type="button" onclick={() => spectate(false)}>Switch to player</button>
    {/if}
    <button class="btn btn-outline-danger" type="button" onclick={() => dispatch('SYSTEM_LEAVE_ROOM')}>Leave room</button>
  </div>

  <div class="mb-2 fw-semibold">Moves</div>
  <div class="d-flex flex-wrap gap-2">
    {#each $allowedMoves as mv}
      <button class="btn btn-primary move-button text-capitalize" type="button"
              onclick={() => move(mv)}
              disabled={!$isParticipant || !$activeRound}>
        {mv}
      </button>
    {/each}
  </div>

  {#if !$isParticipant}
    <div class="text-muted small mt-2">Spectators cannot submit moves.</div>
  {:else if !$activeRound}
    <div class="text-muted small mt-2">Waiting for the next round to start.</div>
  {/if}
</GlassCard>
