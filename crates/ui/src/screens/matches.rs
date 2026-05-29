use super::Screen;
use crate::{
    components::card,
    theme::{colors, font_size, radius, spacing},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MatchFilter {
    #[default]
    All,
    Wins,
    Losses,
}

pub struct MatchScreen {
    filter: MatchFilter,
}

impl MatchScreen {
    pub fn new() -> Self {
        Self { filter: MatchFilter::All }
    }

    fn filter_button(&mut self, ui: &mut egui::Ui, filter: MatchFilter, label: &str) {
        let is_active = self.filter == filter;
        let palette = colors();

        let btn = egui::Button::new(egui::RichText::new(label).size(font_size::BODY))
            .fill(if is_active { palette.sidebar_item_active } else { egui::Color32::TRANSPARENT })
            .stroke(if is_active { egui::Stroke::new(2.0, palette.primary) } else { egui::Stroke::NONE })
            .corner_radius(egui::CornerRadius::same(radius::MEDIUM));

        if ui.add(btn).clicked() {
            self.filter = filter;
        }
    }
}

impl Screen for MatchScreen {
    fn show(&mut self, ui: &mut egui::Ui) {
        let palette = colors();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.vertical(|ui| {
                ui.add_space(spacing::XLARGE);

                ui.label(egui::RichText::new(i18n::message("matches-title")).size(font_size::TITLE).color(palette.text));
                ui.add_space(spacing::MEDIUM);

                // Filter buttons
                ui.horizontal(|ui| {
                    self.filter_button(ui, MatchFilter::All, &i18n::message("matches-filter-all"));
                    self.filter_button(ui, MatchFilter::Wins, &i18n::message("matches-filter-wins"));
                    self.filter_button(ui, MatchFilter::Losses, &i18n::message("matches-filter-losses"));
                });

                ui.add_space(spacing::LARGE);

                card::match_card(ui, "Anti-Mage", "12/3/8", true, "45:32", "2 hours ago");
                ui.add_space(spacing::SMALL);
                card::match_card(ui, "Invoker", "5/7/15", false, "38:15", "5 hours ago");
                ui.add_space(spacing::SMALL);
                card::match_card(ui, "Phantom Assassin", "18/4/6", true, "52:10", "1 day ago");
                ui.add_space(spacing::SMALL);
                card::match_card(ui, "Storm Spirit", "8/5/12", true, "41:20", "2 days ago");
                ui.add_space(spacing::SMALL);
                card::match_card(ui, "Pudge", "3/9/7", false, "35:45", "3 days ago");

                ui.add_space(spacing::XLARGE);
            });
        });
    }
}

impl Default for MatchScreen {
    fn default() -> Self {
        Self::new()
    }
}
