<script lang="ts">
  import { connectionStore } from '../../lib/stores/connection';
  import GlassCard from '../ui/GlassCard.svelte';

  let copied = false;

  async function copyToken() {
    if (!$connectionStore.welcome) return;
    await navigator.clipboard.writeText($connectionStore.welcome.reconnect_token);
    copied = true;
    setTimeout(() => (copied = false), 2000);
  }
</script>

<GlassCard>
  <h2 class="h5 mb-2">Reconnect</h2>
  {#if $connectionStore.welcome}
    <div class="small text-muted mb-2">Save this token to reconnect as the same server identity.</div>
    <div class="input-group">
      <input class="form-control room-id" value={$connectionStore.welcome.reconnect_token} readonly />
      <button class="btn btn-outline-info" type="button" onclick={copyToken}>
        {copied ? 'Copied!' : 'Copy'}
      </button>
    </div>
  {:else}
    <div class="text-muted small">Connect to receive a reconnect token.</div>
  {/if}
</GlassCard>