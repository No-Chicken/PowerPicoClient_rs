//! The live waveform screen: device picker, metric tiles, the painted chart,
//! range analysis and the record actions.

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    forms::{Choice, Select, Slider},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize},
};
use gpui::prelude::*;
use gpui::{
    canvas, div, fill, point, px, size, App, Bounds, Context, Hsla, IntoElement, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, PathBuilder, Pixels, Point,
    ScrollDelta, ScrollEvent, SharedString, Styled, Window,
};
use powerpico_core::{CaptureStatus, PointReading, RenderSeries};

use crate::{
    app::{metric_card, page_header, panel, PowerPico},
    model::{count, format_current, format_power, ChartTarget, Command, DragState, Range, Tone},
    theme as palette,
};

const LEFT_AXIS: f32 = 52.0;
const RIGHT_AXIS: f32 = 62.0;

pub fn page(
    app: &mut PowerPico,
    _window: &mut Window,
    cx: &mut Context<PowerPico>,
) -> impl IntoElement {
    let (colors, dark) = {
        let theme = cx.theme();
        (theme.colors.clone(), theme.is_dark())
    };
    let mode = if dark {
        ely_gpui_component::theme::Mode::Dark
    } else {
        ely_gpui_component::theme::Mode::Light
    };
    let series = palette::series(mode);
    let lang = app.lang;
    let this = cx.entity().downgrade();

    // ---------------------------------------------------------------- header
    let stats_choices = vec![
        Choice::new("global", app.tr("waveform.globalStats")),
        Choice::new("600", app.tr("waveform.tenMinuteStats")),
        Choice::new("60", app.tr("waveform.minuteStats")),
        Choice::new("1", app.tr("waveform.secondStats")),
    ];
    let stats_selected: SharedString = match app.waveform.stats_window {
        None => "global".into(),
        Some(window) if window >= 600.0 => "600".into(),
        Some(window) if window >= 60.0 => "60".into(),
        Some(_) => "1".into(),
    };

    let mut device_choices = vec![Choice::new("", app.tr("waveform.selectDevice"))];
    for device in &app.waveform.devices {
        device_choices.push(Choice::new(device.id.clone(), device.display_name.clone()));
    }

    let capture_busy = matches!(
        app.waveform.capture.status,
        CaptureStatus::Capturing | CaptureStatus::Connecting
    );

    let stats_select = Select::new("stats-window", stats_choices)
        .selected(stats_selected)
        .size(ControlSize::Sm)
        .on_change({
            let this = this.clone();
            move |value, _, cx| {
                let window = if value.as_ref() == "global" {
                    None
                } else {
                    value.parse::<f64>().ok()
                };
                this.update(cx, |app, cx| {
                    app.waveform.stats_window = window;
                    app.queue(Command::RefreshStats(window));
                    cx.notify();
                })
                .ok();
            }
        });

    let device_select = Select::new("capture-device", device_choices)
        .selected(app.waveform.selected_device.clone())
        .size(ControlSize::Sm)
        .disabled(capture_busy)
        .on_change({
            let this = this.clone();
            move |value, _, cx| {
                this.update(cx, |app, cx| {
                    app.waveform.selected_device = value.to_string();
                    cx.notify();
                })
                .ok();
            }
        });

    let refresh_devices = Button::new("refresh-devices", app.tr("common.refresh"))
        .size(ControlSize::Sm)
        .icon(IconName::RefreshCw)
        .disabled(capture_busy)
        .on_click({
            let this = this.clone();
            move |_, _, cx| {
                this.update(cx, |app, cx| {
                    app.queue(Command::RefreshDevices);
                    cx.notify();
                })
                .ok();
            }
        });

    let (connect_label, connect_icon) = if capture_busy {
        (app.tr("common.disconnect"), IconName::Square)
    } else {
        (app.tr("common.connect"), IconName::Play)
    };
    let connect = Button::new("capture-toggle", connect_label)
        .primary()
        .size(ControlSize::Sm)
        .icon(connect_icon)
        .on_click({
            let this = this.clone();
            move |_, _, cx| {
                this.update(cx, |app, cx| {
                    if matches!(
                        app.waveform.capture.status,
                        CaptureStatus::Capturing | CaptureStatus::Connecting
                    ) {
                        app.queue(Command::StopCapture);
                    } else {
                        let device = app.waveform.selected_device.clone();
                        if device.is_empty() {
                            app.notify(app.tr("waveform.noDevice"), Tone::Error);
                        } else {
                            app.waveform.selected_point = None;
                            app.waveform.follow_live = true;
                            app.waveform.time_view = Range::new(0.0, 10.0);
                            app.queue(Command::StartCapture(device));
                        }
                    }
                    cx.notify();
                })
                .ok();
            }
        });

    let header = div()
        .flex()
        .items_center()
        .justify_between()
        .flex_wrap()
        .gap(px(20.0))
        .child(page_header(
            app.tr("waveform.title"),
            app.tr("waveform.subtitle"),
            &colors,
        ))
        .child(
            div()
                .flex()
                .items_center()
                .flex_wrap()
                .gap(px(9.0))
                .child(div().w(px(150.0)).child(stats_select))
                .child(div().w(px(250.0)).child(device_select))
                .child(refresh_devices)
                .child(connect),
        );

    // --------------------------------------------------------------- metrics
    let mut metrics = div().flex().flex_wrap().gap(px(10.0));
    for id in app.settings.waveform_metrics.clone() {
        if id == powerpico_core::MetricId::Unknown {
            continue;
        }
        metrics = metrics.child(metric_card(
            id,
            &app.waveform.summary,
            lang,
            &colors,
            series,
        ));
    }

    // ----------------------------------------------------------------- chart
    let voltage_range = app.waveform.voltage_range();
    let current_range = app.waveform.current_range();
    let time_view = app.waveform.time_view;

    let voltage_axis = axis_column(
        &colors,
        LEFT_AXIS,
        true,
        [
            format!("{:.2} V", voltage_range.max),
            format!("{:.2} V", (voltage_range.min + voltage_range.max) / 2.0),
            format!("{:.2} V", voltage_range.min),
        ],
    )
    .id("voltage-axis")
    .cursor_crosshair()
    .on_scroll(zoom_handler(this.clone()));
    let current_axis = axis_column(
        &colors,
        RIGHT_AXIS,
        false,
        [
            format!("{:.0} µA", current_range.max),
            format!("{:.0} µA", (current_range.min + current_range.max) / 2.0),
            format!("{:.0} µA", current_range.min),
        ],
    )
    .id("current-axis")
    .cursor_crosshair()
    .on_scroll(zoom_handler(this.clone()));

    let plot = chart(app, &colors, cx);
    let empty = app.waveform.series.time.is_empty();
    let plot_area = div()
        .id("plot-surface")
        .relative()
        .flex_1()
        .min_w_0()
        .h_full()
        .on_scroll(zoom_handler(this.clone()))
        .child(plot)
        .when(empty, |area| {
            area.child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(13.0))
                    .text_color(colors.fg_muted)
                    .child(app.tr("waveform.empty")),
            )
        })
        .children(reading_card(app, &colors));

    let ticks = div()
        .flex()
        .justify_between()
        .pl(px(LEFT_AXIS))
        .pr(px(RIGHT_AXIS))
        .pt(px(4.0))
        .text_size(px(10.0))
        .text_color(colors.fg_muted)
        .child(format!("{:.2}s", time_view.min))
        .child(format!("{:.2}s", (time_view.min + time_view.max) / 2.0))
        .child(format!("{:.2}s", time_view.max));

    let live = app.waveform.follow_live;
    let capturing = matches!(app.waveform.capture.status, CaptureStatus::Capturing);
    let points_text: SharedString = format!(
        "{} {}",
        count(app.waveform.summary.point_count),
        app.tr("waveform.points")
    )
    .into();

    let heading = div()
        .flex()
        .items_center()
        .gap(px(7.0))
        .min_h(px(32.0))
        .text_size(px(12.0))
        .text_color(colors.fg_muted)
        .child(div().w(px(18.0)).h(px(2.0)).rounded_full().bg(series.0))
        .child(app.tr("waveform.voltage"))
        .child(
            div()
                .ml(px(10.0))
                .w(px(18.0))
                .h(px(2.0))
                .rounded_full()
                .bg(series.1),
        )
        .child(app.tr("waveform.current"))
        .child(
            div()
                .ml(px(12.0))
                .flex()
                .items_center()
                .gap(px(5.0))
                .child(Icon::new(IconName::Crosshair).size(IconSize::Sm))
                .child(app.tr("waveform.gestureHint")),
        )
        .child(div().flex_1().min_w(px(8.0)))
        .child(points_text)
        .when(capturing && !live, |bar| {
            bar.child(
                Button::new("return-live", app.tr("waveform.returnLive"))
                    .size(ControlSize::Sm)
                    .icon(IconName::Play)
                    .on_click({
                        let this = this.clone();
                        move |_, _, cx| {
                            this.update(cx, |app, cx| {
                                app.return_to_live();
                                cx.notify();
                            })
                            .ok();
                        }
                    }),
            )
        })
        .child(
            Button::new("reset-axes", app.tr("waveform.resetAxes"))
                .size(ControlSize::Sm)
                .icon(IconName::RotateCcw)
                .on_click({
                    let this = this.clone();
                    move |_, _, cx| {
                        this.update(cx, |app, cx| {
                            app.waveform.reset_axes();
                            cx.notify();
                        })
                        .ok();
                    }
                }),
        );

    let chart_card = panel(&colors)
        .flex()
        .flex_col()
        .flex_1()
        .min_h(px(300.0))
        .px(px(16.0))
        .py(px(12.0))
        .gap(px(6.0))
        .child(heading)
        .child(chart_row(
            cx,
            div()
                .flex()
                .flex_1()
                .min_h_0()
                .child(voltage_axis)
                .child(plot_area)
                .child(current_axis),
        ))
        .child(ticks);

    // ------------------------------------------------------------ navigation
    let duration = app.waveform.total.duration.max(0.001);
    let viewport = Slider::new("viewport", app.waveform.time_view.max)
        .range(0.0, duration)
        .step(0.001)
        .label(app.tr("waveform.viewport"))
        .on_change({
            let this = this.clone();
            move |value, _, cx| {
                this.update(cx, |app, cx| {
                    let span = app.waveform.time_view.span();
                    let bounds = app.waveform.time_bounds();
                    let max = bounds.map(|bounds| bounds.max).unwrap_or(duration);
                    let min = bounds.map(|bounds| bounds.min).unwrap_or(0.0);
                    let end = value.clamp(min + span, max.max(min + span));
                    app.waveform.time_view = Range::new((end - span).max(min), end);
                    app.waveform.follow_live = false;
                    app.request_plot();
                    cx.notify();
                })
                .ok();
            }
        });

    let mut navigation = panel(&colors)
        .flex()
        .flex_col()
        .gap(px(8.0))
        .px(px(14.0))
        .py(px(10.0))
        .child(
            div()
                .text_size(px(12.0))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(colors.accent)
                .child(if live {
                    app.tr("waveform.modeLive")
                } else if app.waveform.analysis.is_some() {
                    app.tr("waveform.modeAnalysis")
                } else {
                    app.tr("waveform.modeFrozen")
                }),
        )
        .child(viewport);

    if let Some(draft) = app.waveform.analysis_draft {
        navigation = navigation
            .child(
                Slider::new("range-start", draft.min)
                    .range(0.0, duration)
                    .step(0.001)
                    .label(app.tr("waveform.rangeStart"))
                    .on_change({
                        let this = this.clone();
                        move |value, _, cx| {
                            this.update(cx, |app, cx| {
                                if let Some(mut draft) = app.waveform.analysis_draft {
                                    draft.min = value.min(draft.max - 0.0001);
                                    app.waveform.analysis_draft = Some(draft);
                                    app.queue(Command::AnalyzeRange(draft.min, draft.max));
                                }
                                cx.notify();
                            })
                            .ok();
                        }
                    }),
            )
            .child(
                Slider::new("range-end", draft.max)
                    .range(0.0, duration)
                    .step(0.001)
                    .label(app.tr("waveform.rangeEnd"))
                    .on_change({
                        let this = this.clone();
                        move |value, _, cx| {
                            this.update(cx, |app, cx| {
                                if let Some(mut draft) = app.waveform.analysis_draft {
                                    draft.max = value.max(draft.min + 0.0001);
                                    app.waveform.analysis_draft = Some(draft);
                                    app.queue(Command::AnalyzeRange(draft.min, draft.max));
                                }
                                cx.notify();
                            })
                            .ok();
                        }
                    }),
            );
    }

    // -------------------------------------------------------------- analysis
    let analysis = app.waveform.analysis.clone().map(|stats| {
        let cells: [(SharedString, String); 10] = [
            (
                app.tr("waveform.startTime"),
                format!("{:.6} s", stats.start),
            ),
            (app.tr("waveform.endTime"), format!("{:.6} s", stats.end)),
            (
                app.tr("metrics.duration"),
                format!("{:.6} s", stats.duration),
            ),
            (
                app.tr("metrics.averageVoltage"),
                format!("{:.4} V", stats.voltage_average),
            ),
            (
                app.tr("metrics.peakVoltage"),
                format!("{:.4} V", stats.voltage_peak),
            ),
            (
                app.tr("metrics.averageCurrent"),
                format_current(stats.current_average),
            ),
            (
                app.tr("metrics.peakCurrent"),
                format_current(stats.current_peak),
            ),
            (
                app.tr("metrics.averagePower"),
                format_power(stats.power_average_mw),
            ),
            (
                app.tr("waveform.powerPeak"),
                format_power(stats.power_peak_mw),
            ),
            (
                app.tr("metrics.energy"),
                format!("{:.6} mAh", stats.energy_mah),
            ),
        ];
        let mut grid = div().grid().grid_cols(5).gap(px(8.0));
        for (label, value) in cells {
            grid = grid.child(
                div()
                    .rounded(px(9.0))
                    .bg(colors.sunken)
                    .px(px(10.0))
                    .py(px(8.0))
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(colors.fg_muted)
                            .child(label),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(value),
                    ),
            );
        }
        panel(&colors)
            .flex()
            .flex_col()
            .gap(px(10.0))
            .px(px(14.0))
            .py(px(12.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(13.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(app.tr("waveform.analysisTitle")),
                    )
                    .child(
                        Button::new("close-analysis", app.tr("common.close"))
                            .size(ControlSize::Sm)
                            .on_click({
                                let this = this.clone();
                                move |_, _, cx| {
                                    this.update(cx, |app, cx| {
                                        app.waveform.analysis = None;
                                        app.waveform.analysis_draft = None;
                                        cx.notify();
                                    })
                                    .ok();
                                }
                            }),
                    ),
            )
            .child(grid)
    });

    // ---------------------------------------------------------------- footer
    let record_ready = app.waveform.capture.record_path.is_some();
    let footer = div()
        .flex()
        .items_center()
        .gap(px(9.0))
        .child(
            Button::new("import", app.tr("common.import"))
                .size(ControlSize::Sm)
                .icon(IconName::FolderOpen)
                .on_click({
                    let this = this.clone();
                    move |_, _, cx| {
                        if let Some(path) = pick_file() {
                            this.update(cx, |app, cx| {
                                app.queue(Command::ImportRecording(path));
                                cx.notify();
                            })
                            .ok();
                        }
                    }
                }),
        )
        .child(
            Button::new("export", app.tr("common.export"))
                .size(ControlSize::Sm)
                .icon(IconName::Download)
                .disabled(!record_ready)
                .on_click({
                    let this = this.clone();
                    move |_, _, cx| {
                        if let Some(directory) = pick_directory() {
                            this.update(cx, |app, cx| {
                                app.queue(Command::ExportRecording(directory));
                                cx.notify();
                            })
                            .ok();
                        }
                    }
                }),
        )
        .child(
            Button::new("clear", app.tr("common.clear"))
                .variant(ButtonVariant::Danger)
                .size(ControlSize::Sm)
                .icon(IconName::Trash2)
                .disabled(capturing)
                .on_click({
                    let this = this.clone();
                    move |_, _, cx| {
                        this.update(cx, |app, cx| {
                            app.confirm(crate::model::Confirmation::ClearRecords);
                            cx.notify();
                        })
                        .ok();
                    }
                }),
        )
        .when(app.waveform.capture.error.is_some(), |bar| {
            bar.child(
                div()
                    .ml_auto()
                    .text_size(px(13.0))
                    .text_color(colors.danger)
                    .child(app.waveform.capture.error.clone().unwrap_or_default()),
            )
        });

    div()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .child(header)
        .child(metrics)
        .child(chart_card)
        .child(navigation)
        .children(analysis)
        .child(footer)
}

