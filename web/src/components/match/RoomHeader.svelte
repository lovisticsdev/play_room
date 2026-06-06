<script lang="ts">
  import { sessionStore } from '../../lib/stores/session';
  import { playRoomClient } from '../../lib/client/play-room-client';
  import type { RoomSnapshot } from '../../lib/protocol/types';
  import { gameLabel, phaseLabel, raceToLabel, roundLabel, RACE_TARGETS, type RaceTarget } from '../../lib/protocol/rules';
  import Badge from '../ui/Badge.svelte';
  import RoundTimer from './RoundTimer.svelte';
  import { playerName, roundCountdownDeadline } from '../../lib/view/room-selectors';

  export let room: RoomSnapshot;

  let copied = false;
  let formatBusy = false;
  let formatError: string | null = null;

  async function copyRoomId() {
    await navigator.clipboard.writeText(room.id);
    copied = true;
    setTimeout(() => {
      copied = false;
    }, 1400);
  }

  async function setRaceTarget(targetScore: RaceTarget) {
    if (formatBusy || targetScore === room.rules.target_score) return;

    formatBusy = true;
    formatError = null;

    try {
      await playRoomClient.updateMatchFormat(targetScore);
    } catch (error) {
      formatError = error instanceof Error ? error.message : 'Could not update match format';
    } finally {
      formatBusy = false;
    }
  }

  $: deadline = roundCountdownDeadline(room.phase);
  $: inRound = room.phase.phase === 'in_round';
  $: finished = room.phase.phase === 'finished';
  $: formatEditable = room.host_id === $sessionStore.playerId && ((room.round === 0 && room.phase.phase === 'lobby') || finished);
  $: winnerName = room.phase.phase === 'finished' ? playerName(room, room.phase.winner) : null;
  $: stateTitle = inRound ? 'Round in progress' : finished ? 'Match complete' : 'Lobby ready check';
  $: stateDetail = inRound
    ? 'Moves are locked when the timer hits zero.'
    : finished
      ? `${winnerName} won. The host can start the next match.`
      : 'Ready participants start the next round.';
  $: currentRound = roundLabel(room.round, room.phase);
</script>

<header class="room-header">
  <div class="room-title-block">
    <h1>{room.name}</h1>
    <button class="copy-code" type="button" onclick={copyRoomId}>Code: {room.id.slice(0, 12)} {copied ? '✓' : '⧉'}</button>
  </div>

  <div class="room-round-block">
    <Badge tone={inRound ? 'accent' : finished ? 'success' : 'neutral'}>{phaseLabel(room.phase)}</Badge>
    <small>{currentRound} · {raceToLabel(room.rules.target_score)} · {gameLabel(room.rules.game)}</small>
    {#if formatEditable}
      <div class="race-control" aria-label="Match format">
        {#each RACE_TARGETS as target}
          <button
            type="button"
            class:active={room.rules.target_score === target}
            disabled={formatBusy}
            onclick={() => setRaceTarget(target)}
          >
            {target}
          </button>
        {/each}
      </div>
      {#if formatError}
        <span class="room-header-error">{formatError}</span>
      {/if}
    {/if}
  </div>

  <div class="room-state-block">
    <strong>{stateTitle}</strong>
    <span>{stateDetail}</span>
  </div>

  {#if deadline}
    <RoundTimer deadlineMs={deadline} />
  {/if}
</header>
