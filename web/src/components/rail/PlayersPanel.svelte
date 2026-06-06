<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { currentRoomStore } from '../../lib/stores/current-room';
  import { seatReservationsStore, reservedNameLabel } from '../../lib/stores/seat-reservations';
  import { sessionStore } from '../../lib/stores/session';
  import type { PlayerView } from '../../lib/protocol/types';
  import { spectators } from '../../lib/view/room-selectors';
  import Avatar from '../ui/Avatar.svelte';
  import Badge from '../ui/Badge.svelte';
  import Panel from '../ui/Panel.svelte';

  let now = Date.now();
  let tick: ReturnType<typeof setInterval> | null = null;

  onMount(() => {
    tick = setInterval(() => {
      now = Date.now();
    }, 1000);
  });

  onDestroy(() => {
    if (tick) clearInterval(tick);
  });

  $: room = $currentRoomStore.room;
  $: spectatorList = spectators(room).sort((a, b) => a.name.localeCompare(b.name));
  $: panelTitle = 'Spectators';

  function statusTone(player: PlayerView): 'danger' | 'neutral' {
    if (!player.connected) return 'danger';
    return 'neutral';
  }

  function statusLabel(player: PlayerView): string {
    if (!player.connected) return 'Disconnected';
    return 'Watching';
  }

  function spectatorExpiry(player: PlayerView): number | null | undefined {
    return player.spectator_expires_at_ms ?? $seatReservationsStore.spectatorExpiresAt[player.id];
  }
</script>

<Panel title={panelTitle} compact>
  {#if !room}
    <div class="empty-state compact">No room selected.</div>
  {:else}
    <div class="player-section-title">
      <span>{room.name} ({spectatorList.length})</span>
    </div>

    {#if spectatorList.length === 0}
      <div class="empty-state compact">No spectators.</div>
    {:else}
      <div class="player-list spectator-list">
        {#each spectatorList as player (player.id)}
          <div class="player-row spectator" class:local={player.id === $sessionStore.playerId}>
            <Avatar name={player.name} connected={player.connected} />
            <div class="player-main">
              <div class="player-name-line">
                <strong>{player.name}</strong>
                {#if player.id === room.host_id}<Badge tone="warning">Host</Badge>{/if}
              </div>
              {#if !player.connected}
                <Badge tone="danger">{reservedNameLabel(spectatorExpiry(player), now)}</Badge>
              {:else}
                <Badge tone={statusTone(player)}>{statusLabel(player)}</Badge>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</Panel>