fn axis_column(
    colors: &ely_gpui_component::theme::Palette,
    width: f32,
    leading: bool,
    labels: [String; 3],
) -> gpui::Div {
    let column = div()
        .flex()
        .flex_col()
        .justify_between()
        .flex_none()
        .w(px(width))
        .py(px(2.0))
        .text_size(px(10.0))
        .text_color(colors.fg_muted);
    let column = if leading {
        column.items_end()
    } else {
        column.items_start().pl(px(8.0))
    };
    column
        .child(labels[0].clone())
        .child(labels[1].clone())
        .child(labels[2].clone())
}

/// The reading card floats over the plot at the selected sample.
fn reading_card(
    app: &PowerPico,
    colors: &ely_gpui_component::theme::Palette,
) -> Option<impl IntoElement> {
    let point = app.waveform.selected_point.clone()?;
    if !app.waveform.point_visible() {
        return None;
    }
    let geometry = *app.waveform.geometry.borrow();
    let left = geometry
        .map(|geometry| {
            let width = geometry.plot.size.width.as_f32().max(1.0);
            let ratio = ((point.time - app.waveform.time_view.min) / app.waveform.time_view.span())
                .clamp(0.0, 1.0) as f32;
            (ratio * width + 12.0).min(width - 220.0).max(8.0)
        })
        .unwrap_or(8.0);
    let series = palette::series(if app.waveform.voltage_view.is_some() {
        ely_gpui_component::theme::Mode::Dark
    } else {
        ely_gpui_component::theme::Mode::Light
    });
    let lang = app.lang;
    let row = |label: SharedString, value: String, color: Option<Hsla>| {
        div()
            .flex()
            .justify_between()
            .gap(px(12.0))
            .text_size(px(11.0))
            .child(div().text_color(colors.fg_muted).child(label))
            .child(div().text_color(color.unwrap_or(colors.fg)).child(value))
    };
    Some(
        div()
            .absolute()
            .left(px(left))
            .top(px(10.0))
            .w(px(210.0))
            .rounded(px(12.0))
            .border_1()
            .border_color(colors.border)
            .bg(colors.overlay)
            .px(px(13.0))
            .py(px(11.0))
            .flex()
            .flex_col()
            .gap(px(5.0))
            .child(
                div()
                    .text_size(px(12.0))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(crate::i18n::t(lang, "waveform.pointReading")),
            )
            .child(row(
                crate::i18n::t(lang, "waveform.time"),
                format!("{:.6} s", point.time),
                None,
            ))
            .child(row(
                crate::i18n::t(lang, "waveform.voltage"),
                format!("{:.4} V", point.voltage),
                Some(series.0),
            ))
            .child(row(
                crate::i18n::t(lang, "waveform.current"),
                format_current(point.current),
                Some(series.1),
            ))
            .child(row(
                crate::i18n::t(lang, "waveform.instantPower"),
                format_power(point.power_mw),
                Some(series.2),
            )),
    )
}

