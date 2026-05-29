use super::Screen;
use crate::{
    components::{card, search},
    theme::{colors, font_size, radius, spacing},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HeroFilter {
    #[default]
    All,
    Strength,
    Agility,
    Intelligence,
    Universal,
}

pub struct HeroScreen {
    search_query: String,
    filter: HeroFilter,
}

impl HeroScreen {
    pub fn new() -> Self {
        Self {
            search_query: String::new(),
            filter: HeroFilter::All,
        }
    }

    fn filter_button(&mut self, ui: &mut egui::Ui, filter: HeroFilter, label: &str) {
        let is_active = self.filter == filter;
        let palette = colors();

        let btn = egui::Button::new(egui::RichText::new(label).size(font_size::SMALL))
            .fill(if is_active { palette.sidebar_item_active } else { egui::Color32::TRANSPARENT })
            .stroke(if is_active { egui::Stroke::new(2.0, palette.primary) } else { egui::Stroke::NONE })
            .corner_radius(egui::CornerRadius::same(radius::MEDIUM));

        if ui.add(btn).clicked() {
            self.filter = filter;
        }
    }
}

impl Screen for HeroScreen {
    fn show(&mut self, ui: &mut egui::Ui) {
        let palette = colors();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.vertical(|ui| {
                ui.add_space(spacing::XLARGE);

                ui.label(egui::RichText::new(i18n::message("heroes-title")).size(font_size::TITLE).color(palette.text));
                ui.add_space(spacing::MEDIUM);
                search::search_input(ui, &i18n::message("heroes-search-placeholder"), &mut self.search_query);

                ui.add_space(spacing::SMALL);

                // Filter buttons
                ui.horizontal(|ui| {
                    self.filter_button(ui, HeroFilter::All, &i18n::message("heroes-filter-all"));
                    self.filter_button(ui, HeroFilter::Strength, &i18n::message("heroes-filter-strength"));
                    self.filter_button(ui, HeroFilter::Agility, &i18n::message("heroes-filter-agility"));
                    self.filter_button(ui, HeroFilter::Intelligence, &i18n::message("heroes-filter-intelligence"));
                    self.filter_button(ui, HeroFilter::Universal, &i18n::message("heroes-filter-universal"));
                });

                ui.add_space(spacing::LARGE);

                // Hero grid row 1
                ui.horizontal(|ui| {
                    card::hero_card(ui, "Anti-Mage", "Agility");
                    card::hero_card(ui, "Axe", "Strength");
                    card::hero_card(ui, "Crystal Maiden", "Intelligence");
                    card::hero_card(ui, "Drow Ranger", "Agility");
                    card::hero_card(ui, "Earthshaker", "Strength");
                });
                ui.add_space(spacing::SMALL);

                // Hero grid row 2
                ui.horizontal(|ui| {
                    card::hero_card(ui, "Invoker", "Universal");
                    card::hero_card(ui, "Juggernaut", "Agility");
                    card::hero_card(ui, "Lina", "Intelligence");
                    card::hero_card(ui, "Pudge", "Strength");
                    card::hero_card(ui, "Shadow Fiend", "Agility");
                });

                ui.add_space(spacing::XLARGE);
            });
        });
    }
}

impl Default for HeroScreen {
    fn default() -> Self {
        Self::new()
    }
}
