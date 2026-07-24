<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { open } from '@tauri-apps/plugin-dialog';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { Download, ExternalLink, FileCode2, RefreshCw, Rocket, ShieldAlert, XCircle } from '@lucide/svelte';
  import { api, formatAppError, onEvent, type UnlistenFn } from '../api';
  import type { AppSettings, ExternalLinks, FirmwareDownloadProgress, FirmwareProgress, OfficialFirmwareInfo, SerialDevice } from '../types';

  export let notify: (message: string, tone?: 'info' | 'error' | 'success') => void;
  export let settings: AppSettings;
  export let updateSettings: (settings: AppSettings) => Promise<void>;

  let devices: SerialDevice[] = [];
  let selectedDevice = '';
  let official: OfficialFirmwareInfo | null = null;
  let links: ExternalLinks | null = null;
  let progress: FirmwareProgress = { stage: 'idle', percent: 0, messageKey: 'idle' };
  let downloadProgress: FirmwareDownloadProgress | null = null;
  let loadingOfficial = false;
  let unlisteners: UnlistenFn[] = [];
  $: flashing = !['idle', 'completed', 'cancelled', 'failed'].includes(progress.stage);
  $: downloading = downloadProgress?.stage === 'downloading';
  $: busy = flashing || downloading;
  $: firmwarePath = settings.firmwareMode === 'official' ? official?.localPath || '' : settings.customFirmwarePath;

  function errorMessage(error: unknown) { return formatAppError(error, $_('errors.serialPermission')); }
  function progressText(value: FirmwareProgress) {
    const translated = $_(`firmware.${value.messageKey}`);
    return value.detail ? `${translated}: ${value.detail}` : translated;
  }
  async function refreshDevices() { try { devices = await api.listDevices(); } catch (error) { notify(errorMessage(error), 'error'); } }
  async function refreshOfficial() {
    loadingOfficial = true;
    try { official = await api.officialFirmwareInfo(); }
    catch (error) { notify(errorMessage(error), 'error'); }
    finally { loadingOfficial = false; }
  }
  async function chooseFile() {
    const path = await open({ multiple: false, filters: [{ name: 'Firmware', extensions: ['bin'] }] });
    if (typeof path === 'string') await updateSettings({ ...settings, firmwareMode: 'custom', customFirmwarePath: path });
  }
  async function downloadLatest() {
    if (!official) return;
    if (official.downloaded && settings.localFirmwareVersion === official.version) return notify($_('firmware.alreadyLatest'), 'success');
    try { await api.downloadOfficialFirmware(official.url, official.version, official.releaseDate); }
    catch (error) { notify(errorMessage(error), 'error'); }
  }
  async function flash() {
    if (!selectedDevice) return notify($_('waveform.noDevice'), 'error');
    if (!firmwarePath) return notify($_('firmware.noFile'), 'error');
    try { await api.flashFirmware(selectedDevice, firmwarePath); } catch (error) { notify(errorMessage(error), 'error'); }
  }
  async function cancel() {
    try {
      if (downloading) await api.cancelFirmwareDownload();
      else await api.cancelFlash();
    } catch (error) { notify(errorMessage(error), 'error'); }
  }
  async function loadExternalLinks() {
    try { links = await api.externalLinks(); }
    catch { notify($_('errors.externalLink'), 'error'); }
  }
  async function openExternalLink(url?: string) {
    if (!url) return notify($_('errors.externalLink'), 'error');
    try { await openUrl(url); }
    catch { notify($_('errors.externalLink'), 'error'); }
  }

  onMount(async () => {
    await Promise.all([refreshDevices(), refreshOfficial(), loadExternalLinks()]);
    unlisteners.push(await onEvent<FirmwareProgress>('firmware-progress', (value) => { progress = value; }));
    unlisteners.push(await onEvent<FirmwareProgress>('firmware-finished', (value) => {
      progress = value;
      notify(progressText(value), value.stage === 'completed' ? 'success' : 'error');
    }));
    unlisteners.push(await onEvent<FirmwareDownloadProgress>('firmware-download-progress', async (value) => {
      downloadProgress = value;
      if (value.stage === 'completed' && official) {
        await updateSettings({ ...settings, localFirmwareVersion: official.version, localFirmwareReleaseDate: official.releaseDate });
        await refreshOfficial();
        notify($_('firmware.downloadCompleted'), 'success');
      } else if (value.stage === 'failed') notify(`${$_('firmware.downloadFailed')}${value.detail ? `: ${value.detail}` : ''}`, 'error');
      else if (value.stage === 'cancelled') notify($_('firmware.downloadCancelled'));
    }));
  });
  onDestroy(() => unlisteners.forEach((fn) => fn()));
</script>

