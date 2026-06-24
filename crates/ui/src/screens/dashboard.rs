use super::Screen;
use crate::{
    components::{card, widgets},
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

        widgets::page(ui, |ui| {
            ui.label(egui::RichText::new(i18n::message("dashboard-welcome")).size(font_size::TITLE).strong().color(palette.text));
            ui.add_space(spacing::TINY);
            ui.label(
                egui::RichText::new(i18n::message("dashboard-description"))
                    .size(font_size::BODY)
                    .color(palette.text_secondary),
            );

            ui.add_space(spacing::LARGE);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = spacing::MEDIUM;
                widgets::stat_tile(ui, "128", &i18n::message("friends-watching"));
                widgets::stat_tile(ui, "5", &i18n::message("friends-online"));
                widgets::stat_tile(ui, "12", &i18n::message("nav-matches"));
            });

            ui.add_space(spacing::LARGE);

            widgets::section_label(ui, &i18n::message("dashboard-recent-matches"));
            ui.add_space(spacing::SMALL);
            card::match_card(ui, "Anti-Mage", "12/3/8", true, "45:32", "2 hours ago");
            card::match_card(ui, "Invoker", "5/7/15", false, "38:15", "5 hours ago");
            card::match_card(ui, "Phantom Assassin", "18/4/6", true, "52:10", "1 day ago");

            ui.add_space(spacing::LARGE);

            widgets::section_label(ui, &i18n::message("dashboard-tracked-players"));
            ui.add_space(spacing::SMALL);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = spacing::MEDIUM;
                card::player_card(ui, "Player One", Some("Divine 3"), Some("2 hours ago"));
                card::player_card(ui, "Player Two", Some("Ancient 5"), Some("1 day ago"));
                card::player_card(ui, "Player Three", None, None);
            });
        });
    }
}

impl Default for DashboardScreen {
    fn default() -> Self {
        Self::new()
    }
}