/// The painted chart. The canvas fills the plot area alone, so a pointer
/// position maps straight into data space through the bounds reported here.
fn chart(
    app: &mut PowerPico,
    colors: &ely_gpui_component::theme::Palette,
    cx: &mut Context<PowerPico>,
) -> impl IntoElement {
    let dark = cx.theme().is_dark();
    let mode = if dark {
        ely_gpui_component::theme::Mode::Dark
    } else {
        ely_gpui_component::theme::Mode::Light
    };
    let series = palette::series(mode);
    let grid = palette::grid_line(mode);
    let geometry = app.waveform.geometry.clone();
    let snapshot = Snapshot {
        time: app.waveform.time_view,
        voltage: app.waveform.voltage_range(),
        current: app.waveform.current_range(),
        data: app.waveform.series.clone(),
        analysis: app.waveform.analysis_draft,
        point: app
            .waveform
            .selected_point
            .clone()
            .filter(|_| app.waveform.point_visible()),
        voltage_color: series.0,
        current_color: series.1,
        grid,
        selection_fill: gpui::hsla(0.7472, 0.6, 0.64, 0.16),
        selection_edge: gpui::hsla(0.7472, 0.6, 0.64, 0.75),
        marker_edge: colors.overlay,
    };
    let paint = snapshot.clone();

    let canvas = canvas(
        move |bounds, _, _| {
            *geometry.borrow_mut() = Some(crate::model::PlotGeometry { plot: bounds });
            bounds
        },
        move |bounds, _, window, cx| paint.draw(bounds, window, cx),
    )
    .size_full();

    div().size_full().child(canvas)
}

