//! Device calibration: read, adjust and write the three current ranges.

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    forms::{Choice, NumberInput, Select},
    motion::ProgressBar,
    primitives::{Icon, IconName},
    tables::Table,
    theme::{ActiveTheme, ControlSize, IconSize},
};
use gpui::prelude::*;
use gpui::{Context, IntoElement, ParentElement, SharedString, Styled, Window, div, px};

use crate::{
    app::{PowerPico, field, page_header, panel},
    model::{
        CALIBRATION_SCALES, CalibrationOp, Command, Confirmation, DISPLAY_DIVISORS, ManualWrite,
        offset_limit, valid_calibration,
    },
};

const RANGE_NAMES: [&str; 3] = ["Low", "Mid", "High"];
const RANGE_RESISTANCE: [&str; 3] = ["50 Ω", "0.5 Ω", "0.005 Ω"];

pub fn page(
    app: &mut PowerPico,
    _window: &mut Window,
    cx: &mut Context<PowerPico>,
) -> impl IntoElement {
    let colors = cx.theme().colors.clone();
    let this = cx.entity().downgrade();

    let header = page_header(
        app.tr("calibration.title"),
        app.tr("calibration.subtitle"),
        &colors,
    );

    let hero = panel(&colors)
        .flex()
        .items_center()
        .gap(px(18.0))
        .p(px(22.0))
        .child(
            div()
                .p(px(15.0))
                .rounded(px(16.0))
                .bg(colors.info_subtle)
                .text_color(colors.accent)
                .child(Icon::new(IconName::SlidersHorizontal).size(IconSize::Xl)),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(5.0))
                .child(
                    div()
                        .text_size(px(18.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .child("PowerPico Calibration"),
                )
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(colors.fg_muted)
                        .child(app.tr("calibration.formula")),
                ),
        );

    let warning = div()
        .flex()
        .items_center()
        .gap(px(10.0))
        .px(px(14.0))
        .py(px(14.0))
        .rounded(px(12.0))
        .bg(colors.info_subtle)
        .text_size(px(13.0))
        .child(
            Icon::new(IconName::TriangleAlert)
                .size(IconSize::Lg)
                .color(colors.warning),
        )
        .child(app.tr("calibration.warning"));

    // ------------------------------------------------------------------ form
    let busy = app.calibration.busy();
    let connected = app.calibration.state.connected;
    let mut device_choices = vec![Choice::new("", app.tr("waveform.selectDevice"))];
    for device in &app.calibration.devices {
        device_choices.push(Choice::new(device.id.clone(), device.display_name.clone()));
    }
    let device_select = Select::new("calibration-device", device_choices)
        .selected(app.calibration.selected_device.clone())
        .size(ControlSize::Sm)
        .disabled(busy || connected)
        .on_change({
            let this = this.clone();
            move |value, _, cx| {
                this.update(cx, |app, cx| {
                    app.calibration.selected_device = value.to_string();
                    cx.notify();
                })
                .ok();
            }
        });

    let connect_label = if connected {
        app.tr("common.disconnect")
    } else {
        app.tr("common.connect")
    };
    let connect_enabled = connected || !app.calibration.selected_device.is_empty();
    let row = div()
        .flex()
        .items_end()
        .gap(px(9.0))
        .child(field(
            app.tr("firmware.device"),
            div()
                .flex()
                .items_center()
                .gap(px(9.0))
                .child(device_select)
                .child(
                    Button::new("calibration-refresh", app.tr("common.refresh"))
                        .size(ControlSize::Sm)
                        .icon(IconName::RefreshCw)
                        .disabled(busy || connected)
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
        ))
        .child(
            div().flex_none().child(
                Button::new("calibration-connect", connect_label)
                    .primary()
                    .size(ControlSize::Md)
                    .disabled(busy || !connect_enabled)
                    .on_click({
                        let this = this.clone();
                        move |_, _, cx| {
                            this.update(cx, |app, cx| {
                                app.calibration.pending = true;
                                if app.calibration.state.connected {
                                    app.queue(Command::Calibration(CalibrationOp::Close));
                                } else {
                                    let device = app.calibration.selected_device.clone();
                                    app.queue(Command::Calibration(CalibrationOp::Connect(device)));
                                }
                                cx.notify();
                            })
                            .ok();
                        }
                    }),
            ),
        );

    let editable = app.calibration.editable();
    let actions = div()
        .flex()
        .gap(px(10.0))
        .flex_wrap()
        .child(
            Button::new("calibration-read", app.tr("calibration.read"))
                .size(ControlSize::Sm)
                .disabled(!connected || busy)
                .on_click({
                    let this = this.clone();
                    move |_, _, cx| {
                        this.update(cx, |app, cx| {
                            app.calibration.pending = true;
                            app.queue(Command::Calibration(CalibrationOp::Read));
                            cx.notify();
                        })
                        .ok();
                    }
                }),
        )
        .child(
            Button::new("calibration-auto", app.tr("calibration.auto"))
                .primary()
                .size(ControlSize::Sm)
                .disabled(!connected || busy)
                .on_click({
                    let this = this.clone();
                    move |_, _, cx| {
                        this.update(cx, |app, cx| {
                            app.confirm(Confirmation::AutoZero);
                            cx.notify();
                        })
                        .ok();
                    }
                }),
        )
        .child(
            Button::new("calibration-reset", app.tr("calibration.reset"))
                .variant(ButtonVariant::Danger)
                .size(ControlSize::Sm)
                .disabled(!editable)
                .on_click({
                    let this = this.clone();
                    move |_, _, cx| {
                        this.update(cx, |app, cx| {
                            app.confirm(Confirmation::Reset);
                            cx.notify();
                        })
                        .ok();
                    }
                }),
        );

    let stage_label: SharedString = {
        let mut text = app.tr(&app.calibration.stage_key()).to_string();
        if let Some(range) = app.calibration.stage_range() {
            let index = range.saturating_sub(1).min(2);
            text.push_str(" · ");
            text.push_str(RANGE_NAMES[index]);
        }
        text.into()
    };
    let samples = app.calibration.state.samples;

    let status = div()
        .flex()
        .items_center()
        .flex_wrap()
        .gap(px(12.0))
        .text_size(px(13.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child(stage_label),
        )
        .when(busy, |bar| {
            bar.child(div().flex_1().min_w(px(100.0)).child(ProgressBar::new(
                "calibration-progress",
                (samples as f32 / 60_000.0).clamp(0.0, 1.0),
            )))
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(colors.fg_muted)
                    .child(format!("{} / 60,000", crate::model::count(samples as u64))),
            )
            .when(app.calibration.stage_range().is_some(), |bar| {
                bar.child(
                    Button::new("calibration-cancel", app.tr("common.cancel"))
                        .size(ControlSize::Sm)
                        .on_click({
                            let this = this.clone();
                            move |_, _, cx| {
                                this.update(cx, |app, cx| {
                                    app.queue(Command::CancelCalibration);
                                    cx.notify();
                                })
                                .ok();
                            }
                        }),
                )
            })
        });

    let error = app.calibration.state.error.clone().map(|error| {
        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .text_size(px(13.0))
            .text_color(colors.danger)
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(8.0))
                    .child(Icon::new(IconName::CircleX).size(IconSize::Sm))
                    .child(error),
            )
            .when(app.calibration.state.possibly_committed, |block| {
                block.child(
                    div()
                        .text_color(colors.fg_muted)
                        .child(app.tr("calibration.uncertain")),
                )
            })
    });

    let form = panel(&colors)
        .flex()
        .flex_col()
        .gap(px(14.0))
        .p(px(22.0))
        .child(row)
        .child(actions)
        .child(status)
        .children(error);

    // ---------------------------------------------------------------- ranges
    let mut ranges = panel(&colors).flex().flex_col().p(px(22.0)).child(
        div()
            .text_size(px(12.0))
            .text_color(colors.fg_muted)
            .pb(px(10.0))
            .child(app.tr("calibration.limits")),
    );

    for index in 0..3 {
        let gain = app.calibration.gains[index];
        let offset = app.calibration.offsets[index];
        let limit = offset_limit(index, gain);
        let unit = if index == 2 { "mA" } else { "µA" };
        let valid = valid_calibration(index, gain, offset);
        let gain_input = NumberInput::new(("gain", index), gain)
            .range(0.85, 1.15)
            .step(0.001)
            .precision(6)
            .size(ControlSize::Sm)
            .on_change({
                let this = this.clone();
                move |value, _, cx| {
                    this.update(cx, |app, cx| {
                        app.calibration.gains[index] = value;
                        cx.notify();
                    })
                    .ok();
                }
            });
        let offset_input = NumberInput::new(("offset", index), offset)
            .range(-limit, limit)
            .step(0.001)
            .precision(6)
            .size(ControlSize::Sm)
            .on_change({
                let this = this.clone();
                move |value, _, cx| {
                    this.update(cx, |app, cx| {
                        app.calibration.offsets[index] = value;
                        cx.notify();
                    })
                    .ok();
                }
            });

        let apply = Button::new(("apply", index), app.tr("calibration.apply"))
            .size(ControlSize::Sm)
            .disabled(!editable || !valid)
            .on_click({
                let this = this.clone();
                move |_, _, cx| {
                    this.update(cx, |app, cx| {
                        let gain = app.calibration.gains[index];
                        let offset = app.calibration.offsets[index];
                        app.confirm(Confirmation::Manual(ManualWrite {
                            range: index,
                            gain,
                            offset_ua: offset * DISPLAY_DIVISORS[index],
                        }));
                        cx.notify();
                    })
                    .ok();
                }
            });

        ranges = ranges.child(
            div()
                .flex()
                .items_center()
                .gap(px(14.0))
                .py(px(16.0))
                .border_t_1()
                .border_color(colors.border)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .w(px(190.0))
                        .child(
                            div()
                                .text_size(px(15.0))
                                .font_weight(gpui::FontWeight::BOLD)
                                .child(RANGE_NAMES[index]),
                        )
                        .child(div().text_size(px(12.0)).text_color(colors.fg_muted).child(
                            format!(
                                "{} · {} {}/LSB",
                                RANGE_RESISTANCE[index],
                                CALIBRATION_SCALES[index] / DISPLAY_DIVISORS[index],
                                unit
                            ),
                        )),
                )
                .child(
                    div()
                        .w(px(130.0))
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(colors.fg_muted)
                                .pb(px(5.0))
                                .child(app.tr("calibration.gain")),
                        )
                        .child(gain_input),
                )
                .child(
                    div()
                        .w(px(170.0))
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(colors.fg_muted)
                                .pb(px(5.0))
                                .child(format!("{} ({unit})", app.tr("calibration.offset"))),
                        )
                        .child(offset_input),
                )
                .child(div().flex_1())
                .child(apply),
        );
    }

    // ------------------------------------------------------------- statistics
    let statistics = if app.calibration.state.statistics.is_empty() {
        None
    } else {
        let mut table = Table::new([
            app.tr("calibration.range"),
            app.tr("calibration.samples"),
            format!("{} (LSB)", app.tr("calibration.zero")).into(),
            format!("{} (LSB)", app.tr("calibration.noise")).into(),
        ]);
        for row in &app.calibration.state.statistics {
            let index = usize::from(row.range).saturating_sub(1).min(2);
            table = table.row([
                div().child(RANGE_NAMES[index]).into_any_element(),
                div()
                    .child(crate::model::count(row.sample_count as u64))
                    .into_any_element(),
                div()
                    .child(format!("{:.4}", row.zero_lsb))
                    .into_any_element(),
                div()
                    .child(format!("{:.4}", row.noise_lsb))
                    .into_any_element(),
            ]);
        }
        Some(
            panel(&colors)
                .flex()
                .flex_col()
                .gap(px(10.0))
                .p(px(22.0))
                .child(
                    div()
                        .text_size(px(15.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .child(app.tr("calibration.statistics")),
                )
                .child(table),
        )
    };

    let mut page = div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .max_w(px(1000.0))
        .child(header)
        .child(hero)
        .child(warning)
        .child(form)
        .child(ranges);
    page = page.children(statistics);
    page
}
