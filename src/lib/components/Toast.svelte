<script lang="ts">
  import { X } from '@lucide/svelte';
  export let message = '';
  export let tone: 'info' | 'error' | 'success' = 'info';
  export let onClose: () => void;
</script>

{#if message}
  <div class="toast" class:error={tone === 'error'} class:success={tone === 'success'} role="status">
    <span>{message}</span>
    <button aria-label="Close" onclick={onClose}><X size={15} /></button>
  </div>
{/if}

<style>
  .toast { position: fixed; z-index: 20; right: 24px; bottom: 24px; max-width: 420px; display: flex; gap: 16px; align-items: center; padding: 12px 14px; border-radius: 12px; color: white; background: #33415a; box-shadow: 0 18px 50px rgba(0,0,0,.24); }
  .toast.error { background: #b53f46; }
  .toast.success { background: #16865a; }
  button { border: 0; color: inherit; background: transparent; padding: 2px; display: grid; place-items: center; }
</style>
