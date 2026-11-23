//! Interface copy in the four locales the client ships.
//!
//! Keys mirror the Svelte client's `i18n.ts` one for one, so the wording and
//! the layout of the copy stay identical after the port. Traditional Chinese
//! and Japanese are the base locale with the same overrides the web client had.

use std::{collections::HashMap, sync::OnceLock};

use gpui::SharedString;
use powerpico_core::Language;

/// A resolved interface locale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    ZhCn,
    ZhHk,
    En,
    Ja,
}

impl Lang {
    /// Resolves the stored preference, falling back to the system locale.
    pub fn resolve(language: Language) -> Self {
        match language {
            Language::ZhCn => Self::ZhCn,
            Language::ZhHk => Self::ZhHk,
            Language::En => Self::En,
            Language::Ja => Self::Ja,
            Language::Auto => Self::system(),
        }
    }

    fn system() -> Self {
        let mut tag = std::env::var("LC_ALL")
            .or_else(|_| std::env::var("LC_MESSAGES"))
            .or_else(|_| std::env::var("LANG"))
            .unwrap_or_default()
            .to_lowercase();
        if let Some((_, rest)) = tag.split_once('.') {
            tag.truncate(tag.len() - rest.len() - 1);
        }
        if tag.contains("zh_hk")
            || tag.contains("zh_tw")
            || tag.contains("zh-hk")
            || tag.contains("zh-tw")
        {
            Self::ZhHk
        } else if tag.starts_with("zh") {
            Self::ZhCn
        } else if tag.starts_with("ja") {
            Self::Ja
        } else {
            Self::En
        }
    }

    /// The BCP 47 tag, for accessibility and logs.
    pub fn tag(self) -> &'static str {
        match self {
            Self::ZhCn => "zh-CN",
            Self::ZhHk => "zh-HK",
            Self::En => "en",
            Self::Ja => "ja",
        }
    }
}

type Table = HashMap<&'static str, &'static str>;

fn build(
    base: &'static [(&'static str, &'static str)],
    overrides: &[(&'static str, &'static str)],
) -> Table {
    let mut table: Table = base.iter().copied().collect();
    for (key, value) in overrides {
        table.insert(key, value);
    }
    table
}

fn table(lang: Lang) -> &'static Table {
    match lang {
        Lang::ZhCn => {
            static TABLE: OnceLock<Table> = OnceLock::new();
            TABLE.get_or_init(|| base_zh().iter().copied().collect())
        }
        Lang::En => {
            static TABLE: OnceLock<Table> = OnceLock::new();
            TABLE.get_or_init(|| base_en().iter().copied().collect())
        }
        Lang::ZhHk => {
            static TABLE: OnceLock<Table> = OnceLock::new();
            TABLE.get_or_init(|| build(base_zh(), OVERRIDES_HK))
        }
        Lang::Ja => {
            static TABLE: OnceLock<Table> = OnceLock::new();
            TABLE.get_or_init(|| build(base_en(), OVERRIDES_JA))
        }
    }
}

/// Looks up `key`; unknown keys fall back to English, then to the key itself.
pub fn t(lang: Lang, key: &str) -> SharedString {
    if let Some(value) = table(lang).get(key) {
        return (*value).into();
    }
    if let Some(value) = table(Lang::En).get(key) {
        return (*value).into();
    }
    tracing::warn!("missing translation key: {key}");
    key.to_string().into()
}

fn base_zh() -> &'static [(&'static str, &'static str)] {
    ZH
}

fn base_en() -> &'static [(&'static str, &'static str)] {
    EN
}

