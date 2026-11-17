<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { RefreshCw, SlidersHorizontal, AlertTriangle } from '@lucide/svelte';
  import { api, formatAppError, onEvent, type UnlistenFn } from '../api';
  import { validCalibration, offsetLimit, displayDivisors, calibrationScales } from '../calibration';
  import type { CalibrationState, SerialDevice } from '../types';
  export let notify: (message: string, tone?: 'info' | 'error' | 'success') => void;
  let devices: SerialDevice[] = [];
  let selectedDevice = '';
  let state: CalibrationState = { deviceId: null, connected: false, busy: false, stage: '', samples: 0, values: null, statistics: [], error: null, possiblyCommitted: false };
  let gains = [1, 1, 1];
  let offsets = [0, 0, 0];
  let pending = false;
  let confirmation: { operation: 'auto' | 'manual' | 'reset'; range?: number; gain?: number; offsetUa?: number; text: string } | null = null;
  let confirmationDialog: HTMLDialogElement;
  $: if (confirmation && confirmationDialog && !confirmationDialog.open) confirmationDialog.showModal();
  let unlisten: UnlistenFn | undefined;
  let disposed = false;
  $: busy = state.busy || pending;
  $: editable = state.connected && !!state.values && !busy;
  $: stageKey = state.stage.startsWith('range:') ? 'collecting' : state.stage === 'warning' ? 'warningStatus' : state.stage || 'idle';
  function apply(value: CalibrationState) {
    state = value;
    if (value.deviceId) selectedDevice = value.deviceId;
    if (value.values && !value.busy) { gains = value.values.gains.map(v => Number(v.toFixed(6))); offsets = value.values.offsetsUa.map((v, i) => Number((v / displayDivisors[i]).toFixed(6))); }
  }
  async function attempt(action: () => Promise<void>) {
    pending = true;
    try { await action(); } catch (e) { notify(formatAppError(e, $_('errors.serialPermission')), 'error'); }
    finally { pending = false; }
  }
  async function refresh() { devices = await api.listDevices(); }
  function connect() { return attempt(async () => { apply(state.connected ? await api.disconnectCalibration() : await api.connectCalibration(selectedDevice)); }); }
  function run(operation: 'read' | 'auto' | 'manual' | 'reset', range?: number, gain?: number, offsetUa?: number) {
    return attempt(async () => { await api.runCalibration(operation, range, gain, offsetUa); apply(await api.calibrationState()); });
  }
  function manual(i: number) {
    if (!validCalibration(i, gains[i], offsets[i])) return;
    confirmation = { operation: 'manual', range: i + 1, gain: gains[i], offsetUa: offsets[i] * displayDivisors[i], text: `${$_('calibration.confirmManual')} ${['Low', 'Mid', 'High'][i]}\n${$_('calibration.old')}: k=${state.values!.gains[i]}, b=${state.values!.offsetsUa[i] / displayDivisors[i]} ${i === 2 ? 'mA' : 'µA'}\n${$_('calibration.new')}: k=${gains[i]}, b=${offsets[i]} ${i === 2 ? 'mA' : 'µA'}` };
  }
  function confirm() { const next = confirmation; confirmation = null; if (next) void run(next.operation, next.range, next.gain, next.offsetUa); }
  onMount(() => { void attempt(async () => {
    const stop = await onEvent<CalibrationState>('calibration-state', apply);
    if (disposed) { stop(); return; } unlisten = stop;
    await refresh(); if (!disposed) apply(await api.calibrationState());
  }); });
  onDestroy(() => { disposed = true; unlisten?.(); });
</script>

