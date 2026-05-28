<script lang="ts">
  import { eventLogStore } from '../../lib/stores/event-log';
  import Panel from '../ui/Panel.svelte';
  import Button from '../ui/Button.svelte';

  $: logs = $eventLogStore.logs;
</script>

<Panel title="Event Log">
  <svelte:fragment slot="action">
    {#if logs.length > 0}
      <Button variant="ghost" onclick={() => eventLogStore.clear()}>Clear</Button>
    {/if}
  </svelte:fragment>

  <div class="event-log" aria-live="polite">
    {#if logs.length === 0}
      <div class="empty-state compact">No events yet.</div>
    {:else}
      {#each logs as entry (entry.id)}
        <div class="event-line {entry.level}">
          <time>{entry.at}</time>
          <span>{entry.message}</span>
        </div>
      {/each}
    {/if}
  </div>
</Panel>
