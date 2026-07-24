# PowerPico Client

PowerPico Client 是使用 Tauri 2、Svelte 5 与 Rust 重写的 PowerPico 桌面客户端，面向 macOS 和 Linux，提供高速数据采集、波形分析、记录导入导出和固件升级。

## 平台支持

| 平台 | 架构 | 发布包 | 状态 |
| --- | --- | --- | --- |
| macOS 11+ | Apple Silicon ARM64 | DMG | 支持，当前未签名且未公证 |
| Ubuntu/Debian Linux | x86_64 | DEB、AppImage | 支持 |

Windows、macOS Intel、Linux ARM64 和 RPM 暂不支持，也不属于当前发布目标。

## 已实现功能

- macOS `/dev/cu.*` 与 Linux `/dev/ttyACM*`、`/dev/ttyUSB*` 串口发现及稳定设备标识
- Rust 实现的 1.5 Mbps 数据接收、PowerPico 协议解析和异常采样过滤
- 与原客户端兼容的 L0/L1-L5 分级记录、LOD 波形读取、BIN/CSV 导入导出
- 实时跟随、历史浏览、时间/电压/电流独立缩放和平移及精确原始采样点读取
- 全局、最近 10 分钟、最近 1 分钟和最近 1 秒统计
- 右键拖选区间分析，包括电压、电流、功率、时长、采样数和 mAh
- 可排序、增删并持久化的实时指标卡片
- 官方固件版本查询、下载和本地缓存，以及自定义 BIN 固件
- Bootloader 重新枚举、YMODEM 刷写、进度显示和取消
- 基于 GitHub Releases 的启动更新检查和手动更新检查
- 简体中文、繁体中文、英语、日语和跟随系统语言
- 浅色、深色、跟随系统主题，界面缩放和波形渲染质量设置
- 内部临时记录自动清理、平台应用数据目录和滚动日志

## 开发

需要 Rust 1.77.2+、Node.js 20+ 和 pnpm 10+。

```bash
pnpm install
pnpm tauri dev
```

Arch Linux 开发依赖：

```bash
sudo pacman -S --needed base-devel webkit2gtk-4.1 libappindicator-gtk3 librsvg systemd
```

## 质量检查

```bash
pnpm check
pnpm test
pnpm build

cd src-tauri
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

## 构建

macOS ARM64 未签名 DMG：

```bash
pnpm tauri build --bundles dmg
```

本地 Linux 构建始终使用固定的 Ubuntu 22.04 Docker 容器：

```bash
pnpm tauri:build:linux
```

产物输出到 `artifacts/linux/`。GitHub Actions 不使用该容器，而是在 `ubuntu-22.04` runner 上原生安装依赖并构建 DEB/AppImage。

## Linux 串口权限

客户端不应以 root 运行。Ubuntu/Debian 用户加入 `dialout`，Arch Linux 用户加入 `uucp`，然后注销并重新登录：

```bash
sudo usermod -aG dialout "$USER"  # Ubuntu/Debian
sudo usermod -aG uucp "$USER"    # Arch Linux
```

真机验收项目见 [Linux 测试清单](docs/linux-testing.md)。

## 数据与网络

设置和官方固件保存在系统应用数据目录，临时采集记录保存在应用缓存目录，日志保存在平台日志目录。官方固件信息来自 PowerPico 原版固件服务，客户端更新信息来自本仓库 GitHub Releases。

## 发布

推送严格格式的 `vX.Y.Z` 标签后，GitHub Actions 会执行前后端质量门禁，构建 macOS ARM64 DMG、Linux x86_64 DEB/AppImage，生成 `SHA256SUMS` 并发布正式 GitHub Release。

## 许可证

Copyright (C) 2026 OpenFeastTech。

PowerPico Client 基于 [GNU General Public License v3.0 only](LICENSE) 发布，SPDX 标识为 `GPL-3.0-only`。第三方组件声明见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。仓库中的旧 Python/Qt 客户端仅作为协议和行为兼容性参考，不参与本程序链接或打包。
