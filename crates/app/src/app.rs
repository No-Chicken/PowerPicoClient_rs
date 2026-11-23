//! The application root: shell, navigation, the event pump and the toast layer.

use std::{sync::Arc, time::Duration};

use ely_gpui_component::{
    primitives::{FocusScope, Icon, IconName},
    theme::{ActiveTheme, Mode, Theme},
    typography::Heading,
};
use gpui::prelude::*;
use gpui::{
    div, ease_in_out, prelude::FluentBuilder, px, Animation, AnimationExt, Context, FocusHandle,
    FontWeight, IntoElement, ParentElement, Render, SharedString, Styled, Window,
};
use powerpico_core::{AppPaths, AppSettings, CaptureStatus, ThemeMode};

use crate::{
    i18n::{t, Lang},
    model::{
        metric_key, metric_tone, metric_tone_color, metric_value, CalibrationOp, CalibrationUi,
        Command, Confirmation, CoreEvent, Effect, FirmwareUi, Page, SettingsUi, Toast, Tone,
        WaveformState,
    },
    pages, theme as palette,
};

/// The root view. One entity owns every screen's state so a worker result never
/// has to hop between views.
pub struct PowerPico {
    pub core: Arc<powerpico_core::AppState>,
    pub events: crossbeam_channel::Receiver<(&'static str, serde_json::Value)>,
    pub commands: Vec<Command>,
    pub page: Page,
    pub settings: AppSettings,
    pub lang: Lang,
    pub focus: FocusHandle,
    pub toast: Option<Toast>,
    pub pending: Option<Confirmation>,
    pub waveform: WaveformState,
    pub calibration: CalibrationUi,
    pub firmware: FirmwareUi,
    pub settings_ui: SettingsUi,
    pub applied_mode: Option<Mode>,
    pub applied_scale: u16,
    pub chart_drag: Option<crate::model::DragState>,
    pub window_width: f32,
    pub quit_requested: bool,
    pub startup_check: bool,
}

impl PowerPico {
    pub fn new(paths: AppPaths, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();
        let events = powerpico_core::Events::new(move |name, value| {
            let _ = tx.send((name, value));
        });
        let core = match powerpico_core::AppState::new(paths.clone(), events) {
            Ok(core) => Arc::new(core),
            Err(error) => {
                tracing::error!("failed to start core: {error}");
                let events = powerpico_core::Events::noop();
                Arc::new(
                    powerpico_core::AppState::new(paths.clone(), events)
                        .expect("core paths are unusable"),
                )
            }
        };
        let mut settings = core.settings();
        // Developer aid: `POWERPICO_THEME=light|dark|system` overrides the
        // stored choice for one run, which makes both palettes easy to compare.
        if let Ok(forced) = std::env::var("POWERPICO_THEME") {
            settings.theme = match forced.as_str() {
                "light" => ThemeMode::Light,
                "dark" => ThemeMode::Dark,
                _ => ThemeMode::System,
            };
        }
        let focus = cx.focus_handle();
        window.focus(&focus, cx);

        let mut app = Self {
            core,
            events: rx,
            commands: Vec::new(),
            page: std::env::var("POWERPICO_PAGE")
                .ok()
                .and_then(|page| match page.as_str() {
                    "calibration" => Some(Page::Calibration),
                    "firmware" => Some(Page::Firmware),
                    "settings" => Some(Page::Settings),
                    "waveform" => Some(Page::Waveform),
                    _ => None,
                })
                .unwrap_or(Page::Waveform),
            lang: Lang::resolve(settings.language.clone()),
            settings,
            focus,
            toast: None,
            pending: None,
            waveform: WaveformState::default(),
            calibration: CalibrationUi::default(),
            firmware: FirmwareUi::default(),
            settings_ui: SettingsUi::default(),
            applied_mode: None,
            applied_scale: 0,
            chart_drag: None,
            window_width: 1180.0,
            quit_requested: false,
            startup_check: true,
        };
        tracing::info!(page = ?app.page, lang = app.lang.tag(), "window opened");
        app.seed();
        app.start_pump(cx);
        app
    }

