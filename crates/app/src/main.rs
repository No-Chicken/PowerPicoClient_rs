//! PowerPico Client, rebuilt on GPUI.
//!
//! The Tauri + Svelte client put the device work in Rust and the interface in
//! the webview. Here both halves are Rust: `powerpico-core` owns the device,
//! capture, calibration and firmware threads, and this crate draws the same
//! four screens with GPUI, GPUI Kit and Ely's components.

mod app;
mod i18n;
mod model;
mod pages;
mod theme;

use gpui::{px, size, AppContext, Bounds, TitlebarOptions, WindowBounds, WindowOptions};
use gpui_kit::App;

use crate::app::PowerPico;

fn main() {
    let paths = match powerpico_core::AppPaths::resolve() {
        Ok(paths) => paths,
        Err(error) => {
            eprintln!("PowerPico Client cannot start: {error}");
            std::process::exit(1);
        }
    };
    if let Err(error) = paths.prepare() {
        eprintln!("PowerPico Client cannot prepare its directories: {error}");
        std::process::exit(1);
    }
    powerpico_core::init_logging(&paths);

    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: gpui::point(px(0.0), px(0.0)),
            size: size(px(1180.0), px(800.0)),
        })),
        titlebar: Some(TitlebarOptions {
            title: Some("PowerPico Client".into()),
            appears_transparent: false,
            traffic_light_position: None,
        }),
        ..Default::default()
    };

    gpui_kit::application()
        .with_assets(ely_gpui_component::Assets::before(gpui_kit::assets::Assets))
        .run(move |cx: &mut App| {
            gpui_kit::init(cx);
            ely_gpui_component::init(cx).expect("Ely failed to start");
            theme::install(cx);
            let paths = paths.clone();
            gpui_kit::open_window(options, cx, move |window, cx| {
                cx.new(|cx| PowerPico::new(paths.clone(), window, cx))
            })
            .expect("failed to open the PowerPico window");
        });
}
