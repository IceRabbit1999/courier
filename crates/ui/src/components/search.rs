use crate::{
    icons::action,
    theme::{colors, font_size, spacing},
};

pub fn search_input(ui: &mut egui::Ui, placeholder: &str, value: &mut String) {
    let palette = colors();

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(action::SEARCH).size(font_size::BODY).color(palette.text_muted));
        ui.add(
            egui::TextEdit::singleline(value)
                .hint_text(placeholder)
                .desired_width(ui.available_width())
                .font(egui::FontId::proportional(font_size::BODY))
                .margin(egui::Vec2::splat(spacing::SMALL)),
        );
    });
}
