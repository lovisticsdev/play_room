<script lang="ts">
  import { currentRoomStore, allowedMoves } from '../../lib/stores/current-room';
  import { sessionStore } from '../../lib/stores/session';
  import { playRoomClient } from '../../lib/client/play-room-client';
  import type { Move, PlayerView } from '../../lib/protocol/types';
  import { isFinished, isInRound, moveLabel, moveSymbol } from '../../lib/protocol/rules';
  import { localPlayer, matchWinnerId, participants, playerName } from '../../lib/view/room-selectors';
  import Avatar from '../ui/Avatar.svelte';
  import Badge from '../ui/Badge.svelte';
  import MoveSelector from './MoveSelector.svelte';
  import ResultBanner from './ResultBanner.svelte';
  import RoomHeader from './RoomHeader.svelte';

  let submitError: string | null = null;

  $: room = $currentRoomStore.room;
  $: me = localPlayer(room, $sessionStore.playerId);
  $: participantList = participants(room);
  $: spectatorMode = me?.role === 'spectator';
  $: leftPlayer = spectatorMode ? participantList[0] ?? null : me;
  $: rightPlayer = spectatorMode ? participantList[1] ?? null : participantList.find((player) => player.id !== me?.id) ?? null;
  $: leftLabel = spectatorMode ? 'Player 1' : 'You';
  $: rightLabel = spectatorMode ? 'Player 2' : 'Opponent';
  $: activeRound = room ? isInRound(room.phase) : false;
  $: activeRoundNumber = room?.phase.phase === 'in_round' ? room.phase.round : null;
  $: finished = room ? isFinished(room.phase) : false;
  $: matchWinner = matchWinnerId(room);
  $: canSubmit = Boolean(me?.role === 'participant' && me.connected && activeRound && !$currentRoomStore.localMove);
  $: lockedMove = $currentRoomStore.localMove;

  function roleTone(player: PlayerView | null): 'success' | 'warning' | 'danger' | 'neutral' | 'violet' {
    if (!player) return 'neutral';
    if (!player.connected && player.role === 'participant') return 'warning';
    if (!player.connected) return 'danger';
    if (player.role === 'spectator') return 'violet';
    return player.ready ? 'success' : 'warning';
  }

  function roleLabel(player: PlayerView | null): string {
    if (!player) return 'Open slot';
    if (!player.connected && player.role === 'participant') return 'Seat reserved';
    if (!player.connected) return 'Disconnected';
    if (player.role === 'spectator') return 'Watching';
    return player.ready ? 'Ready' : 'Waiting';
  }

  function playerDisplayName(player: PlayerView | null, fallback: string): string {
    return player?.name ?? fallback;
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
        <span class="duel-label">{leftLabel}</span>
        <div class="duel-person">
          <Avatar name={playerDisplayName(leftPlayer, $sessionStore.displayName ?? 'Guest')} connected={leftPlayer?.connected ?? true} />
          <div>
            <div class="duel-name-line">
              <strong>{playerDisplayName(leftPlayer, spectatorMode ? 'Open slot' : $sessionStore.displayName ?? 'Guest')}</strong>
              {#if leftPlayer?.id === room.host_id}<Badge tone="warning">Host</Badge>{/if}
            </div>
            <Badge tone={roleTone(leftPlayer)}>{roleLabel(leftPlayer)}</Badge>
          </div>
        </div>
      </div>

      <div class="duel-stage">
        {#if activeRound && spectatorMode}
          <strong>Watching round {activeRoundNumber}</strong>
          <span>Moves stay hidden until the round resolves.</span>
        {:else if activeRound}
          <strong>{lockedMove ? 'Waiting for opponent...' : 'Choose your move'}</strong>
          <span>Moves are locked when accepted by the server.</span>
        {:else if finished}
          <strong>{matchWinner ? `${playerName(room, matchWinner)} wins` : 'Match complete'}</strong>
          <span>The final scoreboard is locked until the host starts the next match.</span>
        {:else if spectatorMode}
          <strong>Watching lobby</strong>
          <span>Participants must ready up before the next round starts.</span>
        {:else}
          <strong>Waiting for ready check</strong>
          <span>Participants must be ready before the round starts.</span>
        {/if}
        <div class="pulse-dots" aria-hidden="true"><span></span><span></span><span></span></div>
      </div>

      <div class="duel-card opponent">
        <span class="duel-label">{rightLabel}</span>
        <div class="duel-person">
          <Avatar name={playerDisplayName(rightPlayer, '?')} connected={rightPlayer?.connected ?? true} />
          <div>
            <div class="duel-name-line">
              <strong>{playerDisplayName(rightPlayer, 'Open slot')}</strong>
              {#if rightPlayer?.id === room.host_id}<Badge tone="warning">Host</Badge>{/if}
            </div>
            <Badge tone={roleTone(rightPlayer)}>{roleLabel(rightPlayer)}</Badge>
          </div>
        </div>
      </div>
    </div>

    <div class="choose-divider"><span>{finished ? 'Final score' : spectatorMode ? 'Spectator view' : 'Choose your move'}</span></div>

    {#if !finished && me?.role === 'participant'}
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
    {:else if spectatorMode}
      <div class="locked-banner muted">
        <div>
          <strong>Spectator mode</strong>
          <span>You are watching the two active participants. Use Join as Player when a seat is open.</span>
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