<section class="page">
  <header><h1 class="page-title">{$_('firmware.title')}</h1><p class="page-subtitle">{$_('firmware.subtitle')}</p></header>
  <div class="hero panel">
    <div class="hero-icon"><Rocket size={34}/></div>
    <div class="hero-copy"><h2>PowerPico Firmware</h2><p>{$_('firmware.warning')}</p></div>
    {#if official}<div class="firmware-meta"><strong>{official.version}</strong><span>{official.releaseDate}</span></div>{/if}
  </div>
  <div class="form panel">
    <label><span>{$_('firmware.source')}</span><select class="control" value={settings.firmwareMode} onchange={(event) => updateSettings({ ...settings, firmwareMode: (event.target as HTMLSelectElement).value as AppSettings['firmwareMode'] })} disabled={busy}><option value="official">{$_('firmware.official')}</option><option value="custom">{$_('firmware.custom')}</option></select></label>

    {#if settings.firmwareMode === 'official'}
      <div class="official-row">
        <div><strong>{$_('firmware.latest')}: {official?.version || '—'}</strong><span>{$_('firmware.releaseDate')}: {official?.releaseDate || '—'}</span></div>
        <button class="secondary-button" onclick={refreshOfficial} disabled={busy || loadingOfficial}><RefreshCw size={17}/>{$_('common.refresh')}</button>
        <button class="primary-button" onclick={downloadLatest} disabled={busy || !official}><Download size={17}/>{$_('firmware.downloadLatest')}</button>
        <button class="secondary-button" onclick={() => openExternalLink(links?.firmwareReleaseNotes)}><ExternalLink size={17}/>{$_('firmware.releaseNotes')}</button>
      </div>
    {:else}
      <label><span>{$_('firmware.file')}</span><div class="row"><div class="file control"><FileCode2 size={17}/><span>{settings.customFirmwarePath || $_('firmware.choose')}</span></div><button class="secondary-button" onclick={chooseFile} disabled={busy}>{$_('common.browse')}</button></div></label>
    {/if}

    <label><span>{$_('firmware.device')}</span><div class="row"><select class="control" bind:value={selectedDevice} disabled={busy}><option value="">{$_('waveform.selectDevice')}</option>{#each devices as device}<option value={device.id}>{device.displayName}</option>{/each}</select><button class="secondary-button icon" onclick={refreshDevices} disabled={busy}><RefreshCw size={17}/></button></div></label>

    <div class="status">
      <div class="status-line"><strong>{downloading ? $_('firmware.downloading') : progressText(progress)}</strong><span>{downloading ? downloadProgress?.percent : progress.percent}%</span></div>
      <div class="track"><div class="bar" style:width={`${downloading ? downloadProgress?.percent || 0 : progress.percent}%`}></div></div>
    </div>
    <div class="actions">{#if busy}<button class="danger-button" onclick={cancel}><XCircle size={17}/>{$_('common.cancel')}</button>{:else}<button class="primary-button" onclick={flash}><ShieldAlert size={17}/>{$_('firmware.flash')}</button>{/if}</div>
  </div>
</section>

<style>
  .page { max-width: 960px; margin: 0 auto; padding-bottom: 24px; }
  header { margin-bottom: 22px; }
  .hero, .form { border-radius: 18px; }
  .hero { padding: 22px; display: flex; gap: 17px; align-items: center; margin-bottom: 16px; }
  .hero-icon { width: 58px; height: 58px; border-radius: 17px; display: grid; place-items: center; color: var(--accent); background: var(--accent-soft); }
  .hero-copy { flex: 1; } .firmware-meta { text-align: right; } .firmware-meta strong, .firmware-meta span { display: block; } .firmware-meta span { color: var(--muted); font-size: 12px; margin-top: 4px; }
  h2 { margin: 0; font-size: 18px; } .hero p { margin: 6px 0 0; color: var(--muted); font-size: 13px; }
  .form { padding: 24px; display: grid; gap: 22px; }
  label > span { display: block; font-weight: 650; margin-bottom: 9px; font-size: 13px; }
  .row { display: flex; gap: 9px; } select { width: 100%; } .icon { width: 38px; padding: 0; }
  .file { flex: 1; display: flex; align-items: center; gap: 9px; overflow: hidden; }
  .file span { white-space: nowrap; text-overflow: ellipsis; overflow: hidden; color: var(--muted); }
  .official-row { display: flex; align-items: center; gap: 9px; padding: 15px; border-radius: 12px; background: var(--panel-muted); }
  .official-row > div { flex: 1; } .official-row strong, .official-row span { display: block; } .official-row span { color: var(--muted); margin-top: 4px; font-size: 12px; }
  .status { padding-top: 4px; } .status-line { display: flex; justify-content: space-between; font-size: 13px; margin-bottom: 9px; }
  .track { height: 8px; border-radius: 999px; overflow: hidden; background: var(--panel-muted); }
  .bar { height: 100%; border-radius: inherit; background: linear-gradient(90deg, var(--accent), #55a4ff); transition: width .2s ease; }
  .actions { display: flex; justify-content: flex-end; }
  @media (max-width: 760px) { .official-row { align-items: stretch; flex-direction: column; } .official-row > * { width: 100%; } }
</style>
