use super::Screen;
use crate::{
    components::{card, widgets},
    theme::spacing,
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
}

impl Screen for MatchScreen {
    fn show(&mut self, ui: &mut egui::Ui) {
        widgets::page(ui, |ui| {
            widgets::screen_header(ui, &i18n::message("matches-title"), |_ui| {});
            ui.add_space(spacing::MEDIUM);

            let options = [
                (MatchFilter::All, i18n::message("matches-filter-all")),
                (MatchFilter::Wins, i18n::message("matches-filter-wins")),
                (MatchFilter::Losses, i18n::message("matches-filter-losses")),
            ];
            widgets::segmented(ui, &mut self.filter, &options);

            ui.add_space(spacing::LARGE);

            let matches = [
                ("Anti-Mage", "12/3/8", true, "45:32", "2 hours ago"),
                ("Invoker", "5/7/15", false, "38:15", "5 hours ago"),
                ("Phantom Assassin", "18/4/6", true, "52:10", "1 day ago"),
                ("Storm Spirit", "8/5/12", true, "41:20", "2 days ago"),
                ("Pudge", "3/9/7", false, "35:45", "3 days ago"),
            ];

            for (hero, kda, is_victory, duration, time_ago) in matches {
                let keep = match self.filter {
                    MatchFilter::All => true,
                    MatchFilter::Wins => is_victory,
                    MatchFilter::Losses => !is_victory,
                };
                if keep {
                    card::match_card(ui, hero, kda, is_victory, duration, time_ago);
                }
            }
        });
    }
}

impl Default for MatchScreen {
    fn default() -> Self {
        Self::new()
    }
}
