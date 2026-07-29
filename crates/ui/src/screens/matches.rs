use std::collections::HashMap;

use egui::{Color32, CornerRadius, Sense, Vec2, vec2};
use shared::{Follow, MatchDetail, MatchPlayer, MatchSummary, format_duration, format_relative, game_mode_label};

use crate::{
    components::widgets,
    icons::{action, status},
    theme::{ColorPalette, colors, font_size, radius, spacing},
};

/// Steam CDN base for the `dota_react` hero/item artwork.
const ICON_CDN: &str = "https://cdn.cloudflare.steamstatic.com/apps/dota2/images/dota_react";

/// Aghanim's Scepter badge colour (amber) and Shard badge colour (cyan).
const SCEPTER_COLOR: Color32 = Color32::from_rgb(0xF2, 0xB8, 0x3A);
const SHARD_COLOR: Color32 = Color32::from_rgb(0x49, 0xB6, 0xE8);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MatchFilter {
    #[default]
    All,
    Wins,
    Losses,
}

/// Signals the screen wants to send back to [`crate::app::App`].
pub enum MatchAction {
    /// Load the selected player's stored matches (no network).
    SelectPlayer(String),
    /// Fetch the selected player's recent matches from OpenDota.
    FetchMatches(String),
    /// Open one match's full detail (loaded from the DB or fetched).
    OpenMatch(i64),
    /// Leave the detail view, back to the summary list.
    CloseDetail,
}

pub struct MatchScreen {
    filter: MatchFilter,
    /// `(steam_id, persona_name)` for the player picker.
    players: Vec<(String, String)>,
    selected: Option<String>,
    matches: Vec<MatchSummary>,
    detail: Option<MatchDetail>,
    loading: bool,
    hero_names: HashMap<i32, String>,
    item_names: HashMap<i32, String>,
    /// `hero_id -> internal_name` and `item_id -> short_name`, for CDN icon URLs.
    hero_slugs: HashMap<i32, String>,
    item_slugs: HashMap<i32, String>,
    names_requested: bool,
    /// Mirrors `configs::matches.load_icons`, refreshed each frame in [`Self::show`].
    load_icons: bool,
}

impl MatchScreen {
    pub fn new() -> Self {
        Self {
            filter: MatchFilter::All,
            players: Vec::new(),
            selected: None,
            matches: Vec::new(),
            detail: None,
            loading: false,
            hero_names: HashMap::new(),
            item_names: HashMap::new(),
            hero_slugs: HashMap::new(),
            item_slugs: HashMap::new(),
            names_requested: false,
            load_icons: false,
        }
    }

    /// Refresh the player picker. Cheap no-op unless the set size changed, so it
    /// can be called every frame from the route handler.
    pub fn set_players(&mut self, follows: &[Follow]) {
        if self.players.len() != follows.len() {
            self.players = follows.iter().map(|f| (f.steam_id.clone(), f.persona_name.clone())).collect();
        }
    }

    /// Apply loaded/fetched matches, ignoring results for a player the user has
    /// since switched away from.
    pub fn set_matches(&mut self, steam_id: String, matches: Vec<MatchSummary>) {
        self.loading = false;
        if self.selected.as_deref() == Some(steam_id.as_str()) {
            self.matches = matches;
        }
    }

    pub fn set_detail(&mut self, detail: MatchDetail) {
        self.detail = Some(detail);
        self.loading = false;
    }

