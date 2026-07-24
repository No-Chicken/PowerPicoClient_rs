<script lang="ts">
  export let active = false;
</script>

<span class:active class="dot" aria-hidden="true"></span>

<style>
  .dot { width: 8px; height: 8px; border-radius: 50%; display: inline-block; background: #8d98aa; box-shadow: 0 0 0 4px color-mix(in srgb, #8d98aa 14%, transparent); }
  .dot.active { background: #29b576; box-shadow: 0 0 0 4px color-mix(in srgb, #29b576 16%, transparent); }
</style>