const OVERRIDES_HK: &[(&str, &str)] = &[
    ("app.connected", "裝置已連線"),
    ("app.disconnected", "未連線"),
    ("nav.calibration", "校準"),
    ("nav.waveform", "波形"),
    ("nav.firmware", "韌體"),
    ("nav.settings", "設定"),
    ("common.import", "匯入"),
    ("common.export", "匯出"),
    ("common.clear", "清除"),
    (
        "errors.externalLink",
        "無法開啟外部連結，請檢查系統預設瀏覽器設定後重試。",
    ),
    ("settings.title", "設定"),
    ("settings.language", "介面語言"),
    ("settings.about", "關於開源版本"),
    ("settings.storage", "儲存空間與解除安裝"),
    ("settings.cache", "快取"),
    ("settings.appData", "應用程式資料"),
    ("settings.logs", "記錄"),
    ("settings.systemData", "系統資料"),
    ("settings.clearCache", "清除快取"),
    ("settings.cleaning", "正在清除…"),
    ("settings.cacheCleared", "快取已清除"),
    ("settings.uninstall", "解除安裝 PowerPico Client"),
    ("settings.uninstallHint", "移除應用程式及全部資料"),
    ("settings.uninstalling", "正在解除安裝…"),
    ("calibration.title", "裝置校準"),
    ("calibration.read", "讀取參數"),
    ("calibration.auto", "自動零點校準"),
    ("calibration.reset", "恢復出廠校準"),
    ("firmware.title", "韌體升級"),
    ("firmware.official", "官方韌體"),
    ("firmware.custom", "自訂韌體"),
];

const OVERRIDES_JA: &[(&str, &str)] = &[
    ("app.connected", "デバイス接続済み"),
    ("app.disconnected", "未接続"),
    ("nav.calibration", "校正"),
    ("nav.waveform", "波形"),
    ("nav.firmware", "ファームウェア"),
    ("nav.settings", "設定"),
    ("common.refresh", "更新"),
    ("common.connect", "接続"),
    ("common.disconnect", "切断"),
    ("common.clear", "クリア"),
    ("common.cancel", "キャンセル"),
    (
        "errors.externalLink",
        "外部リンクを開けませんでした。既定のブラウザ設定を確認して再試行してください。",
    ),
    ("settings.title", "設定"),
    ("settings.language", "表示言語"),
    ("settings.appearance", "外観"),
    ("settings.update", "ソフトウェア更新"),
    ("settings.help", "ヘルプ"),
    ("settings.storage", "ストレージとアンインストール"),
    ("settings.cache", "キャッシュ"),
    ("settings.appData", "アプリデータ"),
    ("settings.logs", "ログ"),
    ("settings.systemData", "システムデータ"),
    ("settings.clearCache", "キャッシュを消去"),
    ("settings.cleaning", "消去中…"),
    ("settings.cacheCleared", "キャッシュを消去しました"),
    ("settings.uninstall", "PowerPico Client をアンインストール"),
    ("settings.uninstallHint", "アプリとすべてのデータを削除"),
    ("settings.uninstalling", "アンインストール中…"),
    ("calibration.title", "デバイス校正"),
    ("calibration.read", "パラメーターを読む"),
    ("calibration.auto", "自動ゼロ校正"),
    ("calibration.reset", "工場出荷時に戻す"),
    ("firmware.title", "ファームウェア更新"),
    ("firmware.official", "公式ファームウェア"),
    ("firmware.custom", "カスタムファームウェア"),
    ("waveform.title", "リアルタイム波形"),
    ("waveform.voltage", "電圧"),
    ("waveform.current", "電流"),
    ("waveform.analyze", "範囲解析"),
];

