<script lang="ts">
  import type { Move } from '../../lib/protocol/types';
  import MoveButton from './MoveButton.svelte';

  export let moves: Move[] = [];
  export let selectedMove: Move | null = null;
  export let disabled = false;
  export let onChoose: (move: Move) => void;
</script>

<div class="move-selector" aria-label="Choose your move">
  {#each moves as move (move)}
    <MoveButton {move} selected={selectedMove === move} disabled={disabled || selectedMove !== null} {onChoose} />
  {/each}
</div>
