# Windows 功能测试记录

测试日期：2026-11-17。基线提交：`51123767aeb5354ad4566028f3bd3de3ffc1e588`，应用版本 0.1.3。

## 环境与构建

- Windows 11 26H2 x64 虚拟机；12 GiB 内存。
- Node.js 22.23.5、pnpm 10.18.1、Rust 1.100.0 MSVC、Visual Studio Build Tools 和 Windows SDK 10.0.22621.0。
- Git 2.56.1；工作区 `C:\workspace\PowerPicoClient_rs`。
- Svelte 检查 0 错误、0 警告；前端 15 项测试通过；Windows Rust 后端 25 项测试通过。
- `pnpm tauri build --ci -- --locked --offline` 成功生成 release EXE 与 x64 NSIS 安装包。离线参数依赖本机已有缓存；首次构建按 README 使用 `pnpm tauri build`。

`src-tauri/tauri.windows.conf.json` 自动将 Windows 打包目标设为 NSIS，并跳过需要发布签名密钥的更新产物生成。此设置不代表 Windows 在线更新已经通过验证。

## 实机功能结果

使用 STM32 USB CDC 设备（VID:PID `0483:5740`），Windows 枚举为 `COM3`；通过实际 Tauri WebView 页面操作及后端调用验收。详细交互测试使用同一源码的诊断 debug 构建，临时启用 WebView2 调试端口；正式 release 构建使用正常配置。

| 项目 | 结果 |
| --- | --- |
| 串口枚举、连接与实时波形 | 通过，约 10,000 样本/秒 |
| 连续采集 | 81.279898 秒、812,760 样本，无采集错误 |
| 窗口统计、点读数 | 通过 |
| 滚轮缩放、右键拖动区间分析 | 通过 |
| 重复启动采集 | 返回 `DEVICE_BUSY` |
| 停止与三次重新连接 | 全部成功，停止后回到 idle |
| 正常 release 界面采集 | 通过 UI Automation 选择 COM3、连接和断开，约 5.1 秒 / 50,540 样本 |
| 中文及空格路径导出 | BIN、L1–L5 层级数据和 CSV 均成功 |
| 导出一致性 | CSV 数据行数与 BIN 样本数均为 812,760 |
| BIN 重新导入 | 全量统计与导出前完全一致 |

此次测试验证了数据链路和应用行为，没有用校准仪器验证电压、电流的绝对精度。已进一步完成下方 Bootloader 与 NSIS 安装/覆盖安装/卸载验证。正式在线升级服务、运行中物理拔插与长时间压力测试尚未完成。

## 虚拟机 USB 配置

在虚拟机正常关机后配置 USB 直通，设备缺席时允许虚拟机启动：

```xml
<hostdev mode='subsystem' type='usb' managed='yes'>
  <source startupPolicy='optional' guestReset='off'>
    <vendor id='0x0483'/>
    <product id='0x5740'/>
  </source>
</hostdev>
```

初始状态下串口可打开但没有字节流；设备物理断电重插后，宿主机 3 秒接收 243,175 字节、30,020 个有效样本、0 个无效样本。设置 `guestReset='off'` 后再次冷启动虚拟机，Windows 3 秒接收 243,747 字节，并通过上述应用采集测试。这支持 USB 复位与固件输出状态相关的判断，尚未通过修改固件定位根因。

宿主机通过仅匹配该设备 VID/PID/序列号的 udev 规则赋予运行 QEMU 的用户 USB 节点访问权限。当前规则使用序列号 `398839533132`；换设备时应更新规则。libvirt 配置以 VID/PID 匹配直通，因此宿主机有多台同型号设备时需要进一步区分。

## Bootloader 实机更新（2026-11-17）

设备由用户置于 Boot 模式。使用正常 Windows release 客户端，通过“固件”页下载官方固件、选择 COM3 并点击“开始刷写”，完成 YMODEM 上传与启动阶段，界面显示“固件升级成功 / 100%”。

- 官方固件：v1.1.7，发布日期 2026-08-16，大小 389,432 字节。
- 文件：`https://no-chicken.com/resources/Power-Pico/PowerPico_Firmware_v1.1.7.bin`。
- SHA256：`ce00007576b0fe108b290e2f3763f4e278d47deec781e22db93dd4fd4b2152db`；客户端缓存与官网下载文件一致。
- 记录到 2%（进入 Bootloader）、6% / 82%（上传）、98%（启动）、100%（完成）。
- 未对下载的固件做任何修改，没有重写 Bootloader。
- 新固件恢复后，使用本次新构建的正常 Windows release 版连接 COM3，5.1 秒采集 50,650 样本，正常断开并保留波形。

