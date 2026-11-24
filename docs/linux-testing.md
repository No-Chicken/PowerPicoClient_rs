# Linux 测试清单

日常开发和真机调试在 Arch Linux x86_64 / KDE Wayland 上进行；CI/CD 只在推送 `vX.Y.Z` tag 时于 Ubuntu 24.04 runner 上执行格式、lint、测试、构建与打包，平时提交前需本地跑下面的检查。

## 自动检查

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
```

## 启动与渲染

- `cargo run -p powerpico-app` 能打开 1180×800 的窗口，侧栏、页面与滚动区域布局正常。
- 终端出现 `radv is not a conformant Vulkan implementation` 属正常提示；GPUI 通过 Vulkan 渲染。
- 切换主题（跟随系统/浅色/深色）后所有面板、按钮、输入框同步换色，无需重启。
- 缩放 100%–200% 后文字随之变化，布局不重叠。
- 关闭窗口后进程退出，串口被释放（`lsof /dev/ttyACM*` 为空）。

## 串口权限

- 未加入 `uucp` 组时连接设备会提示串口权限错误，提示文案与旧版一致。
- `sudo usermod -aG uucp $USER` 并重新登录后可以枚举与连接。

## 采集与波形

```bash
POWERPICO_AUTOSTART=1 cargo run -p powerpico-app
```

- 指标卡片每 33 ms 刷新，采样点计数持续增长。
- 滚轮缩放指针所在轴、左键拖动平移时间轴、右键拖动出现紫色分析区间并在松手后显示统计。
- 点击曲线上任意位置出现读数卡片，Esc 或点击空白处不崩溃；读数时间、电压、电流、瞬时功率与采样点一致。
- 导入旧版 `.bin` 记录、导出 CSV/BIN、清除记录后图表回到空态。

## 校准

- 连接后读取参数，三档增益与偏移显示正确，High 档为 mA。
- 写入前出现确认对话框，取消不会写设备。
- 自动零点校准进度条与 `n / 60,000` 同步，取消可用；完成后参数回读一致。
- 校准期间切到其他页面再回来，状态保持；关闭窗口会等待档位恢复。

## 固件

- 官方固件版本与发布日期从固件服务读取，下载进度条更新，完成后版本写入设置。
- 自定义 BIN 通过 GTK 文件对话框选择。
- 刷写时设备重新枚举，进度条推进，完成后弹出成功提示；取消后设备回到正常模式。

### 无窗口自检

`crates/app/examples/firmware.rs` 复用与界面相同的 core 路径，可在没有点击的情况下验证
设备已进入 Bootloader 的场景：

```bash
cargo run -p powerpico-app --example firmware -- list
cargo run -p powerpico-app --example firmware -- download
cargo run -p powerpico-app --example firmware -- flash ~/.local/share/com.openfeasttech.powerpico-client/PowerPico_Firmware.bin
cargo run -p powerpico-app --example firmware -- probe 3
```

`flash` 会打印 connecting / rebooting / searchingBootloader / handshaking / uploading / finishing
各阶段与百分比；`probe` 采集 3 秒并输出点数、时长与最新电压电流，用来确认新固件已启动。

## 日志

```bash
tail -f ~/.local/share/com.openfeasttech.powerpico-client/logs/powerpico.log.$(date +%F)
```

采集、校准、刷写与网络错误都应留下记录，且不包含采样数据本身。
