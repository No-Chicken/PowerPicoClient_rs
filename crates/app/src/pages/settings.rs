//! Settings: appearance, updates, storage, the metric layout and the about card.

use ely_gpui_component::{
    buttons::{Button, ButtonVariant},
    forms::{Checkbox, Choice, Select},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, ControlSize, IconSize},
};
use gpui::prelude::*;
use gpui::{div, px, Context, IntoElement, ParentElement, SharedString, Styled, Window};
use powerpico_core::{Language, MetricId, ThemeMode};

use crate::{
    app::{page_header, panel, PowerPico},
    model::{
        count, format_bytes, metric_key, Command, Confirmation, DEFAULT_METRICS, METRIC_CATALOG,
    },
};

pub fn page(
    app: &mut PowerPico,
    _window: &mut Window,
    cx: &mut Context<PowerPico>,
) -> impl IntoElement {
    let colors = cx.theme().colors.clone();
    let this = cx.entity().downgrade();
    let lang = app.lang;

    let header = page_header(
        app.tr("settings.title"),
        app.tr("settings.subtitle"),
        &colors,
    );

    // -------------------------------------------------------------- appearance
    let theme_select = Select::new(
        "theme",
        vec![
            Choice::new("system", app.tr("settings.system")),
            Choice::new("light", app.tr("settings.light")),
            Choice::new("dark", app.tr("settings.dark")),
        ],
    )
    .selected(match app.settings.theme {
        ThemeMode::System => "system",
        ThemeMode::Light => "light",
        ThemeMode::Dark => "dark",
    })
    .size(ControlSize::Sm)
    .on_change({
        let this = this.clone();
        move |value, _, cx| {
            let theme = match value.as_ref() {
                "light" => ThemeMode::Light,
                "dark" => ThemeMode::Dark,
                _ => ThemeMode::System,
            };
            this.update(cx, |app, cx| {
                let mut settings = app.settings.clone();
                settings.theme = theme;
                app.save_settings(settings);
                cx.notify();
            })
            .ok();
        }
    });

    let language_select = Select::new(
        "language",
        vec![
            Choice::new("auto", app.tr("settings.system")),
            Choice::new("zh-CN", "简体中文"),
            Choice::new("zh-HK", "繁體中文"),
            Choice::new("en", "English"),
            Choice::new("ja", "日本語"),
        ],
    )
    .selected(match app.settings.language {
        Language::Auto => "auto",
        Language::ZhCn => "zh-CN",
        Language::ZhHk => "zh-HK",
        Language::En => "en",
        Language::Ja => "ja",
    })
    .size(ControlSize::Sm)
    .on_change({
        let this = this.clone();
        move |value, _, cx| {
            let language = match value.as_ref() {
                "zh-CN" => Language::ZhCn,
                "zh-HK" => Language::ZhHk,
                "en" => Language::En,
                "ja" => Language::Ja,
                _ => Language::Auto,
            };
            this.update(cx, |app, cx| {
                let mut settings = app.settings.clone();
                settings.language = language;
                app.save_settings(settings);
                cx.notify();
            })
            .ok();
        }
    });

    let scale_select = Select::new(
        "ui-scale",
        vec![
            Choice::new("0", app.tr("settings.auto")),
            Choice::new("100", "100%"),
            Choice::new("125", "125%"),
            Choice::new("150", "150%"),
            Choice::new("175", "175%"),
            Choice::new("200", "200%"),
        ],
    )
    .selected(app.settings.ui_scale.to_string())
    .size(ControlSize::Sm)
    .on_change({
        let this = this.clone();
        move |value, _, cx| {
            let scale = value.parse::<u16>().unwrap_or(0);
            this.update(cx, |app, cx| {
                let mut settings = app.settings.clone();
                settings.ui_scale = scale;
                app.save_settings(settings);
                cx.notify();
            })
            .ok();
        }
    });

    let anti_aliasing = Checkbox::new("anti-aliasing", app.settings.anti_aliasing).on_change({
        let this = this.clone();
        move |checked, _, cx| {
            this.update(cx, |app, cx| {
                let mut settings = app.settings.clone();
                settings.anti_aliasing = checked;
                app.save_settings(settings);
                cx.notify();
            })
            .ok();
        }
    });

    let appearance = group(
        &colors,
        IconName::Palette,
        app.tr("settings.appearance"),
        vec![
            setting_row(
                &colors,
                IconName::Monitor,
                app.tr("settings.theme"),
                Some(app.tr(&format!(
                    "settings.{}",
                    match app.settings.theme {
                        ThemeMode::System => "system",
                        ThemeMode::Light => "light",
                        ThemeMode::Dark => "dark",
                    }
                ))),
                div().w(px(180.0)).child(theme_select).into_any_element(),
            ),
            setting_row(
                &colors,
                IconName::Languages,
                app.tr("settings.language"),
                Some(match app.settings.language {
                    Language::Auto => app.tr("settings.system"),
                    Language::ZhCn => "简体中文".into(),
                    Language::ZhHk => "繁體中文".into(),
                    Language::En => "English".into(),
                    Language::Ja => "日本語".into(),
                }),
                div().w(px(180.0)).child(language_select).into_any_element(),
            ),
            setting_row(
                &colors,
                IconName::Gauge,
                app.tr("settings.uiScale"),
                Some(if app.settings.ui_scale == 0 {
                    app.tr("settings.auto")
                } else {
                    format!("{}%", app.settings.ui_scale).into()
                }),
                div().w(px(180.0)).child(scale_select).into_any_element(),
            ),
            setting_row(
                &colors,
                IconName::Activity,
                app.tr("settings.antiAliasing"),
                None,
                anti_aliasing.into_any_element(),
            ),
        ],
    );

    // ------------------------------------------------------------------ update
    let check_at_startup = Checkbox::new("check-update", app.settings.check_update_at_startup)
        .on_change({
            let this = this.clone();
            move |checked, _, cx| {
                this.update(cx, |app, cx| {
                    let mut settings = app.settings.clone();
                    settings.check_update_at_startup = checked;
                    app.save_settings(settings);
                    cx.notify();
                })
                .ok();
            }
        });

    let checking = app.settings_ui.checking_update;
    let mut update_rows = vec![
        setting_row(
            &colors,
            IconName::RefreshCw,
            app.tr("settings.checkAtStartup"),
            None,
            check_at_startup.into_any_element(),
        ),
        setting_row(
            &colors,
            IconName::ExternalLink,
            app.tr("settings.checkNow"),
            None,
            Button::new(
                "check-update-now",
                if checking {
                    app.tr("settings.checking")
                } else {
                    app.tr("settings.checkNow")
                },
            )
            .size(ControlSize::Sm)
            .disabled(checking)
            .on_click({
                let this = this.clone();
                move |_, _, cx| {
                    this.update(cx, |app, cx| {
                        app.settings_ui.checking_update = true;
                        app.queue(Command::CheckUpdate);
                        cx.notify();
                    })
                    .ok();
                }
            })
            .into_any_element(),
        ),
    ];
    if let Some(version) = app.settings_ui.update_version.clone() {
        update_rows.push(
            div()
                .flex()
                .items_center()
                .gap(px(16.0))
                .p(px(14.0))
                .rounded(px(12.0))
                .bg(colors.sunken)
                .child(
                    div()
                        .flex_1()
                        .child(format!("{}: {version}", app.tr("settings.updateAvailable"))),
                )
                .child(
                    Button::new("open-release", app.tr("common.download"))
                        .primary()
                        .size(ControlSize::Sm)
                        .on_click({
                            let this = this.clone();
                            move |_, _, cx| {
                                this.update(cx, |app, cx| {
                                    let url = app.settings_ui.update.as_ref().map(|info| {
                                        format!(
                                            "https://github.com/No-Chicken/PowerPicoClient_rs/releases/tag/v{}",
                                            info.latest_version
                                        )
                                    });
                                    app.open_link(url);
                                    cx.notify();
                                })
                                .ok();
                            }
                        })
                        .into_any_element(),
                ),
        );
    }
    let update = group(
        &colors,
        IconName::RefreshCw,
        app.tr("settings.update"),
        update_rows,
    );

    // ----------------------------------------------------------------- storage
    let cache_text: SharedString = app
        .settings_ui
        .storage
        .as_ref()
        .map(|usage| format_bytes(usage.cache_bytes).into())
        .unwrap_or_else(|| "—".into());
    let mut storage_rows = vec![setting_row(
        &colors,
        IconName::HardDrive,
        app.tr("settings.cache"),
        Some(cache_text),
        div()
            .flex()
            .gap(px(8.0))
            .child(
                Button::new("refresh-storage", app.tr("common.refresh"))
                    .size(ControlSize::Sm)
                    .on_click({
                        let this = this.clone();
                        move |_, _, cx| {
                            this.update(cx, |app, cx| {
                                app.settings_ui.refreshing_storage = true;
                                app.queue(Command::RefreshStorage);
                                cx.notify();
                            })
                            .ok();
                        }
                    }),
            )
            .child(
                Button::new(
                    "clear-cache",
                    if app.settings_ui.clearing_cache {
                        app.tr("settings.cleaning")
                    } else {
                        app.tr("settings.clearCache")
                    },
                )
                .size(ControlSize::Sm)
                .disabled(app.settings_ui.clearing_cache)
                .on_click({
                    let this = this.clone();
                    move |_, _, cx| {
                        this.update(cx, |app, cx| {
                            app.confirm(Confirmation::ClearCache);
                            cx.notify();
                        })
                        .ok();
                    }
                }),
            )
            .into_any_element(),
    )];
    if let Some(info) = app.settings_ui.uninstall.clone() {
        if info.supported {
            storage_rows.push(
                setting_row(
                    &colors,
                    IconName::Trash2,
                    app.tr("settings.uninstall"),
                    Some(
                        format!(
                            "{} · {}",
                            app.tr("settings.uninstallHint"),
                            format_bytes(info.total_bytes)
                        )
                        .into(),
                    ),
                    Button::new("uninstall", app.tr("settings.uninstall"))
                        .variant(ButtonVariant::Danger)
                        .size(ControlSize::Sm)
                        .disabled(app.settings_ui.uninstalling)
                        .on_click({
                            let this = this.clone();
                            move |_, _, cx| {
                                this.update(cx, |app, cx| {
                                    app.confirm(Confirmation::Uninstall);
                                    cx.notify();
                                })
                                .ok();
                            }
                        })
                        .into_any_element(),
                )
                .text_color(colors.danger),
            );
        }
    }
    let storage = group(
        &colors,
        IconName::HardDrive,
        app.tr("settings.storage"),
        storage_rows,
    );

    // ----------------------------------------------------------------- metrics
    let mut metrics_panel = panel(&colors)
        .flex()
        .flex_col()
        .gap(px(10.0))
        .p(px(20.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(9.0))
                .child(Icon::new(IconName::ChartLine).size(IconSize::Md))
                .child(
                    div()
                        .text_size(px(15.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .child(app.tr("settings.waveformMetrics")),
                ),
        )
        .child(
            div()
                .text_size(px(12.0))
                .text_color(colors.fg_muted)
                .child(app.tr("settings.waveformMetricsHint")),
        );

    let mut grid = div().grid().grid_cols(3).gap(px(9.0));
    let metrics = app.settings.waveform_metrics.clone();
    let last = metrics.len().saturating_sub(1);
    for (index, metric) in metrics.iter().copied().enumerate() {
        if metric == MetricId::Unknown {
            continue;
        }
        grid = grid.child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .px(px(10.0))
                .py(px(8.0))
                .rounded(px(12.0))
                .border_1()
                .border_color(colors.border)
                .bg(colors.sunken)
                .child(
                    Icon::new(IconName::GripVertical)
                        .size(IconSize::Sm)
                        .color(colors.fg_muted),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(px(13.0))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .truncate()
                        .child(app.tr(metric_key(metric))),
                )
                .child(
                    Button::new(("up", index), "↑")
                        .size(ControlSize::Sm)
                        .disabled(index == 0)
                        .on_click({
                            let this = this.clone();
                            move |_, _, cx| {
                                this.update(cx, |app, cx| {
                                    app.move_metric(metric, -1);
                                    cx.notify();
                                })
                                .ok();
                            }
                        }),
                )
                .child(
                    Button::new(("down", index), "↓")
                        .size(ControlSize::Sm)
                        .disabled(index == last)
                        .on_click({
                            let this = this.clone();
                            move |_, _, cx| {
                                this.update(cx, |app, cx| {
                                    app.move_metric(metric, 1);
                                    cx.notify();
                                })
                                .ok();
                            }
                        }),
                )
                .child(
                    Button::new(("remove", index), "×")
                        .size(ControlSize::Sm)
                        .disabled(metrics.len() <= 1)
                        .on_click({
                            let this = this.clone();
                            move |_, _, cx| {
                                this.update(cx, |app, cx| {
                                    app.remove_metric(metric);
                                    cx.notify();
                                })
                                .ok();
                            }
                        }),
                ),
        );
    }
    metrics_panel = metrics_panel.child(grid).child(
        div()
            .text_size(px(13.0))
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .pt(px(8.0))
            .child(app.tr("settings.availableMetrics")),
    );

    let mut available = div().flex().flex_wrap().gap(px(8.0));
    let mut any = false;
    for metric in METRIC_CATALOG {
        if app.settings.waveform_metrics.contains(&metric) {
            continue;
        }
        any = true;
        available = available.child(
            Button::new(("add", metric as usize), app.tr(metric_key(metric)))
                .size(ControlSize::Sm)
                .icon(IconName::Plus)
                .on_click({
                    let this = this.clone();
                    move |_, _, cx| {
                        this.update(cx, |app, cx| {
                            app.add_metric(metric);
                            cx.notify();
                        })
                        .ok();
                    }
                }),
        );
    }
    if !any {
        available = available.child(
            div()
                .text_size(px(12.0))
                .text_color(colors.fg_muted)
                .child(app.tr("settings.allMetricsVisible")),
        );
    }
    metrics_panel = metrics_panel.child(available);

    // -------------------------------------------------------------------- help
    let links = app.settings_ui.links.clone();
    let help = group(
        &colors,
        IconName::CircleHelp,
        app.tr("settings.help"),
        vec![
            setting_row(
                &colors,
                IconName::CircleHelp,
                app.tr("settings.openHelp"),
                None,
                Button::new("open-help", app.tr("common.open"))
                    .size(ControlSize::Sm)
                    .on_click({
                        let this = this.clone();
                        let url = links.as_ref().map(|links| links.help.clone());
                        move |_, _, cx| {
                            this.update(cx, |app, cx| {
                                app.open_link(url.clone());
                                cx.notify();
                            })
                            .ok();
                        }
                    })
                    .into_any_element(),
            ),
            setting_row(
                &colors,
                IconName::ExternalLink,
                app.tr("settings.provideFeedback"),
                None,
                Button::new("open-feedback", app.tr("common.open"))
                    .size(ControlSize::Sm)
                    .on_click({
                        let this = this.clone();
                        let url = links.as_ref().map(|links| links.feedback.clone());
                        move |_, _, cx| {
                            this.update(cx, |app, cx| {
                                app.open_link(url.clone());
                                cx.notify();
                            })
                            .ok();
                        }
                    })
                    .into_any_element(),
            ),
        ],
    );

    let about = panel(&colors)
        .flex()
        .flex_col()
        .gap(px(8.0))
        .p(px(20.0))
        .child(
            div()
                .text_size(px(15.0))
                .font_weight(gpui::FontWeight::BOLD)
                .child(app.tr("settings.about")),
        )
        .child(
            div()
                .text_size(px(13.0))
                .text_color(colors.fg_muted)
                .child(app.tr("settings.aboutText")),
        )
        .child(
            div()
                .text_size(px(12.0))
                .text_color(colors.accent)
                .px(px(10.0))
                .py(px(7.0))
                .rounded(px(8.0))
                .bg(colors.info_subtle)
                .child(format!(
                    "PowerPico Client {} · GPL-3.0-only · gpui-kit 0.9 · Ely",
                    env!("CARGO_PKG_VERSION")
                )),
        );

    div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .max_w(px(940.0))
        .child(header)
        .child(appearance)
        .child(update)
        .child(storage)
        .child(metrics_panel)
        .child(help)
        .child(about)
        .child(
            div()
                .text_size(px(11.0))
                .text_color(colors.fg_muted)
                .child(format!(
                    "{} · {}",
                    lang.tag(),
                    count(DEFAULT_METRICS.len() as u64)
                )),
        )
}

fn group(
    colors: &ely_gpui_component::theme::Palette,
    icon: IconName,
    title: SharedString,
    rows: impl IntoIterator<Item = gpui::Div>,
) -> gpui::Div {
    panel(colors)
        .flex()
        .flex_col()
        .p(px(20.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(9.0))
                .pb(px(6.0))
                .child(Icon::new(icon).size(IconSize::Md))
                .child(
                    div()
                        .text_size(px(15.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .child(title),
                ),
        )
        .children(rows)
}

fn setting_row(
    colors: &ely_gpui_component::theme::Palette,
    icon: IconName,
    title: SharedString,
    subtitle: Option<SharedString>,
    control: gpui::AnyElement,
) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(16.0))
        .py(px(16.0))
        .border_t_1()
        .border_color(colors.border)
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(13.0))
                .child(Icon::new(icon).size(IconSize::Lg))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(3.0))
                        .child(
                            div()
                                .text_size(px(14.0))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child(title),
                        )
                        .when(subtitle.is_some(), |column| {
                            column.child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(colors.fg_muted)
                                    .child(subtitle.unwrap_or_default()),
                            )
                        }),
                ),
        )
        .child(control)
}