    pub fn set_names(&mut self, heroes: HashMap<i32, String>, items: HashMap<i32, String>, hero_slugs: HashMap<i32, String>, item_slugs: HashMap<i32, String>) {
        self.hero_names = heroes;
        self.item_names = items;
        self.hero_slugs = hero_slugs;
        self.item_slugs = item_slugs;
    }

    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }

    pub fn needs_names(&self) -> bool {
        self.hero_names.is_empty() && !self.names_requested
    }

    pub fn mark_names_requested(&mut self) {
        self.names_requested = true;
    }

    pub fn close_detail(&mut self) {
        self.detail = None;
    }

    fn hero_name(&self, hero_id: i32) -> String {
        self.hero_names.get(&hero_id).cloned().unwrap_or_else(|| format!("#{hero_id}"))
    }

    /// The `dota_react` square minimap icon URL for a hero, when its slug is known.
    fn hero_icon_url(&self, hero_id: i32) -> Option<String> {
        let internal = self.hero_slugs.get(&hero_id)?;
        let slug = internal.strip_prefix("npc_dota_hero_").unwrap_or(internal);
        Some(format!("{ICON_CDN}/heroes/icons/{slug}.png"))
    }

    fn item_icon_url(&self, item_id: i32) -> Option<String> {
        let slug = self.item_slugs.get(&item_id)?;
        Some(format!("{ICON_CDN}/items/{slug}.png"))
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<MatchAction> {
        let palette = colors();
        self.load_icons = configs::read().matches.load_icons;
        let mut action = None;

        widgets::page(ui, |ui| {
            if self.detail.is_some() {
                action = self.show_detail(ui, &palette);
            } else {
                action = self.show_list(ui, &palette);
            }
        });

        action
    }

    fn show_list(&mut self, ui: &mut egui::Ui, palette: &ColorPalette) -> Option<MatchAction> {
        let mut action = None;

        widgets::screen_header(ui, &i18n::message("matches-title"), |ui| {
            let can_fetch = self.selected.is_some() && !self.loading;
            let label = if self.loading {
                i18n::message("matches-fetching")
            } else {
                format!("{}  {}", action::SYNC, i18n::message("matches-fetch"))
            };
            if widgets::primary_button_enabled(ui, label, can_fetch).clicked()
                && let Some(steam_id) = &self.selected
            {
                action = Some(MatchAction::FetchMatches(steam_id.clone()));
            }
        });
        ui.add_space(spacing::MEDIUM);

        if self.players.is_empty() {
            self.empty_state(ui, palette, &i18n::message("matches-no-players"));
            return action;
        }

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(i18n::message("matches-select-player")).size(font_size::BODY).color(palette.text_muted));
            ui.add_space(spacing::SMALL);
            let selected_label = self
                .selected
                .as_ref()
                .and_then(|id| self.players.iter().find(|(sid, _)| sid == id))
                .map(|(_, name)| name.clone())
                .unwrap_or_else(|| i18n::message("matches-pick-player"));

            egui::ComboBox::from_id_salt("matches_player_picker")
                .selected_text(selected_label)
                .width(220.0)
                .show_ui(ui, |ui| {
                    for (steam_id, name) in &self.players {
                        let selected = self.selected.as_deref() == Some(steam_id.as_str());
                        if ui.selectable_label(selected, name).clicked() && !selected {
                            self.selected = Some(steam_id.clone());
                            action = Some(MatchAction::SelectPlayer(steam_id.clone()));
                        }
                    }
                });
        });

        ui.add_space(spacing::MEDIUM);

        let options = [
            (MatchFilter::All, i18n::message("matches-filter-all")),
            (MatchFilter::Wins, i18n::message("matches-filter-wins")),
            (MatchFilter::Losses, i18n::message("matches-filter-losses")),
        ];
        widgets::segmented(ui, &mut self.filter, &options);

        ui.add_space(spacing::LARGE);

        if self.selected.is_none() {
            self.empty_state(ui, palette, &i18n::message("matches-pick-player"));
            return action;
        }

        if self.matches.is_empty() {
            let msg = if self.loading {
                i18n::message("matches-loading")
            } else {
                i18n::message("matches-empty")
            };
            self.empty_state(ui, palette, &msg);
            return action;
        }

        let visible = self.matches.iter().filter(|m| match self.filter {
            MatchFilter::All => true,
            MatchFilter::Wins => m.won(),
            MatchFilter::Losses => !m.won(),
        });
        for m in visible {
            let icon = if self.load_icons { self.hero_icon_url(m.hero_id) } else { None };
            if summary_row(ui, palette, icon.as_deref(), &self.hero_name(m.hero_id), m).clicked() {
                action = Some(MatchAction::OpenMatch(m.match_id));
            }
        }

        action
    }

    fn show_detail(&mut self, ui: &mut egui::Ui, palette: &ColorPalette) -> Option<MatchAction> {
        let Some(detail) = &self.detail else { return None };
        let mut action = None;

        widgets::screen_header(ui, &i18n::message("matches-title"), |ui| {
            let back = format!("{}  {}", action::ARROW_LEFT, i18n::message("matches-back"));
            if widgets::ghost_button(ui, back).clicked() {
                action = Some(MatchAction::CloseDetail);
            }
        });
        ui.add_space(spacing::MEDIUM);

        let radiant_win = detail.radiant_win;
        let outcome = if radiant_win { i18n::message("matches-radiant") } else { i18n::message("matches-dire") };
        let outcome_color = if radiant_win { palette.victory } else { palette.defeat };

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("{} {}", outcome, i18n::message("match-victory")))
                    .size(font_size::LARGE)
                    .strong()
                    .color(outcome_color),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(format_duration(detail.duration)).size(font_size::MEDIUM).color(palette.text_secondary));
                ui.add_space(spacing::MEDIUM);
                ui.label(
                    egui::RichText::new(format!("{} : {}", detail.radiant_score, detail.dire_score))
                        .size(font_size::MEDIUM)
                        .strong()
                        .color(palette.text),
                );
                ui.add_space(spacing::MEDIUM);
                ui.label(
                    egui::RichText::new(game_mode_label(&i18n::current_locale(), detail.game_mode))
                        .size(font_size::SMALL)
                        .color(palette.text_muted),
                );
            });
        });

        ui.add_space(spacing::LARGE);

        self.team_section(ui, palette, detail, true, radiant_win);
        ui.add_space(spacing::LARGE);
        self.team_section(ui, palette, detail, false, !radiant_win);

        action
    }

    fn team_section(&self, ui: &mut egui::Ui, palette: &ColorPalette, detail: &MatchDetail, radiant: bool, won: bool) {
        let label = if radiant { i18n::message("matches-radiant") } else { i18n::message("matches-dire") };
        let tag = if won {
            (status::VICTORY, palette.victory, i18n::message("match-victory"))
        } else {
            (status::DEFEAT, palette.defeat, i18n::message("match-defeat"))
        };

        ui.horizontal(|ui| {
            widgets::section_label(ui, &label);
            ui.add_space(spacing::SMALL);
            ui.label(egui::RichText::new(format!("{} {}", tag.0, tag.2)).size(font_size::SMALL).strong().color(tag.1));
        });
        ui.add_space(spacing::SMALL);

        for player in detail.players.iter().filter(|p| p.is_radiant() == radiant) {
            self.player_row(ui, palette, player);
        }
    }

    fn player_row(&self, ui: &mut egui::Ui, palette: &ColorPalette, player: &MatchPlayer) {
        let height = if self.load_icons { 76.0 } else { 64.0 };
        widgets::list_row(ui, height, |ui| {
            ui.add_space(spacing::SMALL);
            if self.load_icons
                && let Some(url) = self.hero_icon_url(player.hero_id)
            {
                ui.add(
                    egui::Image::new(url.as_str())
                        .fit_to_exact_size(Vec2::splat(44.0))
                        .corner_radius(CornerRadius::same(radius::SMALL)),
                );
                ui.add_space(spacing::SMALL);
            }

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(self.hero_name(player.hero_id)).size(font_size::MEDIUM).color(palette.text));
                    ui.add_space(spacing::TINY);
                    aghanim_status(ui, self.load_icons, player.aghanims_scepter, player.aghanims_shard);
                });
                let persona = player.personaname.as_deref().unwrap_or("—");
                ui.label(egui::RichText::new(persona).size(font_size::SMALL).color(palette.text_muted));
                if self.load_icons {
                    self.item_icons(ui, palette, player);
                } else {
                    ui.label(egui::RichText::new(player.item_line(&self.item_names)).size(font_size::SMALL).color(palette.text_secondary));
                }
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(spacing::SMALL);
                stat(ui, palette, &i18n::message("match-net-worth"), &format_thousands(player.net_worth));
                stat(ui, palette, &i18n::message("stats-xpm"), &player.xp_per_min.to_string());
                stat(ui, palette, &i18n::message("stats-gpm"), &player.gold_per_min.to_string());
                stat(ui, palette, &i18n::message("stats-level"), &player.level.to_string());
                let kda = format!("{} / {} / {}", player.kills, player.deaths, player.assists);
                stat(ui, palette, &i18n::message("stats-kda"), &kda);
            });
        });
    }

    /// The player's six inventory slots plus the neutral item, as CDN icons.
    /// Items are landscape art (~88×64), so each slot is drawn wider than tall.
    fn item_icons(&self, ui: &mut egui::Ui, palette: &ColorPalette, player: &MatchPlayer) {
        let slot = vec2(36.0, 27.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = spacing::TINY;
            for &id in &player.items {
                self.item_slot(ui, palette, id, slot);
            }
            ui.add_space(spacing::SMALL);
            self.item_slot(ui, palette, player.item_neutral, slot);
            self.item_slot(ui, palette, player.item_neutral2, slot);
        });
    }

    fn item_slot(&self, ui: &mut egui::Ui, palette: &ColorPalette, item_id: i32, size: Vec2) {
        if item_id != 0
            && let Some(url) = self.item_icon_url(item_id)
        {
            let name = self.item_names.get(&item_id).cloned().unwrap_or_default();
            let image = ui.add(egui::Image::new(url.as_str()).fit_to_exact_size(size).corner_radius(CornerRadius::same(2)));
            if !name.is_empty() {
                image.on_hover_text(name);
            }
        } else {
            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
            ui.painter().rect_filled(rect, CornerRadius::same(2), palette.surface_secondary);
        }
    }

    fn empty_state(&self, ui: &mut egui::Ui, palette: &ColorPalette, message: &str) {
        ui.add_space(spacing::LARGE);
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new(status::OFFLINE).size(font_size::TITLE).color(palette.text_muted));
            ui.add_space(spacing::SMALL);
            ui.label(egui::RichText::new(message).size(font_size::MEDIUM).color(palette.text_secondary));
        });
    }
}

