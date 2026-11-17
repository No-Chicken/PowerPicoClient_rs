# 校准功能与本机测试

测试日期：2026-11-17。环境：本机 Arch Linux / KDE Wayland，实际 PowerPico 串口 `/dev/ttyACM1`，USB 标识 `0483:5740`，序列号 `398839533132`。测试时已断开负载，保持零电流和稳定供电。

## 功能

从左侧“校准”进入，选择设备并连接。读取参数后，可以分别修改 Low / Mid / High 档的增益和零点偏移，或恢复出厂校准。手动写入前显示原值、新值和单位；High 档显示 mA，其余档显示 µA。所有写入都会检查应答，并再次读取设备参数核对。

自动零点校准保留三档原增益，每档切换后先丢弃 0.5 秒数据，再采集 20,000 个原始 ADC 样本。计算 20 个各含 1,000 点的块均值，以其中位数作为零点。参考 ADC 必须处于 1998–2098、电流 ADC 处于 16–4079，块均值标准差不超过 5 LSB，自动零点不超过 ±32 LSB。三档全部通过后一次性写入，并恢复原档位模式。手动增益限制为 0.85–1.15，偏移限制为 ±48 LSB。

校准期间可以切换页面，重新进入会恢复运行状态。采样阶段允许取消；提交后等待写入、核对和档位恢复。关闭窗口会结束采样并等待原档位恢复。写入之后发生通信异常时，提示参数可能已保存，要求重新连接和读取。

校准连接占用串口，采集和固件升级会返回 `DEVICE_BUSY`；断开校准后可继续采集。波形开始采集时读取设备参数，并按 `I = ΔADC × scale × k + b − dividerCurrent` 换算电流。旧固件不支持读取参数时使用标称值，并记录日志。

本次校准真机验收在 Linux 完成；macOS 和 Windows 的校准操作尚待各平台真机验收。

## 自动检查

```sh
pnpm check
pnpm test
pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
cargo build --manifest-path src-tauri/Cargo.toml
```

结果：Svelte / TypeScript 零错误、零警告；前端 22 项测试通过；Rust 37 项常规测试通过；前端生产构建、Rust 编译、格式检查和 Clippy 通过。真实硬件测试默认忽略，已单独显式运行通过。

覆盖协议参考字节和 CRC、任意分片及混合 ADC/控制流恢复、数值边界、异常噪声、错误参数、固定序列重试、取消不写入、失败恢复原档位、提交后不确定状态、仅档位恢复失败时保留已核对的参数，以及三档波形换算。界面测试覆盖读取前禁用编辑、High 单位换算、确认后写入、通信失败禁用编辑和提交后禁用取消。

## 真实设备验收

| 检查 | 结果 |
| --- | --- |
| 读取当前参数并在写入前备份 | 通过 |
| 手动修改 Low 偏移，并保留其他档参数 | 通过 |
| 恢复出厂值，再恢复备份 | 通过 |
| 采样中取消，参数未写入，原档位恢复 | 通过 |
| 三档各采集 20,000 点，保留增益，保存后再次读取核对 | 通过 |
| 桌面界面确认、进度、结果表格、切换页面继续运行 | 通过 |
| 校准连接期间阻止波形采集和固件升级 | 通过 |
| 断开校准后真实采集波形 | 通过，约 2.41 秒、24,110 点 |
| 原生窗口在 `range:1` 采样时关闭 | 通过，进程退出；独立 Python 协议客户端确认参数未变、档位模式恢复为 0 |

一次完整桌面自动校准的采样结果：

| 档位 | 点数 | 零点 / LSB | 块均值标准差 / LSB |
| --- | ---: | ---: | ---: |
| Low | 20,000 | 29.7715 | 0.128823 |
| Mid | 20,000 | 2.6150 | 0.101963 |
| High | 20,000 | 3.9970 | 0.120956 |

测试结束后，设备保留最后一次成功自动校准的结果，三档增益均为 1，偏移依次为 `-8.719482421875 µA`、`-75.1171875 µA`、`-11444.82421875 µA`；档位模式为自动（0）。关闭窗口测试已确认没有覆盖这些参数。

本地原始备份、结果 JSON 和截图保存在 `artifacts/calibration/`（该目录不纳入 Git）：

- `hardware-2026-11-17.json`：首次写入前的原参数、原档位、Rust 硬件测试结果。
- `desktop-2026-11-17.json`：桌面完整自动校准和真实采集结果。
- `desktop-close-2026-11-17.json`：关闭窗口前后参数及档位复核。
- `desktop-2026-11-17.png`、`desktop-read-2026-11-17.png`：桌面校准界面截图。

## 再次运行硬件测试

以下测试会临时修改参数、验证出厂恢复、恢复备份，最后保存新的自动零点校准。仅在已确认断开负载、设备处于零电流状态时运行。使用新的报告文件名保留历次备份。

```sh
mkdir -p artifacts/calibration
POWERPICO_CALIBRATION_PORT=/dev/serial/by-id/usb-STMicroelectronics_STM32_Virtual_ComPort_398839533132-if00 \
POWERPICO_CALIBRATION_REPORT="$PWD/artifacts/calibration/hardware-new.json" \
cargo test --manifest-path src-tauri/Cargo.toml hardware_calibration_smoke -- --ignored --nocapture
```

本机原先的前端依赖来自其他平台，已按既有锁文件执行 `pnpm install --force --frozen-lockfile` 恢复 Linux 依赖，未修改依赖版本或锁文件。桌面验收使用 `LANG=zh_CN.UTF-8 LC_ALL=zh_CN.UTF-8`，因为当前 `C.UTF-8` 会让本机 WebKit 返回不合法的 `C` 语言标签，导致现有 uPlot 模块启动时报错。Wayland 验收同时设置 `WEBKIT_DISABLE_DMABUF_RENDERER=1`。这些设置仅用于本机测试进程。
