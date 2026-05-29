use egui::Vec2;

use crate::{
    icons::status,
    theme::{colors, font_size, radius, spacing},
};

pub fn card(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    let palette = colors();

    egui::Frame::NONE
        .fill(palette.surface)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .corner_radius(egui::CornerRadius::same(radius::MEDIUM))
        .inner_margin(spacing::MEDIUM)
        .show(ui, |ui| {
            add_contents(ui);
        });
}

pub fn match_card(ui: &mut egui::Ui, hero_name: &str, kda: &str, is_victory: bool, duration: &str, time_ago: &str) {
    let palette = colors();
    let outcome_color = if is_victory { palette.victory } else { palette.defeat };
    let outcome_text = if is_victory { i18n::message("match-victory") } else { i18n::message("match-defeat") };
    let outcome_icon = if is_victory { status::VICTORY } else { status::DEFEAT };

    egui::Frame::NONE
        .fill(palette.surface)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .corner_radius(egui::CornerRadius::same(radius::MEDIUM))
        .inner_margin(spacing::MEDIUM)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(hero_name).size(font_size::MEDIUM).color(palette.text));
                    ui.label(egui::RichText::new(format!("KDA: {}", kda)).size(font_size::BODY).color(palette.text_secondary));
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    ui.vertical(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                            ui.label(egui::RichText::new(outcome_text).size(font_size::SMALL).color(outcome_color));
                            ui.label(egui::RichText::new(outcome_icon).size(font_size::SMALL).color(outcome_color));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                            ui.label(egui::RichText::new(duration).size(font_size::SMALL).color(palette.text_muted));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                            ui.label(egui::RichText::new(time_ago).size(font_size::SMALL).color(palette.text_muted));
                        });
                    });
                });
            });
        });
}

pub fn player_card(ui: &mut egui::Ui, name: &str, rank: Option<&str>, last_match: Option<&str>) {
    let palette = colors();

    egui::Frame::NONE
        .fill(palette.surface)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .corner_radius(egui::CornerRadius::same(radius::MEDIUM))
        .inner_margin(spacing::MEDIUM)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width() / 3.0 - spacing::SMALL);
            ui.label(egui::RichText::new(name).size(font_size::MEDIUM).color(palette.text));

            let rank_text = rank.map(|r| r.to_owned()).unwrap_or_else(|| i18n::message("stats-unranked"));
            ui.label(egui::RichText::new(rank_text).size(font_size::SMALL).color(palette.text_secondary));

            let match_text = last_match.map(|m| m.to_owned()).unwrap_or_else(|| i18n::message("matches-empty"));
            ui.label(egui::RichText::new(match_text).size(font_size::SMALL).color(palette.text_muted));
        });
}

pub fn hero_card(ui: &mut egui::Ui, name: &str, attribute: &str) {
    let palette = colors();

    egui::Frame::NONE
        .fill(palette.surface)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .corner_radius(egui::CornerRadius::same(radius::MEDIUM))
        .inner_margin(spacing::MEDIUM)
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(120.0, 140.0));
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new(name).size(font_size::BODY).color(palette.text));
                ui.label(egui::RichText::new(attribute).size(font_size::SMALL).color(palette.text_secondary));
            });
        });
}

pub fn item_card(ui: &mut egui::Ui, name: &str, cost: Option<u32>) {
    let palette = colors();

    egui::Frame::NONE
        .fill(palette.surface)
        .stroke(egui::Stroke::new(1.0, palette.border))
        .corner_radius(egui::CornerRadius::same(radius::MEDIUM))
        .inner_margin(spacing::SMALL)
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(80.0, 100.0));
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new(name).size(font_size::SMALL).color(palette.text));
                if let Some(c) = cost {
                    ui.label(egui::RichText::new(c.to_string()).size(font_size::SMALL).color(palette.warning));
                }
            });
        });
}
