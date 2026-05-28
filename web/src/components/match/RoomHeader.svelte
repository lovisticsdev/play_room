<script lang="ts">
  import type { RoomSnapshot } from '../../lib/protocol/types';
  import { bestOfLabel, gameLabel, phaseLabel } from '../../lib/protocol/rules';
  import Badge from '../ui/Badge.svelte';
  import RoundTimer from './RoundTimer.svelte';
  import { playerName, roundCountdownDeadline } from '../../lib/view/room-selectors';

  export let room: RoomSnapshot;

  let copied = false;

  async function copyRoomId() {
    await navigator.clipboard.writeText(room.id);
    copied = true;
    setTimeout(() => {
      copied = false;
    }, 1400);
  }

  $: deadline = roundCountdownDeadline(room.phase);
  $: inRound = room.phase.phase === 'in_round';
  $: finished = room.phase.phase === 'finished';
  $: winnerName = room.phase.phase === 'finished' ? playerName(room, room.phase.winner) : null;
  $: stateTitle = inRound ? 'Round in progress' : finished ? 'Match complete' : 'Lobby ready check';
  $: stateDetail = inRound
    ? 'Moves are locked when the timer hits zero.'
    : finished
      ? `${winnerName} won. The host can start the next match.`
      : 'Ready participants start the next round.';
</script>

<header class="room-header">
  <div class="room-title-block">
    <h1>{room.name}</h1>
    <button class="copy-code" type="button" onclick={copyRoomId}>Code: {room.id.slice(0, 12)} {copied ? '✓' : '⧉'}</button>
  </div>

  <div class="room-round-block">
    <Badge tone={inRound ? 'accent' : finished ? 'success' : 'neutral'}>{phaseLabel(room.phase)}</Badge>
    <small>{bestOfLabel(room.rules.target_score)} · {gameLabel(room.rules.game)}</small>
  </div>

  <div class="room-state-block">
    <strong>{stateTitle}</strong>
    <span>{stateDetail}</span>
  </div>

  {#if deadline}
    <RoundTimer deadlineMs={deadline} />
  {/if}
</header>