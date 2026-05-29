use std::time::{Duration, Instant};

use egui::Vec2;
use tracing::error;

use crate::{
    icons::action,
    theme::{colors, font_size, radius, spacing},
};

/// A toast event sent through the channel from anywhere in the app.
pub struct ToastEvent {
    pub title: String,
    pub message: String,
    pub status: ToastStatus,
}

/// A cloneable handle for sending toast notifications from any context.
///
/// Wraps an `mpsc::UnboundedSender<ToastEvent>` so that both sync and async
/// code can fire toast notifications without direct access to `ToastManager`.
#[derive(Clone)]
pub struct ToastSender {
    tx: tokio::sync::mpsc::UnboundedSender<ToastEvent>,
}

impl ToastSender {
    pub fn new(tx: tokio::sync::mpsc::UnboundedSender<ToastEvent>) -> Self {
        Self { tx }
    }

    fn send(&self, title: impl Into<String>, message: impl Into<String>, status: ToastStatus) {
        let event = ToastEvent {
            title: title.into(),
            message: message.into(),
            status,
        };
        if let Err(e) = self.tx.send(event) {
            error!("Failed to send toast event: {e}");
        }
    }

    pub fn error(&self, title: impl Into<String>, message: impl Into<String>) {
        self.send(title, message, ToastStatus::Error);
    }

    pub fn success(&self, title: impl Into<String>, message: impl Into<String>) {
        self.send(title, message, ToastStatus::Success);
    }

    pub fn warning(&self, title: impl Into<String>, message: impl Into<String>) {
        self.send(title, message, ToastStatus::Warning);
    }

    pub fn info(&self, title: impl Into<String>, message: impl Into<String>) {
        self.send(title, message, ToastStatus::Info);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastStatus {
    Success,
    Error,
    Warning,
    Info,
}

impl ToastStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            ToastStatus::Success => action::CHECK,
            ToastStatus::Error => action::CLOSE,
            ToastStatus::Warning => action::WARNING,
            ToastStatus::Info => action::INFO,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub id: usize,
    pub title: String,
    pub message: String,
    pub status: ToastStatus,
    pub created_at: Instant,
    pub duration: Duration,
}

impl Toast {
    pub fn new(id: usize, title: impl Into<String>, message: impl Into<String>, status: ToastStatus) -> Self {
        Self {
            id,
            title: title.into(),
            message: message.into(),
            status,
            created_at: Instant::now(),
            duration: Duration::from_secs(configs::read().appearance.toast_duration_secs),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.duration
    }
}

#[derive(Debug, Default)]
pub struct ToastManager {
    toasts: Vec<Toast>,
    next_id: usize,
}

impl ToastManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_error(&mut self, title: impl Into<String>, message: impl Into<String>) {
        self.push(title, message, ToastStatus::Error);
    }

    pub fn push_success(&mut self, title: impl Into<String>, message: impl Into<String>) {
        self.push(title, message, ToastStatus::Success);
    }

    pub fn push_warning(&mut self, title: impl Into<String>, message: impl Into<String>) {
        self.push(title, message, ToastStatus::Warning);
    }

    pub fn push_info(&mut self, title: impl Into<String>, message: impl Into<String>) {
        self.push(title, message, ToastStatus::Info);
    }

    pub fn drain(&mut self, rx: &mut tokio::sync::mpsc::UnboundedReceiver<ToastEvent>) {
        while let Ok(event) = rx.try_recv() {
            self.push(event.title, event.message, event.status);
        }
    }

    fn push(&mut self, title: impl Into<String>, message: impl Into<String>, status: ToastStatus) {
        self.toasts.push(Toast::new(self.next_id, title, message, status));
        self.next_id += 1;
    }

    pub fn has_toasts(&self) -> bool {
        !self.toasts.is_empty()
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        self.toasts.retain(|t| !t.is_expired());

        if self.toasts.is_empty() {
            return;
        }

        ctx.request_repaint_after(Duration::from_millis(16));

        let palette = colors();
        let appearance = &configs::read().appearance;
        let toast_speed = appearance.animation_toast_speed;
        let toast_max_width = appearance.toast_max_width;

        egui::Area::new(egui::Id::new("toast_area"))
            .anchor(egui::Align2::RIGHT_BOTTOM, Vec2::new(-spacing::MEDIUM, -spacing::MEDIUM))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.set_max_width(toast_max_width);

                let mut to_dismiss = Vec::new();

                for toast in self.toasts.iter().rev().take(5) {
                    let age = toast.created_at.elapsed().as_secs_f32();
                    let slide_progress = (age * toast_speed).min(1.0);
                    let slide_offset = (1.0 - slide_progress) * 40.0;

                    let (border_color, icon_color) = match toast.status {
                        ToastStatus::Success => (palette.success, palette.success),
                        ToastStatus::Error => (palette.danger, palette.danger),
                        ToastStatus::Warning => (palette.warning, palette.warning),
                        ToastStatus::Info => (palette.primary, palette.primary),
                    };

                    ui.add_space(slide_offset);

                    egui::Frame::NONE
                        .fill(palette.surface)
                        .stroke(egui::Stroke::new(1.0, border_color))
                        .corner_radius(egui::CornerRadius::same(radius::MEDIUM))
                        .inner_margin(spacing::MEDIUM)
                        .show(ui, |ui| {
                            ui.set_min_width(280.0);
                            ui.set_opacity(slide_progress);

                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(toast.status.icon()).size(font_size::MEDIUM).color(icon_color));
                                ui.label(egui::RichText::new(&toast.title).size(font_size::BODY).color(palette.text));

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui
                                        .add(egui::Button::new(egui::RichText::new(action::CLOSE).size(font_size::SMALL).color(palette.text_muted)).frame(false))
                                        .clicked()
                                    {
                                        to_dismiss.push(toast.id);
                                    }
                                });
                            });

                            ui.label(egui::RichText::new(&toast.message).size(font_size::SMALL).color(palette.text_secondary));
                        });

                    ui.add_space(spacing::SMALL);
                }

                for id in to_dismiss {
                    self.toasts.retain(|t| t.id != id);
                }
            });
    }
}
