use std::collections::{BTreeMap, HashSet};

use shared::{Friend, PersonaState};

use crate::{
    components::{search, widgets},
    icons::{action, status},
    theme::{ColorPalette, animation, colors, font_size, radius, spacing},
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

        widgets::page(ui, |ui| {
            widgets::screen_header(ui, &i18n::message("friends-title"), |ui| {
                let refresh_label = if self.loading {
                    i18n::message("common-loading")
                } else {
                    format!("{}  {}", action::REFRESH, i18n::message("friends-refresh-status"))
                };
                if widgets::primary_button_enabled(ui, refresh_label, !self.loading).clicked() {
                    action = Some(FriendAction::RefreshStatuses);
                }
                ui.add_space(spacing::SMALL);
                let sync_label = format!("{}  {}", action::SYNC, i18n::message("friends-sync"));
                if widgets::ghost_button_enabled(ui, sync_label, !self.loading).clicked() {
                    action = Some(FriendAction::SyncFriends);
                }
            });

            ui.add_space(spacing::MEDIUM);
            search::search_input(ui, &i18n::message("friends-search-placeholder"), &mut self.search_query);
            ui.add_space(spacing::MEDIUM);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(i18n::message("friends-sort")).size(font_size::BODY).color(palette.text_muted));
                ui.add_space(spacing::SMALL);
                let options = FriendSort::ALL.iter().map(|&s| (s, s.label())).collect::<Vec<_>>();
                widgets::segmented(ui, &mut self.sort, &options);
            });

            ui.add_space(spacing::LARGE);

            self.show_list(ui, &palette, load_avatars);
        });

        action
    }

    fn show_list(&mut self, ui: &mut egui::Ui, palette: &ColorPalette, load_avatars: bool) {
        if self.friends.is_empty() {
            let (title, hint) = if self.loading {
                (i18n::message("common-loading"), String::new())
            } else {
                (i18n::message("friends-empty"), i18n::message("friends-empty-hint"))
            };
            ui.add_space(spacing::LARGE);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new(status::OFFLINE).size(font_size::TITLE).color(palette.text_muted));
                ui.add_space(spacing::SMALL);
                ui.label(egui::RichText::new(title).size(font_size::MEDIUM).color(palette.text_secondary));
                if !hint.is_empty() {
                    ui.label(egui::RichText::new(hint).size(font_size::SMALL).color(palette.text_muted));
                }
            });
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
                    friend_row(ui, &self.friends[i], palette, load_avatars);
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
    fn render_groups(&mut self, ui: &mut egui::Ui, palette: &ColorPalette, load_avatars: bool, groups: Vec<(String, String, Vec<usize>)>) {
        for (key, label, idxs) in groups {
            let collapsed = self.collapsed.contains(&key);
            if group_header(ui, palette, &label, idxs.len(), collapsed) {
                if collapsed {
                    self.collapsed.remove(&key);
                } else {
                    self.collapsed.insert(key);
                }
            }

            if !collapsed {
                for i in idxs {
                    friend_row(ui, &self.friends[i], palette, load_avatars);
                }
            }
            ui.add_space(spacing::LARGE);
        }
    }
}

/// Clickable, foldable group heading: `caret  Label  count`. Returns whether it
/// was clicked.
fn group_header(ui: &mut egui::Ui, palette: &ColorPalette, label: &str, count: usize, collapsed: bool) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        let t = ui
            .ctx()
            .animate_bool_with_time(egui::Id::new(("group_caret", label)), !collapsed, 1.0 / animation::hover_speed());
        if caret(ui, palette.text, t).clicked() {
            clicked = true;
        }
        ui.add_space(spacing::TINY);
        let header = egui::RichText::new(label).size(font_size::LARGE).strong().color(palette.text);
        if ui
            .add(egui::Label::new(header).sense(egui::Sense::click()))
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            clicked = true;
        }
        ui.add_space(spacing::SMALL);
        ui.label(egui::RichText::new(count.to_string()).size(font_size::MEDIUM).color(palette.text_muted));
    });
    ui.add_space(spacing::SMALL);
    clicked
}

/// A chevron drawn by hand so it tweens smoothly from pointing-right (`t = 0`,
/// collapsed) to pointing-down (`t = 1`, expanded) instead of snapping between
/// two glyphs.
fn caret(ui: &mut egui::Ui, color: egui::Color32, t: f32) -> egui::Response {
    let size = font_size::LARGE;
    let (rect, response) = ui.allocate_exact_size(egui::Vec2::splat(size), egui::Sense::click());
    let center = rect.center();
    let r = size * 0.22;
    let (sin, cos) = (t * std::f32::consts::FRAC_PI_2).sin_cos();
    let rotate = |x: f32, y: f32| egui::pos2(center.x + x * cos - y * sin, center.y + x * sin + y * cos);
    let stroke = egui::Stroke::new(1.8_f32, color);
    ui.painter().line_segment([rotate(-r, -r), rotate(r, 0.0)], stroke);
    ui.painter().line_segment([rotate(r, 0.0), rotate(-r, r)], stroke);
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn friend_row(ui: &mut egui::Ui, friend: &Friend, palette: &ColorPalette, load_avatars: bool) {
    let dot_color = if friend.in_game() {
        palette.primary
    } else if friend.persona_state.is_online() {
        palette.success
    } else {
        palette.text_muted
    };

    let response = widgets::list_row(ui, 56.0, |ui| {
        ui.add_space(spacing::SMALL);
        widgets::presence_dot(ui, dot_color, friend.persona_state.is_online());
        ui.add_space(spacing::SMALL);

        if load_avatars && !friend.avatar.is_empty() {
            ui.add(
                egui::Image::new(friend.avatar.as_str())
                    .fit_to_exact_size(egui::vec2(36.0, 36.0))
                    .corner_radius(egui::CornerRadius::same(radius::MEDIUM)),
            );
            ui.add_space(spacing::SMALL);
        }

        ui.vertical(|ui| {
            ui.label(egui::RichText::new(&friend.persona_name).size(font_size::MEDIUM).color(palette.text));
            let status_color = if friend.persona_state.is_online() { palette.text_secondary } else { palette.text_muted };
            ui.label(egui::RichText::new(persona_state_label(friend.persona_state)).size(font_size::SMALL).color(status_color));
        });

        if let Some(game) = &friend.game_extra_info {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(spacing::SMALL);
                ui.label(egui::RichText::new(format!("{} {game}", status::IN_GAME)).size(font_size::SMALL).color(palette.primary));
            });
        }
    });

    if response.clicked() && !friend.profile_url.is_empty() {
        ui.ctx().open_url(egui::OpenUrl::new_tab(&friend.profile_url));
    }
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
