# 从 Tauri 2 迁移到 GPUI

`main` 分支把 Tauri 2 + Svelte 5 的前端整体换成了 Rust + GPUI，界面由 [GPUI Kit](https://github.com/longbridge/gpui-kit) 提供行为与组件，视觉与布局遵循 [Ely GPUI Component](https://github.com/ELY-Open-Source/Ely-GPUI-Components) 的组件体系，并沿用旧版客户端的配色、尺寸与信息层级。旧版实现保留在 `tauri2-archive` 分支。

## 仓库结构

```text
crates/
├── core/     powerpico-core：设备、采集、存储、校准、固件、网络与维护
└── app/      powerpico-app：GPUI 窗口、四个页面、i18n、主题与图表绘制
```

`powerpico-core` 不依赖任何 UI 框架。旧版里散落在 Tauri 命令里的逻辑被收进 `AppState` 的方法，进度通过 `Events` 事件总线回传；`powerpico-app` 用一个后台泵把事件与命令排队，再在 UI 线程落地，因此采集、校准、刷写仍然跑在各自的工作线程上。

## 对应关系

| Tauri + Svelte | 现在 |
| :--- | :--- |
| `src/lib/pages/WaveformPage.svelte` + uPlot | `crates/app/src/pages/waveform.rs` + 自绘 `canvas` |
| `CalibrationPage.svelte` | `pages/calibration.rs`（Ely `NumberInput`、`ProgressBar`、`Table`） |
| `FirmwarePage.svelte` | `pages/firmware.rs`（`ProgressBar`、`Select`） |
| `SettingsPage.svelte` | `pages/settings.rs`（`Select`、`Checkbox`、指标布局） |
| `src-tauri/src/lib.rs` 的 `#[tauri::command]` | `powerpico_core::AppState` 方法 + `Command`/`Effect` 枚举 |
| `app.emit(...)` | `powerpico_core::Events::emit` → `crossbeam_channel` → `CoreEvent` |
| `tauri-plugin-dialog` | `rfd`（GTK3 / 原生对话框） |
| `tauri-plugin-opener` | `open` crate |
| `tauri-plugin-updater` | 暂用 Releases 页面（见下） |
| `styles.css` 的 CSS 变量 | `crates/app/src/theme.rs` 覆盖 Ely 语义色板 |
| `i18n.ts` 的四套文案 | `crates/app/src/i18n.rs`（同样的 key，zh-CN / zh-HK / en / ja） |

## 界面还原

- 侧栏 190 px、19 px 圆角面板、42 px 导航项与底部连接状态，与原版一致。
- 主色 `#246bfd`（暗色 `#6d9cff`）、电压 `#ef565d` / 电流 `#2887f0` / 功率 `#20a66a`，直接写入 Ely 的 `Palette`，所以按钮、选择器、进度条、表格都跟随同一套语义色。
- 面板投影、18 px 圆角、13 px 正文字号与 25 px 页面标题沿用旧版设计稿。
- 波形图改为 GPUI 画布：网格、双 Y 轴、两条曲线、区间选择与精确读数点都直接绘制；滚轮缩放、左键拖动平移、右键拖动分析、点击读数的交互与旧版一致。

## 已知差异与后续工作

1. **应用内更新**：Tauri updater 已随 Tauri 移除。当前“发现新版本”打开 Releases 页面；重新接入需要一个 GPUI 侧的下载、校验与替换安装包的实现。
2. **打包与发布**：`release.yml` 只在推送 `vX.Y.Z` tag 时发布。Linux、Windows 和 macOS
   分别在对应的 GitHub Actions 原生 runner 上编译，再由发布 job 汇总三个便携包并生成
   `SHA256SUMS`。DEB/AppImage 与 DMG 仍待补齐，见 [构建说明](./build.md#打包与发布)。
3. **指标拖拽排序**：设置页先提供上移/下移按钮，拖拽排序尚未接入 GPUI 的 drag 机制。
4. **键盘与无障碍**：Ely 组件自带焦点环与 AccessKit 语义，但页面级快捷键（Esc 关闭读数等）还没补。
5. **平台验收**：在 Arch Linux / KDE Wayland 上完成了真机采集、校准端口释放与固件页联调；macOS
   与 Windows 也已完成同样的真机验收，因此仓库根的 `rust-toolchain.toml` 把工具链精确固定在
   `1.98.0`——两个平台的 GPUI 依赖都要求这个版本，浮动在 `stable` 上会随发布漂移。

## 使用 AI 编码助手的技能

GPUI Kit 提供两个技能，安装到用户级目录后可直接查阅：

```bash
npx skills add longbridge/gpui-kit
# 或手动放到 ~/.workbuddy/skills/ 下：
#   gpui-kit、gpui-kit-design-guides
```

版本约束：`gpui-kit = "=0.9.1"` 与 Ely 都固定在 `gpui-pre 0.3.12`，两者必须在同一个 GPUI 快照上，升级时要一起改。