    /// Loads everything the first frame needs; the pump runs these off the UI
    /// thread so the window paints immediately.
    fn seed(&mut self) {
        self.commands.push(Command::RefreshDevices);
        self.commands.push(Command::RefreshCalibration);
        self.commands.push(Command::RefreshStats(None));
        self.commands.push(Command::FetchFirmwareInfo);
        self.commands.push(Command::FetchLinks);
        self.commands.push(Command::RefreshStorage);
        self.commands.push(Command::RefreshUninstall);
        self.commands.push(Command::RefreshPlot {
            start: 0.0,
            end: 10.0,
            width: 900.0,
        });
        if self.settings.check_update_at_startup {
            self.commands.push(Command::CheckUpdate);
        }
    }

    /// Drains worker events and runs queued commands off the UI thread.
    fn start_pump(&self, cx: &mut Context<Self>) {
        let rx = self.events.clone();
        let core = self.core.clone();

        cx.spawn(async move |this, cx| loop {
            cx.background_executor()
                .timer(Duration::from_millis(33))
                .await;

            let mut events = Vec::new();
            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }
            let toast_expired = this
                .update(cx, |app, _| {
                    app.toast
                        .as_ref()
                        .is_some_and(|toast| toast.at.elapsed().as_secs_f32() > 3.5)
                })
                .unwrap_or(false);

            if !events.is_empty() {
                this.update(cx, |app, cx| {
                    for (name, value) in events {
                        app.handle_event(name, value);
                    }
                    cx.notify();
                })
                .ok();
            }
            if toast_expired {
                this.update(cx, |app, cx| {
                    app.toast = None;
                    cx.notify();
                })
                .ok();
            }

            let commands = this
                .update(cx, |app, _| std::mem::take(&mut app.commands))
                .unwrap_or_default();
            for command in commands {
                let core = core.clone();
                let effect = cx
                    .background_executor()
                    .spawn(async move { crate::model::run(command, &core) })
                    .await;
                this.update(cx, |app, cx| {
                    app.apply(effect);
                    if app.quit_requested {
                        cx.quit();
                    }
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    // ---------------------------------------------------------------- events

    pub fn tr(&self, key: &str) -> SharedString {
        t(self.lang, key)
    }

    pub fn queue(&mut self, command: Command) {
        self.commands.push(command);
    }

    pub fn notify(&mut self, message: impl Into<String>, tone: Tone) {
        let message = message.into();
        if message.is_empty() {
            return;
        }
        self.toast = Some(Toast {
            message,
            tone,
            at: std::time::Instant::now(),
        });
    }

    fn handle_event(&mut self, name: &str, value: serde_json::Value) {
        let Some(event) = CoreEvent::parse(name, value) else {
            return;
        };
        match event {
            CoreEvent::Capture(state) => {
                let started = self.waveform.capture.status != CaptureStatus::Capturing
                    && state.status == CaptureStatus::Capturing;
                if let Some(id) = &state.device_id {
                    self.waveform.selected_device = id.clone();
                }
                self.waveform.capture = state;
                if started {
                    self.return_to_live();
                    self.commands.push(Command::RefreshPlot {
                        start: self.waveform.time_view.min,
                        end: self.waveform.time_view.max,
                        width: 1200.0,
                    });
                }
            }
            CoreEvent::Summary(summary) => {
                self.waveform.latest_end = summary.duration;
                if self.waveform.stats_window.is_none() {
                    self.waveform.summary = summary.clone();
                }
                self.waveform.total = summary;
                if self.waveform.follow_live {
                    self.waveform.time_view = self.waveform.live_range();
                    self.request_plot();
                }
            }
            CoreEvent::DataReady => {
                if self.page == Page::Waveform {
                    self.request_plot();
                }
            }
            CoreEvent::Disconnected(message) => {
                self.notify(
                    if message.is_empty() {
                        String::from("device disconnected")
                    } else {
                        message
                    },
                    Tone::Error,
                );
            }
            CoreEvent::Calibration(state) => {
                self.calibration.apply(state);
            }
            CoreEvent::Firmware(progress, finished) => {
                let stage = progress.message_key.clone();
                self.firmware.progress = progress;
                if finished {
                    let tone = if stage == "completed" {
                        Tone::Success
                    } else {
                        Tone::Error
                    };
                    let text = self.tr(&format!("firmware.{stage}"));
                    self.notify(text, tone);
                }
            }
            CoreEvent::Download(progress) => {
                let stage = progress.stage.clone();
                self.firmware.download = Some(progress);
                match stage.as_str() {
                    "completed" => {
                        self.notify(self.tr("firmware.downloadCompleted"), Tone::Success);
                        self.commands.push(Command::FetchFirmwareInfo);
                    }
                    "cancelled" => self.notify(self.tr("firmware.downloadCancelled"), Tone::Info),
                    "failed" => self.notify(self.tr("firmware.downloadFailed"), Tone::Error),
                    _ => {}
                }
            }
        }
    }

    // --------------------------------------------------------------- effects

    fn apply(&mut self, effect: Effect) {
        match effect {
            Effect::Devices(result) => match result {
                Ok(devices) => {
                    self.waveform.devices = devices.clone();
                    self.calibration.devices = devices.clone();
                    self.firmware.devices = devices;
                    let valid = |selected: &str, list: &[powerpico_core::SerialDevice]| {
                        list.iter().any(|device| device.id == selected)
                    };
                    if !valid(&self.waveform.selected_device, &self.waveform.devices) {
                        self.waveform.selected_device.clear();
                    }
                    if !valid(&self.calibration.selected_device, &self.calibration.devices) {
                        self.calibration.selected_device.clear();
                    }
                    if !valid(&self.firmware.selected_device, &self.firmware.devices) {
                        self.firmware.selected_device.clear();
                    }
                    // Developer aid: `POWERPICO_AUTOSTART=<device id>` (or `1`
                    // for the first device) starts capture on launch so the chart
                    // can be inspected without a click.
                    if let Ok(wanted) = std::env::var("POWERPICO_AUTOSTART") {
                        if self.waveform.capture.status == CaptureStatus::Idle
                            && !self.waveform.devices.is_empty()
                        {
                            let device = if wanted.is_empty() || wanted == "1" {
                                self.waveform.devices[0].id.clone()
                            } else {
                                wanted
                            };
                            self.waveform.selected_device = device.clone();
                            self.commands.push(Command::StartCapture(device));
                        }
                    }
                }
                Err(error) => self.notify(error, Tone::Error),
            },
            Effect::Capture(result) => match result {
                Ok(state) => {
                    self.waveform.capture = state;
                    self.waveform.render_pending = false;
                }
                Err(error) => {
                    self.waveform.capture.status = CaptureStatus::Idle;
                    self.notify(error, Tone::Error);
                }
            },
            Effect::Plot { start, end, result } => {
                self.waveform.render_pending = false;
                if let Ok(series) = result {
                    if !series.time.is_empty()
                        || self.waveform.capture.status != CaptureStatus::Capturing
                    {
                        self.waveform.latest_end =
                            self.waveform.latest_end.max(series.available_end);
                        self.waveform.series = series;
                    }
                }
                let _ = (start, end);
                if self.waveform.render_queued {
                    self.waveform.render_queued = false;
                    self.request_plot();
                }
            }
            Effect::Stats { window, result } => {
                if let Ok(summary) = result {
                    if window.is_none() {
                        self.waveform.summary = summary.clone();
                    }
                }
            }
            Effect::Range(result) => match result {
                Ok(stats) => {
                    self.waveform.analysis_draft =
                        Some(crate::model::Range::new(stats.start, stats.end));
                    self.waveform.analysis = Some(stats);
                    self.waveform.follow_live = false;
                }
                Err(error) => self.notify(error, Tone::Error),
            },
            Effect::Point(result) => match result {
                Ok(point) => self.waveform.selected_point = Some(point),
                Err(_) => self.waveform.selected_point = None,
            },
            Effect::Exported(result) => match result {
                Ok(_) => self.notify(self.tr("waveform.exportSuccess"), Tone::Success),
                Err(error) => self.notify(error, Tone::Error),
            },
            Effect::FirmwareInfo(result) => {
                self.firmware.loading_official = false;
                match result {
                    Ok(info) => self.firmware.official = Some(info),
                    Err(error) => self.notify(error, Tone::Error),
                }
            }
            Effect::FirmwareStarted(result) => {
                if let Err(error) = result {
                    self.notify(error, Tone::Error);
                }
            }
            Effect::Calibration(result) => {
                self.calibration.pending = false;
                match result {
                    Ok(state) => self.calibration.apply(state),
                    Err(error) => self.notify(error, Tone::Error),
                }
            }
            Effect::CalibrationStarted(result) => {
                self.calibration.pending = false;
                if let Err(error) = result {
                    self.notify(error, Tone::Error);
                }
            }
            Effect::Settings(result) => match result {
                Ok(settings) => {
                    self.lang = Lang::resolve(settings.language.clone());
                    self.settings = settings;
                }
                Err(error) => self.notify(error, Tone::Error),
            },
            Effect::Storage(result) => {
                self.settings_ui.refreshing_storage = false;
                self.settings_ui.clearing_cache = false;
                match result {
                    Ok(usage) => self.settings_ui.storage = Some(usage),
                    Err(error) => self.notify(error, Tone::Error),
                }
            }
            Effect::Uninstall(result) => match result {
                Ok(info) => self.settings_ui.uninstall = Some(info),
                Err(error) => self.notify(error, Tone::Error),
            },
            Effect::UpdateCheck(result) => {
                self.settings_ui.checking_update = false;
                let silent = std::mem::take(&mut self.startup_check);
                match result {
                    Ok(info) if silent => {
                        if info.update_available {
                            self.settings_ui.update_version = Some(info.latest_version.clone());
                        }
                        self.settings_ui.update = Some(info);
                    }
                    Ok(info) => {
                        if info.update_available {
                            self.settings_ui.update_version = Some(info.latest_version.clone());
                            self.notify(
                                format!(
                                    "{}: {}",
                                    self.tr("settings.updateAvailable"),
                                    info.latest_version
                                ),
                                Tone::Info,
                            );
                        } else {
                            self.notify(self.tr("settings.updateAvailable"), Tone::Success);
                        }
                        self.settings_ui.update = Some(info);
                    }
                    Err(error) if !silent => self.notify(error, Tone::Error),
                    Err(_) => {}
                }
            }
            Effect::Links(result) => match result {
                Ok(links) => self.settings_ui.links = Some(links),
                Err(error) => self.notify(error, Tone::Error),
            },
            Effect::Notify(message, tone) => self.notify(message, tone),
            Effect::Exit => self.quit_requested = true,
        }
    }

    // -------------------------------------------------------------- settings

    pub fn save_settings(&mut self, next: AppSettings) {
        self.apply_local_settings(&next);
        self.commands.push(Command::UpdateSettings(next));
    }

    fn apply_local_settings(&mut self, settings: &AppSettings) {
        self.lang = Lang::resolve(settings.language.clone());
        self.settings = settings.clone();
        // `applied_mode` is left to `render`: it is the record of what the
        // theme actually shows, not of what was requested. Writing the request
        // here made the next frame believe the change had already landed.
    }

    // --------------------------------------------------------------- helpers

    pub fn return_to_live(&mut self) {
        self.waveform.follow_live = true;
        self.waveform.time_view = self.waveform.live_range();
        self.request_plot();
    }

    pub fn request_plot(&mut self) {
        if self.waveform.render_pending {
            self.waveform.render_queued = true;
            return;
        }
        self.waveform.render_pending = true;
        self.waveform.request_revision += 1;
        self.commands.push(Command::RefreshPlot {
            start: self.waveform.time_view.min,
            end: self.waveform.time_view.max,
            width: ((self.window_width * 0.72) as f64).max(500.0),
        });
    }

    pub fn connected(&self) -> bool {
        matches!(
            self.waveform.capture.status,
            CaptureStatus::Connecting | CaptureStatus::Capturing
        ) || self.calibration.state.connected
    }

    pub fn confirm(&mut self, confirmation: Confirmation) {
        self.pending = Some(confirmation);
    }

    pub fn resolve_pending(&mut self, accepted: bool, cx: &mut Context<Self>) {
        let Some(confirmation) = self.pending.take() else {
            return;
        };
        if !accepted {
            cx.notify();
            return;
        }
        match confirmation {
            Confirmation::AutoZero => {
                self.queue(Command::Calibration(CalibrationOp::Auto));
            }
            Confirmation::Reset => {
                self.queue(Command::Calibration(CalibrationOp::Reset));
            }
            Confirmation::Manual(write) => {
                self.queue(Command::Calibration(CalibrationOp::Manual {
                    range: write.range,
                    gain: write.gain,
                    offset_ua: write.offset_ua,
                }))
            }
            Confirmation::ClearRecords => {
                self.waveform.summary = Default::default();
                self.waveform.total = Default::default();
                self.waveform.analysis = None;
                self.waveform.analysis_draft = None;
                self.waveform.series = Default::default();
                self.waveform.latest_end = 0.0;
                self.waveform.selected_point = None;
                self.waveform.time_view = crate::model::Range::new(0.0, 10.0);
                self.waveform.follow_live = true;
                self.queue(Command::ClearRecords)
            }
            Confirmation::ClearCache => {
                self.settings_ui.clearing_cache = true;
                self.queue(Command::ClearCache);
            }
            Confirmation::Uninstall => {
                self.settings_ui.uninstalling = true;
                self.queue(Command::Uninstall);
            }
        }
        cx.notify();
    }

    pub fn open_link(&mut self, url: Option<String>) {
        match url {
            Some(url) => self.queue(Command::OpenExternal(url)),
            None => self.notify(self.tr("errors.externalLink"), Tone::Error),
        }
    }

    // ------------------------------------------------------------------ view

    fn sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let (colors, dark) = {
            let theme = cx.theme();
            (theme.colors.clone(), theme.is_dark())
        };
        let lang = self.lang;
        let page = self.page;
        let connected = self.connected();
        let this = cx.entity().downgrade();

        let mut nav = div().flex().flex_col().gap(px(5.0));
        for candidate in [
            Page::Waveform,
            Page::Calibration,
            Page::Firmware,
            Page::Settings,
        ] {
            let active = candidate == page;
            let this = this.clone();
            let (accent, accent_soft, text, hover, muted) = (
                colors.accent,
                colors.info_subtle,
                colors.fg,
                colors.hover,
                colors.fg_muted,
            );
            nav = nav.child(
                div()
                    .id(("nav", candidate as usize))
                    .h(px(42.0))
                    .w_full()
                    .px(px(11.0))
                    .flex()
                    .items_center()
                    .gap(px(11.0))
                    .rounded(px(11.0))
                    .cursor_pointer()
                    .when(active, |style| style.bg(accent_soft).text_color(accent))
                    .when(!active, |style| style.text_color(muted))
                    .hover(move |style| {
                        if active {
                            style
                        } else {
                            style.bg(hover).text_color(text)
                        }
                    })
                    .child(
                        Icon::new(candidate.icon()).size(ely_gpui_component::theme::IconSize::Md),
                    )
                    .child(
                        div()
                            .text_size(px(13.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(t(lang, candidate.label_key())),
                    )
                    .on_click(move |_, _, cx| {
                        this.update(cx, |app, cx| {
                            app.page = candidate;
                            cx.notify();
                        })
                        .ok();
                    }),
            );
        }

        div()
            .flex()
            .flex_col()
            .w(px(palette::SIDEBAR_WIDTH))
            .flex_none()
            .h_full()
            .rounded(px(19.0))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface)
            .px(px(12.0))
            .pt(px(18.0))
            .pb(px(13.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .px(px(8.0))
                    .pb(px(22.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(36.0))
                            .rounded(px(11.0))
                            .bg(gpui::linear_gradient(
                                145.0,
                                gpui::linear_color_stop(gpui::rgb(0x246bfd), 0.0),
                                gpui::linear_color_stop(gpui::rgb(0x56a6ff), 1.0),
                            ))
                            .text_color(gpui::white())
                            .child(
                                Icon::new(IconName::Zap)
                                    .size(ely_gpui_component::theme::IconSize::Md),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(15.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(colors.fg)
                                    .child("PowerPico"),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(colors.fg_muted)
                                    .child("Client"),
                            ),
                    ),
            )
            .child(nav)
            .child(div().flex_1())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(9.0))
                    .px(px(9.0))
                    .pt(px(12.0))
                    .pb(px(4.0))
                    .child(div().w(px(8.0)).h(px(8.0)).rounded_full().bg(if connected {
                        palette::series(if dark { Mode::Dark } else { Mode::Light }).2
                    } else {
                        colors.fg_subtle
                    }))
                    .child(div().text_size(px(11.0)).text_color(colors.fg_muted).child(
                        if connected {
                            t(lang, "app.connected")
                        } else {
                            t(lang, "app.disconnected")
                        },
                    )),
            )
    }

    fn toast_layer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors.clone();
        let toast = self.toast.clone();
        let visible = toast.is_some();
        let (message, tone) = match toast {
            Some(toast) => (toast.message, toast.tone),
            None => (String::new(), Tone::Info),
        };
        let (bg, border) = match tone {
            Tone::Info => (colors.info_subtle, colors.info),
            Tone::Error => (colors.danger_subtle, colors.danger),
            Tone::Success => (colors.success_subtle, colors.success),
        };
        div()
            .absolute()
            .bottom(px(22.0))
            .right(px(22.0))
            .when(visible, |layer| {
                layer.child(
                    div()
                        .px(px(16.0))
                        .py(px(11.0))
                        .rounded(px(12.0))
                        .border_1()
                        .border_color(border)
                        .bg(bg)
                        .text_color(colors.fg)
                        .text_size(px(13.0))
                        .shadow(vec![gpui::BoxShadow {
                            color: colors.shadow,
                            offset: gpui::point(px(0.0), px(8.0)),
                            blur_radius: px(24.0),
                            spread_radius: px(0.0),
                            inset: false,
                        }])
                        .child(message)
                        .with_animation(
                            "toast-in",
                            Animation::new(Duration::from_millis(180)).with_easing(ease_in_out),
                            |element, progress| element.opacity(progress),
                        ),
                )
            })
    }

    fn confirmation_layer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors.clone();
        let pending = self.pending.clone();
        let this = cx.entity().downgrade();
        let body = pending
            .as_ref()
            .and_then(|confirmation| confirmation.body(self.lang));
        let title = pending
            .as_ref()
            .map(|confirmation| self.tr(confirmation.title_key()))
            .unwrap_or_default();
        let confirm_label = pending
            .as_ref()
            .map(|confirmation| self.tr(confirmation.confirm_key()))
            .unwrap_or_default();
        div().absolute().inset_0().when(pending.is_some(), |layer| {
            layer
                .bg(gpui::hsla(0.0, 0.0, 0.0, 0.42))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w(px(520.0))
                        .rounded(px(18.0))
                        .border_1()
                        .border_color(colors.border)
                        .bg(colors.overlay)
                        .p(px(25.0))
                        .flex()
                        .flex_col()
                        .gap(px(14.0))
                        .child(Heading::h3(title).text_color(colors.fg))
                        .when(body.is_some(), |card| {
                            card.child(
                                div()
                                    .text_size(px(13.0))
                                    .line_height(px(22.0))
                                    .text_color(colors.fg_muted)
                                    .child(body.unwrap_or_default()),
                            )
                        })
                        .child(
                            div()
                                .flex()
                                .justify_end()
                                .gap(px(10.0))
                                .child(
                                    ely_gpui_component::buttons::Button::new(
                                        "confirm-cancel",
                                        self.tr("common.cancel"),
                                    )
                                    .on_click({
                                        let this = this.clone();
                                        move |_, _, cx| {
                                            this.update(cx, |app, cx| {
                                                app.resolve_pending(false, cx)
                                            })
                                            .ok();
                                        }
                                    }),
                                )
                                .child(
                                    ely_gpui_component::buttons::Button::new(
                                        "confirm-ok",
                                        confirm_label,
                                    )
                                    .primary()
                                    .on_click({
                                        let this = this.clone();
                                        move |_, _, cx| {
                                            this.update(cx, |app, cx| {
                                                app.resolve_pending(true, cx)
                                            })
                                            .ok();
                                        }
                                    }),
                                ),
                        ),
                )
        })
    }
}

impl Render for PowerPico {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // System theme follows the window; a stored choice wins over it. Apply
        // the mode before reading colors so this frame already paints with it.
        let wanted = match self.settings.theme {
            ThemeMode::Light => Mode::Light,
            ThemeMode::Dark => Mode::Dark,
            ThemeMode::System => match window.appearance() {
                gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark => Mode::Dark,
                _ => Mode::Light,
            },
        };
        if self.applied_mode != Some(wanted) {
            Theme::set_mode_now(wanted, cx);
            self.applied_mode = Some(wanted);
        }
        if self.applied_scale != self.settings.ui_scale {
            let scale = if self.settings.ui_scale == 0 {
                1.0
            } else {
                self.settings.ui_scale as f32 / 100.0
            };
            Theme::update(cx, |theme| theme.font_scale = scale);
            self.applied_scale = self.settings.ui_scale;
        }
        let colors = cx.theme().colors.clone();
        self.window_width = window.viewport_size().width.as_f32();

        let body = match self.page {
            Page::Waveform => pages::waveform::page(self, window, cx).into_any_element(),
            Page::Calibration => pages::calibration::page(self, window, cx).into_any_element(),
            Page::Firmware => pages::firmware::page(self, window, cx).into_any_element(),
            Page::Settings => pages::settings::page(self, window, cx).into_any_element(),
        };

        let content = div()
            .id("page-scroll")
            .flex()
            .flex_col()
            .items_stretch()
            .justify_start()
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_y_scroll()
            .pr(px(8.0))
            .pb(px(8.0))
            .child(div().w_full().min_w_0().child(body));

        FocusScope::new(&self.focus)
            .root()
            .size_full()
            .bg(colors.bg)
            .text_color(colors.fg)
            .p(px(14.0))
            .flex()
            .gap(px(palette::SHELL_GAP))
            .child(self.sidebar(cx))
            .child(content)
            .child(self.toast_layer(cx))
            .child(self.confirmation_layer(cx))
            .into_any_element()
    }
}

/// A rounded, translucent panel, the shell of every card in the client.
pub fn panel(colors: &ely_gpui_component::theme::Palette) -> gpui::Div {
    div()
        .rounded(px(palette::PANEL_RADIUS))
        .border_1()
        .border_color(colors.border)
        .bg(colors.surface)
        .shadow(vec![gpui::BoxShadow {
            color: colors.shadow,
            offset: gpui::point(px(0.0), px(10.0)),
            blur_radius: px(32.0),
            spread_radius: px(0.0),
            inset: false,
        }])
}

/// The page title block: heading plus subtitle.
pub fn page_header(
    title: SharedString,
    subtitle: SharedString,
    colors: &ely_gpui_component::theme::Palette,
) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .pb(px(16.0))
        .child(
            div()
                .text_size(px(25.0))
                .font_weight(FontWeight::BOLD)
                .text_color(colors.fg)
                .child(title),
        )
        .child(
            div()
                .text_size(px(13.0))
                .text_color(colors.fg_muted)
                .child(subtitle),
        )
}

/// A metric card, the compact stat tiles above the chart.
pub fn metric_card(
    id: powerpico_core::MetricId,
    summary: &powerpico_core::CaptureSummary,
    lang: Lang,
    colors: &ely_gpui_component::theme::Palette,
    series: (gpui::Hsla, gpui::Hsla, gpui::Hsla),
) -> gpui::Div {
    let tone = metric_tone_color(metric_tone(id), series.0, series.1, series.2);
    div()
        .flex()
        .flex_col()
        .gap(px(4.0))
        .px(px(13.0))
        .py(px(11.0))
        .min_w(px(120.0))
        .flex_1()
        .rounded(px(12.0))
        .border_1()
        .border_color(colors.border)
        .bg(colors.surface)
        .child(
            div()
                .text_size(px(11.0))
                .text_color(colors.fg_muted)
                .child(t(lang, metric_key(id))),
        )
        .child(
            div()
                .text_size(px(16.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(tone.unwrap_or(colors.fg))
                .child(metric_value(id, summary)),
        )
}

/// A labelled control: the label above, the control below.
pub fn field(
    label: SharedString,
    control: impl IntoElement,
    colors: &ely_gpui_component::theme::Palette,
) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .flex_1()
        .min_w_0()
        .child(
            div()
                .text_size(px(13.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(colors.fg)
                .child(label),
        )
        .child(control)
}
