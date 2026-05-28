<script lang="ts">
  import { currentRoomStore } from '../../lib/stores/current-room';
  import { sessionStore } from '../../lib/stores/session';
  import type { PlayerView } from '../../lib/protocol/types';
  import { participants, spectators } from '../../lib/view/room-selectors';
  import Avatar from '../ui/Avatar.svelte';
  import Badge from '../ui/Badge.svelte';
  import Panel from '../ui/Panel.svelte';

  $: room = $currentRoomStore.room;
  $: participantList = participants(room).sort((a, b) => b.score - a.score || a.name.localeCompare(b.name));
  $: spectatorList = spectators(room).sort((a, b) => a.name.localeCompare(b.name));
  $: panelTitle = room ? `Players in ${room.name}` : 'Players in Room';

  function statusTone(player: PlayerView): 'success' | 'warning' | 'danger' | 'neutral' {
    if (!player.connected) return 'danger';
    if (player.role === 'spectator') return 'neutral';
    return player.ready ? 'success' : 'warning';
  }

  function statusLabel(player: PlayerView): string {
    if (!player.connected) return 'Disconnected';
    if (player.role === 'spectator') return 'Watching';
    return player.ready ? 'Ready' : 'Waiting';
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
          <div class="player-row" class:local={player.id === $sessionStore.playerId}>
            <Avatar name={player.name} connected={player.connected} />
            <div class="player-main">
              <div class="player-name-line">
                <strong>{player.name}</strong>
                {#if player.id === room.host_id}<Badge tone="warning">Host</Badge>{/if}
              </div>
              <Badge tone={statusTone(player)}>{statusLabel(player)}</Badge>
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
          <div class="player-row spectator">
            <Avatar name={player.name} connected={player.connected} />
            <div class="player-main">
              <div class="player-name-line"><strong>{player.name}</strong></div>
              <Badge tone={statusTone(player)}>{statusLabel(player)}</Badge>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</Panel>
