<script lang="ts">
  import { roomsStore } from '../../lib/stores/rooms';
  import { sessionStore } from '../../lib/stores/session';
  import { isPlayRoomRequestError, playRoomClient } from '../../lib/client/play-room-client';
  import { defaultRules, gameLabel } from '../../lib/protocol/rules';
  import type { GameKind } from '../../lib/protocol/types';
  import Button from '../ui/Button.svelte';
  import TextInput from '../ui/TextInput.svelte';

  let roomName = 'testroom';
  let bestOf: 3 | 5 = 3;
  let game: GameKind = 'rock_paper_scissors_lizard_spock';
  let submitting = false;
  let error: string | null = null;
  let serverSuggestions: string[] = [];

  function slug(value: string): string {
    return value.trim().toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'room';
  }

  function exists(name: string): boolean {
    const normalized = name.trim().toLowerCase();
    return $roomsStore.rooms.some((room) => room.name.trim().toLowerCase() === normalized);
  }

  function buildSuggestions(name: string, displayName: string | null): string[] {
    const base = slug(name);
    const owner = displayName ? slug(displayName) : '';
    const seeds = [`${base}-2`, owner ? `${base}-${owner}` : '', `${base}-3`, `${base}-4`].filter(Boolean);

    return Array.from(new Set(seeds)).filter((candidate) => !exists(candidate)).slice(0, 3);
  }

  function chooseSuggestion(suggestion: string) {
    roomName = suggestion;
    error = null;
    serverSuggestions = [];
  }

  $: trimmedName = roomName.trim();
  $: duplicate = trimmedName.length > 0 && exists(trimmedName);
  $: localSuggestions = duplicate ? buildSuggestions(trimmedName, $sessionStore.displayName) : [];
  $: suggestions = serverSuggestions.length > 0 ? serverSuggestions : localSuggestions;

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (!trimmedName || duplicate) return;

    submitting = true;
    error = null;
    serverSuggestions = [];

    try {
      await playRoomClient.createRoom(trimmedName, defaultRules(game, bestOf));
    } catch (err) {
      error = err instanceof Error ? err.message : 'Create room failed';
      serverSuggestions = isPlayRoomRequestError(err) ? err.suggestions : [];
    } finally {
      submitting = false;
    }
  }
</script>

<form class="create-room-form" onsubmit={submit}>
  <TextInput id="room-name" label="Room name" bind:value={roomName} placeholder="testroom" disabled={submitting} required />

  <div class="option-grid" aria-label="Room format">
    <div class="option-group">
      <span>Game</span>
      <div class="segmented-control">
        <button class:active={game === 'rock_paper_scissors_lizard_spock'} type="button" onclick={() => (game = 'rock_paper_scissors_lizard_spock')}>{gameLabel('rock_paper_scissors_lizard_spock')}</button>
        <button class:active={game === 'rock_paper_scissors'} type="button" onclick={() => (game = 'rock_paper_scissors')}>{gameLabel('rock_paper_scissors')}</button>
      </div>
    </div>

    <div class="option-group">
      <span>Match</span>
      <div class="segmented-control">
        <button class:active={bestOf === 3} type="button" onclick={() => (bestOf = 3)}>Best of 3</button>
        <button class:active={bestOf === 5} type="button" onclick={() => (bestOf = 5)}>Best of 5</button>
      </div>
    </div>
  </div>

  {#if duplicate || serverSuggestions.length > 0}
    <div class="form-warning">
      {error ?? 'Room name already exists.'}
      {#if suggestions.length > 0}
        Try
        {#each suggestions as suggestion, index}
          <button type="button" onclick={() => chooseSuggestion(suggestion)}>{suggestion}</button>{index < suggestions.length - 1 ? ',' : '.'}
        {/each}
      {/if}
    </div>
  {:else if error}
    <div class="form-error">{error}</div>
  {/if}

  <Button type="submit" disabled={submitting || !trimmedName || duplicate} full>
    {submitting ? 'Creating...' : 'Create Room'}
  </Button>
</form>