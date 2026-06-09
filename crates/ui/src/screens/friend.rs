use std::collections::{BTreeMap, HashSet};

use egui::Color32;
use shared::{Friend, PersonaState};

use crate::{
    components::{card, search},
    icons::{action, status},
    theme::{colors, font_size, radius, spacing},
};

/// Signals the screen wants to send back to [`crate::app::App`].
pub enum FriendAction {
    /// Re-fetch the full friend list to pick up newly added/removed friends.
    SyncFriends,
    /// Refresh only the online state and current game of the known friends.
    RefreshStatuses,
}

/// How the watch list is grouped and ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FriendSort {
    /// Online friends first, then offline.
    Status,
    /// Grouped by the game each friend is currently playing.
    Game,
    /// A flat, alphabetical list.
    Name,
}

impl FriendSort {
    const ALL: [FriendSort; 3] = [FriendSort::Status, FriendSort::Game, FriendSort::Name];

    fn label(self) -> String {
        let key = match self {
            FriendSort::Status => "friends-sort-status",
            FriendSort::Game => "friends-sort-game",
            FriendSort::Name => "friends-sort-name",
        };
        i18n::message(key)
    }
}

pub struct FriendScreen {
    search_query: String,
    friends: Vec<Friend>,
    loading: bool,
    sort: FriendSort,
    /// Section keys (see [`Self::render_groups`]) the user has folded away.
    collapsed: HashSet<String>,
}

impl FriendScreen {
    pub fn new() -> Self {
        Self {
            search_query: String::new(),
            friends: Vec::new(),
            loading: false,
            sort: FriendSort::Status,
            // Big groups start folded so a long list isn't rendered up front.
            collapsed: HashSet::from(["offline".to_owned(), "not-in-game".to_owned()]),
        }
    }

    pub fn set_friends(&mut self, friends: Vec<Friend>) {
        self.friends = friends;
        self.loading = false;
    }

    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }

    pub fn friends(&self) -> &[Friend] {
        &self.friends
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<FriendAction> {
        let palette = colors();
        let load_avatars = configs::read().friends.load_avatars;
        let mut action = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.vertical(|ui| {
                ui.add_space(spacing::XLARGE);

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(i18n::message("friends-title")).size(font_size::TITLE).color(palette.text));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.toolbar_button(ui, &palette, action::REFRESH, "friends-sync") {
                            action = Some(FriendAction::SyncFriends);
                        }
                        ui.add_space(spacing::SMALL);
                        if self.toolbar_button(ui, &palette, status::ONLINE, "friends-refresh-status") {
                            action = Some(FriendAction::RefreshStatuses);
                        }
                    });
                });

                ui.add_space(spacing::MEDIUM);
                search::search_input(ui, &i18n::message("friends-search-placeholder"), &mut self.search_query);
                ui.add_space(spacing::MEDIUM);

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(i18n::message("friends-sort")).size(font_size::BODY).color(palette.text_secondary));
                    egui::ComboBox::from_id_salt("friends_sort").selected_text(self.sort.label()).show_ui(ui, |ui| {
                        for opt in FriendSort::ALL {
                            ui.selectable_value(&mut self.sort, opt, opt.label());
                        }
                    });
                });

                ui.add_space(spacing::LARGE);

                self.show_list(ui, &palette, load_avatars);

                ui.add_space(spacing::XLARGE);
            });
        });

        action
    }

    fn toolbar_button(&self, ui: &mut egui::Ui, palette: &crate::theme::ColorPalette, icon: &str, label_key: &str) -> bool {
        let label = if self.loading {
            i18n::message("common-loading")
        } else {
            format!("{icon} {}", i18n::message(label_key))
        };
        let button = egui::Button::new(egui::RichText::new(label).size(font_size::BODY).color(Color32::WHITE))
            .fill(palette.primary)
            .corner_radius(egui::CornerRadius::same(radius::MEDIUM));
        ui.add_enabled(!self.loading, button).clicked()
    }

    fn show_list(&mut self, ui: &mut egui::Ui, palette: &crate::theme::ColorPalette, load_avatars: bool) {
        if self.friends.is_empty() {
            let (title, hint) = if self.loading {
                (i18n::message("common-loading"), String::new())
            } else {
                (i18n::message("friends-empty"), i18n::message("friends-empty-hint"))
            };
            ui.label(egui::RichText::new(title).size(font_size::BODY).color(palette.text_muted));
            if !hint.is_empty() {
                ui.label(egui::RichText::new(hint).size(font_size::SMALL).color(palette.text_muted));
            }
            return;
        }

        let query = self.search_query.to_lowercase();
        let mut indices = (0..self.friends.len())
            .filter(|&i| {
                let friend = &self.friends[i];
                query.is_empty() || friend.persona_name.to_lowercase().contains(&query) || friend.steam_id.contains(&query)
            })
            .collect::<Vec<_>>();

        if indices.is_empty() {
            ui.label(egui::RichText::new(i18n::message("friends-no-results")).size(font_size::BODY).color(palette.text_muted));
            return;
        }

        match self.sort {
            FriendSort::Name => {
                indices.sort_by(|&a, &b| self.friends[a].persona_name.to_lowercase().cmp(&self.friends[b].persona_name.to_lowercase()));
                for &i in &indices {
                    friend_card(ui, &self.friends[i], palette, load_avatars);
                    ui.add_space(spacing::SMALL);
                }
            }
            FriendSort::Status => {
                let online = indices.iter().copied().filter(|&i| self.friends[i].persona_state.is_online()).collect::<Vec<_>>();
                let offline = indices.iter().copied().filter(|&i| !self.friends[i].persona_state.is_online()).collect::<Vec<_>>();
                let mut groups = Vec::new();
                if !online.is_empty() {
                    groups.push(("online".to_owned(), i18n::message("friends-online"), online));
                }
                if !offline.is_empty() {
                    groups.push(("offline".to_owned(), i18n::message("friends-offline"), offline));
                }
                self.render_groups(ui, palette, load_avatars, groups);
            }
            FriendSort::Game => {
                let mut by_game: BTreeMap<String, Vec<usize>> = BTreeMap::new();
                let mut not_in_game = Vec::new();
                for &i in &indices {
                    match &self.friends[i].game_extra_info {
                        Some(game) => by_game.entry(game.clone()).or_default().push(i),
                        None => not_in_game.push(i),
                    }
                }
                let mut groups = by_game.into_iter().map(|(game, idxs)| (format!("game:{game}"), game, idxs)).collect::<Vec<_>>();
                if !not_in_game.is_empty() {
                    groups.push(("not-in-game".to_owned(), i18n::message("friends-not-in-game"), not_in_game));
                }
                self.render_groups(ui, palette, load_avatars, groups);
            }
        }
    }

    /// Render each group under a clickable, foldable header. Folded groups render
    /// none of their cards, so a long list stays cheap until it's expanded.
    fn render_groups(&mut self, ui: &mut egui::Ui, palette: &crate::theme::ColorPalette, load_avatars: bool, groups: Vec<(String, String, Vec<usize>)>) {
        for (key, label, idxs) in groups {
            let collapsed = self.collapsed.contains(&key);
            let caret = if collapsed { action::EXPAND } else { action::CARET_DOWN };
            let header = egui::RichText::new(format!("{caret} {label} ({})", idxs.len())).size(font_size::LARGE).color(palette.text);
            if ui.add(egui::Label::new(header).sense(egui::Sense::click())).clicked() {
                if collapsed {
                    self.collapsed.remove(&key);
                } else {
                    self.collapsed.insert(key);
                }
            }
            ui.add_space(spacing::SMALL);

            if !collapsed {
                for i in idxs {
                    friend_card(ui, &self.friends[i], palette, load_avatars);
                    ui.add_space(spacing::SMALL);
                }
            }
            ui.add_space(spacing::MEDIUM);
        }
    }
}

