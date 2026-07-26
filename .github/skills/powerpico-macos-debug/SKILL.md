---
name: powerpico-macos-debug
description: Debug and verify PowerPico Client on macOS, including Tauri/Svelte/Rust development startup, duplicate installed/debug app isolation, serial-device discovery, STM32 Loader and YMODEM firmware flashing, updater checks, UI automation, and cleanup. Use when reproducing macOS-only failures, running pnpm tauri dev, testing a connected PowerPico or STM32 Virtual ComPort, diagnosing Loader/firmware upgrade errors, validating the settings updater, or preparing a macOS release fix.
---

# PowerPico macOS Debug

Use this workflow from the repository root. Preserve unrelated user changes and keep protocol diagnosis evidence-based.

## 1. Establish a clean debugging baseline

1. Run `git status --short --branch` and inspect existing changes before editing.
2. Inspect the relevant tag or prior fix instead of assuming a regression:
   - `git log --oneline --decorate --graph --all -40`
   - `git diff <known-good-tag>..HEAD -- <relevant-files>`
3. Confirm the connected serial device:
   - `ls -l /dev/cu.usbmodem* /dev/tty.usbmodem*`
   - Prefer `/dev/cu.*` in the client UI on macOS.
4. Check for stale development processes before starting:
   - `lsof -nP -iTCP:1420 -sTCP:LISTEN`
   - `ps ax -o pid=,ppid=,lstart=,command= | rg 'PowerPico Client|target/debug/powerpico|tauri dev|vite'`
   - For ambiguous PIDs, run `lsof -nP -p <pid> | rg 'cwd|TCP'` and compare the working directory and parent PID.
5. Terminate only resolved stale PIDs. Never use broad process-kill patterns when an exact PID is available.

## 2. Start Tauri development without mixing app instances

Run:

```bash
pnpm tauri dev
```

If port 1420 is occupied, identify and stop only the stale Vite process, then restart.

The installed app, an orphaned debug process, and the current `tauri dev` process can all share the same name and bundle identifier. Before trusting a UI result:

1. Inspect running commands and distinguish:
   - `/Applications/PowerPico Client.app/...`
   - orphaned `target/debug/powerpico-client`
   - the current `target/debug/powerpico-client` whose parent is the active Tauri command.
2. Close the installed app and orphaned debug processes when they would interfere.
3. Do not assume Computer Use targeting `PowerPico Client` selected the current dev process. It can silently launch the installed app.

Use `tauri dev` for reproduction, terminal logs, and hot reload. For reliable GUI automation, build a uniquely named debug app from the same source:

```bash
pnpm tauri build --debug --bundles app --config '{"productName":"PowerPico Client Dev","identifier":"com.openfeasttech.powerpico-client.dev","bundle":{"createUpdaterArtifacts":false},"app":{"windows":[{"title":"PowerPico Client Dev","width":1180,"height":800,"minWidth":820,"minHeight":620,"resizable":true,"fullscreen":false}]}}'
```

Target this exact path with Computer Use:

```text
src-tauri/target/debug/bundle/macos/PowerPico Client Dev.app
```

Start or attach by calling Computer Use `get_app_state` with the full `.app` path. After it opens, target `PowerPico Client Dev` and confirm both the window title and accessibility app name before clicking anything.

The temporary identifier intentionally differs from production. Maintenance or uninstall checks may report `application bundle identifier mismatch`; treat that as a debug-build limitation, not the issue under test. Do not use the unique bundle to validate production-identifier-dependent uninstall behavior.

## 3. Operate the UI reliably

When GUI interaction is required, use the Computer Use skill and follow its fresh-state rule:

1. Get the current accessibility tree.
2. Perform the action using the latest element index.
3. Get the state again before the next action.
4. Use a screenshot when toast text, native select menus, or scroll position is not clear from accessibility text.
5. Re-query after rebuilds, relaunches, or messages that the app changed. Never reuse stale element indexes.

For native select controls, clicking an item may leave the menu open. Press Return and then re-query to confirm the selected value.

## 4. Diagnose Loader state before changing protocol code

Recognize these states from serial output:

| Output | State | Required action |
| --- | --- | --- |
| Repeated `##PICO_BOOT##\r\n` | Loader prompt, not the main menu | Send ASCII `0`, wait about 200 ms, then send ASCII `1` |
| Text containing `Main Menu` | Loader main menu | Send ASCII `1` |
| Repeated YMODEM CRC request `C` bytes without menu text | YMODEM receiver already active | Do not inject `0` or `1`; begin the sender handshake |

Important protocol rules:

