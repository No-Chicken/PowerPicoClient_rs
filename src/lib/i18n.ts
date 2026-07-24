import { addMessages, init, locale } from 'svelte-i18n';
import type { Language } from './types';

export const zh = {
  app: { name: 'PowerPico Client', connected: '设备已连接', disconnected: '未连接' },
  nav: { waveform: '波形', firmware: '固件', settings: '设置' },
  common: {
    refresh: '刷新', connect: '连接', disconnect: '断开', clear: '清除', import: '导入',
    export: '导出', cancel: '取消', browse: '选择文件', start: '开始', none: '无', error: '错误',
    close: '关闭', check: '检查', download: '下载', open: '打开', save: '保存',
  },
  errors: {
    serialPermission: '没有串口访问权限。Debian/Ubuntu 请将当前用户加入 dialout 组，Arch Linux 请加入 uucp 组，然后注销并重新登录。',
  },
  waveform: {
    title: '实时波形', subtitle: '采集、查看并导出 PowerPico 测量数据', selectDevice: '选择串口设备',
    voltage: '电压', current: '电流', avgVoltage: '平均电压', peakVoltage: '峰值电压',
    avgCurrent: '平均电流', peakCurrent: '峰值电流', avgPower: '平均功率', duration: '采集时长',
    points: '采样点', empty: '连接设备后，实时波形将显示在这里', imported: '已加载记录',
    exportSuccess: '数据已导出', confirmClear: '确定清除当前记录吗？', noDevice: '请先选择设备',
    resetAxes: '重置坐标轴', returnLive: '返回实时', gestureHint: '滚轮缩放 · 左键拖动平移 · 右键拖动分析 · 点击读数',
    pointReading: '精确采样点', closeReading: '关闭读数', time: '时间', instantPower: '瞬时功率',
    statsMode: '统计范围', globalStats: '全局统计', tenMinuteStats: '最近 10 分钟', minuteStats: '最近 1 分钟', secondStats: '最近 1 秒',
    analyze: '区间分析', analysisTitle: '区间统计', analysisHint: '在图表上按右键拖动可选择分析区间',
    startTime: '开始时间', endTime: '结束时间', powerPeak: '峰值功率', energy: '电量',
    modeLive: '实时跟随', modeFrozen: '历史浏览', modeAnalysis: '区间分析', viewport: '时间位置', rangeStart: '范围起点', rangeEnd: '范围终点',
  },
  metrics: {
    latestVoltage: '实时电压', averageVoltage: '平均电压', peakVoltage: '峰值电压',
    latestCurrent: '实时电流', averageCurrent: '平均电流', peakCurrent: '峰值电流',
    latestPower: '实时功率', averagePower: '平均功率', duration: '采集时长',
    pointCount: '采样点', energy: '能量',
  },
  firmware: {
    title: '固件升级', subtitle: '下载官方固件或使用本地 BIN 文件升级 PowerPico', device: '目标设备',
    file: '固件文件', choose: '选择 .bin 固件', flash: '开始刷写', idle: '等待选择设备和固件',
    warning: '升级过程中请勿拔出设备或关闭应用。', success: '固件升级完成', noFile: '请先选择固件文件',
    official: '官方固件', custom: '自定义固件', source: '固件来源', latest: '最新版本', releaseDate: '发布日期',
    downloadLatest: '下载最新固件', alreadyLatest: '官方固件已是最新版本', releaseNotes: '发行说明',
    downloading: '正在下载固件…', downloadCompleted: '固件下载完成', downloadCancelled: '固件下载已取消', downloadFailed: '固件下载失败',
    connecting: '正在连接 PowerPico…', rebooting: '正在进入引导加载程序…', searchingBootloader: '正在搜索引导加载程序…',
    handshaking: '正在等待引导加载程序握手…', uploading: '正在上传固件…', finishing: '正在启动新固件…', completed: '固件升级成功', cancelled: '固件升级已取消', failed: '固件升级失败',
  },
  settings: {
    title: '设置', subtitle: '调整界面外观和语言', appearance: '外观', theme: '主题',
    system: '跟随系统', light: '浅色', dark: '深色', language: '界面语言',
    about: '关于开源版本', aboutText: '基于 Tauri 2、Rust 和 Svelte 构建，采用 GNU GPL v3 许可证。',
    personalization: '个性化', uiScale: '界面缩放', auto: '自动', antiAliasing: '波形抗锯齿',
    update: '软件更新', checkAtStartup: '启动时检查更新', checkNow: '立即检查', upToDate: '当前已是最新版本', updateAvailable: '发现新版本',
    help: '帮助', feedback: '反馈', openHelp: '打开使用帮助', provideFeedback: '提供反馈',
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
    close: 'Close', check: 'Check', download: 'Download', open: 'Open', save: 'Save',
  },
  errors: {
    serialPermission: 'Serial access was denied. Add your user to dialout on Debian/Ubuntu or uucp on Arch Linux, then sign out and back in.',
  },
  waveform: {
    title: 'Live waveform', subtitle: 'Capture, inspect and export PowerPico measurements', selectDevice: 'Select serial device',
    voltage: 'Voltage', current: 'Current', avgVoltage: 'Average voltage', peakVoltage: 'Peak voltage',
    avgCurrent: 'Average current', peakCurrent: 'Peak current', avgPower: 'Average power', duration: 'Duration',
    points: 'Samples', empty: 'Connect a device to display its live waveform', imported: 'Recording loaded',
    exportSuccess: 'Data exported', confirmClear: 'Clear the current recording?', noDevice: 'Select a device first',
    resetAxes: 'Reset axes', returnLive: 'Return to live', gestureHint: 'Wheel to zoom · left-drag to pan · right-drag to analyze · click to inspect',
    pointReading: 'Exact sample', closeReading: 'Close reading', time: 'Time', instantPower: 'Instant power',
    statsMode: 'Statistics range', globalStats: 'Global statistics', tenMinuteStats: 'Last 10 minutes', minuteStats: 'Last minute', secondStats: 'Last second',
    analyze: 'Range analysis', analysisTitle: 'Range statistics', analysisHint: 'Right-drag on the chart to select an analysis range',
    startTime: 'Start time', endTime: 'End time', powerPeak: 'Peak power', energy: 'Charge',
    modeLive: 'Live follow', modeFrozen: 'History', modeAnalysis: 'Range analysis', viewport: 'Time position', rangeStart: 'Range start', rangeEnd: 'Range end',
  },
  metrics: {
    latestVoltage: 'Live voltage', averageVoltage: 'Average voltage', peakVoltage: 'Peak voltage',
    latestCurrent: 'Live current', averageCurrent: 'Average current', peakCurrent: 'Peak current',
    latestPower: 'Live power', averagePower: 'Average power', duration: 'Duration',
    pointCount: 'Sample count', energy: 'Energy',
  },
  firmware: {
    title: 'Firmware update', subtitle: 'Download official firmware or use a local BIN image', device: 'Target device',
    file: 'Firmware file', choose: 'Choose .bin firmware', flash: 'Flash firmware', idle: 'Choose a device and firmware',
    warning: 'Do not unplug the device or close the application while flashing.', success: 'Firmware update completed', noFile: 'Choose a firmware file first',
    official: 'Official firmware', custom: 'Custom firmware', source: 'Firmware source', latest: 'Latest version', releaseDate: 'Release date',
    downloadLatest: 'Download latest firmware', alreadyLatest: 'Official firmware is already current', releaseNotes: 'Release notes',
    downloading: 'Downloading firmware…', downloadCompleted: 'Firmware download completed', downloadCancelled: 'Firmware download cancelled', downloadFailed: 'Firmware download failed',
    connecting: 'Connecting to PowerPico…', rebooting: 'Rebooting into bootloader…', searchingBootloader: 'Searching for bootloader…',
    handshaking: 'Waiting for bootloader handshake…', uploading: 'Uploading firmware…', finishing: 'Starting updated firmware…', completed: 'Firmware updated successfully', cancelled: 'Firmware update cancelled', failed: 'Firmware update failed',
  },
  settings: {
    title: 'Settings', subtitle: 'Adjust appearance and language', appearance: 'Appearance', theme: 'Theme',
    system: 'Use system setting', light: 'Light', dark: 'Dark', language: 'Language',
    about: 'About the open-source client', aboutText: 'Built with Tauri 2, Rust and Svelte, licensed under GNU GPL v3.',
    personalization: 'Personalization', uiScale: 'Interface scale', auto: 'Auto', antiAliasing: 'Waveform anti-aliasing',
    update: 'Software update', checkAtStartup: 'Check for updates at startup', checkNow: 'Check now', upToDate: 'You are up to date', updateAvailable: 'A new version is available',
    help: 'Help', feedback: 'Feedback', openHelp: 'Open user guide', provideFeedback: 'Provide feedback',
    waveformMetrics: 'Waveform metrics', waveformMetricsHint: 'Drag to reorder, or use the move buttons. Removed metrics can be added again at any time.',
    visibleMetrics: 'Visible waveform metrics', availableMetrics: 'Available metrics', allMetricsVisible: 'All metrics are visible',
    moveUp: 'Move up', moveDown: 'Move down', removeMetric: 'Remove metric',
  },
};

