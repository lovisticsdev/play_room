<script lang="ts">
  import { connectionStore } from '../../lib/stores/connection';
  import { sessionStore } from '../../lib/stores/session';
  import { uiStore } from '../../lib/stores/ui';
  import { currentRoomStore } from '../../lib/stores/current-room';
  import { playRoomClient } from '../../lib/client/play-room-client';
  import Modal from '../ui/Modal.svelte';
  import Badge from '../ui/Badge.svelte';
  import Button from '../ui/Button.svelte';
  import ConnectForm from './ConnectForm.svelte';
  import CreateRoomForm from './CreateRoomForm.svelte';
  import RoomBrowser from './RoomBrowser.svelte';

  $: connected = $connectionStore.status === 'connected';
  $: closeDisabled = !connected || !$currentRoomStore.room;
  $: modalTitle = connected
    ? $currentRoomStore.room
      ? 'Rooms'
      : 'Choose a Room'
    : 'Connect & Browse Rooms';
  $: modalSubtitle = connected
    ? 'Create, join, or watch a room without leaving the match surface behind.'
    : 'Enter your player name, connect, then create, join, or browse a room.';
</script>

<Modal
  open={$uiStore.roomsModalOpen}
  title={modalTitle}
  subtitle={modalSubtitle}
  {closeDisabled}
  onClose={() => uiStore.closeRoomsModal()}
>
  <div class="modal-section">
    <div class="section-title">
      <span>Step 1 · {connected ? 'Connected' : 'Connect'}</span>
      {#if connected}<Badge tone="success">{$sessionStore.displayName ?? 'player'}</Badge>{/if}
    </div>

    {#if connected}
      <div class="connected-card">
        <div class="big-check">✓</div>
        <div>
          <strong>You are connected as <span>{$sessionStore.displayName ?? $sessionStore.playerId}</span></strong>
          <p>You can now join rooms, spectate active games, or create a new one.</p>
        </div>
        <Button variant="secondary" onclick={() => playRoomClient.disconnect()}>Disconnect</Button>
      </div>
    {:else}
      <ConnectForm />
    {/if}
  </div>

  {#if connected}
    <div class="modal-section">
      <div class="section-title">
        <span>Step 2 · Choose a room</span>
        {#if $currentRoomStore.room}<Badge tone="accent">Current: {$currentRoomStore.room.name}</Badge>{/if}
      </div>

      <div class="tabs" role="tablist" aria-label="Room actions">
        <button class:active={$uiStore.roomsModalTab === 'join'} type="button" onclick={() => uiStore.setRoomsModalTab('join')}>Join</button>
        <button class:active={$uiStore.roomsModalTab === 'create'} type="button" onclick={() => uiStore.setRoomsModalTab('create')}>Create</button>
      </div>

      {#if $uiStore.roomsModalTab === 'join'}
        <RoomBrowser />
      {:else}
        <CreateRoomForm />
      {/if}
    </div>
  {/if}
</Modal>
