//! PowerPico's own palette, laid over Ely's tokens.
//!
//! The values are the ones the Tauri + Svelte client shipped: a pale blue-gray
//! ground, white translucent panels, one blue accent, and voltage/current/power
//! hues for the chart. Ely reads semantic tokens, so overriding them is enough
//! for every component to follow.

use ely_gpui_component::theme::{Mode, Palette, Theme};
use gpui::{rgb, rgba, App, Hsla};

fn color(hex: u32) -> Hsla {
    rgb(hex).into()
}

/// Light palette, matching `styles.css` `:root`.
pub fn light() -> Palette {
    let mut palette = Palette::light(false);
    palette.bg = color(0xf4f7fb);
    palette.surface = rgba(0xffffffe0).into();
    palette.sunken = color(0xeef3f9);
    palette.overlay = color(0xffffff);
    palette.hover = color(0xe9eff8);
    palette.active = color(0xdde6f4);
    palette.border = rgba(0x1e2f4b1a).into();
    palette.border_strong = rgba(0x1e2f4b33).into();
    palette.fg = color(0x172033);
    palette.fg_muted = color(0x6d788b);
    palette.fg_subtle = color(0x8b95a6);
    palette.fg_disabled = rgba(0x17203366).into();
    palette.accent = color(0x246bfd);
    palette.accent_hover = color(0x1b5ae0);
    palette.on_accent = color(0xffffff);
    palette.focus = color(0x246bfd);
    palette.link = color(0x246bfd);
    palette.selection = color(0xe8efff);
    palette.success = color(0x20a66a);
    palette.warning = color(0xd08a12);
    palette.danger = color(0xd6444b);
    palette.info = color(0x2887f0);
    palette.success_subtle = color(0xe6f7ef);
    palette.warning_subtle = color(0xfdf3e2);
    palette.danger_subtle = color(0xfdeced);
    palette.info_subtle = color(0xe8efff);
    palette.shadow = rgba(0x1f314f22).into();
    palette.tooltip_bg = color(0x1d2635);
    palette.tooltip_fg = color(0xf4f7fb);
    palette
}

/// Dark palette, matching `styles.css` `:root.dark`.
pub fn dark() -> Palette {
    let mut palette = Palette::dark(false);
    palette.bg = color(0x10141c);
    palette.surface = rgba(0x191f2be6).into();
    palette.sunken = color(0x212938);
    palette.overlay = color(0x191f2b);
    palette.hover = color(0x263042);
    palette.active = color(0x2e3a4f);
    palette.border = rgba(0xdae5ff1a).into();
    palette.border_strong = rgba(0xdae5ff33).into();
    palette.fg = color(0xedf3ff);
    palette.fg_muted = color(0x98a5ba);
    palette.fg_subtle = color(0x7d8ba1);
    palette.fg_disabled = rgba(0xedf3ff66).into();
    palette.accent = color(0x6d9cff);
    palette.accent_hover = color(0x8bb1ff);
    palette.on_accent = color(0x0c1524);
    palette.focus = color(0x6d9cff);
    palette.link = color(0x6d9cff);
    palette.selection = color(0x213252);
    palette.success = color(0x45c98b);
    palette.warning = color(0xe6ad4a);
    palette.danger = color(0xff6b72);
    palette.info = color(0x52a7ff);
    palette.success_subtle = color(0x172a22);
    palette.warning_subtle = color(0x2c2416);
    palette.danger_subtle = color(0x2e1a1d);
    palette.info_subtle = color(0x213252);
    palette.shadow = rgba(0x00000044).into();
    palette.tooltip_bg = color(0x2a3446);
    palette.tooltip_fg = color(0xedf3ff);
    palette
}

/// Voltage, current and power, by mode.
pub fn series(mode: Mode) -> (Hsla, Hsla, Hsla) {
    match mode {
        Mode::Light => (color(0xef565d), color(0x2887f0), color(0x20a66a)),
        Mode::Dark => (color(0xff6b72), color(0x52a7ff), color(0x45c98b)),
    }
}

/// Installs both palettes and the density the client uses.
pub fn install(cx: &mut App) {
    Theme::set_palette(Mode::Light, Some(light()), cx);
    Theme::set_palette(Mode::Dark, Some(dark()), cx);
    Theme::update(cx, |theme| {
        theme.radius_scale = 1.35;
        theme.font_scale = 1.0;
    });
}

/// The corner radius the original panels used.
pub const PANEL_RADIUS: f32 = 18.0;

/// The gap between the sidebar and the page.
pub const SHELL_GAP: f32 = 14.0;

/// Sidebar width.
pub const SIDEBAR_WIDTH: f32 = 190.0;

/// A hairline used for chart grids.
pub fn grid_line(mode: Mode) -> Hsla {
    match mode {
        Mode::Light => rgba(0x8290a521).into(),
        Mode::Dark => rgba(0x8290a52e).into(),
    }
}