const ZH: &[(&str, &str)] = &[
    ("app.name", "PowerPico Client"),
    ("app.connected", "设备已连接"),
    ("app.disconnected", "未连接"),
    ("nav.calibration", "校准"),
    ("nav.waveform", "波形"),
    ("nav.firmware", "固件"),
    ("nav.settings", "设置"),
    ("common.refresh", "刷新"),
    ("common.connect", "连接"),
    ("common.disconnect", "断开"),
    ("common.clear", "清除"),
    ("common.import", "导入"),
    ("common.export", "导出"),
    ("common.cancel", "取消"),
    ("common.browse", "选择文件"),
    ("common.start", "开始"),
    ("common.none", "无"),
    ("common.error", "错误"),
    ("common.close", "关闭"),
    ("common.check", "检查"),
    ("common.download", "下载"),
    ("common.open", "打开"),
    ("common.save", "保存"),
    ("errors.serialPermission", "没有串口访问权限。Debian/Ubuntu 请将当前用户加入 dialout 组，Arch Linux 请加入 uucp 组，然后注销并重新登录。"),
    ("errors.externalLink", "无法打开外部链接，请检查系统默认浏览器设置后重试。"),
    ("waveform.title", "实时波形"),
    ("waveform.subtitle", "采集、查看并导出 PowerPico 测量数据"),
    ("waveform.selectDevice", "选择串口设备"),
    ("waveform.voltage", "电压"),
    ("waveform.current", "电流"),
    ("waveform.points", "采样点"),
    ("waveform.empty", "连接设备后，实时波形将显示在这里"),
    ("waveform.imported", "已加载记录"),
    ("waveform.exportSuccess", "数据已导出"),
    ("waveform.confirmClear", "确定清除当前记录吗？"),
    ("waveform.noDevice", "请先选择设备"),
    ("waveform.resetAxes", "重置坐标轴"),
    ("waveform.returnLive", "返回实时"),
    ("waveform.gestureHint", "滚轮缩放 · 左键拖动平移 · 右键拖动分析 · 点击读数"),
    ("waveform.pointReading", "精确采样点"),
    ("waveform.closeReading", "关闭读数"),
    ("waveform.time", "时间"),
    ("waveform.instantPower", "瞬时功率"),
    ("waveform.statsMode", "统计范围"),
    ("waveform.globalStats", "全局统计"),
    ("waveform.tenMinuteStats", "最近 10 分钟"),
    ("waveform.minuteStats", "最近 1 分钟"),
    ("waveform.secondStats", "最近 1 秒"),
    ("waveform.analyze", "区间分析"),
    ("waveform.analysisTitle", "区间统计"),
    ("waveform.analysisHint", "在图表上按右键拖动可选择分析区间"),
    ("waveform.startTime", "开始时间"),
    ("waveform.endTime", "结束时间"),
    ("waveform.powerPeak", "峰值功率"),
    ("waveform.energy", "电量"),
    ("waveform.modeLive", "实时跟随"),
    ("waveform.modeFrozen", "历史浏览"),
    ("waveform.modeAnalysis", "区间分析"),
    ("waveform.viewport", "时间位置"),
    ("waveform.rangeStart", "范围起点"),
    ("waveform.rangeEnd", "范围终点"),
    ("metrics.latestVoltage", "实时电压"),
    ("metrics.averageVoltage", "平均电压"),
    ("metrics.peakVoltage", "峰值电压"),
    ("metrics.latestCurrent", "实时电流"),
    ("metrics.averageCurrent", "平均电流"),
    ("metrics.peakCurrent", "峰值电流"),
    ("metrics.latestPower", "实时功率"),
    ("metrics.averagePower", "平均功率"),
    ("metrics.duration", "采集时长"),
    ("metrics.pointCount", "采样点"),
    ("metrics.energy", "能量"),
    ("calibration.title", "设备校准"),
    ("calibration.subtitle", "读取、调整并保存 PowerPico 三档电流校准参数"),
    ("calibration.formula", "I = ΔADC × 标称分辨率 × k + b − 分压电流"),
    ("calibration.warning", "自动校准前请断开负载并保持零电流，校准过程中请勿拔出设备。"),
    ("calibration.read", "读取参数"),
    ("calibration.auto", "自动零点校准"),
    ("calibration.reset", "恢复出厂校准"),
    ("calibration.confirmReset", "将全部档位恢复为增益 1、偏移 0？"),
    ("calibration.confirmManual", "确认写入此档位的校准参数？"),
    ("calibration.old", "原参数"),
    ("calibration.new", "新参数"),
    ("calibration.gain", "增益"),
    ("calibration.offset", "零点偏移"),
    ("calibration.apply", "写入"),
    ("calibration.limits", "请先连接并读取参数。增益范围 0.85–1.15；手动偏移限值 ±48 LSB；自动零点限值 ±32 LSB。High 档偏移以 mA 显示。完成操作后请断开校准连接，再进行波形采集或固件升级。"),
    ("calibration.idle", "未连接"),
    ("calibration.connected", "已连接，等待读取参数"),
    ("calibration.start", "正在处理…"),
    ("calibration.collecting", "正在采样"),
    ("calibration.commit", "正在写入参数"),
    ("calibration.verify", "正在校验参数"),
    ("calibration.restore", "正在恢复原档位"),
    ("calibration.completed", "操作完成，参数已回读"),
    ("calibration.cancelled", "校准已取消"),
    ("calibration.failed", "校准失败"),
    ("calibration.warningStatus", "参数已保存，恢复档位失败"),
    ("calibration.uncertain", "设备可能已写入参数，请重新连接并读取参数后确认，避免重复写入。"),
    ("calibration.statistics", "零点采样结果"),
    ("calibration.range", "档位"),
    ("calibration.samples", "采样点数"),
    ("calibration.zero", "零点"),
    ("calibration.noise", "块均值标准差"),
    ("calibration.confirm", "确认校准"),
    ("firmware.title", "固件升级"),
    ("firmware.subtitle", "下载官方固件或使用本地 BIN 文件升级 PowerPico"),
    ("firmware.device", "目标设备"),
    ("firmware.file", "固件文件"),
    ("firmware.choose", "选择 .bin 固件"),
    ("firmware.flash", "开始刷写"),
    ("firmware.idle", "等待选择设备和固件"),
    ("firmware.warning", "升级过程中请勿拔出设备或关闭应用。"),
    ("firmware.success", "固件升级完成"),
    ("firmware.noFile", "请先选择固件文件"),
    ("firmware.official", "官方固件"),
    ("firmware.custom", "自定义固件"),
    ("firmware.source", "固件来源"),
    ("firmware.latest", "最新版本"),
    ("firmware.releaseDate", "发布日期"),
    ("firmware.downloadLatest", "下载最新固件"),
    ("firmware.alreadyLatest", "官方固件已是最新版本"),
    ("firmware.releaseNotes", "发行说明"),
    ("firmware.downloading", "正在下载固件…"),
    ("firmware.downloadCompleted", "固件下载完成"),
    ("firmware.downloadCancelled", "固件下载已取消"),
    ("firmware.downloadFailed", "固件下载失败"),
    ("firmware.connecting", "正在连接 PowerPico…"),
    ("firmware.rebooting", "正在进入引导加载程序…"),
    ("firmware.searchingBootloader", "正在搜索引导加载程序…"),
    ("firmware.handshaking", "正在等待引导加载程序握手…"),
    ("firmware.uploading", "正在上传固件…"),
    ("firmware.finishing", "正在启动新固件…"),
    ("firmware.completed", "固件升级成功"),
    ("firmware.cancelled", "固件升级已取消"),
    ("firmware.failed", "固件升级失败"),
    ("settings.title", "设置"),
    ("settings.subtitle", "调整界面外观和语言"),
    ("settings.appearance", "外观"),
    ("settings.theme", "主题"),
    ("settings.system", "跟随系统"),
    ("settings.light", "浅色"),
    ("settings.dark", "深色"),
    ("settings.language", "界面语言"),
    ("settings.about", "关于开源版本"),
    ("settings.aboutText", "基于 GPUI（Rust）构建的 PowerPico 桌面客户端，采用 GNU GPL v3 许可证。"),
    ("settings.personalization", "个性化"),
    ("settings.uiScale", "界面缩放"),
    ("settings.auto", "自动"),
    ("settings.antiAliasing", "波形抗锯齿"),
    ("settings.update", "软件更新"),
    ("settings.checkAtStartup", "启动时检查更新"),
    ("settings.checkNow", "立即检查"),
    ("settings.upToDate", "当前已是最新版本"),
    ("settings.updateAvailable", "发现新版本"),
    ("settings.checking", "正在检查…"),
    ("settings.updateUnavailable", "当前版本暂未提供在线更新清单，请稍后重试。"),
    ("settings.downloadInstall", "下载并安装"),
    ("settings.downloadingUpdate", "正在下载更新…"),
    ("settings.installing", "正在安装…"),
    ("settings.updateCancelled", "更新已取消"),
    ("settings.storage", "存储与卸载"),
    ("settings.cache", "缓存"),
    ("settings.appData", "应用数据"),
    ("settings.logs", "日志"),
    ("settings.systemData", "系统数据"),
    ("settings.total", "合计"),
    ("settings.application", "应用程序"),
    ("settings.clearCache", "清理缓存"),
    ("settings.cleaning", "正在清理…"),
    ("settings.cacheCleared", "缓存已清理"),
    ("settings.confirmClearCache", "确定清理 PowerPico Client 缓存吗？当前内部采集记录也会被清除。"),
    ("settings.uninstall", "卸载 PowerPico Client"),
    ("settings.uninstallHint", "删除应用和全部应用数据"),
    ("settings.confirmUninstall", "确定彻底卸载 PowerPico Client 吗？此操作会删除全部应用数据。"),
    ("settings.uninstalling", "正在卸载…"),
    ("settings.volumesToEject", "将推出的磁盘映像卷"),
    ("settings.help", "帮助"),
    ("settings.feedback", "反馈"),
    ("settings.openHelp", "打开使用帮助"),
    ("settings.provideFeedback", "提供反馈"),
    ("settings.waveformMetrics", "波形指标"),
    ("settings.waveformMetricsHint", "拖拽调整展示顺序，也可使用按钮移动；移除的指标可随时重新添加。"),
    ("settings.visibleMetrics", "当前显示的波形指标"),
    ("settings.availableMetrics", "可添加指标"),
    ("settings.allMetricsVisible", "所有指标均已显示"),
    ("settings.moveUp", "向上移动"),
    ("settings.moveDown", "向下移动"),
    ("settings.removeMetric", "移除指标"),
];

