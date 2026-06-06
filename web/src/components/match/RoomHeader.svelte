<script lang="ts">
  import { sessionStore } from '../../lib/stores/session';
  import { playRoomClient } from '../../lib/client/play-room-client';
  import type { RoomSnapshot } from '../../lib/protocol/types';
  import {
    gameLabel,
    phaseLabel,
    raceToDescription,
    raceToLabel,
    roundLabel,
    RACE_TARGETS,
    type RaceTarget,
  } from '../../lib/protocol/rules';
  import Badge from '../ui/Badge.svelte';
  import RoundTimer from './RoundTimer.svelte';
  import { playerName, roundCountdownDeadline } from '../../lib/view/room-selectors';
  import { copyText } from '../../lib/browser/clipboard';

  export let room: RoomSnapshot;

  let copied = false;
  let copyError: string | null = null;
  let formatBusy = false;
  let formatError: string | null = null;

  async function copyRoomId() {
    copyError = null;
    if (!(await copyText(room.id))) {
      copyError = 'Could not copy room code';
      return;
    }
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
  $: showStateBlock = inRound || finished;
  $: stateTitle = inRound ? 'Round in progress' : 'Match complete';
  $: stateDetail = inRound
    ? 'Moves are locked when the timer hits zero.'
    : `${winnerName} won. The host can start the next match.`;
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
            title={raceToDescription(target)}
            aria-label={raceToDescription(target)}
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
    {#if copyError}
      <span class="room-header-error">{copyError}</span>
    {/if}
  </div>

  {#if showStateBlock}
    <div class="room-state-block">
      <strong>{stateTitle}</strong>
      <span>{stateDetail}</span>
    </div>
  {/if}

  {#if deadline}
    <RoundTimer deadlineMs={deadline} />
  {/if}
</header>
