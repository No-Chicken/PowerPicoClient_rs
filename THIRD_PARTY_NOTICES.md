# Third-party notices

PowerPico Client itself is distributed under GPL-3.0-only. Its Rust dependencies retain their respective licenses; the authoritative dependency versions are recorded in `Cargo.lock`.

Major direct dependencies include GPUI and GPUI Kit (Apache-2.0, built on Zed Industries' GPUI), Ely GPUI Component (MIT), Lucide icons (ISC), Inter and JetBrains Mono fonts (SIL OFL 1.1), serde, reqwest, serialport, rfd, open and tracing. Linux builds also link system libraries provided by the distribution: Wayland, libxkbcommon, fontconfig, GTK3 and a Vulkan driver.

The QFluentWidgets Pro library used by the legacy Python client is not linked, copied or packaged with this application. The legacy client is used only as a protocol and behavior reference.
