<script lang="ts">
  import type { Move } from '../../lib/protocol/types';
  import { moveLabel, moveSymbol } from '../../lib/protocol/rules';

  export let move: Move;
  export let selected = false;
  export let disabled = false;
  export let onChoose: (move: Move) => void;
</script>

<button
  class="move-button move-{move}"
  class:selected
  {disabled}
  type="button"
  aria-pressed={selected}
  onclick={() => onChoose(move)}
>
  <span class="move-symbol">{moveSymbol(move)}</span>
  <span>{moveLabel(move)}</span>
</button>
