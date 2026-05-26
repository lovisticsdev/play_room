<script lang="ts">
  import type { PlayerScore } from '../lib/protocol';

  export let scores: PlayerScore[] = [];
</script>

<div class="card glass-card mb-3">
  <div class="card-body">
    <h2 class="h5 mb-3">Scoreboard</h2>
    {#if scores.length === 0}
      <div class="text-muted small">No scores yet.</div>
    {:else}
      <div class="list-group">
        {#each scores as score}
          <div class="list-group-item d-flex justify-content-between align-items-center">
            <span>{score.name}</span>
            <span class="badge text-bg-info">{score.score}</span>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
