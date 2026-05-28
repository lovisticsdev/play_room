<script lang="ts">
  export let variant: 'primary' | 'secondary' | 'danger' | 'ghost' | 'success' | 'warning' = 'primary';
  export let type: 'button' | 'submit' | 'reset' = 'button';
  export let disabled = false;
  export let full = false;
</script>

<button class="button {variant}" class:full {type} {disabled} {...$$restProps}>
  <slot />
</button>
