<script lang="ts">
  export let id: string;
  export let label: string | null = null;
  export let value = '';
  export let placeholder = '';
  export let type = 'text';
  export let disabled = false;
  export let required = false;
</script>

<label class="field" for={id}>
  {#if label}<span>{label}</span>{/if}
  <input
    {id}
    class="text-input"
    {type}
    bind:value
    {placeholder}
    {disabled}
    {required}
    {...$$restProps}
  />
</label>
