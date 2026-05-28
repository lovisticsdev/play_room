<script lang="ts">
  import { eventLogStore } from '../../lib/stores/event-log';

  $: notifications = $eventLogStore.logs;
</script>

{#if notifications.length > 0}
  <div class="notification-stack" aria-live="polite" aria-label="Live match notifications">
    {#each notifications as entry (entry.id)}
      <button class={`speech-toast ${entry.level}`} type="button" onclick={() => eventLogStore.dismiss(entry.id)}>
        <time>{entry.at}</time>
        <span>{entry.message}</span>
      </button>
    {/each}
  </div>
{/if}