export const zhHk = {
  ...zh,
  app: { ...zh.app, connected: '裝置已連線', disconnected: '未連線' },
  nav: { waveform: '波形', firmware: '韌體', settings: '設定' },
  common: { ...zh.common, import: '匯入', export: '匯出', clear: '清除' },
  settings: { ...zh.settings, title: '設定', language: '介面語言', about: '關於開源版本' },
  firmware: { ...zh.firmware, title: '韌體升級', official: '官方韌體', custom: '自訂韌體' },
};

export const ja = {
  ...en,
  app: { name: 'PowerPico Client', connected: 'デバイス接続済み', disconnected: '未接続' },
  nav: { waveform: '波形', firmware: 'ファームウェア', settings: '設定' },
  common: { ...en.common, refresh: '更新', connect: '接続', disconnect: '切断', clear: 'クリア', cancel: 'キャンセル' },
  settings: { ...en.settings, title: '設定', language: '表示言語', appearance: '外観', update: 'ソフトウェア更新', help: 'ヘルプ' },
  firmware: { ...en.firmware, title: 'ファームウェア更新', official: '公式ファームウェア', custom: 'カスタムファームウェア' },
  waveform: { ...en.waveform, title: 'リアルタイム波形', voltage: '電圧', current: '電流', analyze: '範囲解析' },
};

addMessages('zh-CN', zh);
addMessages('zh-HK', zhHk);
addMessages('en', en);
addMessages('ja', ja);
init({ fallbackLocale: 'en', initialLocale: 'en' });

export function resolveLocale(value: Language, systemLanguage = navigator.language): Exclude<Language, 'auto'> {
  if (value !== 'auto') return value;
  const normalized = systemLanguage.toLowerCase();
  if (normalized.startsWith('zh-hk') || normalized.startsWith('zh-tw')) return 'zh-HK';
  if (normalized.startsWith('zh')) return 'zh-CN';
  if (normalized.startsWith('ja')) return 'ja';
  return 'en';
}

export function setLocale(value: Language) {
  const resolved = resolveLocale(value);
  locale.set(resolved);
  document.documentElement.lang = resolved;
}
