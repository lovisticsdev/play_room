<script lang="ts">
  import { currentPlayer, currentRoomStore } from '../../lib/stores/current-room';
  import { sessionStore } from '../../lib/stores/session';
  import { playRoomClient } from '../../lib/client/play-room-client';
  import { isFinished, isInRound } from '../../lib/protocol/rules';
  import { uiStore } from '../../lib/stores/ui';
  import Button from '../ui/Button.svelte';
  import Panel from '../ui/Panel.svelte';

  let busy = false;
  let error: string | null = null;

  $: room = $currentRoomStore.room;
  $: inRound = room ? isInRound(room.phase) : false;
  $: finished = room ? isFinished(room.phase) : false;
  $: isHost = Boolean(room?.host_id && room.host_id === $sessionStore.playerId);

  async function run(action: () => Promise<void>) {
    busy = true;
    error = null;

    try {
      await action();
    } catch (err) {
      error = err instanceof Error ? err.message : 'Action failed';
    } finally {
      busy = false;
    }
  }
</script>

<Panel compact>
  {#if !room}
    <div class="rail-actions">
      <Button onclick={() => uiStore.openRoomsModal('join')} full>Open Rooms</Button>
    </div>
  {:else}
    <div class="rail-actions">
      {#if finished}
        {#if isHost}
          <Button variant="success" disabled={busy} onclick={() => run(() => playRoomClient.startNextMatch())} full>
            Play Again
          </Button>
        {:else}
          <div class="action-note">Waiting for the host to start the next match.</div>
        {/if}
      {:else if inRound}
        <div class="action-note">Round in progress. Moves are handled on the match board.</div>
      {:else if $currentPlayer?.role === 'participant'}
        <Button
          variant={$currentPlayer.ready ? 'warning' : 'success'}
          disabled={busy}
          onclick={() => run(() => playRoomClient.setReady(!$currentPlayer?.ready))}
          full
        >
          {$currentPlayer.ready ? 'Unready' : 'Ready'}
        </Button>
        <Button variant="secondary" disabled={busy} onclick={() => run(() => playRoomClient.setSpectator(true))} full>
          Watch as Spectator
        </Button>
      {:else if $currentPlayer?.role === 'spectator'}
        <Button variant="secondary" disabled={busy} onclick={() => run(() => playRoomClient.setSpectator(false))} full>
          Join as Player
        </Button>
      {/if}

      <Button variant="secondary" onclick={() => uiStore.openRoomsModal('join')} full>Browse Rooms</Button>
      <Button variant="danger" disabled={busy} onclick={() => run(() => playRoomClient.leaveRoom())} full>Leave Room</Button>
    </div>
  {/if}

  {#if error}
    <div class="form-error">{error}</div>
  {/if}
</Panel>