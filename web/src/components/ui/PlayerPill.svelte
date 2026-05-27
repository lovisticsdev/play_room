<script lang="ts">
  import type { PlayerView, PlayerId } from '../../lib/protocol/types';

  export let player: PlayerView;
  export let hostId: PlayerId | null = null;
</script>

<div class="player-pill" title={player.id}>
  <span class="fw-semibold">{player.name}</span>
  {#if player.id === hostId}
    <span class="badge badge-soft ms-1">host</span>
  {/if}
  {#if player.ready}
    <span class="badge text-bg-success ms-1">ready</span>
  {/if}
  {#if !player.connected}
    <span class="badge text-bg-danger ms-1">offline</span>
  {/if}
  {#if player.role === 'participant'}
    <span class="text-muted ms-2">{player.score}</span>
  {/if}
</div>
