<script lang="ts">
  import type { PlayRoomSocket } from '../lib/websocket';
  import { appState, applyServerResult, pushLog } from '../lib/client-state';
  import type { Move, RoomSnapshot } from '../lib/protocol';
  import { isInRound, phaseLabel } from '../lib/protocol';
  import { setReadyRequest, setSpectatorRequest, submitMoveRequest } from '../lib/commands';

  export let socket: PlayRoomSocket;
  export let room: RoomSnapshot;

  const moves: Move[] = ['rock', 'paper', 'scissors', 'lizard', 'spock'];

  $: currentPlayer = $appState.welcome
    ? room.players.find((player) => player.id === $appState.welcome?.player_id) ?? null
    : null;
  $: isParticipant = currentPlayer?.role === 'participant';
  $: isReady = currentPlayer?.ready ?? false;
  $: activeRound = isInRound(room.phase);
  $: allowedMoves = room.rules.game === 'rock_paper_scissors'
    ? moves.slice(0, 3)
    : moves;

  async function ready(readyState: boolean): Promise<void> {
    await socket.request(setReadyRequest(readyState)).then(applyServerResult).catch(reportError);
  }

  async function setSpectator(spectator: boolean): Promise<void> {
    await socket.request(setSpectatorRequest(spectator)).then(applyServerResult).catch(reportError);
  }

  async function submitMove(move: Move): Promise<void> {
    await socket.request(submitMoveRequest(move)).then(applyServerResult).catch(reportError);
  }

  async function leaveRoom(): Promise<void> {
    await socket.request({ type: 'leave_room' }).then(applyServerResult).catch(reportError);
  }

  function reportError(error: unknown): void {
    pushLog('error', error instanceof Error ? error.message : String(error));
  }
</script>

<div class="card glass-card mb-3">
  <div class="card-body">
    <div class="d-flex align-items-start justify-content-between gap-3 mb-3">
      <div>
        <h2 class="h5 mb-1">Game</h2>
        <div class="text-muted small">
          {phaseLabel(room.phase)} · {room.rules.game.replaceAll('_', ' ')} · target {room.rules.target_score}
        </div>
      </div>
      {#if activeRound}
        <span class="badge text-bg-warning">round timer active</span>
      {:else}
        <span class="badge text-bg-secondary">{room.phase.phase}</span>
      {/if}
    </div>

    <div class="d-flex flex-wrap gap-2 mb-3">
      {#if isParticipant}
        {#if isReady}
          <button class="btn btn-outline-warning" type="button" onclick={() => ready(false)}>Unready</button>
        {:else}
          <button class="btn btn-success" type="button" onclick={() => ready(true)}>Ready</button>
        {/if}
        <button class="btn btn-outline-warning" type="button" onclick={() => setSpectator(true)}>Switch to spectator</button>
      {:else}
        <button class="btn btn-outline-info" type="button" onclick={() => setSpectator(false)}>Switch to player</button>
      {/if}
      <button class="btn btn-outline-danger" type="button" onclick={leaveRoom}>Leave room</button>
    </div>

    <div class="mb-2 fw-semibold">Moves</div>
    <div class="d-flex flex-wrap gap-2">
      {#each allowedMoves as move}
        <button class="btn btn-primary move-button text-capitalize" type="button" onclick={() => submitMove(move)} disabled={!isParticipant || !activeRound}>
          {move}
        </button>
      {/each}
    </div>

    {#if !isParticipant}
      <div class="text-muted small mt-2">Spectators cannot submit moves.</div>
    {:else if !activeRound}
      <div class="text-muted small mt-2">Moves unlock when a round starts.</div>
    {/if}
  </div>
</div>
