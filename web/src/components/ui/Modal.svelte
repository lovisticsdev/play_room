<script lang="ts">
  export let open = false;
  export let title: string;
  export let subtitle: string | null = null;
  export let closeDisabled = false;
  export let onClose: (() => void) | null = null;

  function close() {
    if (closeDisabled) return;
    onClose?.();
  }
</script>

{#if open}
  <div class="modal-backdrop" role="presentation" onclick={close}></div>
  <div class="modal-shell" role="dialog" aria-modal="true" aria-labelledby="modal-title">
    <header class="modal-header">
      <div class="modal-icon" aria-hidden="true">↔</div>
      <div>
        <h2 id="modal-title">{title}</h2>
        {#if subtitle}<p>{subtitle}</p>{/if}
      </div>
      {#if !closeDisabled}
        <button class="modal-close" type="button" aria-label="Close" onclick={close}>×</button>
      {/if}
    </header>
    <slot />
  </div>
{/if}