impl PowerPico {
    /// Adds a metric to the waveform layout and persists it.
    pub fn add_metric(&mut self, metric: MetricId) {
        let mut metrics = self.settings.waveform_metrics.clone();
        if metrics.contains(&metric) {
            return;
        }
        metrics.push(metric);
        self.commit_metrics(metrics);
    }

    pub fn remove_metric(&mut self, metric: MetricId) {
        let mut metrics = self.settings.waveform_metrics.clone();
        if metrics.len() <= 1 {
            return;
        }
        metrics.retain(|item| *item != metric);
        self.commit_metrics(metrics);
    }

    pub fn move_metric(&mut self, metric: MetricId, delta: i32) {
        let mut metrics = self.settings.waveform_metrics.clone();
        let Some(from) = metrics.iter().position(|item| *item == metric) else {
            return;
        };
        let to = from as i32 + delta;
        if to < 0 || to as usize >= metrics.len() {
            return;
        }
        metrics.swap(from, to as usize);
        self.commit_metrics(metrics);
    }

    fn commit_metrics(&mut self, metrics: Vec<MetricId>) {
        if metrics.is_empty() {
            return;
        }
        let mut settings = self.settings.clone();
        settings.waveform_metrics = metrics;
        settings.firmware_mode = self.settings.firmware_mode.clone();
        self.save_settings(settings);
    }
}
