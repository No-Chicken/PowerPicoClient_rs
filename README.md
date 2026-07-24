# PowerPico Client

PowerPico Client 是基于 Tauri 2、Svelte 5 和 Rust 开发的开源跨平台桌面客户端，用于采集、查看和导出 PowerPico 测量数据，并支持设备固件升级。

本仓库采用 MIT 许可证。原 Python/Qt 客户端仅作为协议与兼容性参考，不属于本仓库的授权范围。

## 功能特性

- 跨平台串口设备发现，完整支持 macOS `/dev/cu.*` 设备路径
- 使用 Rust 实现 1.5 Mbps 数据采集与 PowerPico 二进制协议解析
- 支持与旧版客户端兼容的 L0/Ln 记录文件及 CSV 导出
- 实时显示电压、电流波形，时间轴、电压轴和电流轴可独立缩放与平移
- 支持精确查看原始采样点，并锁定显示电压、电流和功率
- 可持久化和重新排序的指标卡片，显示实时值、平均值、峰值、时长、计数和能量
- 支持通过 YMODEM 本地刷写固件及重新发现引导加载器设备
- 提供简体中文和英文界面，以及浅色、深色和跟随系统主题
- 支持构建 macOS ARM64 应用及未签名 DMG 安装包

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

安装依赖并启动开发模式：

```bash
pnpm install
pnpm tauri dev
```

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

## 数据目录

应用设置保存在操作系统的应用数据目录中，采集记录保存在应用缓存目录中，日志保存在平台日志目录中。应用不会依赖启动时的当前工作目录。

## 许可证

本项目基于 [MIT License](LICENSE) 开源。第三方组件及其许可证信息请参阅 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
