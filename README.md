# PowerPico Client

PowerPico Client 是基于 Tauri 2、Svelte 5 和 Rust 开发的开源跨平台桌面客户端，用于采集、查看和导出 PowerPico 测量数据，并支持设备固件升级。

本仓库采用 MIT 许可证。原 Python/Qt 客户端仅作为协议与兼容性参考，不属于本仓库的授权范围。

## 功能特性

- 跨平台串口设备发现，支持 macOS `/dev/cu.*` 以及 Linux `/dev/ttyACM*`、`/dev/ttyUSB*` 设备路径
- 使用 Rust 实现 1.5 Mbps 数据采集与 PowerPico 二进制协议解析
- 支持与旧版客户端兼容的 L0/Ln 记录文件及 CSV 导出
- 实时显示电压、电流波形，时间轴、电压轴和电流轴可独立缩放与平移
- 支持精确查看原始采样点，并锁定显示电压、电流和功率
- 可持久化和重新排序的指标卡片，显示实时值、平均值、峰值、时长、计数和能量
- 支持通过 YMODEM 本地刷写固件及重新发现引导加载器设备
- 提供简体中文和英文界面，以及浅色、深色和跟随系统主题
- 支持构建 macOS ARM64 应用、未签名 DMG，以及 Linux x86_64 DEB/AppImage 安装包

## 技术栈

- Tauri 2
- Svelte 5
- TypeScript
- Rust
- Vite
- pnpm

## 开发环境

请先安装以下工具：

- Rust 1.77.2 或更高版本
- Node.js 20 或更高版本
- pnpm 10 或更高版本

Arch Linux 开发调试需要 Tauri、WebKitGTK 和串口设备发现所需的系统库：

```bash
sudo pacman -S --needed base-devel webkit2gtk-4.1 libappindicator-gtk3 librsvg systemd
```

安装依赖并启动开发模式：

```bash
pnpm install
pnpm tauri dev
```

Linux 日常开发以 Arch Linux 为主，直接运行 `pnpm tauri dev`，应用将访问宿主机串口设备。安装包不在 Arch Linux 上生成。

## 质量检查

在项目根目录执行前端检查、测试和构建：

```bash
pnpm check
pnpm test
pnpm build
```

检查 Rust 代码：

```bash
cd src-tauri
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

## 构建应用

构建当前平台的安装包：

```bash
pnpm tauri build
```

构建未签名的 macOS DMG：

```bash
pnpm tauri build --bundles dmg
```

Linux x86_64 DEB 和 AppImage 统一使用 Ubuntu 22.04 容器构建，宿主机只需要 Docker：

```bash
pnpm tauri:build:linux
```

构建脚本会创建固定 Node.js、pnpm、Rust 和系统依赖版本的 Ubuntu 22.04 镜像，并将安装包导出到 `artifacts/linux/`。宿主机的 `node_modules`、Rust target 和 Arch 系统库不会进入安装包。当前首版仅支持 x86_64；ARM64 和 RPM 尚未纳入发布验证。

## Linux 串口权限

PowerPico 客户端不应使用 root 权限运行。如果打开串口时提示权限不足，请将当前用户加入发行版的串口用户组：

Ubuntu/Debian：

```bash
sudo usermod -aG dialout "$USER"
```

Arch Linux：

```bash
sudo usermod -aG uucp "$USER"
```

修改用户组后需要注销并重新登录。可使用 `groups` 和 `ls -l /dev/ttyACM* /dev/ttyUSB*` 检查当前用户与设备权限。首版不会安装匹配范围过宽的 udev 规则。

Linux 真机功能验收步骤参见 [Linux 测试清单](docs/linux-testing.md)。

## 数据目录

应用设置保存在操作系统的应用数据目录中，采集记录保存在应用缓存目录中，日志保存在平台日志目录中。应用不会依赖启动时的当前工作目录。

## 许可证

本项目基于 [MIT License](LICENSE) 开源。第三方组件及其许可证信息请参阅 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
