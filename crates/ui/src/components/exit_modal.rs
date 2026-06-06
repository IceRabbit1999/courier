use egui::{Color32, Vec2};

use crate::theme::{colors, font_size, radius, spacing};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitAction {
    Confirm,
    Cancel,
}

#[derive(Debug, Default)]
pub struct ExitModal {
    pub visible: bool,
}

impl ExitModal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn ui(&mut self, ctx: &egui::Context) -> Option<ExitAction> {
        if !self.visible {
            return None;
        }

        let mut action = None;

        let overlay_opacity = configs::read().appearance.exit_overlay_opacity;

        // Semi-transparent overlay
        let screen_rect = ctx.content_rect();
        let overlay_layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("exit_modal_overlay"));
        let painter = ctx.layer_painter(overlay_layer);
        painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(overlay_opacity));

        egui::Window::new(i18n::message("exit-confirm-title"))
            .collapsible(false)
            .resizable(false)
            .fixed_size(Vec2::new(400.0, 0.0))
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let palette = colors();

                ui.add_space(spacing::SMALL);
                ui.label(
                    egui::RichText::new(i18n::message("exit-confirm-message"))
                        .size(font_size::BODY)
                        .color(palette.text_secondary),
                );
                ui.add_space(spacing::SMALL);
                ui.label(
                    egui::RichText::new(i18n::message("exit-confirm-save-note"))
                        .size(font_size::SMALL)
                        .color(palette.text_muted),
                );
                ui.add_space(spacing::MEDIUM);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let confirm = ui.add(
                        egui::Button::new(egui::RichText::new(i18n::message("button-exit")).size(font_size::SMALL).color(Color32::WHITE))
                            .fill(palette.danger)
                            .corner_radius(egui::CornerRadius::same(radius::MEDIUM))
                            .min_size(Vec2::new(80.0, 32.0)),
                    );
                    if confirm.clicked() {
                        action = Some(ExitAction::Confirm);
                    }

                    ui.add_space(spacing::SMALL);

                    let cancel = ui.add(
                        egui::Button::new(egui::RichText::new(i18n::message("button-cancel")).size(font_size::SMALL).color(palette.text))
                            .stroke(egui::Stroke::new(1_f32, palette.border))
                            .fill(Color32::TRANSPARENT)
                            .corner_radius(egui::CornerRadius::same(radius::MEDIUM))
                            .min_size(Vec2::new(80.0, 32.0)),
                    );
                    if cancel.clicked() {
                        action = Some(ExitAction::Cancel);
                    }
                });
            });

        action
    }
}
