use super::Screen;
use crate::{
    components::{card, search},
    theme::{colors, font_size, spacing},
};

pub struct FriendScreen {
    search_query: String,
}

impl FriendScreen {
    pub fn new() -> Self {
        Self { search_query: String::new() }
    }
}

impl Screen for FriendScreen {
    fn show(&mut self, ui: &mut egui::Ui) {
        let palette = colors();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.vertical(|ui| {
                ui.add_space(spacing::XLARGE);

                ui.label(egui::RichText::new(i18n::message("friends-title")).size(font_size::TITLE).color(palette.text));
                ui.add_space(spacing::MEDIUM);
                search::search_input(ui, &i18n::message("friends-search-placeholder"), &mut self.search_query);

                ui.add_space(spacing::LARGE);

                ui.label(egui::RichText::new(i18n::message("friends-watching")).size(font_size::LARGE).color(palette.text));
                ui.add_space(spacing::SMALL);

                if self.search_query.is_empty() {
                    card::player_card(ui, "Player One", Some("Divine 3"), Some("2 hours ago"));
                    ui.add_space(spacing::SMALL);
                    card::player_card(ui, "Player Two", Some("Ancient 5"), Some("1 day ago"));
                } else {
                    ui.label(egui::RichText::new(i18n::message("friends-no-results")).size(font_size::BODY).color(palette.text_muted));
                }

                ui.add_space(spacing::XLARGE);
            });
        });
    }
}

impl Default for FriendScreen {
    fn default() -> Self {
        Self::new()
    }
}
