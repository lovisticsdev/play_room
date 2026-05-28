<script lang="ts">
  export let name = '?';
  export let connected = true;

  $: initials = name
    .trim()
    .split(/\s+/)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase() ?? '')
    .join('') || '?';
</script>

<div class="avatar" class:offline={!connected} aria-hidden="true">
  <span>{initials}</span>
</div>
