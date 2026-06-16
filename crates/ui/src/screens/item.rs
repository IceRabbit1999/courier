use egui::vec2;

use super::Screen;
use crate::{
    components::{card, search, widgets},
    theme::spacing,
};

pub struct ItemScreen {
    search_query: String,
}

impl ItemScreen {
    pub fn new() -> Self {
        Self { search_query: String::new() }
    }
}

impl Screen for ItemScreen {
    fn show(&mut self, ui: &mut egui::Ui) {
        widgets::page(ui, |ui| {
            widgets::screen_header(ui, &i18n::message("items-title"), |_ui| {});
            ui.add_space(spacing::MEDIUM);

            search::search_input(ui, &i18n::message("items-search-placeholder"), &mut self.search_query);

            ui.add_space(spacing::LARGE);

            let items = [
                ("Tango", Some(90)),
                ("Clarity", Some(50)),
                ("Salve", Some(100)),
                ("Faerie Fire", Some(70)),
                ("Ward", Some(50)),
                ("Dust", Some(80)),
                ("Power Treads", Some(1400)),
                ("Blink Dagger", Some(2250)),
                ("Black King Bar", Some(4050)),
                ("Butterfly", Some(4975)),
                ("Divine Rapier", Some(5950)),
                ("Refresher", Some(5000)),
            ];

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(spacing::SMALL, spacing::SMALL);
                for (name, cost) in items {
                    card::item_card(ui, name, cost);
                }
            });
        });
    }
}

impl Default for ItemScreen {
    fn default() -> Self {
        Self::new()
    }
}