/// Wheel over the plot or either axis strip zooms that axis.
fn zoom_handler(
    this: gpui::WeakEntity<PowerPico>,
) -> impl Fn(&ScrollEvent, &mut Window, &mut gpui::App) + 'static {
    move |event: &ScrollEvent, _, cx| {
        let delta = match event.delta {
            ScrollDelta::Lines(lines) => lines.y as f64 * 24.0,
            ScrollDelta::Pixels(pixels) => pixels.y.as_f32() as f64,
        };
        this.update(cx, |app, cx| {
            app.chart_zoom(event.position, delta);
            cx.notify();
        })
        .ok();
    }
}

/// The chart's row. Mouse events bubble from the plot and both axis strips into
/// it, so one handler set covers the whole chart. Wheel events do not bubble,
/// which is why each strip registers its own scroll handler.
fn chart_row(cx: &mut Context<PowerPico>, content: gpui::Div) -> impl IntoElement {
    let this = cx.entity().downgrade();
    content
        .id("chart-row")
        .cursor_crosshair()
        .on_mouse_down(MouseButton::Left, {
            let this = this.clone();
            move |event: &MouseDownEvent, _, cx| {
                this.update(cx, |app, cx| {
                    app.chart_begin(event.position, false);
                    cx.notify();
                })
                .ok();
            }
        })
        .on_mouse_down(MouseButton::Right, {
            let this = this.clone();
            move |event: &MouseDownEvent, _, cx| {
                this.update(cx, |app, cx| {
                    app.chart_begin(event.position, true);
                    cx.notify();
                })
                .ok();
            }
        })
        .on_mouse_move({
            let this = this.clone();
            move |event: &MouseMoveEvent, _, cx| {
                this.update(cx, |app, cx| {
                    // A release outside the chart never reached the row; the
                    // next move without a button ends the stale gesture.
                    if event.pressed_button.is_none() {
                        app.chart_end(event.position);
                    } else if app.chart_move(event.position) {
                        cx.notify();
                    }
                })
                .ok();
            }
        })
        .on_mouse_up(MouseButton::Left, {
            let this = this.clone();
            move |event: &MouseUpEvent, _, cx| {
                this.update(cx, |app, cx| {
                    app.chart_end(event.position);
                    cx.notify();
                })
                .ok();
            }
        })
        .on_mouse_up(MouseButton::Right, {
            let this = this.clone();
            move |event: &MouseUpEvent, _, cx| {
                this.update(cx, |app, cx| {
                    app.chart_end(event.position);
                    cx.notify();
                })
                .ok();
            }
        })
}

