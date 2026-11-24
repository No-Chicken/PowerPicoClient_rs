//! Firmware update: official download or a local BIN, then YMODEM flashing.

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    forms::{Choice, Select},
    motion::ProgressBar,
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize},
};
use gpui::{Context, IntoElement, ParentElement, SharedString, Styled, Window, div, px};
use powerpico_core::FirmwareMode;

use crate::{
    app::{PowerPico, field, page_header, panel},
    model::{Command, Tone},
};

pub fn page(
    app: &mut PowerPico,
    _window: &mut Window,
    cx: &mut Context<PowerPico>,
) -> impl IntoElement {
    let colors = cx.theme().colors.clone();
    let this = cx.entity().downgrade();

    let header = page_header(
        app.tr("firmware.title"),
        app.tr("firmware.subtitle"),
        &colors,
    );

    let official_version = app
        .firmware
        .official
        .as_ref()
        .map(|info| info.version.clone())
        .unwrap_or_else(|| "—".into());
    let official_date = app
        .firmware
        .official
        .as_ref()
        .map(|info| info.release_date.clone())
        .unwrap_or_else(|| "—".into());

    let hero = panel(&colors)
        .flex()
        .items_center()
        .gap(px(17.0))
        .p(px(22.0))
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .size(px(58.0))
                .rounded(px(17.0))
                .bg(colors.info_subtle)
                .text_color(colors.accent)
                .child(Icon::new(IconName::Rocket).size(IconSize::Xl)),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .gap(px(6.0))
                .child(
                    div()
                        .text_size(px(18.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .child("PowerPico Firmware"),
                )
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(colors.fg_muted)
                        .child(app.tr("firmware.warning")),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .items_end()
                .gap(px(4.0))
                .child(
                    div()
                        .text_size(px(14.0))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(official_version),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(colors.fg_muted)
                        .child(official_date),
                ),
        );

    // ------------------------------------------------------------------ form
    let busy = app.firmware.busy();
    let official_mode = app.settings.firmware_mode == FirmwareMode::Official;

    let source_select = Select::new(
        "firmware-source",
        vec![
            Choice::new("official", app.tr("firmware.official")),
            Choice::new("custom", app.tr("firmware.custom")),
        ],
    )
    .selected(if official_mode { "official" } else { "custom" })
    .size(ControlSize::Sm)
    .disabled(busy)
    .on_change({
        let this = this.clone();
        move |value, _, cx| {
            let mode = if value.as_ref() == "official" {
                FirmwareMode::Official
            } else {
                FirmwareMode::Custom
            };
            this.update(cx, |app, cx| {
                let mut settings = app.settings.clone();
                settings.firmware_mode = mode;
                app.save_settings(settings);
                cx.notify();
            })
            .ok();
        }
    });

    let mut form = panel(&colors)
        .flex()
        .flex_col()
        .gap(px(20.0))
        .p(px(24.0))
        .child(field(
            app.tr("firmware.source"),
            div().w(px(220.0)).child(source_select),
            &colors,
        ));

    if official_mode {
        let latest = app
            .firmware
            .official
            .as_ref()
            .map(|info| info.version.clone())
            .unwrap_or_else(|| "—".into());
        let date = app
            .firmware
            .official
            .as_ref()
            .map(|info| info.release_date.clone())
            .unwrap_or_else(|| "—".into());
        let release_notes = app
            .settings_ui
            .links
            .as_ref()
            .map(|links| links.firmware_release_notes.clone());
        let loaded = app.firmware.official.is_some();
        form = form.child(
            div()
                .flex()
                .items_center()
                .gap(px(9.0))
                .p(px(15.0))
                .rounded(px(12.0))
                .bg(colors.sunken)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .gap(px(4.0))
                        .child(
                            div()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child(format!("{}: {latest}", app.tr("firmware.latest"))),
                        )
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(colors.fg_muted)
                                .child(format!("{}: {date}", app.tr("firmware.releaseDate"))),
                        ),
                )
                .child(
                    Button::new("firmware-refresh", app.tr("common.refresh"))
                        .size(ControlSize::Sm)
                        .icon(IconName::RefreshCw)
                        .disabled(busy || app.firmware.loading_official)
                        .on_click({
                            let this = this.clone();
                            move |_, _, cx| {
                                this.update(cx, |app, cx| {
                                    app.firmware.loading_official = true;
                                    app.queue(Command::FetchFirmwareInfo);
                                    cx.notify();
                                })
                                .ok();
                            }
                        }),
                )
                .child(
                    Button::new("firmware-download", app.tr("firmware.downloadLatest"))
                        .primary()
                        .size(ControlSize::Sm)
                        .icon(IconName::Download)
                        .disabled(busy || !loaded)
                        .on_click({
                            let this = this.clone();
                            move |_, _, cx| {
                                this.update(cx, |app, cx| {
                                    if let Some(info) = app.firmware.official.clone() {
                                        if info.downloaded
                                            && app.settings.local_firmware_version == info.version
                                        {
                                            app.notify(
                                                app.tr("firmware.alreadyLatest"),
                                                Tone::Success,
                                            );
                                        } else {
                                            app.queue(Command::DownloadFirmware {
                                                url: info.url,
                                                version: info.version,
                                                release_date: info.release_date,
                                            });
                                        }
                                    }
                                    cx.notify();
                                })
                                .ok();
                            }
                        }),
                )
                .child(
                    Button::new("firmware-notes", app.tr("firmware.releaseNotes"))
                        .size(ControlSize::Sm)
                        .icon(IconName::ExternalLink)
                        .on_click({
                            let this = this.clone();
                            move |_, _, cx| {
                                this.update(cx, |app, cx| {
                                    app.open_link(release_notes.clone());
                                    cx.notify();
                                })
                                .ok();
                            }
                        }),
                ),
        );
    } else {
        let path = app.settings.custom_firmware_path.clone();
        let shown: SharedString = if path.is_empty() {
            app.tr("firmware.choose")
        } else {
            path.clone().into()
        };
        form = form.child(field(
            app.tr("firmware.file"),
            div()
                .flex()
                .items_center()
                .gap(px(9.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(9.0))
                        .flex_1()
                        .min_w_0()
                        .px(px(12.0))
                        .h(px(32.0))
                        .rounded(px(8.0))
                        .border_1()
                        .border_color(colors.border)
                        .bg(colors.overlay)
                        .text_size(px(12.0))
                        .text_color(colors.fg_muted)
                        .child(Icon::new(IconName::FileCode).size(IconSize::Sm))
                        .child(div().truncate().child(shown)),
                )
                .child(
                    Button::new("firmware-browse", app.tr("common.browse"))
                        .size(ControlSize::Sm)
                        .disabled(busy)
                        .on_click({
                            let this = this.clone();
                            move |_, _, cx| {
                                let chosen = rfd::FileDialog::new()
                                    .add_filter("Firmware", &["bin"])
                                    .pick_file();
                                if let Some(path) = chosen {
                                    this.update(cx, |app, cx| {
                                        let mut settings = app.settings.clone();
                                        settings.firmware_mode = FirmwareMode::Custom;
                                        settings.custom_firmware_path = path.display().to_string();
                                        app.save_settings(settings);
                                        cx.notify();
                                    })
                                    .ok();
                                }
                            }
                        }),
                ),
            &colors,
        ));
    }

    let mut device_choices = vec![Choice::new("", app.tr("waveform.selectDevice"))];
    for device in &app.firmware.devices {
        device_choices.push(Choice::new(device.id.clone(), device.display_name.clone()));
    }
    let device_select = Select::new("firmware-device", device_choices)
        .selected(app.firmware.selected_device.clone())
        .size(ControlSize::Sm)
        .disabled(busy)
        .on_change({
            let this = this.clone();
            move |value, _, cx| {
                this.update(cx, |app, cx| {
                    app.firmware.selected_device = value.to_string();
                    cx.notify();
                })
                .ok();
            }
        });

    form = form.child(field(
        app.tr("firmware.device"),
        div()
            .flex()
            .items_center()
            .gap(px(9.0))
            .child(device_select)
            .child(
                Button::new("firmware-devices-refresh", app.tr("common.refresh"))
                    .size(ControlSize::Sm)
                    .icon(IconName::RefreshCw)
                    .disabled(busy)
                    .on_click({
                        let this = this.clone();
                        move |_, _, cx| {
                            this.update(cx, |app, cx| {
                                app.queue(Command::RefreshDevices);
                                cx.notify();
                            })
                            .ok();
                        }
                    }),
            ),
        &colors,
    ));

    let status_text: SharedString = if app.firmware.downloading() {
        app.tr("firmware.downloading")
    } else {
        let key = format!("firmware.{}", app.firmware.progress.message_key);
        let text = app.tr(&key);
        match &app.firmware.progress.detail {
            Some(detail) if !detail.is_empty() => format!("{text}: {detail}").into(),
            _ => text,
        }
    };
    let percent = app.firmware.percent();

    form = form
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(9.0))
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .text_size(px(13.0))
                        .child(
                            div()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child(status_text),
                        )
                        .child(format!("{percent}%")),
                )
                .child(ProgressBar::new(
                    "firmware-progress",
                    percent as f32 / 100.0,
                )),
        )
        .child(div().flex().justify_end().child(if busy {
            Button::new("firmware-cancel", app.tr("common.cancel"))
                .variant(ButtonVariant::Danger)
                .size(ControlSize::Sm)
                .icon(IconName::CircleX)
                .on_click({
                    let this = this.clone();
                    move |_, _, cx| {
                        this.update(cx, |app, cx| {
                            if app.firmware.downloading() {
                                app.queue(Command::CancelDownload);
                            } else {
                                app.queue(Command::CancelFlash);
                            }
                            cx.notify();
                        })
                        .ok();
                    }
                })
        } else {
            Button::new("firmware-flash", app.tr("firmware.flash"))
                .primary()
                .size(ControlSize::Sm)
                .icon(IconName::Shield)
                .on_click({
                    let this = this.clone();
                    move |_, _, cx| {
                        this.update(cx, |app, cx| {
                            let device = app.firmware.selected_device.clone();
                            if device.is_empty() {
                                app.notify(app.tr("waveform.noDevice"), Tone::Error);
                            } else {
                                let path = firmware_path(app);
                                match path {
                                    Some(path) if !path.is_empty() => {
                                        app.queue(Command::FlashFirmware {
                                            device_id: device,
                                            path: std::path::PathBuf::from(path),
                                        });
                                    }
                                    _ => app.notify(app.tr("firmware.noFile"), Tone::Error),
                                }
                            }
                            cx.notify();
                        })
                        .ok();
                    }
                })
        }));

    div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .max_w(px(960.0))
        .child(header)
        .child(hero)
        .child(form)
}

fn firmware_path(app: &PowerPico) -> Option<String> {
    match app.settings.firmware_mode {
        FirmwareMode::Official => app
            .firmware
            .official
            .as_ref()
            .and_then(|info| info.local_path.clone()),
        FirmwareMode::Custom => Some(app.settings.custom_firmware_path.clone()),
    }
}
