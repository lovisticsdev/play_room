<script lang="ts">
  import { currentRoomStore, allowedMoves } from '../../lib/stores/current-room';
  import { sessionStore } from '../../lib/stores/session';
  import { playRoomClient } from '../../lib/client/play-room-client';
  import type { Move, PlayerView } from '../../lib/protocol/types';
  import { isFinished, isInRound, moveLabel, moveSymbol } from '../../lib/protocol/rules';
  import { localPlayer, matchWinnerId, opponent, playerName } from '../../lib/view/room-selectors';
  import Avatar from '../ui/Avatar.svelte';
  import Badge from '../ui/Badge.svelte';
  import MoveSelector from './MoveSelector.svelte';
  import ResultBanner from './ResultBanner.svelte';
  import RoomHeader from './RoomHeader.svelte';

  let submitError: string | null = null;

  $: room = $currentRoomStore.room;
  $: me = localPlayer(room, $sessionStore.playerId);
  $: rival = opponent(room, $sessionStore.playerId);
  $: activeRound = room ? isInRound(room.phase) : false;
  $: finished = room ? isFinished(room.phase) : false;
  $: matchWinner = matchWinnerId(room);
  $: canSubmit = Boolean(me?.role === 'participant' && activeRound && !$currentRoomStore.localMove);
  $: lockedMove = $currentRoomStore.localMove;

  function roleTone(player: PlayerView | null): 'success' | 'warning' | 'danger' | 'neutral' | 'violet' {
    if (!player) return 'neutral';
    if (!player.connected) return 'danger';
    if (player.role === 'spectator') return 'violet';
    return player.ready ? 'success' : 'warning';
  }

  function roleLabel(player: PlayerView | null): string {
    if (!player) return 'Open slot';
    if (!player.connected) return 'Disconnected';
    if (player.role === 'spectator') return 'Watching';
    return player.ready ? 'Ready' : 'Waiting';
  }

  async function chooseMove(move: Move) {
    submitError = null;

    try {
      await playRoomClient.submitMove(move);
    } catch (error) {
      submitError = error instanceof Error ? error.message : 'Move rejected';
    }
  }
</script>

<section class="match-surface" class:idle={!room}>
  {#if room}
    <RoomHeader {room} />

    <div class="duel-arena" class:finished>
      <div class="duel-card self">
        <span class="duel-label">You</span>
        <div class="duel-person">
          <Avatar name={me?.name ?? $sessionStore.displayName ?? 'Guest'} connected={me?.connected ?? true} />
          <div>
            <div class="duel-name-line">
              <strong>{me?.name ?? $sessionStore.displayName ?? 'Guest'}</strong>
              {#if me?.id === room.host_id}<Badge tone="warning">Host</Badge>{/if}
            </div>
            <Badge tone={roleTone(me)}>{roleLabel(me)}</Badge>
          </div>
        </div>
      </div>

      <div class="duel-stage">
        {#if activeRound}
          <strong>{lockedMove ? 'Waiting for opponent...' : 'Choose your move'}</strong>
          <span>Moves are locked when accepted by the server.</span>
        {:else if finished}
          <strong>{matchWinner ? `${playerName(room, matchWinner)} wins` : 'Match complete'}</strong>
          <span>The final scoreboard is locked until the host starts the next match.</span>
        {:else}
          <strong>Waiting for ready check</strong>
          <span>Participants must be ready before the round starts.</span>
        {/if}
        <div class="pulse-dots" aria-hidden="true"><span></span><span></span><span></span></div>
      </div>

      <div class="duel-card opponent">
        <span class="duel-label">Opponent</span>
        <div class="duel-person">
          <Avatar name={rival?.name ?? '?'} connected={rival?.connected ?? true} />
          <div>
            <div class="duel-name-line">
              <strong>{rival?.name ?? 'Open slot'}</strong>
              {#if rival?.id === room.host_id}<Badge tone="warning">Host</Badge>{/if}
            </div>
            <Badge tone={roleTone(rival)}>{roleLabel(rival)}</Badge>
          </div>
        </div>
      </div>
    </div>

    <div class="choose-divider"><span>{finished ? 'Final score' : 'Choose your move'}</span></div>

    {#if !finished}
      <MoveSelector moves={$allowedMoves} selectedMove={lockedMove} disabled={!canSubmit} onChoose={chooseMove} />
    {/if}

    {#if lockedMove && !finished}
      <div class="locked-banner">
        <div>
          <strong>Move locked</strong>
          <span>You chose {moveLabel(lockedMove)} {moveSymbol(lockedMove)}.</span>
        </div>
        <span>You cannot change your move now.</span>
      </div>
    {:else if me?.role === 'spectator'}
      <div class="locked-banner muted">
        <div>
          <strong>Spectator mode</strong>
          <span>Spectators can watch the room but cannot submit moves.</span>
        </div>
      </div>
    {:else if finished}
      <div class="locked-banner muted">
        <div>
          <strong>Match locked</strong>
          <span>The host can start the next match from the action panel.</span>
        </div>
      </div>
    {:else if !activeRound}
      <div class="locked-banner muted">
        <div>
          <strong>No active round</strong>
          <span>Use the action panel to ready up for the next round.</span>
        </div>
      </div>
    {/if}

    {#if submitError}
      <div class="form-error">{submitError}</div>
    {/if}

    <ResultBanner result={$currentRoomStore.lastResult} {room} />
  {/if}
</section>