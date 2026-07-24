<script lang="ts">
  import { _ } from 'svelte-i18n';
  import {
    ArrowDown, ArrowUp, ChartNoAxesCombined, GripVertical, Languages,
    MonitorCog, Palette, Plus, Scale, X,
  } from '@lucide/svelte';
  import { metricById, metricCatalog } from '../metrics';
  import { addMetric as addMetricToLayout, moveMetric as moveMetricInLayout, removeMetric as removeMetricFromLayout, reorderMetric } from '../metricLayout';
  import type { AppSettings, Language, MetricId, ThemeMode } from '../types';

  export let settings: AppSettings;
  export let update: (settings: AppSettings) => Promise<void>;

  let draggedMetric: MetricId | null = null;

  function themeChanged(event: Event) {
    void update({ ...settings, theme: (event.target as HTMLSelectElement).value as ThemeMode });
  }

  function languageChanged(event: Event) {
    void update({ ...settings, language: (event.target as HTMLSelectElement).value as Language });
  }

  function saveMetrics(waveformMetrics: MetricId[]) {
    if (waveformMetrics.length > 0) void update({ ...settings, waveformMetrics });
  }

  function addMetric(metric: MetricId) {
    saveMetrics(addMetricToLayout(settings.waveformMetrics, metric));
  }

  function removeMetric(metric: MetricId) {
    saveMetrics(removeMetricFromLayout(settings.waveformMetrics, metric));
  }

  function moveMetric(metric: MetricId, delta: -1 | 1) {
    saveMetrics(moveMetricInLayout(settings.waveformMetrics, metric, delta));
  }

  function dropMetric(target: MetricId) {
    if (!draggedMetric || draggedMetric === target) return;
    const next = reorderMetric(settings.waveformMetrics, draggedMetric, target);
    draggedMetric = null;
    saveMetrics(next);
  }
</script>

<section class="page">
  <header><h1 class="page-title">{$_('settings.title')}</h1><p class="page-subtitle">{$_('settings.subtitle')}</p></header>

  <div class="group panel">
    <h2><Palette size={19}/>{$_('settings.appearance')}</h2>
    <div class="setting"><div class="label"><MonitorCog size={20}/><div><strong>{$_('settings.theme')}</strong><span>{$_(`settings.${settings.theme}`)}</span></div></div><select class="control" value={settings.theme} onchange={themeChanged}><option value="system">{$_('settings.system')}</option><option value="light">{$_('settings.light')}</option><option value="dark">{$_('settings.dark')}</option></select></div>
    <div class="setting"><div class="label"><Languages size={20}/><div><strong>{$_('settings.language')}</strong><span>{settings.language === 'zh-CN' ? '简体中文' : 'English'}</span></div></div><select class="control" value={settings.language} onchange={languageChanged}><option value="zh-CN">简体中文</option><option value="en">English</option></select></div>
  </div>

  <div class="group panel metrics-setting">
    <h2><ChartNoAxesCombined size={19}/>{$_('settings.waveformMetrics')}</h2>
    <p class="hint">{$_('settings.waveformMetricsHint')}</p>

    <div class="metric-preview" role="list" aria-label={$_('settings.visibleMetrics')}>
      {#each settings.waveformMetrics as metricId, index (metricId)}
        {@const metric = metricById.get(metricId)}
        {#if metric}
          <div
            class="metric-item"
            role="listitem"
            draggable="true"
            ondragstart={() => draggedMetric = metricId}
            ondragend={() => draggedMetric = null}
            ondragover={(event) => event.preventDefault()}
            ondrop={() => dropMetric(metricId)}
          >
            <GripVertical size={17} class="grip"/>
            <span>{$_(metric.labelKey)}</span>
            <div class="metric-actions">
              <button onclick={() => moveMetric(metricId, -1)} disabled={index === 0} title={$_('settings.moveUp')} aria-label={$_('settings.moveUp')}><ArrowUp size={15}/></button>
              <button onclick={() => moveMetric(metricId, 1)} disabled={index === settings.waveformMetrics.length - 1} title={$_('settings.moveDown')} aria-label={$_('settings.moveDown')}><ArrowDown size={15}/></button>
              <button class="remove" onclick={() => removeMetric(metricId)} disabled={settings.waveformMetrics.length === 1} title={$_('settings.removeMetric')} aria-label={$_('settings.removeMetric')}><X size={15}/></button>
            </div>
          </div>
        {/if}
      {/each}
    </div>

    <h3>{$_('settings.availableMetrics')}</h3>
    <div class="available-list">
      {#each metricCatalog.filter((metric) => !settings.waveformMetrics.includes(metric.id)) as metric (metric.id)}
        <button class="add-metric" onclick={() => addMetric(metric.id)}><Plus size={15}/>{$_(metric.labelKey)}</button>
      {:else}
        <span class="all-visible">{$_('settings.allMetricsVisible')}</span>
      {/each}
    </div>
  </div>

  <div class="group panel about"><h2><Scale size={19}/>{$_('settings.about')}</h2><p>{$_('settings.aboutText')}</p><code>PowerPico Client 0.1.0 · MIT</code></div>
</section>

<style>
  .page { max-width: 940px; margin: 0 auto; padding-bottom: 24px; }
  header { margin-bottom: 22px; }
  .group { border-radius: 18px; padding: 20px 22px; margin-bottom: 16px; }
  h2 { display: flex; gap: 9px; align-items: center; margin: 0 0 10px; font-size: 15px; }
  h3 { margin: 20px 0 10px; font-size: 13px; }
  .setting { display: flex; align-items: center; justify-content: space-between; padding: 16px 0; border-top: 1px solid var(--border); }
  .label { display: flex; align-items: center; gap: 13px; }
  .label strong, .label span { display: block; }
  .label strong { font-size: 14px; }
  .label span, .about p, .hint { color: var(--muted); font-size: 12px; margin-top: 4px; }
  select { width: 180px; }
  .hint { margin: -2px 0 15px; }
  .metric-preview { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 9px; }
  .metric-item { min-width: 0; min-height: 48px; display: flex; align-items: center; gap: 8px; padding: 8px 8px 8px 10px; border: 1px solid var(--border); border-radius: 12px; background: var(--panel-muted); }
  .metric-item > span { min-width: 0; flex: 1; font-size: 13px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .metric-item :global(.grip) { color: var(--muted); cursor: grab; flex: 0 0 auto; }
  .metric-actions { display: flex; gap: 2px; }
  .metric-actions button { width: 27px; height: 27px; display: grid; place-items: center; padding: 0; border: 0; border-radius: 7px; color: var(--muted); background: transparent; }
  .metric-actions button:hover:not(:disabled) { color: var(--text); background: var(--panel-solid); }
  .metric-actions .remove:hover:not(:disabled) { color: #d6444b; }
  .available-list { display: flex; flex-wrap: wrap; gap: 8px; }
  .add-metric { min-height: 34px; display: inline-flex; align-items: center; gap: 6px; padding: 0 11px; color: var(--accent); border: 1px solid color-mix(in srgb, var(--accent) 25%, var(--border)); border-radius: 9px; background: var(--accent-soft); }
  .all-visible { color: var(--muted); font-size: 12px; }
  .about p { font-size: 13px; }
  code { display: inline-block; margin-top: 8px; color: var(--accent); background: var(--accent-soft); border-radius: 8px; padding: 7px 10px; }
  @media (max-width: 850px) { .metric-preview { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
  @media (max-width: 620px) { .metric-preview { grid-template-columns: 1fr; } .setting { gap: 12px; } }
</style>
