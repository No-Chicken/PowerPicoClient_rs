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
格式、clippy 与测试，然后在**同一台 Ubuntu runner** 上构建并打包两个产物，最后创建或更新对应的
GitHub Release，附带 `SHA256SUMS`：

- `powerpico-client-<tag>-linux-x86_64.tar.gz`（`cargo build --release`）
- `powerpico-client-<tag>-windows-x86_64.zip`（`cargo xwin` 交叉编译的便携版）

手动触发（workflow_dispatch）只构建产物、不发布。

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

### Windows 交叉编译细节

Windows 版由 `cargo xwin build --target x86_64-pc-windows-msvc` 在 Linux 上产出，本机复现：

```bash
rustup target add x86_64-pc-windows-msvc
cargo install --locked cargo-xwin          # 需要 clang、lld（llvm 的 lld-link）
RUSTFLAGS="-C target-feature=+crt-static" \
  cargo xwin build --release --target x86_64-pc-windows-msvc -p powerpico-app
# 产物：target/x86_64-pc-windows-msvc/release/powerpico-client.exe
```

两个关键点：

- **C 运行时静态链接**（`+crt-static`）：exe 不再依赖 `VCRUNTIME140.dll`，开箱即用。
  其余动态依赖（`icuuc.dll`、`d3dcompiler_47.dll` 等）从 Windows 10 1703 / 8.1 起就是系统自带。
- **着色器编译绕过**：`gpui-pre-windows` 的 build 脚本用 fxc.exe 预编译 HLSL，但它的
  `cfg(target_os = "windows")` 看到的是**宿主**系统，交叉编译时整段被跳过。`Cargo.toml` 里
  给该 crate 单独开了 `debug-assertions`，让它走运行时编译路径（系统自带
  `d3dcompiler_47.dll`，启动时多花几毫秒），于是不再需要 Windows SDK。若上游改用
  `CARGO_CFG_TARGET_OS` 修复，可删掉该 profile 覆盖。

另外 `crates/app/src/main.rs` 顶部用 `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`
保证 release 版双击运行不弹控制台；debug 版保留控制台以便看日志。

尚未纳入 tag 流水线的产物：

- Linux AppImage / DEB：需要补 `.desktop`、图标与 `linuxdeploy` 步骤。
- macOS DMG：需要 `icns`、签名与公证；Ely 对 `gpui-pre-macos` 的 vendor patch 目前只在它自己的
  workspace 生效，跨仓库构建前要先确认。
- Windows NSIS 安装包：便携 ZIP 已由交叉编译产出，需要安装器时再补 `ico` 资源与 Authenticode 签名。

应用内更新还依赖这些产物与更新清单；在补回打包之前，“发现新版本”只打开 Releases 页面。
