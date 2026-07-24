<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { Activity, Cpu, Settings as SettingsIcon, Zap } from '@lucide/svelte';
  import { api, formatAppError, onEvent, type UnlistenFn } from './lib/api';
  import { setLocale } from './lib/i18n';
  import { defaultWaveformMetrics } from './lib/metricLayout';
  import type { AppSettings } from './lib/types';
  import FirmwarePage from './lib/pages/FirmwarePage.svelte';
  import SettingsPage from './lib/pages/SettingsPage.svelte';
  import WaveformPage from './lib/pages/WaveformPage.svelte';
  import StatusDot from './lib/components/StatusDot.svelte';
  import Toast from './lib/components/Toast.svelte';

  type Page = 'waveform' | 'firmware' | 'settings';
  let page: Page = 'waveform';
  let connected = false;
  let settings: AppSettings = {
    theme: 'system', language: 'zh-CN',
    waveformMetrics: [...defaultWaveformMetrics],
  };
  let toast = { message: '', tone: 'info' as 'info' | 'error' | 'success' };
  let unlistenCapture: UnlistenFn | undefined;

  function notify(message: string, tone: 'info' | 'error' | 'success' = 'info') {
    toast = { message, tone };
    window.setTimeout(() => { if (toast.message === message) toast = { ...toast, message: '' }; }, 3500);
  }

  function applyTheme(theme: AppSettings['theme']) {
    const dark = theme === 'dark' || (theme === 'system' && matchMedia('(prefers-color-scheme: dark)').matches);
    document.documentElement.classList.toggle('dark', dark);
  }

  async function updateSettings(next: AppSettings) {
    settings = await api.updateSettings(next);
    setLocale(settings.language);
    applyTheme(settings.theme);
  }

  onMount(async () => {
    try {
      settings = await api.getSettings();
      setLocale(settings.language); applyTheme(settings.theme);
      const state = await api.captureState(); connected = state.status === 'capturing';
      unlistenCapture = await onEvent<{ status: string }>('capture-state-changed', (next) => { connected = next.status === 'capturing'; });
      const media = matchMedia('(prefers-color-scheme: dark)');
      media.addEventListener('change', () => { if (settings.theme === 'system') applyTheme('system'); });
    } catch (error) { notify(formatAppError(error, $_('errors.serialPermission')), 'error'); }
  });
  onDestroy(() => unlistenCapture?.());
</script>

<div class="shell">
  <aside class="panel">
    <div class="brand"><div class="logo"><Zap size={21} fill="currentColor"/></div><div><strong>PowerPico</strong><span>Client</span></div></div>
    <nav>
      <button class:active={page === 'waveform'} onclick={() => page = 'waveform'} aria-label={$_('nav.waveform')}><Activity size={19}/><span>{$_('nav.waveform')}</span></button>
      <button class:active={page === 'firmware'} onclick={() => page = 'firmware'} aria-label={$_('nav.firmware')}><Cpu size={19}/><span>{$_('nav.firmware')}</span></button>
      <button class:active={page === 'settings'} onclick={() => page = 'settings'} aria-label={$_('nav.settings')}><SettingsIcon size={19}/><span>{$_('nav.settings')}</span></button>
    </nav>
    <div class="connection"><StatusDot active={connected}/><span>{connected ? $_('app.connected') : $_('app.disconnected')}</span></div>
  </aside>
  <main>
    {#if page === 'waveform'}<WaveformPage {notify} {settings}/>{:else if page === 'firmware'}<FirmwarePage {notify}/>{:else}<SettingsPage {settings} update={updateSettings}/>{/if}
  </main>
</div>
<Toast message={toast.message} tone={toast.tone} onClose={() => toast = { ...toast, message: '' }}/>

<style>
  .shell { height: 100%; display: grid; grid-template-columns: 190px minmax(0, 1fr); gap: 14px; padding: 14px; }
  aside { border-radius: 19px; padding: 18px 12px 13px; display: flex; flex-direction: column; min-height: 0; }
  .brand { display: flex; align-items: center; gap: 10px; padding: 2px 8px 22px; }
  .logo { width: 36px; height: 36px; border-radius: 11px; display: grid; place-items: center; color: white; background: linear-gradient(145deg, #246bfd, #56a6ff); box-shadow: 0 8px 24px rgba(36,107,253,.28); }
  .brand strong, .brand span { display: block; } .brand strong { font-size: 15px; } .brand span { color: var(--muted); font-size: 11px; margin-top: 1px; }
  nav { display: grid; gap: 5px; }
  nav button { width: 100%; height: 42px; padding: 0 11px; display: flex; align-items: center; gap: 11px; border: 0; border-radius: 11px; color: var(--muted); background: transparent; font-weight: 600; }
  nav button:hover { color: var(--text); background: var(--panel-muted); }
  nav button.active { color: var(--accent); background: var(--accent-soft); }
  .connection { margin-top: auto; display: flex; align-items: center; gap: 9px; padding: 12px 9px 4px; color: var(--muted); font-size: 11px; }
  main { min-width: 0; min-height: 0; overflow: auto; padding: 17px 19px 8px 5px; }
  @media (max-width: 850px) { .shell { grid-template-columns: 72px minmax(0, 1fr); } .brand > div:last-child, nav span, .connection span { display: none; } aside { align-items: center; } nav button { width: 44px; justify-content: center; padding: 0; } }
</style>
