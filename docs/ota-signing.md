# macOS OTA 签名密钥配置

PowerPico Client 使用 Tauri updater 对 macOS OTA 更新包签名。发布工作流从 GitHub Actions 仓库级 Secrets 读取私钥，客户端使用仓库中配置的公钥验证更新包。

## 密钥与配置位置

本地文件：

- `.tauri-signing-private.key`：私钥，只能保存在受控设备和 GitHub Actions Secrets 中。
- `.tauri-signing-private.key.pub`：公钥，可以公开。

两个文件均已加入 `.gitignore`。私钥不得提交到 Git、Release、Issue、日志或聊天记录。

客户端公钥位于 `src-tauri/tauri.conf.json` 的 `plugins.updater.pubkey`。发布工作流位于 `.github/workflows/release.yml`，使用以下 Secrets：

| Secret | 是否必需 | 内容 |
| --- | --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | 必需 | `.tauri-signing-private.key` 的完整原始内容 |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 仅加密私钥需要 | 生成私钥时设置的密码；无密码时可以不创建 |

当前工作流没有声明 GitHub Environment，因此必须使用仓库级 Actions Secrets，不要创建 environment secret 或 Actions variable。

## 方法一：GitHub 网页配置

1. 打开仓库 `No-Chicken/PowerPicoClient_rs`。
2. 进入 **Settings → Secrets and variables → Actions**。
3. 选择 **Secrets** 标签页，点击 **New repository secret**。
4. 创建 `TAURI_SIGNING_PRIVATE_KEY`：
   - Name：`TAURI_SIGNING_PRIVATE_KEY`
   - Secret：复制 `.tauri-signing-private.key` 的完整内容，包括开头的注释行和末尾内容，不要只复制其中一行。
5. 如果私钥设置了密码，再创建 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`，值为原始密码。
6. 保存后只能看到 Secret 名称和更新时间，GitHub 不会再次显示值。

macOS 可使用以下命令将私钥完整复制到剪贴板；执行后不要粘贴到除 GitHub Secret 输入框以外的位置：

```bash
pbcopy < .tauri-signing-private.key
```

## 方法二：GitHub CLI 配置

先确认 `gh auth status` 显示已登录到有仓库管理权限的账号。

从文件标准输入设置私钥，避免密钥进入 shell 历史：

```bash
gh secret set TAURI_SIGNING_PRIVATE_KEY \
  --repo No-Chicken/PowerPicoClient_rs \
  --app actions \
  < .tauri-signing-private.key
```

如果私钥带密码，使用交互式输入设置密码。不要把密码写在 `--body` 参数中：

```bash
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD \
  --repo No-Chicken/PowerPicoClient_rs \
  --app actions
```

确认 Secret 名称已经存在：

```bash
gh secret list --repo No-Chicken/PowerPicoClient_rs --app actions
```

该命令只显示名称和更新时间，不会读取 Secret 内容。

## 检查公钥是否匹配

`.pub` 文件是公开信息，并且 Tauri 生成的文件内容本身已经是配置所需的 Base64 文本。去除文件末尾换行后的内容应与 `src-tauri/tauri.conf.json` 中的 `plugins.updater.pubkey` 完全一致：

```bash
tr -d '\r\n' < .tauri-signing-private.key.pub
```

不要再次对 `.pub` 文件运行 Base64 编码，否则会得到错误的双重编码值。

如果不一致，停止发布。不要直接替换客户端公钥后继续发布，因为已经安装的旧客户端只信任旧公钥。

## 发布前验证

1. 确认版本一致：`package.json`、`src-tauri/tauri.conf.json` 和 `src-tauri/Cargo.toml` 必须与 `vX.Y.Z` tag 一致。
2. 确认 `.github/release-notes/vX.Y.Z.md` 存在且非空。
3. 确认 Actions Secrets 名称存在。
4. 推送 tag，或在 Actions 页面手动运行 Release workflow 并选择已有 tag。
5. 在 `Build macOS ARM64` job 中确认以下步骤成功：
   - `Require updater signing key`
   - `Sign macOS updater archive`
6. 发布完成后确认 Release 包含：
   - `PowerPico-Client_X.Y.Z_macos_aarch64.app.tar.gz`
   - `PowerPico-Client_X.Y.Z_macos_aarch64.app.tar.gz.sig`
   - `latest.json`
   - `SHA256SUMS`
7. 检查更新清单：

```bash
curl -L -f -sS \
  https://github.com/No-Chicken/PowerPicoClient_rs/releases/latest/download/latest.json
```

`latest.json` 必须包含当前版本，以及非空的 `platforms.darwin-aarch64.signature` 和正确的 OTA 包 URL。

## 常见错误

### `Missing TAURI_SIGNING_PRIVATE_KEY repository secret`

Secret 未创建、名称拼写错误，或错误地创建成了 Actions variable / environment secret。按本文创建仓库级 Actions Secret。

### `A public key has been found, but no private key`

构建环境加载到了 updater 公钥，但没有获得私钥。检查 Secret 是否存在，以及 workflow 是否仍将 `${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}` 映射到同名环境变量。

### 签名密码错误

重新设置 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。不要重新生成密钥来绕过密码错误。

### Release 没有 `latest.json`

检查 macOS 构建、签名和 `Prepare release assets` job。旧 Release 可能创建于 OTA 工作流加入之前；这不代表当前前端 JSON 解析失败。

## 备份与轮换

- 将私钥和密码分别保存在受控的离线密码库或加密备份中。
- 不要只依赖 GitHub Secret；GitHub 无法导出已保存的 Secret。
- 丢失当前私钥后，无法为已安装客户端生成其信任的更新签名。
- 不要直接用新公钥替换 `tauri.conf.json` 后发布。正确轮换需要先用旧密钥发布一个同时信任新公钥的过渡版本，再切换签名密钥。
- 怀疑私钥泄露时立即停止发布，限制仓库与 Actions 权限，并制定兼容旧客户端的密钥迁移方案。
