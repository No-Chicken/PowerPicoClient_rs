import { addMessages, init, locale } from 'svelte-i18n';

export const zh = {
  app: { name: 'PowerPico Client', connected: '设备已连接', disconnected: '未连接' },
  nav: { waveform: '波形', firmware: '固件', settings: '设置' },
  common: {
    refresh: '刷新', connect: '连接', disconnect: '断开', clear: '清除', import: '导入',
    export: '导出', cancel: '取消', browse: '选择文件', start: '开始', none: '无', error: '错误',
  },
  waveform: {
    title: '实时波形', subtitle: '采集、查看并导出 PowerPico 测量数据', selectDevice: '选择串口设备',
    voltage: '电压', current: '电流', avgVoltage: '平均电压', peakVoltage: '峰值电压',
    avgCurrent: '平均电流', peakCurrent: '峰值电流', avgPower: '平均功率', duration: '采集时长',
    points: '采样点', empty: '连接设备后，实时波形将显示在这里', imported: '已加载记录',
    exportSuccess: '数据已导出', confirmClear: '确定清除当前记录吗？', noDevice: '请先选择设备',
    resetAxes: '重置坐标轴', returnLive: '返回实时', gestureHint: '滚轮缩放 · 拖动平移 · 点击读数',
    pointReading: '精确采样点', closeReading: '关闭读数', time: '时间', instantPower: '瞬时功率',
  },
  metrics: {
    latestVoltage: '实时电压', averageVoltage: '平均电压', peakVoltage: '峰值电压',
    latestCurrent: '实时电流', averageCurrent: '平均电流', peakCurrent: '峰值电流',
    latestPower: '实时功率', averagePower: '平均功率', duration: '采集时长',
    pointCount: '采样点', energy: '能量',
  },
  firmware: {
    title: '固件升级', subtitle: '通过本地 BIN 文件更新 PowerPico 固件', device: '目标设备',
    file: '固件文件', choose: '选择 .bin 固件', flash: '开始刷写', idle: '等待选择设备和固件',
    warning: '升级过程中请勿拔出设备或关闭应用。', success: '固件升级完成', noFile: '请先选择固件文件',
  },
  settings: {
    title: '设置', subtitle: '调整界面外观和语言', appearance: '外观', theme: '主题',
    system: '跟随系统', light: '浅色', dark: '深色', language: '界面语言',
    about: '关于开源版本', aboutText: '基于 Tauri 2、Rust 和 Svelte 构建，采用 MIT 许可证。',
    waveformMetrics: '波形指标', waveformMetricsHint: '拖拽调整展示顺序，也可使用按钮移动；移除的指标可随时重新添加。',
    visibleMetrics: '当前显示的波形指标', availableMetrics: '可添加指标', allMetricsVisible: '所有指标均已显示',
    moveUp: '向上移动', moveDown: '向下移动', removeMetric: '移除指标',
  },
};

export const en = {
  app: { name: 'PowerPico Client', connected: 'Device connected', disconnected: 'Disconnected' },
  nav: { waveform: 'Waveform', firmware: 'Firmware', settings: 'Settings' },
  common: {
    refresh: 'Refresh', connect: 'Connect', disconnect: 'Disconnect', clear: 'Clear', import: 'Import',
    export: 'Export', cancel: 'Cancel', browse: 'Browse', start: 'Start', none: 'None', error: 'Error',
  },
  waveform: {
    title: 'Live waveform', subtitle: 'Capture, inspect and export PowerPico measurements', selectDevice: 'Select serial device',
    voltage: 'Voltage', current: 'Current', avgVoltage: 'Average voltage', peakVoltage: 'Peak voltage',
    avgCurrent: 'Average current', peakCurrent: 'Peak current', avgPower: 'Average power', duration: 'Duration',
    points: 'Samples', empty: 'Connect a device to display its live waveform', imported: 'Recording loaded',
    exportSuccess: 'Data exported', confirmClear: 'Clear the current recording?', noDevice: 'Select a device first',
    resetAxes: 'Reset axes', returnLive: 'Return to live', gestureHint: 'Wheel to zoom · drag to pan · click to inspect',
    pointReading: 'Exact sample', closeReading: 'Close reading', time: 'Time', instantPower: 'Instant power',
  },
  metrics: {
    latestVoltage: 'Live voltage', averageVoltage: 'Average voltage', peakVoltage: 'Peak voltage',
    latestCurrent: 'Live current', averageCurrent: 'Average current', peakCurrent: 'Peak current',
    latestPower: 'Live power', averagePower: 'Average power', duration: 'Duration',
    pointCount: 'Sample count', energy: 'Energy',
  },
  firmware: {
    title: 'Firmware update', subtitle: 'Update PowerPico from a local BIN image', device: 'Target device',
    file: 'Firmware file', choose: 'Choose .bin firmware', flash: 'Flash firmware', idle: 'Choose a device and firmware',
    warning: 'Do not unplug the device or close the application while flashing.', success: 'Firmware update completed', noFile: 'Choose a firmware file first',
  },
  settings: {
    title: 'Settings', subtitle: 'Adjust appearance and language', appearance: 'Appearance', theme: 'Theme',
    system: 'Use system setting', light: 'Light', dark: 'Dark', language: 'Language',
    about: 'About the open-source client', aboutText: 'Built with Tauri 2, Rust and Svelte, licensed under MIT.',
    waveformMetrics: 'Waveform metrics', waveformMetricsHint: 'Drag to reorder, or use the move buttons. Removed metrics can be added again at any time.',
    visibleMetrics: 'Visible waveform metrics', availableMetrics: 'Available metrics', allMetricsVisible: 'All metrics are visible',
    moveUp: 'Move up', moveDown: 'Move down', removeMetric: 'Remove metric',
  },
};

addMessages('zh-CN', zh);
addMessages('en', en);
init({ fallbackLocale: 'en', initialLocale: 'zh-CN' });

export function setLocale(value: 'zh-CN' | 'en') {
  locale.set(value);
  document.documentElement.lang = value;
}
