# Third-party notices

PowerPico Client itself is distributed under GPL-3.0-only. Its JavaScript and Rust dependencies retain their respective licenses; the authoritative dependency versions are recorded in `pnpm-lock.yaml` and `src-tauri/Cargo.lock`.

Major direct dependencies include Tauri, Svelte, Tailwind CSS, Lucide, uPlot, serde, reqwest, serialport and tracing. Linux packages also depend on system WebKitGTK and related desktop libraries provided by the target distribution.

The QFluentWidgets Pro library used by the legacy Python client is not linked, copied or packaged with this application. The legacy client is used only as a protocol and behavior reference.
