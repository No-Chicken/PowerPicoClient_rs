<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { open } from '@tauri-apps/plugin-dialog';
  import { FileCode2, RefreshCw, Rocket, ShieldAlert, XCircle } from '@lucide/svelte';
  import { api, formatAppError, onEvent, type UnlistenFn } from '../api';
  import type { FirmwareProgress, SerialDevice } from '../types';

  export let notify: (message: string, tone?: 'info' | 'error' | 'success') => void;
  let devices: SerialDevice[] = [];
  let selectedDevice = '';
  let firmwarePath = '';
  let progress: FirmwareProgress = { stage: 'idle', percent: 0, message: '' };
  let unlisteners: UnlistenFn[] = [];
  $: busy = !['idle', 'completed', 'cancelled', 'failed'].includes(progress.stage);

  function errorMessage(error: unknown) { return formatAppError(error, $_('errors.serialPermission')); }
  async function refreshDevices() { try { devices = await api.listDevices(); } catch (error) { notify(errorMessage(error), 'error'); } }
  async function chooseFile() {
    const path = await open({ multiple: false, filters: [{ name: 'Firmware', extensions: ['bin'] }] });
    if (typeof path === 'string') firmwarePath = path;
  }
  async function flash() {
    if (!selectedDevice) return notify($_('waveform.noDevice'), 'error');
    if (!firmwarePath) return notify($_('firmware.noFile'), 'error');
    try { await api.flashFirmware(selectedDevice, firmwarePath); } catch (error) { notify(errorMessage(error), 'error'); }
  }
  async function cancel() { try { await api.cancelFlash(); } catch (error) { notify(errorMessage(error), 'error'); } }

  onMount(async () => {
    await refreshDevices();
    unlisteners.push(await onEvent<FirmwareProgress>('firmware-progress', (value) => { progress = value; }));
    unlisteners.push(await onEvent<FirmwareProgress>('firmware-finished', (value) => {
      progress = value;
      notify(value.message, value.stage === 'completed' ? 'success' : 'error');
    }));
  });
  onDestroy(() => unlisteners.forEach((fn) => fn()));
</script>

<section class="page">
  <header><h1 class="page-title">{$_('firmware.title')}</h1><p class="page-subtitle">{$_('firmware.subtitle')}</p></header>
  <div class="hero panel">
    <div class="hero-icon"><Rocket size={34}/></div>
    <div><h2>PowerPico Firmware</h2><p>{$_('firmware.warning')}</p></div>
  </div>
  <div class="form panel">
    <label><span>{$_('firmware.device')}</span><div class="row"><select class="control" bind:value={selectedDevice} disabled={busy}><option value="">{$_('waveform.selectDevice')}</option>{#each devices as device}<option value={device.id}>{device.displayName}</option>{/each}</select><button class="secondary-button icon" onclick={refreshDevices} disabled={busy}><RefreshCw size={17}/></button></div></label>
    <label><span>{$_('firmware.file')}</span><div class="row"><div class="file control"><FileCode2 size={17}/><span>{firmwarePath || $_('firmware.choose')}</span></div><button class="secondary-button" onclick={chooseFile} disabled={busy}>{$_('common.browse')}</button></div></label>
    <div class="status">
      <div class="status-line"><strong>{progress.message || $_('firmware.idle')}</strong><span>{progress.percent}%</span></div>
      <div class="track"><div class="bar" style:width={`${progress.percent}%`}></div></div>
    </div>
    <div class="actions">
      {#if busy}<button class="danger-button" onclick={cancel}><XCircle size={17}/>{$_('common.cancel')}</button>{:else}<button class="primary-button" onclick={flash}><ShieldAlert size={17}/>{$_('firmware.flash')}</button>{/if}
    </div>
  </div>
</section>

<style>
  .page { max-width: 920px; margin: 0 auto; }
  header { margin-bottom: 22px; }
  .hero, .form { border-radius: 18px; }
  .hero { padding: 22px; display: flex; gap: 17px; align-items: center; margin-bottom: 16px; }
  .hero-icon { width: 58px; height: 58px; border-radius: 17px; display: grid; place-items: center; color: var(--accent); background: var(--accent-soft); }
  h2 { margin: 0; font-size: 18px; } .hero p { margin: 6px 0 0; color: var(--muted); font-size: 13px; }
  .form { padding: 24px; display: grid; gap: 22px; }
  label > span { display: block; font-weight: 650; margin-bottom: 9px; font-size: 13px; }
  .row { display: flex; gap: 9px; } select { flex: 1; } .icon { width: 38px; padding: 0; }
  .file { flex: 1; display: flex; align-items: center; gap: 9px; overflow: hidden; }
  .file span { white-space: nowrap; text-overflow: ellipsis; overflow: hidden; color: var(--muted); }
  .status { padding-top: 4px; } .status-line { display: flex; justify-content: space-between; font-size: 13px; margin-bottom: 9px; }
  .track { height: 8px; border-radius: 999px; overflow: hidden; background: var(--panel-muted); }
  .bar { height: 100%; min-width: 0; border-radius: inherit; background: linear-gradient(90deg, var(--accent), #55a4ff); transition: width .2s ease; }
  .actions { display: flex; justify-content: flex-end; }
</style>