const EN: &[(&str, &str)] = &[
    ("app.name", "PowerPico Client"),
    ("app.connected", "Device connected"),
    ("app.disconnected", "Disconnected"),
    ("nav.calibration", "Calibration"),
    ("nav.waveform", "Waveform"),
    ("nav.firmware", "Firmware"),
    ("nav.settings", "Settings"),
    ("common.refresh", "Refresh"),
    ("common.connect", "Connect"),
    ("common.disconnect", "Disconnect"),
    ("common.clear", "Clear"),
    ("common.import", "Import"),
    ("common.export", "Export"),
    ("common.cancel", "Cancel"),
    ("common.browse", "Browse"),
    ("common.start", "Start"),
    ("common.none", "None"),
    ("common.error", "Error"),
    ("common.close", "Close"),
    ("common.check", "Check"),
    ("common.download", "Download"),
    ("common.open", "Open"),
    ("common.save", "Save"),
    ("errors.serialPermission", "Serial access was denied. Add your user to dialout on Debian/Ubuntu or uucp on Arch Linux, then sign out and back in."),
    ("errors.externalLink", "Could not open the external link. Check your default browser settings and try again."),
    ("waveform.title", "Live waveform"),
    ("waveform.subtitle", "Capture, inspect and export PowerPico measurements"),
    ("waveform.selectDevice", "Select serial device"),
    ("waveform.voltage", "Voltage"),
    ("waveform.current", "Current"),
    ("waveform.points", "Samples"),
    ("waveform.empty", "Connect a device to display its live waveform"),
    ("waveform.imported", "Recording loaded"),
    ("waveform.exportSuccess", "Data exported"),
    ("waveform.confirmClear", "Clear the current recording?"),
    ("waveform.noDevice", "Select a device first"),
    ("waveform.resetAxes", "Reset axes"),
    ("waveform.returnLive", "Return to live"),
    ("waveform.gestureHint", "Wheel to zoom · left-drag to pan · right-drag to analyze · click to inspect"),
    ("waveform.pointReading", "Exact sample"),
    ("waveform.closeReading", "Close reading"),
    ("waveform.time", "Time"),
    ("waveform.instantPower", "Instant power"),
    ("waveform.statsMode", "Statistics range"),
    ("waveform.globalStats", "Global statistics"),
    ("waveform.tenMinuteStats", "Last 10 minutes"),
    ("waveform.minuteStats", "Last minute"),
    ("waveform.secondStats", "Last second"),
    ("waveform.analyze", "Range analysis"),
    ("waveform.analysisTitle", "Range statistics"),
    ("waveform.analysisHint", "Right-drag on the chart to select an analysis range"),
    ("waveform.startTime", "Start time"),
    ("waveform.endTime", "End time"),
    ("waveform.powerPeak", "Peak power"),
    ("waveform.energy", "Charge"),
    ("waveform.modeLive", "Live follow"),
    ("waveform.modeFrozen", "History"),
    ("waveform.modeAnalysis", "Range analysis"),
    ("waveform.viewport", "Time position"),
    ("waveform.rangeStart", "Range start"),
    ("waveform.rangeEnd", "Range end"),
    ("metrics.latestVoltage", "Live voltage"),
    ("metrics.averageVoltage", "Average voltage"),
    ("metrics.peakVoltage", "Peak voltage"),
    ("metrics.latestCurrent", "Live current"),
    ("metrics.averageCurrent", "Average current"),
    ("metrics.peakCurrent", "Peak current"),
    ("metrics.latestPower", "Live power"),
    ("metrics.averagePower", "Average power"),
    ("metrics.duration", "Duration"),
    ("metrics.pointCount", "Sample count"),
    ("metrics.energy", "Energy"),
    ("calibration.title", "Device calibration"),
    ("calibration.subtitle", "Read, adjust and save the three PowerPico current ranges"),
    ("calibration.formula", "I = ΔADC × nominal scale × k + b − divider current"),
    ("calibration.warning", "Disconnect the load and keep zero current before automatic calibration. Do not unplug the device during calibration."),
    ("calibration.read", "Read parameters"),
    ("calibration.auto", "Automatic zero calibration"),
    ("calibration.reset", "Restore factory calibration"),
    ("calibration.confirmReset", "Reset all ranges to gain 1 and offset 0?"),
    ("calibration.confirmManual", "Write calibration parameters for this range?"),
    ("calibration.old", "Previous"),
    ("calibration.new", "New"),
    ("calibration.gain", "Gain"),
    ("calibration.offset", "Zero offset"),
    ("calibration.apply", "Apply"),
    ("calibration.limits", "Connect and read parameters before editing. Gain: 0.85–1.15; manual offset: ±48 LSB; automatic zero: ±32 LSB. High range offsets are displayed in mA. Disconnect calibration before waveform acquisition or firmware updates."),
    ("calibration.idle", "Disconnected"),
    ("calibration.connected", "Connected; read parameters to edit"),
    ("calibration.start", "Working…"),
    ("calibration.collecting", "Collecting samples"),
    ("calibration.commit", "Saving parameters"),
    ("calibration.verify", "Verifying parameters"),
    ("calibration.restore", "Restoring original range"),
    ("calibration.completed", "Completed; parameters read back"),
    ("calibration.cancelled", "Calibration cancelled"),
    ("calibration.failed", "Calibration failed"),
    ("calibration.warningStatus", "Saved; range restoration failed"),
    ("calibration.uncertain", "Parameters may have been saved. Reconnect and read them before writing again."),
    ("calibration.statistics", "Zero calibration statistics"),
    ("calibration.range", "Range"),
    ("calibration.samples", "Samples"),
    ("calibration.zero", "Zero"),
    ("calibration.noise", "Block mean standard deviation"),
    ("calibration.confirm", "Confirm calibration"),
    ("firmware.title", "Firmware update"),
    ("firmware.subtitle", "Download official firmware or use a local BIN image"),
    ("firmware.device", "Target device"),
    ("firmware.file", "Firmware file"),
    ("firmware.choose", "Choose .bin firmware"),
    ("firmware.flash", "Flash firmware"),
    ("firmware.idle", "Choose a device and firmware"),
    ("firmware.warning", "Do not unplug the device or close the application while flashing."),
    ("firmware.success", "Firmware update completed"),
    ("firmware.noFile", "Choose a firmware file first"),
    ("firmware.official", "Official firmware"),
    ("firmware.custom", "Custom firmware"),
    ("firmware.source", "Firmware source"),
    ("firmware.latest", "Latest version"),
    ("firmware.releaseDate", "Release date"),
    ("firmware.downloadLatest", "Download latest firmware"),
    ("firmware.alreadyLatest", "Official firmware is already current"),
    ("firmware.releaseNotes", "Release notes"),
    ("firmware.downloading", "Downloading firmware…"),
    ("firmware.downloadCompleted", "Firmware download completed"),
    ("firmware.downloadCancelled", "Firmware download cancelled"),
    ("firmware.downloadFailed", "Firmware download failed"),
    ("firmware.connecting", "Connecting to PowerPico…"),
    ("firmware.rebooting", "Rebooting into bootloader…"),
    ("firmware.searchingBootloader", "Searching for bootloader…"),
    ("firmware.handshaking", "Waiting for bootloader handshake…"),
    ("firmware.uploading", "Uploading firmware…"),
    ("firmware.finishing", "Starting updated firmware…"),
    ("firmware.completed", "Firmware updated successfully"),
    ("firmware.cancelled", "Firmware update cancelled"),
    ("firmware.failed", "Firmware update failed"),
    ("settings.title", "Settings"),
    ("settings.subtitle", "Adjust appearance and language"),
    ("settings.appearance", "Appearance"),
    ("settings.theme", "Theme"),
    ("settings.system", "Use system setting"),
    ("settings.light", "Light"),
    ("settings.dark", "Dark"),
    ("settings.language", "Language"),
    ("settings.about", "About the open-source client"),
    ("settings.aboutText", "A GPUI (Rust) PowerPico desktop client, licensed under GNU GPL v3."),
    ("settings.personalization", "Personalization"),
    ("settings.uiScale", "Interface scale"),
    ("settings.auto", "Auto"),
    ("settings.antiAliasing", "Waveform anti-aliasing"),
    ("settings.update", "Software update"),
    ("settings.checkAtStartup", "Check for updates at startup"),
    ("settings.checkNow", "Check now"),
    ("settings.upToDate", "You are up to date"),
    ("settings.updateAvailable", "A new version is available"),
    ("settings.checking", "Checking…"),
    ("settings.updateUnavailable", "The update manifest is not available for this version yet. Try again later."),
    ("settings.downloadInstall", "Download and install"),
    ("settings.downloadingUpdate", "Downloading update…"),
    ("settings.installing", "Installing…"),
    ("settings.updateCancelled", "Update cancelled"),
    ("settings.storage", "Storage and uninstall"),
    ("settings.cache", "Cache"),
    ("settings.appData", "Application data"),
    ("settings.logs", "Logs"),
    ("settings.systemData", "System data"),
    ("settings.total", "Total"),
    ("settings.application", "Application"),
    ("settings.clearCache", "Clear cache"),
    ("settings.cleaning", "Cleaning…"),
    ("settings.cacheCleared", "Cache cleared"),
    ("settings.confirmClearCache", "Clear the PowerPico Client cache? The current internal recording will also be removed."),
    ("settings.uninstall", "Uninstall PowerPico Client"),
    ("settings.uninstallHint", "Remove the application and all its data"),
    ("settings.confirmUninstall", "Completely uninstall PowerPico Client? All application data will be deleted."),
    ("settings.uninstalling", "Uninstalling…"),
    ("settings.volumesToEject", "Disk image volumes to eject"),
    ("settings.help", "Help"),
    ("settings.feedback", "Feedback"),
    ("settings.openHelp", "Open user guide"),
    ("settings.provideFeedback", "Provide feedback"),
    ("settings.waveformMetrics", "Waveform metrics"),
    ("settings.waveformMetricsHint", "Drag to reorder, or use the move buttons. Removed metrics can be added again at any time."),
    ("settings.visibleMetrics", "Visible waveform metrics"),
    ("settings.availableMetrics", "Available metrics"),
    ("settings.allMetricsVisible", "All metrics are visible"),
    ("settings.moveUp", "Move up"),
    ("settings.moveDown", "Move down"),
    ("settings.removeMetric", "Remove metric"),
];