/// One match in the list: win/loss accent, optional hero icon, hero + KDA, then
/// GPM/XPM over the outcome and relative time. Clickable (opens the detail view).
fn summary_row(ui: &mut egui::Ui, palette: &ColorPalette, hero_icon: Option<&str>, hero_name: &str, m: &MatchSummary) -> egui::Response {
    let won = m.won();
    let outcome_color = if won { palette.victory } else { palette.defeat };
    let outcome_text = if won { i18n::message("match-victory") } else { i18n::message("match-defeat") };

    widgets::list_row(ui, 58.0, |ui| {
        ui.add_space(spacing::SMALL);
        widgets::accent_bar(ui, outcome_color);
        ui.add_space(spacing::MEDIUM);

        if let Some(url) = hero_icon {
            ui.add(egui::Image::new(url).fit_to_exact_size(Vec2::splat(40.0)).corner_radius(CornerRadius::same(radius::SMALL)));
            ui.add_space(spacing::SMALL);
        }

        ui.vertical(|ui| {
            ui.label(egui::RichText::new(hero_name).size(font_size::MEDIUM).color(palette.text));
            let kda = format!("{} / {} / {}", m.kills, m.deaths, m.assists);
            ui.label(egui::RichText::new(kda).size(font_size::SMALL).color(palette.text_secondary));
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(spacing::SMALL);
            ui.with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
                ui.label(
                    egui::RichText::new(format!("{outcome_text} · {}", format_duration(m.duration)))
                        .size(font_size::SMALL)
                        .color(outcome_color),
                );
                ui.label(
                    egui::RichText::new(format_relative(&i18n::current_locale(), m.start_time))
                        .size(font_size::SMALL)
                        .color(palette.text_muted),
                );
            });
            ui.add_space(spacing::MEDIUM);
            ui.with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
                ui.label(
                    egui::RichText::new(format!("{} {}", i18n::message("stats-gpm"), m.gold_per_min))
                        .size(font_size::SMALL)
                        .color(palette.text_muted),
                );
                ui.label(
                    egui::RichText::new(format!("{} {}", i18n::message("stats-xpm"), m.xp_per_min))
                        .size(font_size::SMALL)
                        .color(palette.text_muted),
                );
            });
        });
    })
}

