<script lang="ts">
  export let title: string | null = null;
  export let eyebrow: string | null = null;
  export let compact = false;
</script>

<section class="panel" class:compact>
  {#if title || eyebrow}
    <header class="panel-header">
      <div>
        {#if eyebrow}<div class="panel-eyebrow">{eyebrow}</div>{/if}
        {#if title}<h2>{title}</h2>{/if}
      </div>
      <slot name="action" />
    </header>
  {/if}
  <div class="panel-body">
    <slot />
  </div>
</section>
