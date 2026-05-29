use super::Screen;
use crate::{
    components::{card, search},
    theme::{colors, font_size, spacing},
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
        let palette = colors();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.vertical(|ui| {
                ui.add_space(spacing::XLARGE);

                ui.label(egui::RichText::new(i18n::message("items-title")).size(font_size::TITLE).color(palette.text));
                ui.add_space(spacing::MEDIUM);
                search::search_input(ui, &i18n::message("items-search-placeholder"), &mut self.search_query);

                ui.add_space(spacing::LARGE);

                // Items row 1
                ui.horizontal(|ui| {
                    card::item_card(ui, "Tango", Some(90));
                    card::item_card(ui, "Clarity", Some(50));
                    card::item_card(ui, "Salve", Some(100));
                    card::item_card(ui, "Faerie Fire", Some(70));
                    card::item_card(ui, "Ward", Some(50));
                    card::item_card(ui, "Dust", Some(80));
                });
                ui.add_space(spacing::SMALL);

                // Items row 2
                ui.horizontal(|ui| {
                    card::item_card(ui, "Power Treads", Some(1400));
                    card::item_card(ui, "Blink Dagger", Some(2250));
                    card::item_card(ui, "Black King Bar", Some(4050));
                    card::item_card(ui, "Butterfly", Some(4975));
                    card::item_card(ui, "Divine Rapier", Some(5950));
                    card::item_card(ui, "Refresher", Some(5000));
                });

                ui.add_space(spacing::XLARGE);
            });
        });
    }
}

impl Default for ItemScreen {
    fn default() -> Self {
        Self::new()
    }
}
