<script lang="ts">
  import type { RoomSummary } from '../../lib/protocol/types';
  import { roomPhaseTag, roomSummaryMeta, type RoomAction } from '../../lib/view/room-selectors';
  import { playRoomClient } from '../../lib/client/play-room-client';
  import Button from '../ui/Button.svelte';

  export let room: RoomSummary;
  export let action: RoomAction;

  $: actionLabel = action === 'current' ? 'Current' : action === 'join' ? 'Join' : 'Watch';

  async function act() {
    if (action === 'current') return;
    if (action === 'join') {
      await playRoomClient.joinRoom(room.id);
      return;
    }
    await playRoomClient.spectateRoom(room.id);
  }
</script>

<div class="room-row" class:current={action === 'current'}>
  <div class="room-row-main">
    <strong>{room.name}</strong>
    <span>{roomPhaseTag(room.phase)}</span>
  </div>
  <div class="room-row-meta">
    <span>Players {room.players} / {room.max_players}</span>
    <span>Watchers {room.spectators}</span>
    <span>{roomSummaryMeta(room)}</span>
  </div>
  <Button variant={action === 'watch' ? 'secondary' : 'primary'} disabled={action === 'current'} onclick={act}>{actionLabel}</Button>
</div>