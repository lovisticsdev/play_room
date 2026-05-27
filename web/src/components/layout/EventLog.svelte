<script lang="ts">
  import { logStore } from '../../lib/stores/event-log';
  import GlassCard from '../ui/GlassCard.svelte';

  const LevelClasses: Record<string, string> = {
    success: 'text-success',
    warning: 'text-warning',
    error: 'text-danger',
    protocol: 'text-info',
    info: 'text-light',
  };

  $: logs = $logStore.logs;
</script>

<GlassCard>
  <div class="d-flex justify-content-between align-items-center mb-3">
    <h2 class="h5 mb-0">Event Log</h2>
    {#if logs.length > 0}
      <button class="btn btn-sm btn-outline-secondary" onclick={() => logStore.clear()}>Clear</button>
    {/if}
  </div>

  <div class="event-log" aria-live="polite">
    {#if logs.length === 0}
      <div class="text-muted small">No events yet.</div>
    {:else}
      {#each logs as entry (entry.id)}
        <div class="event-line">
          <span class="text-muted">[{entry.at}]</span>
          <span class={LevelClasses[entry.level] || 'text-light'}> {entry.level}</span>
          <span> - {entry.message}</span>
        </div>
      {/each}
    {/if}
  </div>
</GlassCard>
