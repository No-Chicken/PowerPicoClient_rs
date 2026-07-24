<script lang="ts">
  export let label: string;
  export let value: string;
  export let tone: 'default' | 'voltage' | 'current' | 'power' = 'default';
</script>

<div class="metric" class:voltage={tone === 'voltage'} class:current={tone === 'current'} class:power={tone === 'power'}>
  <span>{label}</span>
  <strong>{value}</strong>
</div>

<style>
  .metric { min-width: 0; padding: 15px 16px; border-radius: 14px; background: var(--panel-muted); border: 1px solid var(--border); }
  span { color: var(--muted); font-size: 12px; }
  strong { display: block; margin-top: 7px; font-size: 21px; letter-spacing: -.03em; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .voltage strong { color: var(--voltage); }
  .current strong { color: var(--current); }
  .power strong { color: var(--power); }
</style>
