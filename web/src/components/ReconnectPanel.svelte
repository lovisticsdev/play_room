<script lang="ts">
  import { appState } from '../lib/client-state';

  async function copyToken(): Promise<void> {
    if (!$appState.welcome) return;
    await navigator.clipboard.writeText($appState.welcome.reconnect_token);
  }
</script>

<div class="card glass-card mb-3">
  <div class="card-body">
    <h2 class="h5 mb-2">Reconnect</h2>
    {#if $appState.welcome}
      <div class="small text-muted mb-2">Save this token to reconnect as the same server identity.</div>
      <div class="input-group">
        <input class="form-control room-id" value={$appState.welcome.reconnect_token} readonly />
        <button class="btn btn-outline-info" type="button" onclick={copyToken}>Copy</button>
      </div>
    {:else}
      <div class="text-muted small">Connect to receive a reconnect token.</div>
    {/if}
  </div>
</div>
