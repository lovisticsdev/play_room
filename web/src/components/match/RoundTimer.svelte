<script lang="ts">
  import { onDestroy, onMount } from 'svelte';

  export let deadlineMs: number | null = null;

  let now = Date.now();
  let interval: ReturnType<typeof setInterval> | null = null;

  onMount(() => {
    interval = setInterval(() => {
      now = Date.now();
    }, 250);
  });

  onDestroy(() => {
    if (interval) clearInterval(interval);
  });

  $: remainingMs = deadlineMs ? Math.max(0, deadlineMs - now) : 0;
  $: seconds = Math.ceil(remainingMs / 1000);
</script>

<div class="round-timer" aria-label="Round timer">
  <strong>{seconds}</strong>
  <span>SEC</span>
</div>
