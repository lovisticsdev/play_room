<script lang="ts">
  import { currentRoomStore } from '../../lib/stores/current-room';
  import PlayersPanel from './PlayersPanel.svelte';
</script>

{#if $currentRoomStore.room}
  <section class="spectators-dock" aria-label="Room spectators">
    <PlayersPanel />
  </section>
{/if}
