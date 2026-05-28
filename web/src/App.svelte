<script lang="ts">
  import { onMount } from 'svelte';
  import AppShell from './components/shell/AppShell.svelte';
  import { playRoomClient } from './lib/client/play-room-client';

  onMount(() => {
    playRoomClient.start();

    return () => {
      playRoomClient.stop();
    };
  });
</script>

<AppShell />
