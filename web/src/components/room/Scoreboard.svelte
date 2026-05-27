<script lang="ts">
  import { flip } from 'svelte/animate';
  import { fade } from 'svelte/transition';
  import type { PlayerScore } from '../../lib/protocol/types';

  export let scores: PlayerScore[] = [];

  // Sort scores descending to enable deterministic flip animations
  $: sortedScores = [...scores].sort((a, b) => b.score - a.score);
</script>

<div class="card glass-card mb-3">
  <div class="card-body">
    <h2 class="h5 mb-3">Scoreboard</h2>
    {#if sortedScores.length === 0}
      <div class="text-muted small">No scores yet.</div>
    {:else}
      <div class="list-group">
        {#each sortedScores as score (score.player_id)}
          <div class="list-group-item d-flex justify-content-between align-items-center"
               animate:flip={{ duration: 300 }}
               transition:fade>
            <span>{score.name}</span>
            <span class="badge text-bg-info">{score.score}</span>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>