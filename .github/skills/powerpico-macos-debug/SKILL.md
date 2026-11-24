---
name: powerpico-macos-debug
description: Debug and verify PowerPico Client on macOS, including GPUI development startup, stray-process isolation, serial-device discovery, STM32 Loader and YMODEM firmware flashing, capture probing, update-check behaviour, UI automation, and cleanup. Use when reproducing macOS-only failures, running the GPUI app from source, testing a connected PowerPico or STM32 Virtual ComPort, diagnosing Loader/firmware upgrade errors, or preparing a macOS release fix.
---

# PowerPico macOS Debug

Use this workflow from the repository root. Preserve unrelated user changes and keep protocol diagnosis evidence-based.

The client is a pure Rust GPUI app: `crates/core` owns device, capture, calibration and firmware threads,
`crates/app` draws the window. There is no Tauri, no Node.js and no webview — any instruction that mentions
`pnpm`, `src-tauri` or Vite belongs to the archived `tauri2-archive` branch, not to current `main`.

## 1. Establish a clean debugging baseline

1. Run `git status --short --branch` and inspect existing changes before editing.
2. Inspect the relevant tag or prior fix instead of assuming a regression:
   - `git log --oneline --decorate --graph --all -40`
   - `git diff <known-good-tag>..HEAD -- <relevant-files>`
3. Confirm the connected serial device:
   - `ls -l /dev/cu.usbmodem* /dev/tty.usbmodem*`
   - Prefer `/dev/cu.*` in the client UI on macOS.
4. Check for stale processes before starting:
   - `ps ax -o pid=,ppid=,lstart=,command= | rg 'powerpico-client|cargo run'`
   - For ambiguous PIDs, run `lsof -nP -p <pid> | rg 'cwd|TCP'` and compare the working directory and parent PID.
5. Terminate only resolved stale PIDs. Never use broad process-kill patterns when an exact PID is available.

## 2. Start the GPUI build

```bash
cargo run -p powerpico-app
```

`rust-toolchain.toml` pins rustc `1.98.0`; the first `cargo` command downloads that toolchain plus `rustfmt`
and `clippy`, so a cold start is slow once and fast afterwards.

Useful launch switches (see `docs/build.md`):

| Variable | Effect |
| :--- | :--- |
| `POWERPICO_PAGE=waveform\|calibration\|firmware\|settings` | Open one page directly. |
| `POWERPICO_AUTOSTART=1` | Start capture as soon as a device is listed. |
| `POWERPICO_THEME=light\|dark\|system` | Force a palette for this run without writing settings. |

The binary is `target/debug/powerpico-client`. There is no `.app` bundle and no DMG yet, so `cargo run` is the
app under test; do not look for `src-tauri/target/debug/bundle`.

Two process-identity traps remain:

1. An older **Tauri** build may still be installed at `/Applications/PowerPico Client.app`. It shares the
   bundle identifier `com.openfeasttech.powerpico-client` and therefore the same data directory. Close it
   before testing, or its settings and firmware files will look like debug results.
2. `cargo run` and a directly launched `target/debug/powerpico-client` are the same app with the same name.
   Confirm which PID owns the window before trusting a UI observation.

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

The port lives in `crates/core/src/firmware.rs`, `ymodem.rs` and `protocol.rs`. When the Rust implementation
looks suspect, compare against the legacy Python client (`power_pico_pyqt`, ignored by Git) as protocol
evidence — not as code to copy blindly.

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

Run the headless path first — it exercises the same core code as the UI and prints every stage:

```bash
cargo run -p powerpico-app --example firmware -- list
cargo run -p powerpico-app --example firmware -- download
cargo run -p powerpico-app --example firmware -- flash "$HOME/Library/Application Support/com.openfeasttech.powerpico-client/PowerPico_Firmware.bin"
cargo run -p powerpico-app --example firmware -- probe 3
```

Then confirm the complete UI sequence:

1. Device is visible as STM32 Virtual ComPort.
2. Select the `/dev/cu.usbmodem...` device.
3. Ensure the firmware is either downloaded by the client's official-firmware flow or explicitly supplied by the user as a compatible PowerPico `.bin`. Confirm it is non-empty; verify the published checksum when one is available. Never substitute an unrelated image merely to exercise the UI.
4. Start flashing.
5. Confirm progress passes Loader entry and YMODEM handshake.
6. Wait for the localized success message (`固件升级成功 100%` or its translation).
7. The protocol waits about 500 ms after transfer, sends ASCII `3`, and expects the device to leave Loader mode. Allow the existing rediscovery window (about 8 seconds) for the application port to return.
8. Reconnect at 1,500,000 baud and confirm valid capture frames arrive — `probe 3` prints the point count, duration and the latest voltage/current.

If a new error replaces the original one, identify its exact stage before editing again. Add bounded context to errors or capture raw bytes rather than guessing.

## 7. Update check behaviour

The in-app updater was removed with Tauri. "Check now" in Settings only compares `CARGO_PKG_VERSION` with the
GitHub Releases redirect and then opens the Releases page; there is no `latest.json`, signature or download step.

Before suspecting the client, inspect the redirect chain directly:

```bash
curl -L -i --connect-timeout 15 --max-time 30 https://github.com/No-Chicken/PowerPicoClient_rs/releases/latest
gh release view <tag> --repo No-Chicken/PowerPicoClient_rs --json tagName,assets,url,publishedAt
```

A `404` or a missing redirect means the release itself is wrong, not the version parsing in
`crates/core/src/network.rs`.

## 8. Run validation in proportion to the change

Always:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

For a window or layout regression, `cargo run -p powerpico-app --example minimal` renders a single Ely button
and isolates GPUI/Ely wiring from PowerPico's own pages.

Before handoff, run `git diff --check` and inspect `git status --short --branch`.

For fragile firmware or macOS application-lifecycle changes, unit tests are necessary but not sufficient.
Complete the relevant real UI or real-device verification.

macOS-only limitations to state rather than debug:

- Uninstall requires a packaged `.app` bundle; running from `cargo run` reports
  `uninstall is only available from a packaged macOS app`.
- DMG packaging, signing and notarization are not part of the release workflow yet, so a macOS release build is
  produced with `cargo build --release -p powerpico-app` and shipped manually.

## 9. Clean up after debugging

1. Close the app under test.
2. Stop the active `cargo run` session with Ctrl-C.
3. Confirm no stale `powerpico-client` or `cargo run` process remains.
4. Restore serial settings if direct diagnostics changed them.
5. Leave build output under `target`; do not add it to Git.
6. Preserve unrelated working-tree changes.
7. Commit only when the user explicitly requests a commit.

Report the root cause, code changes, test results, real-device/UI result, current branch, and whether changes are committed.
