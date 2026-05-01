<script lang="ts">
  let { children } = $props();
</script>

<!-- Bare layout: no navigation, no background, no padding.
     The overlay window is decorations=false + transparent; only the page renders. -->
{@render children()}
