<script lang="ts">
  import type { RoomSnapshot, RoundResult } from '../../lib/protocol/types';
  import { bestOfLabel, moveLabel } from '../../lib/protocol/rules';
  import { matchWinnerId, playerName, winnerId } from '../../lib/view/room-selectors';

  export let result: RoundResult | null = null;
  export let room: RoomSnapshot | null = null;

  $: roundWinner = result ? winnerId(result.outcome) : null;
  $: matchWinner = matchWinnerId(room);
  $: roundTitle = !result
    ? ''
    : roundWinner
      ? `${playerName(room, roundWinner)} wins round ${result.round}`
      : result.outcome === 'draw'
        ? `Round ${result.round} is a draw`
        : `Round ${result.round} had no contest`;
  $: finalTitle = matchWinner ? `${playerName(room, matchWinner)} wins the match` : room?.phase.phase === 'finished' ? 'Match complete' : '';
</script>

{#if room?.phase.phase === 'finished'}
  <div class="result-banner match-result">
    <div>
      <span>{bestOfLabel(room.rules.target_score)}</span>
      <strong>{finalTitle}</strong>
    </div>
    <div class="submitted-moves final-scoreline">
      {#each room.scoreboard as score (score.player_id)}
        <span>{score.name}: {score.score}</span>
      {/each}
    </div>
  </div>
{:else if result}
  <div class="result-banner">
    <div>
      <span>Resolved</span>
      <strong>{roundTitle}</strong>
    </div>
    <div class="submitted-moves">
      {#each Object.entries(result.submitted) as [playerId, move]}
        <span>{playerName(room, playerId)}: {move ? moveLabel(move) : 'No move'}</span>
      {/each}
    </div>
  </div>
{/if}