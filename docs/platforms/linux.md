# Linux 安装与构建

## 支持范围

发布目标为 Ubuntu/Debian x86_64。DEB 包基于 Ubuntu 22.04 构建，目标系统为 Debian 11+、Ubuntu 22.04+ 和 Linux Mint 21+；AppImage 面向多数 x86_64 Linux 发行版。RPM 当前不在发布计划内。

请从 [GitHub Releases](https://github.com/No-Chicken/PowerPicoClient_rs/releases/latest) 下载对应的 DEB 或 AppImage，并使用同一 Release 中的 `SHA256SUMS` 校验文件完整性。

安装 DEB：

```bash
sudo apt install ./PowerPico-Client_<版本>_amd64.deb
```

运行 AppImage：

```bash
chmod +x PowerPico-Client_<版本>_amd64.AppImage
./PowerPico-Client_<版本>_amd64.AppImage
```

## 开发环境

Arch Linux x86_64 可用于日常开发和真机调试。安装 Tauri 所需系统依赖：

```bash
sudo pacman -S --needed base-devel webkit2gtk-4.1 libappindicator-gtk3 librsvg systemd
```

加入串口设备访问组后注销并重新登录。Ubuntu/Debian 使用 `dialout`，Arch Linux 使用 `uucp`：

```bash
sudo usermod -aG dialout "$USER"  # Ubuntu / Debian
sudo usermod -aG uucp "$USER"     # Arch Linux
```

安装 Node.js 20+、pnpm 10+ 和 Rust 1.77.2+，然后在项目根目录执行：

```bash
pnpm install --frozen-lockfile
pnpm tauri dev
```

## 构建

本地 DEB 和 AppImage 使用固定的 Ubuntu 22.04 Docker 容器构建。宿主机需要 Docker：

```bash
pnpm tauri:build:linux
```

产物写入 `artifacts/linux/`。GitHub Actions 在 `ubuntu-22.04` runner 上原生构建这两种格式。Linux 真机验收项目见 [Linux 测试清单](../linux-testing.md)。
