# Windows 安装与构建

## 安装

当前发布目标为 Windows x86_64。请从 [GitHub Releases](https://github.com/No-Chicken/PowerPicoClient_rs/releases/latest) 下载对应版本，并使用同一 Release 中的 `SHA256SUMS` 校验文件完整性。

| 格式 | 文件 | 使用方式 |
| --- | --- | --- |
| NSIS 安装版 | `PowerPico-Client_<版本>_windows_x86_64-setup.exe` | 按当前用户安装，提供开始菜单及系统卸载入口；缺少 WebView2 时引导安装 |
| 便携 ZIP | `PowerPico-Client_<版本>_windows_x86_64-portable.zip` | 解压全部文件后运行 `powerpico-client.exe`；系统需要已有 WebView2 Runtime |

便携版附带使用说明、GPL 许可证和第三方声明。便携版与安装版共用当前用户的 AppData 设置、固件、采集记录和日志；移动程序文件夹不会迁移这些数据。更新便携版时关闭客户端，再解压替换程序文件。应用内更新使用 NSIS 安装器，会转为安装版。官方设备使用 Windows USB CDC 串口驱动。

## 开发环境

安装 Node.js 20+、pnpm 10+、Rust 1.77.2+ 的 `x86_64-pc-windows-msvc` 工具链、Visual Studio Build Tools（C++ 桌面工具与 Windows SDK）和 WebView2 Runtime。

在项目根目录安装依赖并启动开发版本：

```powershell
pnpm install --frozen-lockfile
pnpm tauri dev
```

## 构建

Windows 平台配置默认生成当前用户 NSIS 安装包。本地构建命令如下，不需要发布 updater 私钥：

```powershell
pnpm tauri build
```

原始产物位于 `src-tauri/target/release/`。同时运行检查并生成 NSIS 安装包和便携 ZIP：

```powershell
pnpm tauri:build:windows
```

依赖已缓存时可离线构建；正式发布构建通过环境变量提供 updater 私钥和可选密码：

```powershell
pnpm tauri:build:windows -Offline
pnpm tauri:build:windows -Updater
```

打包脚本自动发现 MSVC Build Tools、安装锁定依赖、运行 Svelte 检查、前端测试、Rust 格式检查、Clippy 和后端测试，然后构建 x64 NSIS 和便携 ZIP。标准化产物及包含两种分发文件的 `SHA256SUMS` 写入 `artifacts/windows/`；显式 x64 目标的原始产物位于 `src-tauri/target/x86_64-pc-windows-msvc/release/`。

`-Updater` 会为 NSIS 安装器生成 `.exe.sig`。Tauri 更新签名与 Windows Authenticode 代码签名是两种机制；本项目尚未配置 Authenticode。便携 ZIP 不作为 Tauri updater 载荷。完整实机验收范围见 [Windows 测试记录](../windows-testing.md)。