/// Everything the paint pass needs, snapshotted so the canvas can own it.
#[derive(Clone)]
struct Snapshot {
    time: Range,
    voltage: Range,
    current: Range,
    data: RenderSeries,
    analysis: Option<Range>,
    point: Option<PointReading>,
    voltage_color: Hsla,
    current_color: Hsla,
    grid: Hsla,
    selection_fill: Hsla,
    selection_edge: Hsla,
    marker_edge: Hsla,
}

impl Snapshot {
    fn x(&self, bounds: Bounds<Pixels>, time: f64) -> f32 {
        let ratio = (time - self.time.min) / self.time.span();
        bounds.origin.x.as_f32() + ratio as f32 * bounds.size.width.as_f32()
    }

    fn y(&self, bounds: Bounds<Pixels>, value: f64, range: Range) -> f32 {
        let ratio = (value - range.min) / range.span();
        bounds.origin.y.as_f32() + (1.0 - ratio as f32) * bounds.size.height.as_f32()
    }

    fn draw(&self, bounds: Bounds<Pixels>, window: &mut Window, _: &mut App) {
        let (left, top) = (bounds.origin.x.as_f32(), bounds.origin.y.as_f32());
        let (width, height) = (bounds.size.width.as_f32(), bounds.size.height.as_f32());
        let one = px(1.0);

        for step in 1..5 {
            let y = top + height * step as f32 / 5.0;
            window.paint_quad(fill(
                Bounds {
                    origin: point(px(left), px(y)),
                    size: size(px(width), one),
                },
                self.grid,
            ));
            let x = left + width * step as f32 / 5.0;
            window.paint_quad(fill(
                Bounds {
                    origin: point(px(x), px(top)),
                    size: size(one, px(height)),
                },
                self.grid,
            ));
        }

        if let Some(range) = self.analysis {
            let x0 = self.x(bounds, range.min);
            let x1 = self.x(bounds, range.max);
            let selection = Bounds {
                origin: point(px(x0.min(x1)), px(top)),
                size: size(px((x1 - x0).abs().max(1.0)), px(height)),
            };
            window.paint_quad(fill(selection, self.selection_fill));
            window.paint_quad(fill(
                Bounds {
                    origin: point(px(x0.min(x1)), px(top)),
                    size: size(px(1.0), px(height)),
                },
                self.selection_edge,
            ));
            window.paint_quad(fill(
                Bounds {
                    origin: point(px(x0.max(x1)), px(top)),
                    size: size(px(1.0), px(height)),
                },
                self.selection_edge,
            ));
        }

        self.trace(
            window,
            bounds,
            &self.data.voltage_average,
            self.voltage,
            self.voltage_color,
        );
        self.trace(
            window,
            bounds,
            &self.data.current_average,
            self.current,
            self.current_color,
        );

        if let Some(reading) = &self.point {
            let x = self.x(bounds, reading.time);
            window.paint_quad(fill(
                Bounds {
                    origin: point(px(x), px(top)),
                    size: size(one, px(height)),
                },
                self.marker_edge.opacity(0.55),
            ));
            let vy = self.y(bounds, reading.voltage, self.voltage);
            let cy = self.y(bounds, reading.current, self.current);
            self.dot(window, x, vy, self.voltage_color);
            self.dot(window, x, cy, self.current_color);
        }
    }

