use std::collections::{BTreeMap, HashSet};

use shared::{Follow, Friend};

use crate::{
    components::{search, widgets},
    icons::{action, status},
    theme::{ColorPalette, animation, colors, font_size, radius, spacing},
};

/// Signals the screen wants to send back to [`crate::app::App`].
pub enum FollowAction {
    /// Resolve `input` (a Steam64 id or profile URL) and add it to the list.
    AddPlayer(String),
    /// Stop following one player.
    RemovePlayer(String),
    /// Copy every current friend into the follow list.
    AddAllFriends,
    /// Remove every current friend from the follow list.
    RemoveAllFriends,
    /// Refresh the online state and current game of the known follows.
    RefreshStatuses,
    /// Follow the chosen subset of the friend list.
    AddSelectedFriends(Vec<String>),
    /// Enroll (or unenroll) one player in background match tracking.
    SetTracked(String, bool),
}

/// How the follow list is grouped and ordered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FollowSort {
    /// Online players first, then offline.
    Status,
    /// Grouped by the game each player is currently playing.
    Game,
    /// A flat, alphabetical list.
    Name,
}

impl FollowSort {
    const ALL: [FollowSort; 3] = [FollowSort::Status, FollowSort::Game, FollowSort::Name];

    fn label(self) -> String {
        let key = match self {
            FollowSort::Status => "follows-sort-status",
            FollowSort::Game => "follows-sort-game",
            FollowSort::Name => "follows-sort-name",
        };
        i18n::message(key)
    }
}

pub struct FollowScreen {
    search_query: String,
    add_input: String,
    follows: Vec<Follow>,
    loading: bool,
    sort: FollowSort,
    /// Section keys (see [`Self::render_groups`]) the user has folded away.
    collapsed: HashSet<String>,
    /// Steam ids whose recently-played-games panel is expanded.
    expanded: HashSet<String>,

    /// Mirrors the Friends screen's list, offered as candidates in the picker.
    available_friends: Vec<Friend>,
    picker_open: bool,
    picker_query: String,
    /// Steam ids checked in the picker, pending confirmation.
    picker_selected: HashSet<String>,
}

impl FollowScreen {
    pub fn new() -> Self {
        Self {
            search_query: String::new(),
            add_input: String::new(),
            follows: Vec::new(),
            loading: false,
            sort: FollowSort::Status,
            collapsed: HashSet::from(["offline".to_owned(), "not-in-game".to_owned()]),
            expanded: HashSet::new(),
            available_friends: Vec::new(),
            picker_open: false,
            picker_query: String::new(),
            picker_selected: HashSet::new(),
        }
    }

    pub fn set_follows(&mut self, follows: Vec<Follow>) {
        self.follows = follows;
        self.loading = false;
    }

