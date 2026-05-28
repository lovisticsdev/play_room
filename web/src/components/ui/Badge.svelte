<script lang="ts">
  export let tone: 'neutral' | 'accent' | 'success' | 'warning' | 'danger' | 'violet' = 'neutral';
</script>

<span class="badge {tone}">
  <slot />
</span>