    fn trace(
        &self,
        window: &mut Window,
        bounds: Bounds<Pixels>,
        values: &[f32],
        range: Range,
        color: Hsla,
    ) {
        if self.data.time.len() < 2 {
            return;
        }
        let mut builder = PathBuilder::stroke(px(1.5));
        let mut started = false;
        for (index, time) in self.data.time.iter().enumerate() {
            let Some(value) = values.get(index) else {
                continue;
            };
            if !value.is_finite() {
                continue;
            }
            let point = point(
                px(self.x(bounds, *time)),
                px(self.y(bounds, *value as f64, range)),
            );
            if started {
                builder.line_to(point);
            } else {
                builder.move_to(point);
                started = true;
            }
        }
        if let Ok(path) = builder.build() {
            window.paint_path(path, color);
        }
    }

    fn dot(&self, window: &mut Window, x: f32, y: f32, color: Hsla) {
        let radius = 4.5;
        window.paint_quad(gpui::PaintQuad {
            bounds: Bounds {
                origin: point(px(x - radius), px(y - radius)),
                size: size(px(radius * 2.0), px(radius * 2.0)),
            },
            corner_radii: px(radius).into(),
            background: color.into(),
            border_widths: px(2.0).into(),
            border_color: self.marker_edge,
            border_style: gpui::BorderStyle::Solid,
        });
    }
}

