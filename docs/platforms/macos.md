# macOS 安装与构建

## 安装

当前发布目标为 macOS 11+ Apple Silicon。请从 [GitHub Releases](https://github.com/No-Chicken/PowerPicoClient_rs/releases/latest) 下载 `PowerPico-Client_<版本>_macos_aarch64.dmg`，并使用同一 Release 中的 `SHA256SUMS` 校验文件完整性。

打开 DMG，将 `PowerPico Client.app` 拖入“应用程序”，然后在 Finder 侧边栏推出 `PowerPico Client` 磁盘映像卷。首次运行时，在 Finder 中按住 Control 点击应用并选择“打开”。安装包目前未进行 Apple Developer ID 签名和公证。

设置页提供缓存空间统计、一键清理和完整卸载。完整卸载会将 App 移入废纸篓、删除应用数据，并推出仍挂载且经过应用标识校验的 PowerPico Client DMG 卷；下载目录中的原始 DMG 文件不会被删除。客户端支持应用内检查、下载、签名校验和安装更新；OTA 更新包不会挂载新的 DMG 卷。

## 开发环境

安装 Node.js 20+、pnpm 10+、Rust 1.77.2+ 和 Xcode Command Line Tools。克隆仓库后，在项目根目录安装依赖并启动开发版本：

```bash
pnpm install --frozen-lockfile
pnpm tauri dev
```

## 构建

构建未签名 DMG：

```bash
pnpm tauri build --bundles app,dmg
```

Tauri 原始产物位于 `src-tauri/target/release/bundle/`。正式发布还会生成签名 OTA 更新包；发布密钥配置见[macOS OTA 签名密钥配置](../ota-signing.md)。
