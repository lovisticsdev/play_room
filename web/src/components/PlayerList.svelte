<script lang="ts">
  import type { PlayerId, PlayerView } from '../lib/protocol';

  export let players: PlayerView[] = [];
  export let hostId: PlayerId | null = null;

  $: participants = players.filter((player) => player.role === 'participant');
  $: spectators = players.filter((player) => player.role === 'spectator');
</script>

<div class="mb-3">
  <h3 class="h6">Participants</h3>
  {#if participants.length === 0}
    <div class="text-muted small">No participants.</div>
  {:else}
    <div class="d-flex flex-wrap gap-2">
      {#each participants as player}
        <div class="player-pill">
          <span class="fw-semibold">{player.name}</span>
          {#if player.id === hostId}<span class="badge badge-soft ms-1">host</span>{/if}
          {#if player.ready}<span class="badge text-bg-success ms-1">ready</span>{/if}
          {#if !player.connected}<span class="badge text-bg-danger ms-1">offline</span>{/if}
          <span class="text-muted ms-2">{player.score}</span>
        </div>
      {/each}
    </div>
  {/if}
</div>

<div>
  <h3 class="h6">Spectators</h3>
  {#if spectators.length === 0}
    <div class="text-muted small">No spectators.</div>
  {:else}
    <div class="d-flex flex-wrap gap-2">
      {#each spectators as player}
        <div class="player-pill">
          <span>{player.name}</span>
          {#if !player.connected}<span class="badge text-bg-danger ms-1">offline</span>{/if}
        </div>
      {/each}
    </div>
  {/if}
</div>
