use egui::{CornerRadius, Margin, Stroke};

use crate::{
    icons::action,
    theme::{colors, font_size, radius, spacing},
};

pub fn search_input(ui: &mut egui::Ui, placeholder: &str, value: &mut String) {
    let palette = colors();

    egui::Frame::NONE
        .fill(palette.surface_secondary)
        .stroke(Stroke::new(1_f32, palette.border))
        .corner_radius(CornerRadius::same(radius::MEDIUM))
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(spacing::TINY);
                ui.label(egui::RichText::new(action::SEARCH).size(font_size::BODY).color(palette.text_muted));
                ui.add_space(spacing::SMALL);
                ui.add(
                    egui::TextEdit::singleline(value)
                        .hint_text(placeholder)
                        .desired_width(ui.available_width())
                        .font(egui::FontId::proportional(font_size::BODY))
                        .frame(egui::Frame::NONE),
                );
            });
        });
}