- Application mode uses 1,500,000 baud. Loader menus and YMODEM use 115,200 baud.
- A device already in Loader mode must not be treated as an application-mode device.
- Do not treat the `C` in `Erase Complete` as the YMODEM CRC request.
- Require two consecutive `0x43` CRC requests or an explicit Loader state transition before sending the header. Allow up to 60 seconds for the initial erase and handshake; use approximately 10 seconds for per-packet ACK/CRC waits.
- Do not reject the initial handshake merely because Loader progress or prompt text exceeds a byte-count threshold. Use the protocol condition plus a bounded timeout.
- Avoid native input-buffer clearing around Loader state transitions on macOS USB CDC. `tcflush`/`ClearBuffer::Input` can return `EINVAL`, and the YMODEM wait loop can safely consume preceding menu or erase text.
- Do not probe the menu between periodic CRC requests. An injected byte can cause the receiver to discard the following YMODEM header.

Compare behavior with the legacy implementation when the Rust port is suspect:

```text
../app/component/flash_worker.py
../app/component/ymodem_sender.py
```

Treat the legacy client as protocol evidence, not code to copy blindly.

## 5. Capture serial evidence without contaminating later tests

Prefer bounded logging inside the Rust code or a serial library that restores configuration on close.

If direct `termios` or `stty` access is unavoidable:

1. Close every app holding the port.
2. Save the exact original settings with `stty -f <port> -g`.
3. Use a bounded read timeout and capture raw bytes as both hex and escaped text.
4. Restore the saved settings immediately, even when the read fails.
5. If a diagnostic changed the port and Rust later reports `serial error: Invalid argument`, restore a standard baseline before retrying:

```bash
stty -f /dev/cu.usbmodemXXXXXXXX sane 115200 cs8 -parenb -cstopb -ixon -ixoff clocal cread
```

Do not infer a code defect from `EINVAL` until the diagnostic serial settings have been restored.

## 6. Verify firmware fixes on the real device

Do not stop after the original error disappears. Verify the complete sequence:

1. Device is visible as STM32 Virtual ComPort.
2. Select the `/dev/cu.usbmodem...` device.
3. Ensure the firmware is either downloaded by the client's official-firmware flow or explicitly supplied by the user as a compatible PowerPico `.bin`. Confirm it is non-empty; verify the published checksum when one is available. Never substitute an unrelated image merely to exercise the UI.
4. Start flashing.
5. Confirm progress passes Loader entry and YMODEM handshake.
6. Wait for `固件升级成功 100%` or the equivalent localized success message.
7. The current protocol waits about 500 ms after transfer, sends ASCII `3`, and expects the device to leave Loader mode. Allow up to the existing rediscovery window (about 8 seconds) for the application port to return.
8. Reconnect at 1,500,000 baud and confirm valid capture frames arrive when capture verification is in scope.

If a new error replaces the original one, identify its exact stage before editing again. Add bounded context to errors or capture raw bytes rather than guessing.

## 7. Diagnose update-check failures

Check the configured endpoint in `src-tauri/tauri.conf.json` outside the UI first:

```bash
curl -L -i --connect-timeout 15 --max-time 30 https://github.com/No-Chicken/PowerPicoClient_rs/releases/latest/download/latest.json
```

Interpret the complete redirect chain. A redirect to a release followed by `404 Not Found` means the latest Release lacks `latest.json`; it is not a frontend JSON parser regression.

Confirm assets and workflow history with:

```bash
gh release view <tag> --repo No-Chicken/PowerPicoClient_rs --json tagName,assets,url,publishedAt
gh run list --repo No-Chicken/PowerPicoClient_rs --workflow release.yml --limit 10
```

Before changing updater code, compare the release tag with current workflow/config. Older tags may predate updater artifacts even when current main generates:

- signed `.app.tar.gz`
- `.app.tar.gz.sig`
- `latest.json`
- `SHA256SUMS`

Handle a known missing manifest with an accurate localized informational message. Do not hide unrelated network, signature, or installation failures.

## 8. Run validation in proportion to the change

For Rust or protocol changes, run:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
```

For frontend or updater changes, run:

```bash
pnpm check
pnpm test
pnpm build
```

Before handoff, run `git diff --check` and inspect `git status --short --branch`.

For fragile firmware, updater, or macOS application-lifecycle changes, unit tests are necessary but not sufficient. Complete the relevant real UI or real-device verification.

## 9. Clean up after debugging

1. Close the uniquely named debug app.
2. Stop the active `tauri dev` session with Ctrl-C.
3. Confirm no stale Vite, installed app, or orphaned debug process remains.
4. Restore serial settings if direct diagnostics changed them.
5. Leave generated debug bundles under `target`; do not add them to Git.
6. Preserve unrelated working-tree changes.
7. Commit only when the user explicitly requests a commit.

Report the root cause, code changes, test results, real-device/UI result, current branch, and whether changes are committed.
