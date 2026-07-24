<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { open } from '@tauri-apps/plugin-dialog';
  import {
    Crosshair, FolderInput, FolderOutput, Radio, RefreshCw, RotateCcw, Trash2, Usb, Unplug, X,
  } from '@lucide/svelte';
  import uPlot from 'uplot';
  import { api, formatAppError, onEvent, type UnlistenFn } from '../api';
  import { formatCurrent, formatPower, metricById } from '../metrics';
  import { createRefreshScheduler } from '../refreshScheduler';
  import type {
    AppSettings, CaptureState, CaptureSummary, PointReading, RenderSeries, SerialDevice,
  } from '../types';
  import {
    autoRange, exceedsDragThreshold, panRange, rangeSpan, zoomRange, type NumericRange,
  } from '../waveformMath';
  import MetricCard from '../components/MetricCard.svelte';

  export let notify: (message: string, tone?: 'info' | 'error' | 'success') => void;
  export let settings: AppSettings;

  type InteractionTarget = 'time' | 'voltage' | 'current';
  interface DragState {
    target: InteractionTarget;
    startX: number;
    startY: number;
    lastX: number;
    lastY: number;
    dragged: boolean;
    startedInPlot: boolean;
  }

  let devices: SerialDevice[] = [];
  let selectedDevice = '';
  let state: CaptureState = { status: 'idle' };
  let summary: CaptureSummary = emptySummary();
  let chartHost: HTMLDivElement;
  let chart: uPlot | undefined;
  let resizeObserver: ResizeObserver | undefined;
  let unlisteners: UnlistenFn[] = [];
  let currentSeries: RenderSeries = emptySeries();
  let renderPending = false;
  let renderQueued = false;
  let latestEnd = 0;
  let timeViewport: NumericRange = { min: 0, max: 10 };
  let voltageViewport: NumericRange | null = null;
  let currentViewport: NumericRange | null = null;
  let followLive = true;
  let selectedPoint: PointReading | null = null;
  let dragState: DragState | null = null;
  let viewRevision = 0;
  const refreshScheduler = createRefreshScheduler(() => void refreshPlot(), 34);

  function errorMessage(error: unknown) {
    return formatAppError(error, $_('errors.serialPermission'));
  }

  function emptySummary(): CaptureSummary {
    return {
      pointCount: 0, duration: 0, latestVoltage: 0, voltageAverage: 0, voltagePeak: 0,
      latestCurrent: 0, currentAverage: 0, currentPeak: 0, latestPowerMw: 0,
      powerAverageMw: 0, energyMah: 0,
    };
  }

  function emptySeries(): RenderSeries {
    return {
      time: [], voltageMin: [], voltageMax: [], voltageAverage: [], currentMin: [],
      currentMax: [], currentAverage: [], aggregated: false, availableStart: 0, availableEnd: 0,
    };
  }

  async function refreshDevices() {
    try {
      devices = await api.listDevices();
      if (!devices.some((device) => device.id === selectedDevice)) selectedDevice = '';
    } catch (error) { notify(errorMessage(error), 'error'); }
  }

  async function toggleCapture() {
    try {
      if (state.status === 'capturing' || state.status === 'connecting') {
        state = await api.stopCapture();
      } else {
        if (!selectedDevice) return notify($_('waveform.noDevice'), 'error');
        clearSelection();
        followLive = true;
        timeViewport = { min: 0, max: 10 };
        state = await api.startCapture(selectedDevice);
      }
    } catch (error) { notify(errorMessage(error), 'error'); }
  }

  async function importRecording() {
    const path = await open({ multiple: false, filters: [{ name: 'PowerPico Binary', extensions: ['bin'] }] });
    if (typeof path !== 'string') return;
    try {
      state = await api.importRecording(path);
      summary = await api.stats();
      latestEnd = summary.duration;
      followLive = false;
      timeViewport = { min: 0, max: Math.max(summary.duration, 0.001) };
      voltageViewport = null;
      currentViewport = null;
      clearSelection();
      await refreshPlot();
      notify($_('waveform.imported'), 'success');
    } catch (error) { notify(errorMessage(error), 'error'); }
  }

  async function exportRecording() {
    const directory = await open({ directory: true, multiple: false });
    if (typeof directory !== 'string') return;
    try {
      await api.exportRecording(directory);
      notify($_('waveform.exportSuccess'), 'success');
    } catch (error) { notify(errorMessage(error), 'error'); }
  }

  async function clearRecords() {
    if (!confirm($_('waveform.confirmClear'))) return;
    try {
      await api.clearRecords();
      summary = emptySummary();
      state = await api.captureState();
      latestEnd = 0;
      followLive = true;
      timeViewport = { min: 0, max: 10 };
      voltageViewport = null;
      currentViewport = null;
      currentSeries = emptySeries();
      clearSelection();
      updateChart(currentSeries);
    } catch (error) { notify(errorMessage(error), 'error'); }
  }

  function liveRange(): NumericRange {
    const end = Math.max(latestEnd, summary.duration);
    return { min: Math.max(0, end - 10), max: Math.max(10, end) };
  }

  function timeBounds(): NumericRange | undefined {
    const end = Math.max(latestEnd, summary.duration, currentSeries.availableEnd);
    if (end <= 0) return undefined;
    return { min: Math.min(0, currentSeries.availableStart), max: end };
  }

  function returnToLive() {
    followLive = true;
    timeViewport = liveRange();
    applyScale('x', timeViewport);
    scheduleRefresh();
  }

  function resetAxes() {
    voltageViewport = null;
    currentViewport = null;
    applyYScales();
  }

  function resetAxis(target: 'voltage' | 'current') {
    if (target === 'voltage') voltageViewport = null;
    else currentViewport = null;
    applyYScales();
  }

  function createChart() {
    if (!chartHost) return;
    chart = new uPlot({
      width: Math.max(chartHost.clientWidth, 300),
      height: Math.max(chartHost.clientHeight, 320),
      cursor: { drag: { x: false, y: false, setScale: false }, points: { show: false } },
      scales: { x: { time: false }, voltage: { auto: false }, current: { auto: false } },
      axes: [
        { stroke: '#8290a5', grid: { stroke: 'rgba(130,144,165,.13)' }, values: (_u, values) => values.map((v) => `${v.toFixed(2)}s`) },
        { scale: 'voltage', stroke: '#ef565d', grid: { stroke: 'rgba(130,144,165,.10)' }, label: 'V', size: 62 },
        { scale: 'current', side: 1, stroke: '#2887f0', grid: { show: false }, label: 'µA', size: 72 },
      ],
      series: [
        {},
        { label: 'Voltage', scale: 'voltage', stroke: '#ef565d', width: 1.5 },
        { label: 'Current', scale: 'current', stroke: '#2887f0', width: 1.5 },
      ],
      legend: { show: false },
      hooks: { draw: [drawSelection] },
    }, [[], [], []], chartHost);

    installInteractions(chart);
    resizeObserver = new ResizeObserver(() => {
      if (!chart || !chartHost.clientWidth || !chartHost.clientHeight) return;
      chart.setSize({ width: chartHost.clientWidth, height: chartHost.clientHeight });
      viewRevision += 1;
      scheduleRefresh();
    });
    resizeObserver.observe(chartHost);
  }

  function geometry() {
    if (!chart) return null;
    const over = chart.root.querySelector('.u-over') as HTMLElement | null;
    if (!over) return null;
    return { over, overRect: over.getBoundingClientRect(), rootRect: chart.root.getBoundingClientRect() };
  }

  function targetAt(clientX: number, clientY: number): InteractionTarget | null {
    const geo = geometry();
    if (!geo) return null;
    const { overRect, rootRect } = geo;
    if (clientY < rootRect.top || clientY > rootRect.bottom || clientX < rootRect.left || clientX > rootRect.right) return null;
    if (clientX < overRect.left) return 'voltage';
    if (clientX > overRect.right) return 'current';
    return 'time';
  }

  function isInPlot(clientX: number, clientY: number): boolean {
    const rect = geometry()?.overRect;
    return !!rect && clientX >= rect.left && clientX <= rect.right && clientY >= rect.top && clientY <= rect.bottom;
  }

  function scaleRange(scale: 'voltage' | 'current'): NumericRange {
    const current = chart?.scales[scale];
    return {
      min: typeof current?.min === 'number' ? current.min : 0,
      max: typeof current?.max === 'number' ? current.max : 1,
    };
  }

  function installInteractions(instance: uPlot) {
    const root = instance.root;
    root.addEventListener('wheel', handleWheel, { passive: false });
    root.addEventListener('pointerdown', handlePointerDown);
    root.addEventListener('pointermove', handlePointerMove);
    root.addEventListener('pointerup', handlePointerUp);
    root.addEventListener('pointercancel', () => dragState = null);
    root.addEventListener('dblclick', handleDoubleClick);
  }

  function handleWheel(event: WheelEvent) {
    if (!chart) return;
    const target = targetAt(event.clientX, event.clientY);
    const geo = geometry();
    if (!target || !geo) return;
    event.preventDefault();
    const factor = Math.exp(event.deltaY * 0.0015);
    if (target === 'time') {
      const bounds = timeBounds();
      if (!bounds) return;
      const position = Math.min(geo.overRect.width, Math.max(0, event.clientX - geo.overRect.left));
      const center = chart.posToVal(position, 'x');
      timeViewport = zoomRange(timeViewport, center, factor, bounds, 0.001);
      followLive = false;
      applyScale('x', timeViewport);
      scheduleRefresh();
    } else {
      const position = Math.min(geo.overRect.height, Math.max(0, event.clientY - geo.overRect.top));
      const center = chart.posToVal(position, target);
      const range = target === 'voltage' ? (voltageViewport ?? scaleRange('voltage')) : (currentViewport ?? scaleRange('current'));
      const next = zoomRange(range, center, factor, undefined, Math.max(rangeSpan(range) / 10_000, 1e-9));
      if (target === 'voltage') voltageViewport = next;
      else currentViewport = next;
      applyScale(target, next);
    }
  }

  function handlePointerDown(event: PointerEvent) {
    if (event.button !== 0) return;
    const target = targetAt(event.clientX, event.clientY);
    if (!target) return;
    dragState = {
      target, startX: event.clientX, startY: event.clientY, lastX: event.clientX, lastY: event.clientY,
      dragged: false, startedInPlot: isInPlot(event.clientX, event.clientY),
    };
    chart?.root.setPointerCapture(event.pointerId);
  }

  function handlePointerMove(event: PointerEvent) {
    if (!dragState || !chart) return;
    if (!dragState.dragged && exceedsDragThreshold(dragState.startX, dragState.startY, event.clientX, event.clientY)) {
      dragState.dragged = true;
    }
    if (!dragState.dragged) return;
    const rect = geometry()?.overRect;
    if (!rect) return;
    const deltaX = event.clientX - dragState.lastX;
    const deltaY = event.clientY - dragState.lastY;
    if (dragState.target === 'time') {
      timeViewport = panRange(timeViewport, -deltaX, rect.width, timeBounds());
      followLive = false;
      applyScale('x', timeViewport);
      scheduleRefresh();
    } else if (dragState.target === 'voltage') {
      voltageViewport = panRange(voltageViewport ?? scaleRange('voltage'), deltaY, rect.height);
      applyScale('voltage', voltageViewport);
    } else {
      currentViewport = panRange(currentViewport ?? scaleRange('current'), deltaY, rect.height);
      applyScale('current', currentViewport);
    }
    dragState.lastX = event.clientX;
    dragState.lastY = event.clientY;
  }

  function handlePointerUp(event: PointerEvent) {
    const completed = dragState;
    dragState = null;
    if (!completed?.dragged && completed?.startedInPlot) void selectAtPointer(event.clientX);
  }

  function handleDoubleClick(event: MouseEvent) {
    const target = targetAt(event.clientX, event.clientY);
    if (target === 'voltage' || target === 'current') resetAxis(target);
  }

  async function selectAtPointer(clientX: number) {
    if (!chart || currentSeries.time.length === 0) return clearSelection();
    const rect = geometry()?.overRect;
    if (!rect) return;
    const time = chart.posToVal(clientX - rect.left, 'x');
    if (time < currentSeries.availableStart || time > currentSeries.availableEnd) return clearSelection();
    try {
      selectedPoint = await api.pointAt(time);
      viewRevision += 1;
      chart.redraw();
    } catch {
      clearSelection();
    }
  }

  function clearSelection() {
    selectedPoint = null;
    viewRevision += 1;
    chart?.redraw();
  }

  function selectedVisible(): boolean {
    void viewRevision;
    return !!selectedPoint && selectedPoint.time >= timeViewport.min && selectedPoint.time <= timeViewport.max;
  }

  function tooltipStyle(): string {
    void viewRevision;
    if (!chart || !selectedPoint || !selectedVisible()) return 'display:none';
    const geo = geometry();
    if (!geo) return 'display:none';
    const hostRect = chartHost.getBoundingClientRect();
    const x = geo.overRect.left - hostRect.left + chart.valToPos(selectedPoint.time, 'x');
    const y = geo.overRect.top - hostRect.top + 12;
    const left = Math.min(chartHost.clientWidth - 230, Math.max(8, x + 12));
    return `left:${left}px;top:${y}px`;
  }

  function drawSelection(instance: uPlot) {
    if (!selectedPoint || !selectedVisible()) return;
    const { ctx, bbox } = instance;
    const x = instance.valToPos(selectedPoint.time, 'x', true);
    const voltageY = instance.valToPos(selectedPoint.voltage, 'voltage', true);
    const currentY = instance.valToPos(selectedPoint.current, 'current', true);
    ctx.save();
    ctx.setLineDash([5 * uPlot.pxRatio, 4 * uPlot.pxRatio]);
    ctx.strokeStyle = 'rgba(130, 144, 165, .9)';
    ctx.lineWidth = uPlot.pxRatio;
    ctx.beginPath();
    ctx.moveTo(x, bbox.top);
    ctx.lineTo(x, bbox.top + bbox.height);
    ctx.stroke();
    ctx.setLineDash([]);
    drawPoint(ctx, x, voltageY, '#ef565d', bbox);
    drawPoint(ctx, x, currentY, '#2887f0', bbox);
    ctx.restore();
  }

  function drawPoint(ctx: CanvasRenderingContext2D, x: number, y: number, color: string, bbox: uPlot.BBox) {
    if (y < bbox.top || y > bbox.top + bbox.height) return;
    ctx.fillStyle = color;
    ctx.strokeStyle = 'white';
    ctx.lineWidth = 2 * uPlot.pxRatio;
    ctx.beginPath();
    ctx.arc(x, y, 4.5 * uPlot.pxRatio, 0, Math.PI * 2);
    ctx.fill();
    ctx.stroke();
  }

  function applyScale(scale: 'x' | 'voltage' | 'current', range: NumericRange) {
    chart?.setScale(scale, range);
    viewRevision += 1;
  }

  function applyYScales() {
    if (!chart) return;
    const voltageAuto = autoRange([...currentSeries.voltageMin, ...currentSeries.voltageMax]);
    const currentAuto = autoRange([...currentSeries.currentMin, ...currentSeries.currentMax]);
    applyScale('voltage', voltageViewport ?? voltageAuto);
    applyScale('current', currentViewport ?? currentAuto);
  }

  function updateChart(data: RenderSeries) {
    currentSeries = data;
    latestEnd = Math.max(data.availableEnd, latestEnd);
    chart?.setData([data.time, data.voltageAverage, data.currentAverage] as uPlot.AlignedData, false);
    applyScale('x', timeViewport);
    applyYScales();
  }

  function scheduleRefresh() {
    refreshScheduler.schedule();
  }

  async function refreshPlot() {
    if (!chart) return;
    if (renderPending) {
      renderQueued = true;
      return;
    }
    renderPending = true;
    try {
      if (followLive && state.status === 'capturing') timeViewport = liveRange();
      const data = await api.renderData(timeViewport.min, timeViewport.max, Math.max(chart.width, 500));
      if (state.status === 'capturing' && data.time.length === 0 && currentSeries.time.length > 0) {
        return;
      }
      updateChart(data);
    } catch { /* no active recording yet */ }
    finally {
      renderPending = false;
      if (renderQueued) {
        renderQueued = false;
        scheduleRefresh();
      }
    }
  }

  function keydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && selectedPoint) clearSelection();
  }

  onMount(async () => {
    createChart();
    window.addEventListener('keydown', keydown);
    await refreshDevices();
    state = await api.captureState();
    if (state.deviceId) selectedDevice = state.deviceId;
    try {
      summary = await api.stats();
      latestEnd = summary.duration;
      if (state.status === 'capturing') timeViewport = liveRange();
      else if (summary.duration > 0) {
        followLive = false;
        timeViewport = { min: 0, max: summary.duration };
      }
    } catch { /* empty */ }
    unlisteners.push(await onEvent<CaptureState>('capture-state-changed', (value) => {
      const started = state.status !== 'capturing' && value.status === 'capturing';
      state = value;
      if (value.deviceId) selectedDevice = value.deviceId;
      if (started) returnToLive();
    }));
    unlisteners.push(await onEvent<CaptureSummary>('capture-summary-updated', (value) => {
      summary = value;
      latestEnd = value.duration;
      if (followLive && state.status === 'capturing') {
        timeViewport = liveRange();
        scheduleRefresh();
      }
    }));
    unlisteners.push(await onEvent('capture-data-ready', scheduleRefresh));
    unlisteners.push(await onEvent<import('../types').AppError>('device-disconnected', (error) => notify(errorMessage(error), 'error')));
    await refreshPlot();
  });

  onDestroy(() => {
    refreshScheduler.cancel();
    window.removeEventListener('keydown', keydown);
    unlisteners.forEach((fn) => fn());
    resizeObserver?.disconnect();
    chart?.destroy();
  });