/// Aghanim's Scepter and Shard status, shown as the real item icons — lit
/// when acquired, dimmed when not — so each player's upgrades read at a
/// glance. Falls back to plain colour badges when icon loading is disabled.
fn aghanim_status(ui: &mut egui::Ui, load_icons: bool, scepter: bool, shard: bool) {
    if load_icons {
        let scepter_url = format!("{ICON_CDN}/items/ultimate_scepter.png");
        let shard_url = format!("{ICON_CDN}/items/aghanims_shard.png");
        aghanim_icon(ui, &scepter_url, scepter, &i18n::message("matches-aghanim-scepter"));
        ui.add_space(spacing::TINY);
        aghanim_icon(ui, &shard_url, shard, &i18n::message("matches-aghanim-shard"));
    } else {
        aghanim_badge(ui, SCEPTER_COLOR, scepter, vec2(7.0, 16.0), &i18n::message("matches-aghanim-scepter"));
        ui.add_space(spacing::TINY);
        aghanim_badge(ui, SHARD_COLOR, shard, vec2(7.0, 11.0), &i18n::message("matches-aghanim-shard"));
    }
}

fn aghanim_icon(ui: &mut egui::Ui, url: &str, active: bool, hover: &str) {
    let tint = if active { Color32::WHITE } else { Color32::from_white_alpha(60) };
    ui.add(egui::Image::new(url).fit_to_exact_size(vec2(22.0, 16.0)).corner_radius(CornerRadius::same(2)).tint(tint))
        .on_hover_text(hover);
}

