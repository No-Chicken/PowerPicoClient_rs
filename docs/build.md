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

## 打包与发布

CI/CD 只在推送 `vX.Y.Z` tag 时运行（`.github/workflows/release.yml`）：先在 Ubuntu 24.04 上跑
格式、clippy 与测试，再构建 release 二进制并打成
`powerpico-client-<tag>-linux-x86_64.tar.gz`，最后创建或更新对应的 GitHub Release，
附带 `SHA256SUMS`。手动触发（workflow_dispatch）只构建产物、不发布。

```bash
# 版本号需要与 tag 一致（tag 会校验 Cargo.toml 里的版本）
git tag v0.3.0
git push origin v0.3.0
```

由于检查也只在打 tag 时跑，合并前请本地执行：

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

尚未纳入 tag 流水线的产物：

- Linux AppImage / DEB：需要补 `.desktop`、图标与 `linuxdeploy` 步骤。
- macOS DMG：需要 `icns`、签名与公证；Ely 对 `gpui-pre-macos` 的 vendor patch 目前只在它自己的
  workspace 生效，跨仓库构建前要先确认。
- Windows NSIS / 便携 ZIP：需要 `ico` 资源与 Authenticode 签名。

应用内更新还依赖这些产物与更新清单；在补回打包之前，“发现新版本”只打开 Releases 页面。