<section class="page">
  <header><h1 class="page-title">{$_('calibration.title')}</h1><p class="page-subtitle">{$_('calibration.subtitle')}</p></header>
  <div class="hero panel"><div class="hero-icon"><SlidersHorizontal size={30}/></div><div><h2>PowerPico Calibration</h2><p>{$_('calibration.formula')}</p></div></div>
  <div class="warning"><AlertTriangle size={20}/><span>{$_('calibration.warning')}</span></div>
  <div class="panel form">
    <label for="calibration-device">{$_('firmware.device')}</label>
    <div class="row"><select id="calibration-device" class="control" bind:value={selectedDevice} disabled={busy || state.connected}><option value="">{$_('waveform.selectDevice')}</option>{#each devices as device}<option value={device.id}>{device.displayName}</option>{/each}</select><button class="secondary-button" aria-label={$_('common.refresh')} onclick={() => attempt(refresh)} disabled={busy || state.connected}><RefreshCw size={17}/></button><button class="primary-button" onclick={connect} disabled={busy || (!selectedDevice && !state.connected)}>{state.connected ? $_('common.disconnect') : $_('common.connect')}</button></div>
    <div class="actions"><button class="secondary-button" onclick={() => run('read')} disabled={!state.connected || busy}>{$_('calibration.read')}</button><button class="primary-button" onclick={() => confirmation = { operation: 'auto', text: $_('calibration.warning') }} disabled={!state.connected || busy}>{$_('calibration.auto')}</button><button class="danger-button" onclick={() => confirmation = { operation: 'reset', text: $_('calibration.confirmReset') }} disabled={!editable}>{$_('calibration.reset')}</button></div>
    <div class="status" role="status"><strong>{$_(`calibration.${stageKey}`)}{state.stage.startsWith('range:') ? ` · ${['Low', 'Mid', 'High'][Number(state.stage.split(':')[1]) - 1]}` : ''}</strong>{#if state.busy}<progress max="60000" value={state.samples}></progress><span>{state.samples.toLocaleString()} / 60,000</span>{#if state.stage.startsWith('range:')}<button class="secondary-button" onclick={() => attempt(async () => { await api.cancelCalibration(); })}>{$_('common.cancel')}</button>{/if}{/if}</div>
    {#if state.error}<div class="error" role="alert">{state.error}{#if state.possiblyCommitted}<p>{$_('calibration.uncertain')}</p>{/if}</div>{/if}
  </div>
  <div class="panel ranges">
    <p class="hint">{$_('calibration.limits')}</p>
    {#each ['Low', 'Mid', 'High'] as name, i}
      <div class="range">
        <div><h3>{name}</h3><span class="hint">{['50 Ω', '0.5 Ω', '0.005 Ω'][i]} · {calibrationScales[i] / displayDivisors[i]} {i === 2 ? 'mA' : 'µA'}/LSB</span></div>
        <label>{$_('calibration.gain')}<input class="control" aria-label={`${name} ${$_('calibration.gain')}`} type="number" min="0.85" max="1.15" step="0.001" bind:value={gains[i]} disabled={!editable}/></label>
        <label>{$_('calibration.offset')} ({i === 2 ? 'mA' : 'µA'})<input class="control" aria-label={`${name} ${$_('calibration.offset')}`} type="number" min={-offsetLimit(i, gains[i])} max={offsetLimit(i, gains[i])} step="any" bind:value={offsets[i]} disabled={!editable}/></label>
        <button class="secondary-button" onclick={() => manual(i)} disabled={!editable || !validCalibration(i, gains[i], offsets[i])}>{$_('calibration.apply')}</button>
      </div>
    {/each}
  </div>
  {#if state.statistics.length}<div class="panel results"><h2>{$_('calibration.statistics')}</h2><table><thead><tr><th>{$_('calibration.range')}</th><th>{$_('calibration.samples')}</th><th>{$_('calibration.zero')} (LSB)</th><th>{$_('calibration.noise')} (LSB)</th></tr></thead><tbody>{#each state.statistics as result}<tr><td>{['Low', 'Mid', 'High'][result.range - 1]}</td><td>{result.sampleCount.toLocaleString()}</td><td>{result.zeroLsb.toFixed(4)}</td><td>{result.noiseLsb.toFixed(4)}</td></tr>{/each}</tbody></table></div>{/if}
</section>
{#if confirmation}
  <dialog bind:this={confirmationDialog} class="panel dialog" aria-labelledby="calibration-confirm" oncancel={() => confirmation = null}><h2 id="calibration-confirm">{$_('calibration.confirm')}</h2><p>{confirmation.text}</p><div class="actions"><button class="secondary-button" onclick={() => confirmation = null}>{$_('common.cancel')}</button><button class="primary-button" onclick={confirm}>{$_('calibration.apply')}</button></div></dialog>
{/if}
<style>
  .page { max-width: 1000px; margin: 0 auto; padding-bottom: 24px; } header { margin-bottom: 22px; }
  .hero { display:flex; align-items:center; gap:18px; padding:22px; border-radius:18px; } .hero-icon { color:var(--accent); background:var(--accent-soft); padding:15px; border-radius:16px; }
  h2 { margin:0; font-size:18px; } .hero p { color:var(--muted); font-size:13px; margin-bottom:0; }
  .warning { display:flex; align-items:center; gap:10px; margin:16px 0; padding:14px; border-radius:12px; background:var(--accent-soft); font-size:13px; }
  .form,.ranges,.results { padding:22px; border-radius:18px; margin-bottom:16px; } .form { display:grid; gap:14px; }
  .row,.actions { display:flex; gap:10px; flex-wrap:wrap; } select { flex:1; min-width:180px; } label { font-size:13px; font-weight:600; }
  .status { display:flex; align-items:center; flex-wrap:wrap; gap:12px; font-size:13px; } progress { flex:1; min-width:100px; accent-color:var(--accent); }
  .error { color:var(--danger, #dc3545); overflow-wrap:anywhere; font-size:13px; } .hint { color:var(--muted); font-size:12px; }
  .range { display:grid; grid-template-columns:minmax(170px,1fr) 125px 170px auto; gap:14px; align-items:center; padding:18px 0; border-bottom:1px solid var(--border); } .range:last-child { border:0; } h3 { margin:0 0 5px; font-size:15px; } input { width:100%; margin-top:7px; }
  table { width:100%; border-collapse:collapse; margin-top:16px; text-align:left; font-size:13px; } th,td { padding:10px 6px; border-bottom:1px solid var(--border); }
  .dialog::backdrop { background:#0007; } .dialog { max-width:560px; padding:25px; border-radius:18px; color:var(--text); } .dialog p { white-space:pre-line; line-height:1.7; } .dialog .actions { justify-content:flex-end; }
  @media (max-width:1050px) { .range {grid-template-columns:1fr 1fr;} .range > div {grid-column:1 / -1;} }
</style>
