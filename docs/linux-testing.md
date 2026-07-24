# Linux 测试清单

首版发布目标为 Ubuntu/Debian x86_64。日常开发和真机调试在 Arch Linux x86_64 上执行，DEB/AppImage 只通过 Ubuntu 22.04 容器生成。

## 自动检查

```bash
pnpm install --frozen-lockfile
pnpm check
pnpm test
pnpm build
pnpm tauri dev
cd src-tauri
cargo fmt --check
cargo clippy --locked -- -D warnings
cargo test --locked
```

## 安装包与桌面集成

1. 在安装 Docker 的宿主机执行 `pnpm tauri:build:linux`，确认 `artifacts/linux/` 中生成 DEB 和 AppImage。
2. 确认容器构建没有复用宿主机的 `node_modules` 或 `src-tauri/target`。
3. 在 Ubuntu 22.04/24.04 启动 AppImage 并安装 DEB，检查浅色、深色和跟随系统主题。
4. 检查固件文件、记录导入和导出对话框。
5. 检查设置、记录和日志分别写入系统应用数据、缓存和日志目录。
6. 关闭应用并确认没有残留 PowerPico Client 进程。

## PowerPico 真机

1. 在未加入串口用户组的测试账号上尝试采集，确认界面显示可操作的权限提示。
2. 加入 `dialout`（Ubuntu/Debian）或 `uucp`（Arch）并重新登录，确认设备能被发现。
3. 以 1.5 Mbps 连续采集至少 15 分钟，检查实时波形、统计、停止、导出和重新导入。
4. 采集过程中拔出设备，确认应用进入错误状态且可以重新刷新并连接设备。
5. 连接其他 USB 串口设备后执行固件升级，确认 bootloader 重枚举不会误选无关设备。
6. 完成 YMODEM 上传并启动新固件，再次连接并采集数据。

记录测试发行版、桌面环境、会话类型（Wayland/X11）、PowerPico VID/PID、固件版本和每项结果。Ubuntu 22.04 或 24.04 的安装、启动和真机流程全部通过后，Linux 包才可作为发布候选。
