<script lang="ts">
  import TopBar from './TopBar.svelte';
  import Notifications from './Notifications.svelte';
  import MatchSurface from '../match/MatchSurface.svelte';
  import RightRail from '../rail/RightRail.svelte';
  import RoomsModal from '../rooms/RoomsModal.svelte';
</script>

<div class="app-shell">
  <TopBar />
  <main class="app-layout">
    <MatchSurface />
    <RightRail />
  </main>
  <RoomsModal />
  <Notifications />
</div>
