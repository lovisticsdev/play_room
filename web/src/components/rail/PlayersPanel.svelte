<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { currentRoomStore } from '../../lib/stores/current-room';
  import { seatReservationsStore, reservedNameLabel, reservedSeatLabel } from '../../lib/stores/seat-reservations';
  import { sessionStore } from '../../lib/stores/session';
  import type { PlayerView } from '../../lib/protocol/types';
  import { participants, spectators } from '../../lib/view/room-selectors';
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
  $: participantList = participants(room).sort((a, b) => b.score - a.score || a.name.localeCompare(b.name));
  $: spectatorList = spectators(room).sort((a, b) => a.name.localeCompare(b.name));
  $: panelTitle = room ? `Players in ${room.name}` : 'Players in Room';

  function statusTone(player: PlayerView): 'success' | 'warning' | 'danger' | 'neutral' {
    if (!player.connected && player.role === 'participant') return 'warning';
    if (!player.connected) return 'danger';
    if (player.role === 'spectator') return 'neutral';
    return player.ready ? 'success' : 'warning';
  }

  function statusLabel(player: PlayerView): string {
    if (!player.connected) return 'Disconnected';
    if (player.role === 'spectator') return 'Watching';
    return player.ready ? 'Ready' : 'Waiting';
  }

  function seatExpiry(player: PlayerView): number | null | undefined {
    return player.participant_seat_expires_at_ms ?? $seatReservationsStore.participantExpiresAt[player.id];
  }

  function spectatorExpiry(player: PlayerView): number | null | undefined {
    return player.spectator_expires_at_ms ?? $seatReservationsStore.spectatorExpiresAt[player.id];
  }
</script>

<Panel title={panelTitle}>
  {#if !room}
    <div class="empty-state compact">No room selected.</div>
  {:else}
    <div class="player-section-title">
      <span>Participants ({participantList.length} / {room.rules.max_players})</span>
      <span>Score</span>
    </div>

    {#if participantList.length === 0}
      <div class="empty-state compact">No participants.</div>
    {:else}
      <div class="player-list">
        {#each participantList as player (player.id)}
          <div class="player-row" class:local={player.id === $sessionStore.playerId} class:reserved={!player.connected && player.role === 'participant'}>
            <Avatar name={player.name} connected={player.connected} />
            <div class="player-main">
              <div class="player-name-line">
                <strong>{player.name}</strong>
                {#if player.id === room.host_id}<Badge tone="warning">Host</Badge>{/if}
              </div>
              {#if !player.connected && player.role === 'participant'}
                <Badge tone="warning">{reservedSeatLabel(seatExpiry(player), now)}</Badge>
              {:else}
                <Badge tone={statusTone(player)}>{statusLabel(player)}</Badge>
              {/if}
            </div>
            <div class="score-value">{player.score}</div>
          </div>
        {/each}
      </div>
    {/if}

    <div class="player-section-title spectators-title">
      <span>Spectators ({spectatorList.length})</span>
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