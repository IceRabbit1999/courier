use egui::vec2;

use super::Screen;
use crate::{
    components::{card, search, widgets},
    theme::spacing,
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
}

impl Screen for HeroScreen {
    fn show(&mut self, ui: &mut egui::Ui) {
        widgets::page(ui, |ui| {
            widgets::screen_header(ui, &i18n::message("heroes-title"), |_ui| {});
            ui.add_space(spacing::MEDIUM);

            search::search_input(ui, &i18n::message("heroes-search-placeholder"), &mut self.search_query);
            ui.add_space(spacing::MEDIUM);

            let options = [
                (HeroFilter::All, i18n::message("heroes-filter-all")),
                (HeroFilter::Strength, i18n::message("heroes-filter-strength")),
                (HeroFilter::Agility, i18n::message("heroes-filter-agility")),
                (HeroFilter::Intelligence, i18n::message("heroes-filter-intelligence")),
                (HeroFilter::Universal, i18n::message("heroes-filter-universal")),
            ];
            widgets::segmented(ui, &mut self.filter, &options);

            ui.add_space(spacing::LARGE);

            let heroes = [
                ("Anti-Mage", "Agility"),
                ("Axe", "Strength"),
                ("Crystal Maiden", "Intelligence"),
                ("Drow Ranger", "Agility"),
                ("Earthshaker", "Strength"),
                ("Invoker", "Universal"),
                ("Juggernaut", "Agility"),
                ("Lina", "Intelligence"),
                ("Pudge", "Strength"),
                ("Shadow Fiend", "Agility"),
            ];

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(spacing::SMALL, spacing::SMALL);
                for (name, attribute) in heroes {
                    card::hero_card(ui, name, attribute);
                }
            });
        });
    }
}

impl Default for HeroScreen {
    fn default() -> Self {
        Self::new()
    }
}