fn friend_card(ui: &mut egui::Ui, friend: &Friend, palette: &crate::theme::ColorPalette, load_avatars: bool) {
    card::card(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            if load_avatars && !friend.avatar.is_empty() {
                ui.add(
                    egui::Image::new(friend.avatar.as_str())
                        .fit_to_exact_size(egui::vec2(40.0, 40.0))
                        .corner_radius(egui::CornerRadius::same(radius::SMALL)),
                );
                ui.add_space(spacing::SMALL);
            }

            let (icon, color) = if friend.in_game() {
                (status::ONLINE, palette.success)
            } else if friend.persona_state.is_online() {
                (status::ONLINE, palette.primary)
            } else {
                (status::OFFLINE, palette.text_muted)
            };
            ui.label(egui::RichText::new(icon).size(font_size::SMALL).color(color));

            ui.vertical(|ui| {
                ui.hyperlink_to(egui::RichText::new(&friend.persona_name).size(font_size::MEDIUM).color(palette.text), &friend.profile_url);

                let (status_text, status_color) = if let Some(game) = &friend.game_extra_info {
                    (format!("{}: {game}", i18n::message("friends-status-in-game")), palette.success)
                } else if friend.persona_state.is_online() {
                    (persona_state_label(friend.persona_state), palette.text_secondary)
                } else {
                    (persona_state_label(friend.persona_state), palette.text_muted)
                };
                ui.label(egui::RichText::new(status_text).size(font_size::SMALL).color(status_color));
            });
        });
    });
}

fn persona_state_label(state: PersonaState) -> String {
    let key = match state {
        PersonaState::Offline => "friends-status-offline",
        PersonaState::Online => "friends-status-online",
        PersonaState::Busy => "friends-status-busy",
        PersonaState::Away => "friends-status-away",
        PersonaState::Snooze => "friends-status-snooze",
        PersonaState::LookingToTrade => "friends-status-looking-to-trade",
        PersonaState::LookingToPlay => "friends-status-looking-to-play",
    };
    i18n::message(key)
}

impl Default for FriendScreen {
    fn default() -> Self {
        Self::new()
    }
}