/// Opens a native file picker on the UI thread, which owns the dialog.
pub fn pick_file() -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .add_filter("PowerPico Binary", &["bin"])
        .pick_file()
}

pub fn pick_directory() -> Option<std::path::PathBuf> {
    rfd::FileDialog::new().pick_folder()
}

// ------------------------------------------------------------- interactions

impl PowerPico {
    fn plot_bounds(&self) -> Option<Bounds<Pixels>> {
        self.waveform
            .geometry
            .borrow()
            .as_ref()
            .map(|geometry| geometry.plot)
    }

    fn value_at(&self, x: f32, _y: f32) -> Option<f64> {
        let bounds = self.plot_bounds()?;
        let width = bounds.size.width.as_f32().max(1.0);
        let rx = ((x - bounds.origin.x.as_f32()) / width).clamp(0.0, 1.0) as f64;
        Some(self.waveform.time_view.min + rx * self.waveform.time_view.span())
    }

    fn target_at(&self, position: Point<Pixels>) -> Option<ChartTarget> {
        let bounds = self.plot_bounds()?;
        let top = bounds.origin.y - px(4.0);
        let bottom = bounds.origin.y + bounds.size.height + px(6.0);
        if position.y < top || position.y > bottom {
            return None;
        }
        if position.x < bounds.origin.x {
            Some(ChartTarget::Voltage)
        } else if position.x > bounds.origin.x + bounds.size.width {
            Some(ChartTarget::Current)
        } else {
            Some(ChartTarget::Time)
        }
    }

    pub fn chart_begin(&mut self, position: Point<Pixels>, analysis: bool) {
        if let Some(target) = self.target_at(position) {
            self.chart_drag = Some(DragState {
                target,
                start: position,
                last: position,
                dragged: false,
                started_in_plot: target == ChartTarget::Time,
                analysis: analysis && target == ChartTarget::Time,
            });
        }
    }