fn aghanim_badge(ui: &mut egui::Ui, color: Color32, active: bool, size: Vec2, hover: &str) {
    // Reserve the scepter's full height for both badges so the shorter shard
    // badge sits centred on the same baseline.
    let (rect, response) = ui.allocate_exact_size(vec2(size.x, 16.0), Sense::hover());
    let badge = egui::Rect::from_center_size(rect.center(), size);
    if active {
        let glow = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 70);
        ui.painter().rect_filled(badge.expand(2.0), CornerRadius::same(3), glow);
        ui.painter().rect_filled(badge, CornerRadius::same(2), color);
    } else {
        let dim = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 45);
        ui.painter().rect_filled(badge, CornerRadius::same(2), dim);
    }
    response.on_hover_text(hover);
}

/// A tiny right-aligned `value` over `label` stat block.
fn stat(ui: &mut egui::Ui, palette: &ColorPalette, label: &str, value: &str) {
    ui.add_space(spacing::SMALL);
    ui.allocate_ui_with_layout(vec2(56.0, 0.0), egui::Layout::top_down(egui::Align::Center), |ui| {
        ui.label(egui::RichText::new(value).size(font_size::BODY).strong().color(palette.text));
        ui.label(egui::RichText::new(label).size(font_size::SMALL).color(palette.text_muted));
    });
}

fn format_thousands(value: i32) -> String {
    if value >= 1000 { format!("{:.1}k", value as f32 / 1000.0) } else { value.to_string() }
}

impl Default for MatchScreen {
    fn default() -> Self {
        Self::new()
    }
}
