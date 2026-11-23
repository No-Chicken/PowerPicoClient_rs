# 构建与运行

PowerPico Client 现在是一个纯 Rust 桌面应用：`powerpico-core` 负责设备与数据，`powerpico-app` 使用 GPUI、GPUI Kit 和 Ely 组件绘制界面。仓库不再需要 Node.js、pnpm 或 Tauri CLI。

## 依赖

- Rust 1.98 或更新版本（GPUI 快照与 Ely 要求 edition 2024）。
- Linux：
  - Wayland 或 X11 会话；
  - `wayland`、`libxkbcommon`、`fontconfig` 开发库；
  - Vulkan 驱动（GPUI 通过 Vulkan 渲染，AMD/Intel 使用 Mesa `radv`/`anv`）；
  - GTK3（文件选择对话框使用 `rfd` 的 GTK 后端）。
  - Arch Linux：`sudo pacman -S base-devel wayland libxkbcommon fontconfig gtk3 vulkan-radeon`（Intel 显卡改用 `vulkan-intel`）。
  - Debian/Ubuntu：`sudo apt install build-essential libwayland-dev libxkbcommon-dev libfontconfig1-dev libgtk-3-dev libvulkan1 mesa-vulkan-drivers`。
- macOS：Xcode 命令行工具。GPUI 在运行时编译 Metal shader，因此无需完整 Xcode。
- Windows：MSVC 工具链；Vulkan 驱动随显卡驱动安装。

## 运行

```bash
cargo run -p powerpico-app
```

首次构建会编译 GPUI、GPUI Kit 与 Ely 的组件，耗时较长；之后的增量构建只有几秒。仓库的 `profile.dev` 已经把依赖设为 `opt-level = 3`、本 crate 设为 `1`，便于调试同时保持波形流畅。

发布构建：

```bash
cargo build --release -p powerpico-app
# 产物：target/release/powerpico-client
```

## 开发辅助环境变量

| 变量 | 作用 |
| :--- | :--- |
| `POWERPICO_PAGE=waveform\|calibration\|firmware\|settings` | 启动时直接打开指定页面，便于逐页检查布局。 |
| `POWERPICO_AUTOSTART=1`（或某个设备 ID） | 设备列表加载后自动开始采集，用于在没有手动点击的情况下检查实时波形。 |
| `POWERPICO_THEME=light\|dark\|system` | 本次运行强制使用指定主题，便于对比两套配色（不写回设置）。 |

## 数据目录

与旧版保持一致，因此升级后设置和已下载的官方固件仍然可用：

| 平台 | 应用数据 | 缓存 | 日志 |
| :--- | :--- | :--- | :--- |
| Linux | `~/.local/share/com.openfeasttech.powerpico-client` | `~/.cache/com.openfeasttech.powerpico-client` | 应用数据目录下的 `logs/` |
| macOS | `~/Library/Application Support/com.openfeasttech.powerpico-client` | `~/Library/Caches/…` | `~/Library/Logs/…` |
| Windows | `%APPDATA%\com.openfeasttech.powerpico-client` | `%LOCALAPPDATA%\…\cache` | `%LOCALAPPDATA%\…\logs` |

## 质量检查

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
```

## 打包

目前 `cargo build --release` 已经得到可独立运行的二进制，但各平台的安装包（DEB/AppImage、DMG、NSIS）尚未重建。旧的 Tauri 打包脚本与发布工作流已随 Tauri 一并移除；需要时按下面的方向补齐：

- Linux：`.desktop` 文件 + 图标，AppImage 可用 `linuxdeploy` 打包。
- macOS：`cargo bundle`/`create-dmg`，并补回签名、公证与图标（`icns`）。
- Windows：`cargo-wix` 或 NSIS 脚本，补回 `ico` 资源与 Authenticode 签名。

应用内更新同样依赖发布产物与更新清单；在重新设计更新通道之前，“发现新版本”只打开 Releases 页面。
