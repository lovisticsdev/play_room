<script lang="ts">
  import { appState } from '../lib/client-state';

  function levelClass(level: string): string {
    switch (level) {
      case 'success': return 'text-success';
      case 'warning': return 'text-warning';
      case 'error': return 'text-danger';
      case 'protocol': return 'text-info';
      default: return 'text-light';
    }
  }
</script>

<div class="card glass-card">
  <div class="card-body">
    <h2 class="h5 mb-3">Event Log</h2>
    <div class="event-log">
      {#if $appState.eventLog.length === 0}
        <div class="text-muted small">No events yet.</div>
      {:else}
        {#each $appState.eventLog as entry}
          <div class="event-line">
            <span class="text-muted">[{entry.at}]</span>
            <span class={levelClass(entry.level)}> {entry.level}</span>
            <span> — {entry.message}</span>
          </div>
        {/each}
      {/if}
    </div>
  </div>
</div>