此宿主机使用 libvirt 原始 USB hostdev。设备进入 Boot 模式及刷写后重新枚举时，宿主机 USB device number 会变化；原有 QEMU 连接可能失效。按用户要求，正常关闭虚拟机后重新启动，让持久配置重新选取设备。串口消失需要排查虚拟机 USB 接入，不能仅凭这一现象判定刷写失败。原生 Windows 插拔不依赖此宿主机操作。

## Windows 构建与安装包

推荐 `pnpm tauri:build:windows`：自动初始化 x64 MSVC、锁定安装依赖、检查 Svelte、前端测试、Rust 格式/Clippy/后端测试，再生成 NSIS 和 SHA256SUMS。本机 `-Offline` 全流程通过：Svelte 0 错误/0 警告、前端 15 项、后端 25 项。Rust 1.100 新增 Clippy 检查要求下载百分比使用 `checked_div`，已保持原有未知长度为 0、下载中至多 99% 的行为修复。

产物：`artifacts/windows/PowerPico-Client_0.1.3_windows_x86_64-setup.exe`；原始 EXE 在 `src-tauri/target/x86_64-pc-windows-msvc/release/`。NSIS 当前用户安装、覆盖安装两次均返回 0，生成系统卸载入口；静默卸载返回 0，已移除安装文件及注册信息，并保留已有应用数据。验证使用独立安装目录，不删除采集记录。

GitHub Actions 已新增 Windows 2022 x64 构建，使用 `-Updater` 生成签名安装器并上传 Release；聚合工作流增加 Windows 安装器、签名、SHA256SUMS 和 `latest.json` 的 `windows-x86_64`。本地普通构建不需要发布私钥，正式 updater 构建需要与仓库公钥对应的 `TAURI_SIGNING_PRIVATE_KEY` 和可选密码。私钥不进入仓库。

Release 工作流通过 actionlint，资产聚合脚本已在本地使用 macOS/Linux 占位文件和实际 Windows 测试签名执行，确认 Windows URL、签名与 SHA256SUMS 均正确纳入。正式 GitHub Actions 发布尚未实际触发；Windows 更新安装也未连接正式服务做端到端验证。应用内完整卸载仍只支持 macOS，Windows 使用 NSIS 提供的系统卸载入口。安装器尚未做 Authenticode 代码签名。

`-Updater` 本地已用仓库外临时密钥和临时公钥覆盖进行完整签名构建，随后用独立 Ed25519 实现校验安装器签名及 trusted comment，均通过。测试结束删除临时私钥、恢复仓库公钥并重新生成普通交付安装包；交付安装包不带临时测试签名。

## Windows 便携版与 CI

`pnpm tauri:build:windows` 同时生成 `artifacts/windows/PowerPico-Client_0.1.3_windows_x86_64-setup.exe`、`PowerPico-Client_0.1.3_windows_x86_64-portable.zip` 和校验文件。ZIP 根目录包含 `powerpico-client.exe`、`README.txt`、`LICENSE.txt`、`THIRD_PARTY_NOTICES.md`；EXE 来自同一次 release 构建，临时打包目录会自动清理。

便携版不创建系统安装记录，依赖电脑已有的 WebView2 Runtime。应用数据仍在 AppData，与安装版共享。解压替换用于便携版更新；应用内 updater 仍使用 NSIS 并转为安装版。

新增独立 Windows CI：PR、main 推送和手动运行无需私钥即可检查并构建两种产物；校验 SHA256、解压内容和 EXE 与构建产物的一致性，再上传 CI artifact。Release Windows job 同时上传 NSIS、便携 ZIP 和安装器更新签名；聚合阶段检查 ZIP 文件完整性与可执行文件，生成包含两种分发格式的 SHA256SUMS，updater manifest 继续指向 NSIS。

实际验证（2026-11-17）：完整 Windows 离线构建通过，Svelte 0 错误/0 警告、前端 15 项、Rust 后端 25 项，fmt/Clippy 通过。已执行 CI 中同一段校验脚本，确认 NSIS 和 ZIP 的 SHA256 正确、ZIP 包含全部四个必要文件、解压后的 EXE 与 release EXE 完全一致。便携版从中文和空格目录启动，通过 UI 连接 COM3，5.1 秒采集 50,780 样本并正常停止。

Windows CI 和 Release workflow 通过 actionlint。Release 聚合脚本以真实便携 ZIP、此前签名测试使用的 Windows NSIS 和 macOS/Linux 占位资产在本地执行成功：两种 Windows 包均进入全局校验列表，updater URL 仍指向 NSIS；删除 ZIP 的负向测试确认发布被阻止。尚未向 GitHub 推送或实际触发 Actions。
