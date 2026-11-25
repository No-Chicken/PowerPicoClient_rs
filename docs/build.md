# 构建与运行

PowerPico Client 现在是一个纯 Rust 桌面应用：`powerpico-core` 负责设备与数据，`powerpico-app` 使用 GPUI、GPUI Kit 和 Ely 组件绘制界面。仓库不再需要 Node.js、pnpm 或 Tauri CLI。

## 依赖

- Rust 1.98（GPUI 快照与 Ely 要求 edition 2024）。仓库根的 `rust-toolchain.toml` 把工具链固定为
  `1.98.0` 并带上 `rustfmt`、`clippy`，在该目录第一次执行 `cargo` 时 rustup 会自动安装，无需手动切换。
- Linux：
  - Wayland 或 X11 会话；
  - `wayland`、`libxkbcommon`、`fontconfig` 开发库；
  - Vulkan 驱动（GPUI 通过 Vulkan 渲染，AMD/Intel 使用 Mesa `radv`/`anv`）；
  - GTK3（文件选择对话框使用 `rfd` 的 GTK 后端）。
  - Arch Linux：`sudo pacman -S base-devel wayland libxkbcommon fontconfig gtk3 vulkan-radeon`（Intel 显卡改用 `vulkan-intel`）。
  - Debian/Ubuntu：`sudo apt install build-essential libwayland-dev libxkbcommon-dev libfontconfig1-dev libgtk-3-dev libvulkan1 mesa-vulkan-drivers`。
- macOS：Xcode 命令行工具。GPUI 在运行时编译 Metal shader，因此无需完整 Xcode。
- Windows：MSVC 工具链和 Windows SDK；GPUI Windows 后端使用 Direct3D 11，显卡驱动需要
  提供 D3D feature level 10.1 或更高。

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

CI/CD 在推送 `vX.Y.Z` tag 时运行（`.github/workflows/release.yml`）。流程先在 Ubuntu 24.04
上执行格式检查、Clippy、测试并校验 tag 与 `Cargo.toml` 版本一致，然后让三个原生 runner
分别构建对应平台的发布包，最后由发布 job 汇总资产并创建 GitHub Release：

- `powerpico-client-<tag>-linux-x86_64.tar.gz`，Linux x86_64 便携包；
- `powerpico-client-<tag>-windows-x86_64.zip`，Windows x86_64 便携包；
- `powerpico-client-<tag>-macos-aarch64.tar.gz`，Apple Silicon macOS 便携包。

macOS 产物仍是未打包的可执行文件，解压后在终端运行，首次运行可能需要移除隔离属性：

```bash
xattr -d com.apple.quarantine powerpico-client
```

发布命令：

```bash
VERSION=0.4.1
git tag "v${VERSION}"
git push origin "v${VERSION}"
```

手动触发 `workflow_dispatch` 会在三个 runner 上构建并上传 Actions artifacts，但不会创建
GitHub Release。Release 中的 `SHA256SUMS` 覆盖三个平台的全部压缩包。

合并前请本地执行：

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Windows job 使用原生 `windows-2022` runner 与 MSVC 工具链。构建前从 Windows SDK 定位
`fxc.exe`，由 `gpui-pre-windows` 在构建阶段把 HLSL 编译为嵌入式 shader bytecode，发布包不
依赖 Linux Cargo 路径或运行时 shader 源文件。`crates/app/src/main.rs` 的 release 配置仍让
双击运行不弹控制台，debug 构建保留控制台以便查看日志。

尚未纳入流水线的产物：

- Linux AppImage / DEB：需要补 `.desktop`、图标与 `linuxdeploy` 步骤。
- macOS DMG 与 `.app` 包：需要 `icns`、签名与公证；Ely 对 `gpui-pre-macos` 的 vendor patch 目前
  只在它自己的 workspace 生效，跨仓库构建前要先确认。
- Windows NSIS 安装包：便携 ZIP 已可产出，需要安装器时再补 `ico` 资源与 Authenticode 签名。

应用内更新还依赖这些产物与更新清单；在补回打包之前，“发现新版本”只打开 Releases 页面。
