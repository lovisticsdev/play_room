<script lang="ts">
  import type { PlayerView, PlayerId } from '../../lib/protocol/types';
  import PlayerPill from '../ui/PlayerPill.svelte';

  export let players: PlayerView[] = [];
  export let hostId: PlayerId | null = null;

  $: participants = players.filter((p) => p.role === 'participant');
  $: spectators = players.filter((p) => p.role === 'spectator');
</script>

<div class="mb-3">
  <h3 class="h6">Participants</h3>
  {#if participants.length === 0}
    <div class="text-muted small">No participants.</div>
  {:else}
    <div class="d-flex flex-wrap gap-2">
      {#each participants as player (player.id)}
        <PlayerPill {player} {hostId} />
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
      {#each spectators as player (player.id)}
        <PlayerPill {player} {hostId} />
      {/each}
    </div>
  {/if}
</div>