use std::path::PathBuf;

use configs::{ThemePreference, UpdateChannel};
use egui::Color32;

use crate::{
    components::{search, widgets},
    theme::{colors, font_size, radius, spacing},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsAction {
    ThemeChanged(ThemePreference),
    StoragePathChanged(PathBuf),
    ProxyChanged(Option<String>),
    SyncStaticData,
    Reset,
}

/// i18n keys (section title + the labels it contains) that a section is matched
/// against when the user types in the settings search box. Empty query shows all.
struct Section {
    keys: &'static [&'static str],
}

const STORAGE: Section = Section {
    keys: &["settings-storage", "settings-app-path", "settings-storage-path"],
};
const GENERAL: Section = Section {
    keys: &["settings-general", "settings-language", "settings-theme"],
};
const APPEARANCE: Section = Section {
    keys: &[
        "settings-appearance",
        "settings-appearance-sidebar-width",
        "settings-appearance-sidebar-collapsed-width",
        "settings-appearance-sidebar-item-height",
        "settings-appearance-sidebar-icon-size",
        "settings-appearance-animation-sidebar",
        "settings-appearance-animation-toast",
        "settings-appearance-animation-hover",
        "settings-appearance-toast-max-width",
        "settings-appearance-toast-duration",
        "settings-appearance-exit-overlay-opacity",
    ],
};
const TRACKING: Section = Section {
    keys: &["settings-tracking", "settings-poll-interval", "settings-background-tracking"],
};
const GAMES: Section = Section {
    keys: &["settings-games", "settings-dota2", "settings-dota2-enabled", "settings-dota2-steam-id"],
};
const FRIENDS: Section = Section {
    keys: &["settings-friends", "settings-friends-load-avatars", "settings-friends-recent-games-limit"],
};
const MATCHES: Section = Section {
    keys: &["settings-matches", "settings-matches-max-history", "settings-matches-load-icons"],
};
const GAME_DATA: Section = Section { keys: &["settings-game-data"] };
const NETWORK: Section = Section {
    keys: &["settings-network", "settings-network-proxy"],
};
const SECRETS: Section = Section {
    keys: &[
        "settings-secrets",
        "settings-secrets-steam-web-api-key",
        "settings-secrets-stratz-api-token",
        "settings-secrets-opendota-api-key",
    ],
};
const NOTIFICATIONS: Section = Section {
    keys: &[
        "settings-notifications",
        "settings-desktop-notifications-enabled",
        "settings-desktop-notifications-sound",
        "settings-notify-new-match",
    ],
};
const UPDATES: Section = Section {
    keys: &["settings-updates", "settings-check-updates", "settings-update-channel"],
};
const ABOUT: Section = Section {
    keys: &["settings-about", "settings-version"],
};

pub struct SettingScreen {
    search_query: String,

    app_path: String,
    storage_path: String,
    reset_confirming: bool,
    syncing: bool,

    dota2_steam_id: String,

    proxy: String,

    steam_web_api_key: String,
    stratz_api_token: String,
    opendota_api_key: String,
}

impl SettingScreen {
    pub fn new() -> Self {
        let bootstrap = configs::bootstrap();
        let config = configs::read();
        Self {
            search_query: String::new(),
            app_path: bootstrap.app_path().display().to_string(),
            storage_path: bootstrap.storage_path().display().to_string(),
            reset_confirming: false,
            syncing: false,
            dota2_steam_id: config.games.dota2.steam_id.clone().unwrap_or_default(),
            proxy: config.network.proxy.clone().unwrap_or_default(),
            steam_web_api_key: config.secrets.steam_web_api_key.clone().unwrap_or_default(),
            stratz_api_token: config.secrets.stratz_api_token.clone().unwrap_or_default(),
            opendota_api_key: config.secrets.opendota_api_key.clone().unwrap_or_default(),
        }
    }

    fn section_visible(query: &str, section: &Section) -> bool {
        query.is_empty() || section.keys.iter().any(|key| i18n::message(key).to_lowercase().contains(query))
    }

