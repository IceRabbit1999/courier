use super::Screen;
use crate::{
    components::card,
    theme::{colors, font_size, spacing},
};

pub struct DashboardScreen {}

impl DashboardScreen {
    pub fn new() -> Self {
        Self {}
    }
}

impl Screen for DashboardScreen {
    fn show(&mut self, ui: &mut egui::Ui) {
        let palette = colors();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.add_space(spacing::XLARGE);

            ui.vertical(|ui| {
                ui.add_space(spacing::XLARGE);

                // Header
                ui.label(egui::RichText::new(i18n::message("dashboard-welcome")).size(font_size::TITLE).color(palette.text));
                ui.label(
                    egui::RichText::new(i18n::message("dashboard-description"))
                        .size(font_size::BODY)
                        .color(palette.text_secondary),
                );

                ui.add_space(spacing::LARGE);

                // Recent matches
                ui.label(egui::RichText::new(i18n::message("dashboard-recent-matches")).size(font_size::LARGE).color(palette.text));
                ui.add_space(spacing::SMALL);
                card::match_card(ui, "Anti-Mage", "12/3/8", true, "45:32", "2 hours ago");
                ui.add_space(spacing::SMALL);
                card::match_card(ui, "Invoker", "5/7/15", false, "38:15", "5 hours ago");
                ui.add_space(spacing::SMALL);
                card::match_card(ui, "Phantom Assassin", "18/4/6", true, "52:10", "1 day ago");

                ui.add_space(spacing::LARGE);

                // Tracked players
                ui.label(egui::RichText::new(i18n::message("dashboard-tracked-players")).size(font_size::LARGE).color(palette.text));
                ui.add_space(spacing::SMALL);
                ui.horizontal(|ui| {
                    card::player_card(ui, "Player One", Some("Divine 3"), Some("2 hours ago"));
                    card::player_card(ui, "Player Two", Some("Ancient 5"), Some("1 day ago"));
                    card::player_card(ui, "Player Three", None, None);
                });

                ui.add_space(spacing::XLARGE);
            });
        });
    }
}

impl Default for DashboardScreen {
    fn default() -> Self {
        Self::new()
    }
}