    /// Refresh the picker's candidate list. Cheap no-op unless the set size
    /// changed, so it can be called every frame from the route handler.
    pub fn set_friends(&mut self, friends: &[Friend]) {
        if self.available_friends.len() != friends.len() {
            self.available_friends = friends.to_vec();
        }
    }

    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }

    pub fn follows(&self) -> &[Follow] {
        &self.follows
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<FollowAction> {
        let palette = colors();
        let (load_avatars, recent_games_limit) = {
            let friends = &configs::read().friends;
            (friends.load_avatars, friends.recent_games_limit)
        };
        let mut action = None;

        widgets::page(ui, |ui| {
            widgets::screen_header(ui, &i18n::message("follows-title"), |ui| {
                let refresh_label = if self.loading {
                    i18n::message("common-loading")
                } else {
                    format!("{}  {}", action::REFRESH, i18n::message("follows-refresh-status"))
                };
                if widgets::primary_button_enabled(ui, refresh_label, !self.loading).clicked() {
                    action = Some(FollowAction::RefreshStatuses);
                }
                ui.add_space(spacing::SMALL);
                let remove_all_label = i18n::message("follows-remove-all-friends");
                if widgets::ghost_button(ui, remove_all_label).clicked() {
                    action = Some(FollowAction::RemoveAllFriends);
                }
                ui.add_space(spacing::SMALL);
                let add_all_label = i18n::message("follows-add-all-friends");
                if widgets::ghost_button(ui, add_all_label).clicked() {
                    action = Some(FollowAction::AddAllFriends);
                }
                ui.add_space(spacing::SMALL);
                let select_label = format!("{}  {}", action::CHECK, i18n::message("follows-select-from-friends"));
                if widgets::ghost_button(ui, select_label).clicked() {
                    self.picker_query.clear();
                    self.picker_selected.clear();
                    self.picker_open = true;
                }
            });

            ui.add_space(spacing::MEDIUM);
            self.show_add_input(ui, &mut action);
            ui.add_space(spacing::MEDIUM);

            search::search_input(ui, &i18n::message("follows-search-placeholder"), &mut self.search_query);
            ui.add_space(spacing::MEDIUM);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(i18n::message("follows-sort")).size(font_size::BODY).color(palette.text_muted));
                ui.add_space(spacing::SMALL);
                let options = FollowSort::ALL.iter().map(|&s| (s, s.label())).collect::<Vec<_>>();
                widgets::segmented(ui, &mut self.sort, &options);
            });

            ui.add_space(spacing::LARGE);

            self.show_list(ui, &palette, load_avatars, recent_games_limit, &mut action);
        });

        self.show_picker(ui.ctx(), &mut action);

        action
    }

    /// A lightweight popover listing friends not already followed, each with a
    /// toggle, plus a search box and select-all/none shortcuts. Confirming emits
    /// [`FollowAction::AddSelectedFriends`] for the checked ids.
    ///
    /// Unlike [`crate::components::exit_modal::ExitModal`], this doesn't dim the
    /// background: following friends is a normal, freely reversible action, not
    /// a one-way confirmation, so it shouldn't read as blocking the rest of the
    /// UI.
    fn show_picker(&mut self, ctx: &egui::Context, action: &mut Option<FollowAction>) {
        if !self.picker_open {
            return;
        }

        let palette = colors();

        let mut open = true;
        egui::Window::new(i18n::message("follows-picker-title"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .fixed_size(egui::Vec2::new(820.0, 420.0))
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                search::search_input(ui, &i18n::message("follows-picker-search-placeholder"), &mut self.picker_query);
                ui.add_space(spacing::SMALL);

                let query = self.picker_query.to_lowercase();
                let candidates = self
                    .available_friends
                    .iter()
                    .filter(|f| !self.follows.iter().any(|flw| flw.steam_id == f.steam_id))
                    .filter(|f| query.is_empty() || f.persona_name.to_lowercase().contains(&query) || f.steam_id.contains(&query))
                    .collect::<Vec<_>>();

                ui.horizontal(|ui| {
                    if widgets::ghost_button(ui, i18n::message("follows-picker-select-all")).clicked() {
                        self.picker_selected.extend(candidates.iter().map(|f| f.steam_id.clone()));
                    }
                    ui.add_space(spacing::SMALL);
                    // Only the currently visible candidates are cleared, so a
                    // search-narrowed "none" doesn't discard hidden selections.
                    if widgets::ghost_button(ui, i18n::message("follows-picker-select-none")).clicked() {
                        self.picker_selected.retain(|id| candidates.iter().all(|f| &f.steam_id != id));
                    }
                });
                ui.add_space(spacing::SMALL);

                if self.available_friends.is_empty() {
                    ui.label(
                        egui::RichText::new(i18n::message("follows-picker-empty-no-friends"))
                            .size(font_size::BODY)
                            .color(palette.text_muted),
                    );
                } else if candidates.is_empty() {
                    ui.label(
                        egui::RichText::new(i18n::message("follows-picker-empty-all-followed"))
                            .size(font_size::BODY)
                            .color(palette.text_muted),
                    );
                } else {
                    egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                        let toggles = candidates
                            .iter()
                            .map(|friend| {
                                let mut checked = self.picker_selected.contains(&friend.steam_id);
                                let dot_color = if friend.persona_state.is_online() { palette.success } else { palette.text_muted };
                                widgets::list_row(ui, 44.0, |ui| {
                                    ui.add_space(spacing::SMALL);
                                    widgets::presence_dot(ui, dot_color, friend.persona_state.is_online());
                                    ui.add_space(spacing::SMALL);
                                    ui.label(egui::RichText::new(&friend.persona_name).size(font_size::BODY).color(palette.text));
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        widgets::toggle(ui, &mut checked);
                                    });
                                });
                                (friend.steam_id.clone(), checked)
                            })
                            .collect::<Vec<_>>();
                        for (steam_id, checked) in toggles {
                            if checked {
                                self.picker_selected.insert(steam_id);
                            } else {
                                self.picker_selected.remove(&steam_id);
                            }
                        }
                    });
                }

                ui.add_space(spacing::MEDIUM);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let count = self.picker_selected.len();
                    let confirm_label = if count > 0 {
                        format!("{} ({count})", i18n::message("follows-picker-confirm"))
                    } else {
                        i18n::message("follows-picker-confirm")
                    };
                    if widgets::primary_button_enabled(ui, confirm_label, count > 0).clicked() {
                        *action = Some(FollowAction::AddSelectedFriends(self.picker_selected.drain().collect()));
                        self.picker_open = false;
                    }
                    ui.add_space(spacing::SMALL);
                    if widgets::ghost_button(ui, i18n::message("follows-picker-cancel")).clicked() {
                        self.picker_open = false;
                    }
                });
            });

        if !open {
            self.picker_open = false;
        }
    }

    /// The "paste a Steam64 id or profile URL" input row.
    fn show_add_input(&mut self, ui: &mut egui::Ui, action: &mut Option<FollowAction>) {
        let palette = colors();
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let can_add = !self.add_input.trim().is_empty();
                let add_label = format!("{}  {}", action::ADD, i18n::message("follows-add"));
                if widgets::primary_button_enabled(ui, add_label, can_add).clicked() {
                    *action = Some(FollowAction::AddPlayer(self.add_input.trim().to_owned()));
                    self.add_input.clear();
                }
                ui.add_space(spacing::SMALL);
                egui::Frame::NONE
                    .fill(palette.surface_secondary)
                    .stroke(egui::Stroke::new(1_f32, palette.border))
                    .corner_radius(egui::CornerRadius::same(radius::MEDIUM))
                    .inner_margin(egui::Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.add_input)
                                .hint_text(i18n::message("follows-input-placeholder"))
                                .desired_width(ui.available_width())
                                .font(egui::FontId::proportional(font_size::BODY))
                                .frame(egui::Frame::NONE),
                        );
                    });
            });
        });
    }

    fn show_list(&mut self, ui: &mut egui::Ui, palette: &ColorPalette, load_avatars: bool, limit: usize, action: &mut Option<FollowAction>) {
        if self.follows.is_empty() {
            let (title, hint) = if self.loading {
                (i18n::message("common-loading"), String::new())
            } else {
                (i18n::message("follows-empty"), i18n::message("follows-empty-hint"))
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
        let mut indices = (0..self.follows.len())
            .filter(|&i| {
                let follow = &self.follows[i];
                query.is_empty() || follow.persona_name.to_lowercase().contains(&query) || follow.steam_id.contains(&query)
            })
            .collect::<Vec<_>>();

        if indices.is_empty() {
            ui.label(egui::RichText::new(i18n::message("follows-no-results")).size(font_size::BODY).color(palette.text_muted));
            return;
        }

        match self.sort {
            FollowSort::Name => {
                indices.sort_by(|&a, &b| self.follows[a].persona_name.to_lowercase().cmp(&self.follows[b].persona_name.to_lowercase()));
                self.render_rows(ui, palette, load_avatars, limit, &indices, action);
            }
            FollowSort::Status => {
                let online = indices.iter().copied().filter(|&i| self.follows[i].persona_state.is_online()).collect::<Vec<_>>();
                let offline = indices.iter().copied().filter(|&i| !self.follows[i].persona_state.is_online()).collect::<Vec<_>>();
                let mut groups = Vec::new();
                if !online.is_empty() {
                    groups.push(("online".to_owned(), i18n::message("follows-online"), online));
                }
                if !offline.is_empty() {
                    groups.push(("offline".to_owned(), i18n::message("follows-offline"), offline));
                }
                self.render_groups(ui, palette, load_avatars, limit, groups, action);
            }
            FollowSort::Game => {
                let mut by_game: BTreeMap<String, Vec<usize>> = BTreeMap::new();
                let mut not_in_game = Vec::new();
                for &i in &indices {
                    match &self.follows[i].game_extra_info {
                        Some(game) => by_game.entry(game.clone()).or_default().push(i),
                        None => not_in_game.push(i),
                    }
                }
                let mut groups = by_game.into_iter().map(|(game, idxs)| (format!("game:{game}"), game, idxs)).collect::<Vec<_>>();
                if !not_in_game.is_empty() {
                    groups.push(("not-in-game".to_owned(), i18n::message("follows-not-in-game"), not_in_game));
                }
                self.render_groups(ui, palette, load_avatars, limit, groups, action);
            }
        }
    }

    /// Render each group under a clickable, foldable header. Folded groups render
    /// none of their cards, so a long list stays cheap until it's expanded.
    fn render_groups(
        &mut self,
        ui: &mut egui::Ui,
        palette: &ColorPalette,
        load_avatars: bool,
        limit: usize,
        groups: Vec<(String, String, Vec<usize>)>,
        action: &mut Option<FollowAction>,
    ) {
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
                self.render_rows(ui, palette, load_avatars, limit, &idxs, action);
            }
            ui.add_space(spacing::LARGE);
        }
    }

    /// Render the follow rows at `idxs`, applying and updating each row's
    /// recently-played-games expansion and surfacing a remove click as an
    /// action. Toggles/removals are gathered first so the immutable borrow of
    /// `self.follows` during rendering doesn't clash with mutating `self`.
    fn render_rows(&mut self, ui: &mut egui::Ui, palette: &ColorPalette, load_avatars: bool, limit: usize, idxs: &[usize], action: &mut Option<FollowAction>) {
        let mut toggles = Vec::new();
        let mut removed = None;
        let mut track_toggled = None;
        for &i in idxs {
            let follow = &self.follows[i];
            let expanded = self.expanded.contains(&follow.steam_id);
            let row = follow_row(ui, follow, palette, load_avatars, expanded, limit);
            if row.recent_toggled {
                toggles.push(follow.steam_id.clone());
            }
            if row.remove_clicked {
                removed = Some(follow.steam_id.clone());
            }
            if row.track_clicked {
                track_toggled = Some((follow.steam_id.clone(), !follow.tracked));
            }
        }
        for id in toggles {
            if !self.expanded.remove(&id) {
                self.expanded.insert(id);
            }
        }
        if let Some(steam_id) = removed {
            *action = Some(FollowAction::RemovePlayer(steam_id));
        }
        if let Some((steam_id, tracked)) = track_toggled {
            *action = Some(FollowAction::SetTracked(steam_id, tracked));
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
        if widgets::caret(ui, palette.text, t).clicked() {
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

/// The interactions a follow row surfaces back to [`FollowScreen::render_rows`].
struct FollowRowResponse {
    recent_toggled: bool,
    remove_clicked: bool,
    track_clicked: bool,
}

/// One follow-list row. A row click that isn't on a control opens the player's
/// Steam profile.
fn follow_row(ui: &mut egui::Ui, follow: &Follow, palette: &ColorPalette, load_avatars: bool, expanded: bool, limit: usize) -> FollowRowResponse {
    let dot_color = if follow.in_game() {
        palette.primary
    } else if follow.persona_state.is_online() {
        palette.success
    } else {
        palette.text_muted
    };
    let has_games = !follow.recent_games.is_empty();
    let mut toggled = false;
    let mut remove_clicked = false;
    let mut track_clicked = false;

    let response = widgets::list_row(ui, 56.0, |ui| {
        ui.add_space(spacing::SMALL);
        widgets::presence_dot(ui, dot_color, follow.persona_state.is_online());
        ui.add_space(spacing::SMALL);

        if load_avatars && !follow.avatar.is_empty() {
            ui.add(
                egui::Image::new(follow.avatar.as_str())
                    .fit_to_exact_size(egui::vec2(36.0, 36.0))
                    .corner_radius(egui::CornerRadius::same(radius::MEDIUM)),
            );
            ui.add_space(spacing::SMALL);
        }

        ui.vertical(|ui| {
            ui.label(egui::RichText::new(&follow.persona_name).size(font_size::MEDIUM).color(palette.text));
            let status_color = if follow.persona_state.is_online() { palette.text_secondary } else { palette.text_muted };
            ui.label(
                egui::RichText::new(widgets::persona_state_label(follow.persona_state))
                    .size(font_size::SMALL)
                    .color(status_color),
            );
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(egui::Label::new(egui::RichText::new(action::DELETE).size(font_size::BODY).color(palette.text_muted)).sense(egui::Sense::click()))
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                remove_clicked = true;
            }
            ui.add_space(spacing::MEDIUM);

            let (track_icon, track_color, track_hint) = if follow.tracked {
                (action::TRACKING, palette.primary, i18n::message("follows-untrack-hint"))
            } else {
                (action::TRACK, palette.text_muted, i18n::message("follows-track-hint"))
            };
            if ui
                .add(egui::Label::new(egui::RichText::new(track_icon).size(font_size::BODY).color(track_color)).sense(egui::Sense::click()))
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text(track_hint)
                .clicked()
            {
                track_clicked = true;
            }
            ui.add_space(spacing::MEDIUM);

            if has_games {
                let t = ui
                    .ctx()
                    .animate_bool_with_time(egui::Id::new(("follow_caret", &follow.steam_id)), expanded, 1.0 / animation::hover_speed());
                if widgets::caret(ui, palette.text_muted, t).clicked() {
                    toggled = true;
                }
                ui.add_space(spacing::TINY);
                let games_label = format!("{} {}", status::IN_GAME, follow.recent_games.len());
                ui.label(egui::RichText::new(games_label).size(font_size::SMALL).color(palette.text_muted));
            }
            if let Some(game) = &follow.game_extra_info {
                ui.add_space(spacing::SMALL);
                ui.label(egui::RichText::new(format!("{} {game}", status::IN_GAME)).size(font_size::SMALL).color(palette.primary));
            }
        });
    });

    if response.clicked() && !toggled && !remove_clicked && !track_clicked && !follow.profile_url.is_empty() {
        ui.ctx().open_url(egui::OpenUrl::new_tab(&follow.profile_url));
    }

    if expanded && has_games {
        widgets::recent_games_panel(ui, &follow.recent_games, palette, limit);
    }

    FollowRowResponse {
        recent_toggled: toggled,
        remove_clicked,
        track_clicked,
    }
}

impl Default for FollowScreen {
    fn default() -> Self {
        Self::new()
    }
}