    fn language_code_to_static(code: &str) -> &'static str {
        match code {
            "zh-CN" => "zh-CN",
            "en" => "en",
            _ => "auto",
        }
    }

    fn app_path_changed(&self) -> bool {
        let bootstrap = configs::bootstrap();
        let current = bootstrap.app_path().display().to_string();
        self.app_path != current
    }

    fn storage_path_changed(&self) -> bool {
        let bootstrap = configs::bootstrap();
        let current = bootstrap.storage_path().display().to_string();
        self.storage_path != current
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Vec<SettingsAction> {
        let mut actions = Vec::new();
        let palette = colors();
        let config = configs::read().clone();

        let mut general = config.general.clone();
        let mut notification = config.notification.clone();
        let mut appearance = config.appearance.clone();
        let mut tracking = config.tracking.clone();
        let mut games = config.games.clone();
        let mut friends = config.friends.clone();
        let mut matches = config.matches.clone();

        let mut current_language = Self::language_code_to_static(&general.language);

        let mut changed = false;

        let query = self.search_query.to_lowercase();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.vertical(|ui| {
                ui.add_space(spacing::LARGE);

                ui.label(egui::RichText::new(i18n::message("settings-title")).size(font_size::TITLE).strong().color(palette.text));

                ui.add_space(spacing::MEDIUM);

                search::search_input(ui, &i18n::message("settings-search-placeholder"), &mut self.search_query);

                ui.add_space(spacing::LARGE);

                let mut shown_any = false;

                if Self::section_visible(&query, &STORAGE) {
                    self.storage_section(ui);
                    ui.add_space(spacing::MEDIUM);
                    shown_any = true;
                }

                if Self::section_visible(&query, &GENERAL) {
                    if let Some(a) = Self::general_section(ui, &mut current_language, &mut general) {
                        actions.push(a);
                        changed = true;
                    }
                    ui.add_space(spacing::MEDIUM);
                    shown_any = true;
                }

                if Self::section_visible(&query, &APPEARANCE) {
                    if Self::appearance_section(ui, &mut appearance) {
                        changed = true;
                    }
                    ui.add_space(spacing::MEDIUM);
                    shown_any = true;
                }

                if Self::section_visible(&query, &TRACKING) {
                    if Self::tracking_section(ui, &mut tracking) {
                        changed = true;
                    }
                    ui.add_space(spacing::MEDIUM);
                    shown_any = true;
                }

                if Self::section_visible(&query, &GAMES) {
                    if self.games_section(ui, &mut games) {
                        changed = true;
                    }
                    ui.add_space(spacing::MEDIUM);
                    shown_any = true;
                }

                if Self::section_visible(&query, &FRIENDS) {
                    if Self::friends_section(ui, &mut friends) {
                        changed = true;
                    }
                    ui.add_space(spacing::MEDIUM);
                    shown_any = true;
                }

                if Self::section_visible(&query, &MATCHES) {
                    if Self::matches_section(ui, &mut matches) {
                        changed = true;
                    }
                    ui.add_space(spacing::MEDIUM);
                    shown_any = true;
                }

                if Self::section_visible(&query, &GAME_DATA) {
                    if self.game_data_section(ui) {
                        actions.push(SettingsAction::SyncStaticData);
                    }
                    ui.add_space(spacing::MEDIUM);
                    shown_any = true;
                }

                if Self::section_visible(&query, &NETWORK) {
                    self.network_section(ui);
                    ui.add_space(spacing::MEDIUM);
                    shown_any = true;
                }

                if Self::section_visible(&query, &SECRETS) {
                    self.secrets_section(ui);
                    ui.add_space(spacing::MEDIUM);
                    shown_any = true;
                }

                if Self::section_visible(&query, &NOTIFICATIONS) {
                    if Self::notifications_section(ui, &mut notification) {
                        changed = true;
                    }
                    ui.add_space(spacing::MEDIUM);
                    shown_any = true;
                }

                if Self::section_visible(&query, &UPDATES) {
                    if Self::updates_section(ui, &mut general) {
                        changed = true;
                    }
                    ui.add_space(spacing::MEDIUM);
                    shown_any = true;
                }

                if Self::section_visible(&query, &ABOUT) {
                    Self::about_section(ui);
                    shown_any = true;
                }

                if !shown_any {
                    ui.add_space(spacing::LARGE);
                    ui.label(
                        egui::RichText::new(i18n::message("settings-search-no-results"))
                            .size(font_size::BODY)
                            .color(palette.text_muted),
                    );
                }

                ui.add_space(spacing::LARGE);

                ui.horizontal(|ui| {
                    if widgets::primary_button(ui, i18n::message("settings-save")).clicked() {
                        if self.storage_path_changed() {
                            actions.push(SettingsAction::StoragePathChanged(PathBuf::from(&self.storage_path)));
                        }
                        actions.push(SettingsAction::ProxyChanged(Self::optional(&self.proxy)));
                        self.save_to_disk();
                    }

                    ui.add_space(spacing::MEDIUM);

                    // Reset button with double confirmation
                    if self.reset_confirming {
                        ui.label(
                            egui::RichText::new(i18n::message("settings-reset-confirm-message"))
                                .size(font_size::SMALL)
                                .color(palette.danger),
                        );

                        let confirm_btn = egui::Button::new(egui::RichText::new(i18n::message("common-confirm")).size(font_size::BODY).color(Color32::WHITE))
                            .fill(palette.danger)
                            .corner_radius(egui::CornerRadius::same(radius::MEDIUM));

                        if ui.add(confirm_btn).clicked() {
                            self.reset_confirming = false;
                            actions.push(SettingsAction::Reset);
                        }

                        let cancel_btn = egui::Button::new(egui::RichText::new(i18n::message("common-cancel")).size(font_size::BODY).color(palette.text))
                            .corner_radius(egui::CornerRadius::same(radius::MEDIUM));

                        if ui.add(cancel_btn).clicked() {
                            self.reset_confirming = false;
                        }
                    } else {
                        let reset_btn = egui::Button::new(egui::RichText::new(i18n::message("settings-reset")).size(font_size::BODY).color(palette.danger))
                            .corner_radius(egui::CornerRadius::same(radius::MEDIUM));

                        if ui.add(reset_btn).clicked() {
                            self.reset_confirming = true;
                        }
                    }
                });

                ui.add_space(spacing::XLARGE);
            });
        });

        // Sync language change back
        if current_language != Self::language_code_to_static(&general.language) {
            general.language = current_language.to_owned();
            changed = true;
        }

        // Sync string fields back to config
        games.dota2.steam_id = if self.dota2_steam_id.is_empty() { None } else { Some(self.dota2_steam_id.clone()) };

        let network = configs::NetworkConfig {
            proxy: Self::optional(&self.proxy),
        };

        let secrets = configs::SecretsConfig {
            steam_web_api_key: Self::optional(&self.steam_web_api_key),
            stratz_api_token: Self::optional(&self.stratz_api_token),
            opendota_api_key: Self::optional(&self.opendota_api_key),
        };

        if changed
            || config.tracking != tracking
            || config.games != games
            || config.friends != friends
            || config.matches != matches
            || config.network != network
            || config.secrets != secrets
        {
            configs::update(|cfg| {
                cfg.general = general;
                cfg.notification = notification;
                cfg.appearance = appearance;
                cfg.tracking = tracking;
                cfg.games = games;
                cfg.friends = friends;
                cfg.matches = matches;
                cfg.network = network;
                cfg.secrets = secrets;
            });
        }

        actions
    }

    pub fn set_syncing(&mut self, syncing: bool) {
        self.syncing = syncing;
    }

    fn optional(value: &str) -> Option<String> {
        if value.is_empty() { None } else { Some(value.to_owned()) }
    }

    fn save_to_disk(&self) {
        if self.app_path_changed() {
            let new_path = PathBuf::from(&self.app_path);
            if let Err(e) = configs::migrate_app_path(new_path) {
                tracing::error!("Failed to migrate app data: {e}");
            }
        }

        if let Err(e) = configs::save() {
            tracing::error!("Failed to save config: {e}");
        }
    }

    fn section_frame(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
        let palette = colors();
        egui::Frame::NONE
            .fill(palette.surface)
            .stroke(egui::Stroke::new(1_f32, palette.border))
            .corner_radius(egui::CornerRadius::same(radius::MEDIUM))
            .inner_margin(spacing::MEDIUM)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                add_contents(ui);
            });
    }

    fn storage_section(&mut self, ui: &mut egui::Ui) {
        let palette = colors();
        Self::section_frame(ui, |ui| {
            ui.label(egui::RichText::new(i18n::message("settings-storage")).size(font_size::LARGE).color(palette.text));
            ui.add_space(spacing::SMALL);

            ui.label(egui::RichText::new(i18n::message("settings-app-path")).size(font_size::BODY).color(palette.text));
            ui.label(
                egui::RichText::new(i18n::message("settings-app-path-description"))
                    .size(font_size::SMALL)
                    .color(palette.text_muted),
            );
            ui.add_space(spacing::SMALL);

            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.app_path)
                        .desired_width(ui.available_width() - 100.0)
                        .font(egui::FontId::proportional(font_size::BODY)),
                );
                if ui.button(i18n::message("settings-browse")).clicked()
                    && let Some(path) = rfd::FileDialog::new().set_title("Choose App Data Directory").pick_folder()
                {
                    self.app_path = path.display().to_string();
                }
            });

            if self.app_path_changed() {
                ui.add_space(spacing::SMALL);
                ui.label(
                    egui::RichText::new(i18n::message("settings-path-will-migrate"))
                        .size(font_size::SMALL)
                        .color(palette.primary),
                );
            }

            ui.add_space(spacing::MEDIUM);

            ui.label(egui::RichText::new(i18n::message("settings-storage-path")).size(font_size::BODY).color(palette.text));
            ui.label(
                egui::RichText::new(i18n::message("settings-storage-path-description"))
                    .size(font_size::SMALL)
                    .color(palette.text_muted),
            );
            ui.add_space(spacing::SMALL);

            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.storage_path)
                        .desired_width(ui.available_width() - 100.0)
                        .font(egui::FontId::proportional(font_size::BODY)),
                );
                if ui.button(i18n::message("settings-browse")).clicked()
                    && let Some(path) = rfd::FileDialog::new().set_title("Choose Storage Directory").pick_folder()
                {
                    self.storage_path = path.display().to_string();
                }
            });

            if self.storage_path_changed() {
                ui.add_space(spacing::SMALL);
                ui.label(
                    egui::RichText::new(i18n::message("settings-path-will-migrate"))
                        .size(font_size::SMALL)
                        .color(palette.primary),
                );
            }
        });
    }

    fn network_section(&mut self, ui: &mut egui::Ui) {
        let palette = colors();
        Self::section_frame(ui, |ui| {
            ui.label(egui::RichText::new(i18n::message("settings-network")).size(font_size::LARGE).color(palette.text));
            ui.label(
                egui::RichText::new(i18n::message("settings-network-proxy-description"))
                    .size(font_size::SMALL)
                    .color(palette.text_muted),
            );
            ui.add_space(spacing::SMALL);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(i18n::message("settings-network-proxy")).size(font_size::BODY).color(palette.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.proxy)
                            .desired_width(240.0)
                            .hint_text(i18n::message("settings-network-proxy-placeholder"))
                            .font(egui::FontId::proportional(font_size::BODY)),
                    );
                });
            });
        });
    }

    fn secrets_section(&mut self, ui: &mut egui::Ui) {
        let palette = colors();
        Self::section_frame(ui, |ui| {
            ui.label(egui::RichText::new(i18n::message("settings-secrets")).size(font_size::LARGE).color(palette.text));
            ui.label(
                egui::RichText::new(i18n::message("settings-secrets-description"))
                    .size(font_size::SMALL)
                    .color(palette.text_muted),
            );
            ui.add_space(spacing::SMALL);

            for (label_key, value) in [
                ("settings-secrets-steam-web-api-key", &mut self.steam_web_api_key),
                ("settings-secrets-stratz-api-token", &mut self.stratz_api_token),
                ("settings-secrets-opendota-api-key", &mut self.opendota_api_key),
            ] {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(i18n::message(label_key)).size(font_size::BODY).color(palette.text));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add(
                            egui::TextEdit::singleline(value)
                                .desired_width(240.0)
                                .password(true)
                                .font(egui::FontId::proportional(font_size::BODY)),
                        );
                    });
                });
            }
        });
    }

    fn general_section(ui: &mut egui::Ui, current_language: &mut &'static str, general: &mut configs::GeneralConfig) -> Option<SettingsAction> {
        let palette = colors();
        let mut action = None;
        Self::section_frame(ui, |ui| {
            ui.label(egui::RichText::new(i18n::message("settings-general")).size(font_size::LARGE).color(palette.text));
            ui.add_space(spacing::SMALL);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(i18n::message("settings-language")).size(font_size::BODY).color(palette.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let options = ["auto", "en", "zh-CN"];
                    egui::ComboBox::from_id_salt("settings_language").selected_text(*current_language).show_ui(ui, |ui| {
                        for opt in &options {
                            if ui.selectable_value(current_language, opt, *opt).changed() && *opt != "auto" {
                                i18n::set_locale(opt);
                            }
                        }
                    });
                });
            });

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(i18n::message("settings-theme")).size(font_size::BODY).color(palette.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let before = general.theme;
                    let options = [
                        (ThemePreference::Dark, i18n::message("settings-theme-dark")),
                        (ThemePreference::Light, i18n::message("settings-theme-light")),
                        (ThemePreference::System, i18n::message("settings-theme-system")),
                    ];
                    widgets::segmented(ui, &mut general.theme, &options);
                    if general.theme != before {
                        action = Some(SettingsAction::ThemeChanged(general.theme));
                    }
                });
            });
        });
        action
    }

    fn appearance_section(ui: &mut egui::Ui, appearance: &mut configs::AppearanceConfig) -> bool {
        let palette = colors();
        let mut changed = false;

        Self::section_frame(ui, |ui| {
            ui.label(egui::RichText::new(i18n::message("settings-appearance")).size(font_size::LARGE).color(palette.text));
            ui.add_space(spacing::SMALL);

            let label_width = 200.0;

            egui::Grid::new("appearance_grid")
                .num_columns(3)
                .spacing([spacing::MEDIUM, spacing::SMALL])
                .min_col_width(label_width)
                .show(ui, |ui| {
                    ui.spacing_mut().slider_width = 200.0;

                    changed |= Self::grid_slider_row(ui, &i18n::message("settings-appearance-sidebar-width"), &mut appearance.sidebar_width, 120.0..=400.0);
                    changed |= Self::grid_slider_row(
                        ui,
                        &i18n::message("settings-appearance-sidebar-collapsed-width"),
                        &mut appearance.sidebar_collapsed_width,
                        40.0..=120.0,
                    );
                    changed |= Self::grid_slider_row(
                        ui,
                        &i18n::message("settings-appearance-sidebar-item-height"),
                        &mut appearance.sidebar_item_height,
                        28.0..=80.0,
                    );
                    changed |= Self::grid_slider_row(ui, &i18n::message("settings-appearance-sidebar-icon-size"), &mut appearance.sidebar_icon_size, 12.0..=40.0);
                    changed |= Self::grid_slider_row(
                        ui,
                        &i18n::message("settings-appearance-animation-sidebar"),
                        &mut appearance.animation_sidebar_speed,
                        1.0..=20.0,
                    );
                    changed |= Self::grid_slider_row(ui, &i18n::message("settings-appearance-animation-toast"), &mut appearance.animation_toast_speed, 1.0..=30.0);
                    changed |= Self::grid_slider_row(ui, &i18n::message("settings-appearance-animation-hover"), &mut appearance.animation_hover_speed, 1.0..=30.0);
                    changed |= Self::grid_slider_row(ui, &i18n::message("settings-appearance-toast-max-width"), &mut appearance.toast_max_width, 200.0..=600.0);

                    // Toast duration (u64)
                    ui.label(
                        egui::RichText::new(i18n::message("settings-appearance-toast-duration"))
                            .size(font_size::BODY)
                            .color(palette.text),
                    );
                    let mut dur = appearance.toast_duration_secs as f32;
                    if ui.add(egui::Slider::new(&mut dur, 1.0..=30.0).integer().show_value(false)).changed() {
                        appearance.toast_duration_secs = dur as u64;
                        changed = true;
                    }
                    ui.end_row();

                    ui.label(
                        egui::RichText::new(i18n::message("settings-appearance-exit-overlay-opacity"))
                            .size(font_size::BODY)
                            .color(palette.text),
                    );
                    let mut opacity = appearance.exit_overlay_opacity as f32;
                    if ui.add(egui::Slider::new(&mut opacity, 0.0..=255.0).integer().show_value(false)).changed() {
                        appearance.exit_overlay_opacity = opacity as u8;
                        changed = true;
                    }
                    ui.end_row();
                });
        });

        changed
    }

    fn grid_slider_row(ui: &mut egui::Ui, label: &str, value: &mut f32, range: std::ops::RangeInclusive<f32>) -> bool {
        let palette = colors();
        let mut changed = false;

        ui.label(egui::RichText::new(label).size(font_size::BODY).color(palette.text));
        if ui.add(egui::Slider::new(value, range)).changed() {
            changed = true;
        }
        ui.end_row();

        changed
    }

    fn tracking_section(ui: &mut egui::Ui, tracking: &mut configs::TrackingConfig) -> bool {
        let palette = colors();
        let mut changed = false;
        Self::section_frame(ui, |ui| {
            ui.label(egui::RichText::new(i18n::message("settings-tracking")).size(font_size::LARGE).color(palette.text));
            ui.add_space(spacing::SMALL);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(i18n::message("settings-poll-interval")).size(font_size::BODY).color(palette.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut mins = tracking.poll_interval_mins as f32;
                    if ui.add(egui::Slider::new(&mut mins, 1.0..=60.0).integer().suffix(" min")).changed() {
                        tracking.poll_interval_mins = mins as u32;
                        changed = true;
                    }
                });
            });

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(i18n::message("settings-background-tracking")).size(font_size::BODY).color(palette.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::toggle(ui, &mut tracking.background_tracking).changed() {
                        changed = true;
                    }
                });
            });
        });
        changed
    }

    fn games_section(&mut self, ui: &mut egui::Ui, games: &mut configs::GamesConfig) -> bool {
        let palette = colors();
        let mut changed = false;
        Self::section_frame(ui, |ui| {
            ui.label(egui::RichText::new(i18n::message("settings-games")).size(font_size::LARGE).color(palette.text));
            ui.add_space(spacing::SMALL);

            // Dota 2 sub-section
            ui.label(egui::RichText::new(i18n::message("settings-dota2")).size(font_size::BODY).strong().color(palette.text));
            ui.add_space(spacing::SMALL);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(i18n::message("settings-dota2-enabled")).size(font_size::BODY).color(palette.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::toggle(ui, &mut games.dota2.enabled).changed() {
                        changed = true;
                    }
                });
            });

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(i18n::message("settings-dota2-steam-id")).size(font_size::BODY).color(palette.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.dota2_steam_id)
                            .desired_width(200.0)
                            .font(egui::FontId::proportional(font_size::BODY)),
                    );
                });
            });
        });
        changed
    }

    fn friends_section(ui: &mut egui::Ui, friends: &mut configs::FriendsConfig) -> bool {
        let palette = colors();
        let mut changed = false;
        Self::section_frame(ui, |ui| {
            ui.label(egui::RichText::new(i18n::message("settings-friends")).size(font_size::LARGE).color(palette.text));
            ui.add_space(spacing::SMALL);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(i18n::message("settings-friends-load-avatars"))
                            .size(font_size::BODY)
                            .color(palette.text),
                    );
                    ui.label(
                        egui::RichText::new(i18n::message("settings-friends-load-avatars-description"))
                            .size(font_size::SMALL)
                            .color(palette.text_muted),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::toggle(ui, &mut friends.load_avatars).changed() {
                        changed = true;
                    }
                });
            });

            ui.add_space(spacing::SMALL);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(i18n::message("settings-friends-recent-games-limit"))
                            .size(font_size::BODY)
                            .color(palette.text),
                    );
                    ui.label(
                        egui::RichText::new(i18n::message("settings-friends-recent-games-limit-description"))
                            .size(font_size::SMALL)
                            .color(palette.text_muted),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut limit = friends.recent_games_limit as f32;
                    if ui.add(egui::Slider::new(&mut limit, 1.0..=10.0).integer()).changed() {
                        friends.recent_games_limit = limit as usize;
                        changed = true;
                    }
                });
            });
        });
        changed
    }

    fn matches_section(ui: &mut egui::Ui, matches: &mut configs::MatchesConfig) -> bool {
        let palette = colors();
        let mut changed = false;
        Self::section_frame(ui, |ui| {
            ui.label(egui::RichText::new(i18n::message("settings-matches")).size(font_size::LARGE).color(palette.text));
            ui.add_space(spacing::SMALL);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(i18n::message("settings-matches-max-history")).size(font_size::BODY).color(palette.text));
                    ui.label(
                        egui::RichText::new(i18n::message("settings-matches-max-history-description"))
                            .size(font_size::SMALL)
                            .color(palette.text_muted),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut limit = matches.max_match_history as f32;
                    if ui.add(egui::Slider::new(&mut limit, 1.0..=25.0).integer()).changed() {
                        matches.max_match_history = limit as usize;
                        changed = true;
                    }
                });
            });

            ui.add_space(spacing::SMALL);

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(i18n::message("settings-matches-load-icons")).size(font_size::BODY).color(palette.text));
                    ui.label(
                        egui::RichText::new(i18n::message("settings-matches-load-icons-description"))
                            .size(font_size::SMALL)
                            .color(palette.text_muted),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::toggle(ui, &mut matches.load_icons).changed() {
                        changed = true;
                    }
                });
            });
        });
        changed
    }

    /// "Game Data" section: a manual trigger to (re)sync the static hero/item
    /// reference data into the database. Returns `true` when sync is requested.
    fn game_data_section(&self, ui: &mut egui::Ui) -> bool {
        let palette = colors();
        let mut sync_requested = false;
        Self::section_frame(ui, |ui| {
            ui.label(egui::RichText::new(i18n::message("settings-game-data")).size(font_size::LARGE).color(palette.text));
            ui.label(
                egui::RichText::new(i18n::message("settings-game-data-description"))
                    .size(font_size::SMALL)
                    .color(palette.text_muted),
            );
            ui.add_space(spacing::SMALL);

            let label = if self.syncing {
                i18n::message("settings-game-data-syncing")
            } else {
                i18n::message("settings-game-data-sync")
            };
            if widgets::primary_button_enabled(ui, label, !self.syncing).clicked() {
                sync_requested = true;
            }
        });
        sync_requested
    }

    fn notifications_section(ui: &mut egui::Ui, notification: &mut configs::NotificationsConfig) -> bool {
        let palette = colors();
        let mut changed = false;
        Self::section_frame(ui, |ui| {
            ui.label(egui::RichText::new(i18n::message("settings-notifications")).size(font_size::LARGE).color(palette.text));
            ui.add_space(spacing::SMALL);

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(i18n::message("settings-desktop-notifications-enabled"))
                        .size(font_size::BODY)
                        .color(palette.text),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::toggle(ui, &mut notification.desktop.enabled).changed() {
                        changed = true;
                    }
                });
            });

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(i18n::message("settings-desktop-notifications-sound"))
                        .size(font_size::BODY)
                        .color(palette.text),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::toggle(ui, &mut notification.desktop.sound).changed() {
                        changed = true;
                    }
                });
            });

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(i18n::message("settings-notify-new-match")).size(font_size::BODY).color(palette.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::toggle(ui, &mut notification.desktop.notify_new_match).changed() {
                        changed = true;
                    }
                });
            });
        });
        changed
    }

    fn updates_section(ui: &mut egui::Ui, general: &mut configs::GeneralConfig) -> bool {
        let palette = colors();
        let mut changed = false;
        Self::section_frame(ui, |ui| {
            ui.label(egui::RichText::new(i18n::message("settings-updates")).size(font_size::LARGE).color(palette.text));
            ui.add_space(spacing::SMALL);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(i18n::message("settings-check-updates")).size(font_size::BODY).color(palette.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if widgets::toggle(ui, &mut general.check_updates).changed() {
                        changed = true;
                    }
                });
            });

            ui.add_space(spacing::SMALL);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(i18n::message("settings-update-channel")).size(font_size::BODY).color(palette.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let before = general.update_channel;
                    let options = [
                        (UpdateChannel::Beta, i18n::message("settings-update-beta")),
                        (UpdateChannel::Stable, i18n::message("settings-update-stable")),
                    ];
                    widgets::segmented(ui, &mut general.update_channel, &options);
                    if general.update_channel != before {
                        changed = true;
                    }
                });
            });
        });
        changed
    }

    fn about_section(ui: &mut egui::Ui) {
        let palette = colors();
        Self::section_frame(ui, |ui| {
            ui.label(egui::RichText::new(i18n::message("settings-about")).size(font_size::LARGE).color(palette.text));
            ui.add_space(spacing::SMALL);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(i18n::message("settings-version")).size(font_size::BODY).color(palette.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(env!("CARGO_PKG_VERSION")).size(font_size::BODY).color(palette.text_secondary));
                });
            });
        });
    }
}

impl Default for SettingScreen {
    fn default() -> Self {
        Self::new()
    }
}