</script>

<section class="page">
  <header>
    <div><h1 class="page-title">{$_('waveform.title')}</h1><p class="page-subtitle">{$_('waveform.subtitle')}</p></div>
    <div class="toolbar">
      <select class="control device" bind:value={selectedDevice} disabled={state.status === 'capturing'} aria-label={$_('waveform.selectDevice')}>
        <option value="">{$_('waveform.selectDevice')}</option>
        {#each devices as device}<option value={device.id}>{device.displayName}</option>{/each}
      </select>
      <button class="secondary-button icon" onclick={refreshDevices} disabled={state.status === 'capturing'} title={$_('common.refresh')}><RefreshCw size={17}/></button>
      <button class="primary-button" onclick={toggleCapture}>
        {#if state.status === 'capturing'}<Unplug size={17}/>{$_('common.disconnect')}{:else}<Usb size={17}/>{$_('common.connect')}{/if}
      </button>
    </div>
  </header>

  <div class="metrics">
    {#each settings.waveformMetrics as metricId (metricId)}
      {@const metric = metricById.get(metricId)}
      {#if metric}<MetricCard label={$_(metric.labelKey)} value={metric.format(summary)} tone={metric.tone}/>{/if}
    {/each}
  </div>

  <div class="chart-card panel">
    <div class="chart-heading">
      <span class="legend voltage"></span>{$_('waveform.voltage')}
      <span class="legend current"></span>{$_('waveform.current')}
      <span class="gesture-hint"><Crosshair size={14}/>{$_('waveform.gestureHint')}</span>
      <span class="samples">{summary.pointCount.toLocaleString()} {$_('waveform.points')}</span>
      {#if state.status === 'capturing' && !followLive}<button class="chart-action live" onclick={returnToLive}><Radio size={14}/>{$_('waveform.returnLive')}</button>{/if}
      <button class="chart-action" onclick={resetAxes} title={$_('waveform.resetAxes')}><RotateCcw size={14}/>{$_('waveform.resetAxes')}</button>
    </div>
    <div class="chart" bind:this={chartHost}>
      {#if currentSeries.time.length === 0}
        <div class="empty-state">{$_('waveform.empty')}</div>
      {/if}
      {#if selectedPoint && selectedVisible()}
        <div class="reading-card" style={tooltipStyle()}>
          <button class="close-reading" onclick={clearSelection} aria-label={$_('waveform.closeReading')}><X size={14}/></button>
          <strong>{$_('waveform.pointReading')}</strong>
          <dl>
            <div><dt>{$_('waveform.time')}</dt><dd>{selectedPoint.time.toFixed(6)} s</dd></div>
            <div><dt>{$_('waveform.voltage')}</dt><dd class="voltage-text">{selectedPoint.voltage.toFixed(4)} V</dd></div>
            <div><dt>{$_('waveform.current')}</dt><dd class="current-text">{formatCurrent(selectedPoint.current)}</dd></div>
            <div><dt>{$_('waveform.instantPower')}</dt><dd class="power-text">{formatPower(selectedPoint.powerMw)}</dd></div>
          </dl>
        </div>
      {/if}
    </div>
  </div>

  <footer>
    <button class="secondary-button" onclick={importRecording}><FolderInput size={17}/>{$_('common.import')}</button>
    <button class="secondary-button" onclick={exportRecording} disabled={!state.recordPath}><FolderOutput size={17}/>{$_('common.export')}</button>
    <button class="danger-button" onclick={clearRecords} disabled={state.status === 'capturing'}><Trash2 size={17}/>{$_('common.clear')}</button>
    {#if state.error}<span class="error">{state.error}</span>{/if}
  </footer>
</section>

<style>
  .page { width: 100%; min-width: 0; height: 100%; display: grid; grid-template-rows: auto auto minmax(300px, 1fr) auto; gap: 16px; }
  header, footer, .toolbar { display: flex; align-items: center; }
  header, footer, .toolbar, .metrics, .chart-card { min-width: 0; }
  header { justify-content: space-between; gap: 20px; }
  .toolbar { gap: 9px; }
  .device { width: min(310px, 30vw); }
  .icon { width: 38px; padding: 0; }
  .metrics { display: grid; grid-template-columns: repeat(6, minmax(110px, 1fr)); gap: 10px; }
  .chart-card { min-height: 0; border-radius: 18px; padding: 12px 16px 10px; display: flex; flex-direction: column; }
  .chart-heading { min-height: 32px; display: flex; align-items: center; gap: 7px; color: var(--muted); font-size: 12px; }
  .legend { width: 18px; height: 2px; margin-left: 10px; border-radius: 2px; }
  .legend.voltage { background: var(--voltage); margin-left: 0; }
  .legend.current { background: var(--current); }
  .gesture-hint { margin-left: 12px; display: inline-flex; align-items: center; gap: 5px; }
  .samples { margin-left: auto; }
  .chart-action { height: 28px; display: inline-flex; align-items: center; gap: 5px; padding: 0 9px; color: var(--muted); background: var(--panel-muted); border: 1px solid var(--border); border-radius: 8px; }
  .chart-action:hover { color: var(--text); }
  .chart-action.live { color: var(--accent); background: var(--accent-soft); }
  .chart { min-width: 0; min-height: 0; flex: 1; user-select: none; position: relative; overflow: hidden; }
  .chart :global(.uplot) { max-width: 100%; }
  .empty-state { position: absolute; z-index: 2; inset: 0; display: grid; place-items: center; color: var(--muted); font-size: 13px; pointer-events: none; }
  .reading-card { position: absolute; z-index: 20; width: 220px; padding: 12px 13px; color: var(--text); background: color-mix(in srgb, var(--panel-solid) 94%, transparent); border: 1px solid var(--border); border-radius: 12px; box-shadow: 0 12px 34px rgba(20, 33, 55, .18); backdrop-filter: blur(14px); pointer-events: auto; }
  .reading-card > strong { display: block; margin-bottom: 8px; font-size: 12px; }
  .close-reading { position: absolute; top: 7px; right: 7px; width: 24px; height: 24px; display: grid; place-items: center; padding: 0; color: var(--muted); border: 0; border-radius: 6px; background: transparent; }
  .close-reading:hover { color: var(--text); background: var(--panel-muted); }
  dl { margin: 0; display: grid; gap: 5px; }
  dl div { display: flex; justify-content: space-between; gap: 12px; font-size: 11px; }
  dt { color: var(--muted); }
  dd { margin: 0; font-variant-numeric: tabular-nums; }
  .voltage-text { color: var(--voltage); }
  .current-text { color: var(--current); }
  .power-text { color: var(--power); }
  footer { gap: 9px; }
  .error { margin-left: auto; color: #d6444b; font-size: 13px; }
  @media (max-width: 1150px) { .metrics { grid-template-columns: repeat(3, minmax(110px, 1fr)); } .gesture-hint { display: none; } }
  @media (max-width: 1050px) { header { align-items: stretch; flex-direction: column; gap: 12px; } .toolbar { width: 100%; } .device { width: auto; flex: 1; } .toolbar button { flex: 0 0 auto; white-space: nowrap; } }
  @media (max-width: 760px) { .metrics { grid-template-columns: repeat(2, minmax(110px, 1fr)); } .chart-action { font-size: 0; } }
</style>
