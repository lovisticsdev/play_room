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
  import Button from '../ui/Button.svelte';

  let submitError: string | null = null;
  let actionError: string | null = null;
  let actionBusy = false;

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
  $: isHost = Boolean(room?.host_id && room.host_id === $sessionStore.playerId);
  $: matchStarted = Boolean(room && room.round > 0);
  $: roleSwitchLocked = matchStarted && !finished;
  $: seatOpen = Boolean(room && participantList.length < room.rules.max_players);
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

  async function runMatchAction(action: () => Promise<void>) {
    actionBusy = true;
    actionError = null;

    try {
      await action();
    } catch (error) {
      actionError = error instanceof Error ? error.message : 'Action failed';
    } finally {
      actionBusy = false;
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
          <div class="duel-score">
            <span>Score</span>
            <strong>{leftPlayer?.score ?? 0}</strong>
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
          <div class="duel-score">
            <span>Score</span>
            <strong>{rightPlayer?.score ?? 0}</strong>
          </div>
        </div>
      </div>
    </div>

    <div class="match-action-strip">
      <div class="match-action-copy">
        {#if finished}
          <strong>{matchWinner ? 'Match complete' : 'Match ended'}</strong>
          <span>{isHost ? 'Start a clean match when everyone is ready.' : 'Waiting for the host to start the next match.'}</span>
        {:else if activeRound}
          <strong>{spectatorMode ? 'Spectating live round' : lockedMove ? 'Move locked' : 'Your move'}</strong>
          <span>{spectatorMode ? 'Moves stay hidden until the result lands.' : lockedMove ? 'Wait for the round to resolve.' : 'Pick one move before the timer expires.'}</span>
        {:else if me?.role === 'participant'}
          <strong>Ready check</strong>
          <span>{roleSwitchLocked ? 'This match is underway; player seats stay locked until it ends.' : me.ready ? 'You are marked ready for the next round.' : 'Ready up from here when you want the next round to start.'}</span>
        {:else if spectatorMode}
          <strong>{roleSwitchLocked ? 'Match underway' : seatOpen ? 'Seat available' : 'Watching room'}</strong>
          <span>{roleSwitchLocked ? 'Player seats are locked until the match ends.' : seatOpen ? 'Join as a player from the arena when you want to play.' : 'Both player seats are occupied.'}</span>
        {:else}
          <strong>Room controls</strong>
          <span>Join the room from the browser to play or watch.</span>
        {/if}
      </div>

      <div class="match-actions">
        {#if finished && isHost}
          <Button variant="success" disabled={actionBusy} onclick={() => runMatchAction(() => playRoomClient.startNextMatch())}>
            Play Again
          </Button>
        {:else if !finished && !activeRound && me?.role === 'participant'}
          <Button
            variant={me.ready ? 'warning' : 'success'}
            disabled={actionBusy || !me.connected}
            onclick={() => runMatchAction(() => playRoomClient.setReady(!me?.ready))}
          >
            {me.ready ? 'Unready' : 'Ready'}
          </Button>
          {#if !roleSwitchLocked}
            <Button variant="secondary" disabled={actionBusy} onclick={() => runMatchAction(() => playRoomClient.setSpectator(true))}>
              Watch as Spectator
            </Button>
          {/if}
        {:else if !finished && !activeRound && spectatorMode}
          {#if roleSwitchLocked}
            <Button variant="secondary" disabled>Match Locked</Button>
          {:else}
            <Button variant="primary" disabled={actionBusy || !seatOpen} onclick={() => runMatchAction(() => playRoomClient.setSpectator(false))}>
              {seatOpen ? 'Join as Player' : 'Seats Full'}
            </Button>
          {/if}
        {/if}

        <Button variant="danger" disabled={actionBusy} onclick={() => runMatchAction(() => playRoomClient.leaveRoom())}>
          Leave Room
        </Button>
      </div>
    </div>

    {#if finished}
      <div class="choose-divider"><span>Final score</span></div>
    {:else if activeRound && spectatorMode}
      <div class="choose-divider"><span>Spectator view</span></div>
    {:else if activeRound && me?.role === 'participant'}
      <div class="choose-divider"><span>{lockedMove ? 'Move locked' : 'Choose your move'}</span></div>
      <MoveSelector moves={$allowedMoves} selectedMove={lockedMove} disabled={!canSubmit} onChoose={chooseMove} />
    {/if}

    {#if lockedMove && activeRound && !finished}
      <div class="locked-banner">
        <div>
          <strong>Move locked</strong>
          <span>You chose {moveLabel(lockedMove)} {moveSymbol(lockedMove)}.</span>
        </div>
        <span>You cannot change your move now.</span>
      </div>
    {/if}

    {#if actionError}
      <div class="form-error match-action-error">{actionError}</div>
    {/if}

    {#if submitError}
      <div class="form-error">{submitError}</div>
    {/if}

    <ResultBanner result={$currentRoomStore.lastResult} {room} />
  {/if}
</section>