    pub fn chart_move(&mut self, position: Point<Pixels>) -> bool {
        let Some(mut drag) = self.chart_drag.take() else {
            return false;
        };
        let dx = (position.x - drag.start.x).as_f32();
        let dy = (position.y - drag.start.y).as_f32();
        if !drag.dragged && (dx * dx + dy * dy).sqrt() > 4.0 {
            drag.dragged = true;
        }
        if !drag.dragged {
            self.chart_drag = Some(drag);
            return false;
        }
        if drag.analysis {
            let first = self.value_at(drag.start.x.as_f32(), drag.start.y.as_f32());
            let second = self.value_at(position.x.as_f32(), position.y.as_f32());
            if let (Some(a), Some(b)) = (first, second) {
                self.waveform.analysis_draft = Some(Range::new(a.min(b), a.max(b)));
            }
            self.chart_drag = Some(drag);
            return true;
        }
        let delta_x = (position.x - drag.last.x).as_f32() as f64;
        let delta_y = (position.y - drag.last.y).as_f32() as f64;
        match drag.target {
            ChartTarget::Time => {
                let width = self
                    .plot_bounds()
                    .map(|bounds| bounds.size.width.as_f32() as f64)
                    .unwrap_or(1.0);
                let bounds = self.waveform.time_bounds();
                self.waveform.time_view = self.waveform.time_view.pan(-delta_x, width, bounds);
                self.waveform.follow_live = false;
                self.request_plot();
            }
            ChartTarget::Voltage => {
                let height = self
                    .plot_bounds()
                    .map(|bounds| bounds.size.height.as_f32() as f64)
                    .unwrap_or(1.0);
                let range = self.waveform.voltage_range();
                self.waveform.voltage_view = Some(range.pan(delta_y, height, None));
            }
            ChartTarget::Current => {
                let height = self
                    .plot_bounds()
                    .map(|bounds| bounds.size.height.as_f32() as f64)
                    .unwrap_or(1.0);
                let range = self.waveform.current_range();
                self.waveform.current_view = Some(range.pan(delta_y, height, None));
            }
        }
        drag.last = position;
        self.chart_drag = Some(drag);
        true
    }

    pub fn chart_end(&mut self, position: Point<Pixels>) {
        let Some(drag) = self.chart_drag.take() else {
            return;
        };
        if drag.analysis && drag.dragged {
            if let Some(draft) = self.waveform.analysis_draft {
                if draft.span() > 0.0001 {
                    self.queue(Command::AnalyzeRange(draft.min, draft.max));
                    self.waveform.follow_live = false;
                }
            }
            return;
        }
        if drag.dragged || !drag.started_in_plot {
            return;
        }
        let Some(time) = self.value_at(position.x.as_f32(), position.y.as_f32()) else {
            return;
        };
        let series = &self.waveform.series;
        if time < series.available_start || time > series.available_end {
            self.waveform.selected_point = None;
            return;
        }
        self.queue(Command::PointAt(time));
    }

    pub fn chart_zoom(&mut self, position: Point<Pixels>, delta: f64) {
        let Some(target) = self.target_at(position) else {
            return;
        };
        let Some(bounds) = self.plot_bounds() else {
            return;
        };
        let width = bounds.size.width.as_f32() as f64;
        let height = bounds.size.height.as_f32() as f64;
        let factor = (delta * 0.0015).exp();
        match target {
            ChartTarget::Time => {
                let position_x = (position.x.as_f32() - bounds.origin.x.as_f32()) as f64;
                let ratio = (position_x / width).clamp(0.0, 1.0);
                let center = self.waveform.time_view.min + ratio * self.waveform.time_view.span();
                let bounds = self.waveform.time_bounds();
                self.waveform.time_view =
                    self.waveform.time_view.zoom(center, factor, bounds, 0.001);
                self.waveform.follow_live = false;
                self.request_plot();
            }
            ChartTarget::Voltage | ChartTarget::Current => {
                let position_y = (position.y.as_f32() - bounds.origin.y.as_f32()) as f64;
                let ratio = (position_y / height).clamp(0.0, 1.0);
                let range = if target == ChartTarget::Voltage {
                    self.waveform.voltage_range()
                } else {
                    self.waveform.current_range()
                };
                let center = range.max - ratio * range.span();
                let minimum = (range.span() / 10_000.0).max(1e-9);
                let next = range.zoom(center, factor, None, minimum);
                if target == ChartTarget::Voltage {
                    self.waveform.voltage_view = Some(next);
                } else {
                    self.waveform.current_view = Some(next);
                }
            }
        }
    }
}
