use std::path::PathBuf;

use configs::{ThemePreference, UpdateChannel};
use egui::Color32;

use crate::theme::{colors, font_size, radius, spacing};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsAction {
    ThemeChanged(ThemePreference),
    Reset,
}

pub struct SettingScreen {
    app_path: String,
    reset_confirming: bool,

    dota2_steam_id: String,
    dota2_opendota_key: String,
}

impl SettingScreen {
    pub fn new() -> Self {
        let bootstrap = configs::bootstrap();
        let config = configs::read();
        Self {
            app_path: bootstrap.app_path().display().to_string(),
            reset_confirming: false,
            dota2_steam_id: config.games.dota2.steam_id.clone().unwrap_or_default(),
            dota2_opendota_key: config.games.dota2.opendota_api_key.clone().unwrap_or_default(),
        }
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

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<SettingsAction> {
        let mut action = None;
        let palette = colors();
        let config = configs::read().clone();

        let mut general = config.general.clone();
        let mut notification = config.notification.clone();
        let mut appearance = config.appearance.clone();
        let mut tracking = config.tracking.clone();
        let mut games = config.games.clone();

        let mut current_language = Self::language_code_to_static(&general.language);

        let mut changed = false;

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.set_min_width(ui.available_width());

            ui.vertical(|ui| {
                ui.add_space(spacing::XLARGE);

                ui.label(egui::RichText::new(i18n::message("settings-title")).size(font_size::TITLE).color(palette.text));

                ui.add_space(spacing::LARGE);

                self.storage_section(ui);
                ui.add_space(spacing::MEDIUM);

                if let Some(a) = Self::general_section(ui, &mut current_language, &mut general) {
                    action = Some(a);
                    changed = true;
                }
                ui.add_space(spacing::MEDIUM);

                if Self::appearance_section(ui, &mut appearance) {
                    changed = true;
                }
                ui.add_space(spacing::MEDIUM);

                if Self::tracking_section(ui, &mut tracking) {
                    changed = true;
                }
                ui.add_space(spacing::MEDIUM);

                if self.games_section(ui, &mut games) {
                    changed = true;
                }
                ui.add_space(spacing::MEDIUM);

                if Self::notifications_section(ui, &mut notification) {
                    changed = true;
                }
                ui.add_space(spacing::MEDIUM);

                if Self::updates_section(ui, &mut general) {
                    changed = true;
                }
                ui.add_space(spacing::MEDIUM);

                Self::about_section(ui);

                ui.add_space(spacing::LARGE);

                ui.horizontal(|ui| {
                    // Save button
                    let save_btn = egui::Button::new(egui::RichText::new(i18n::message("settings-save")).size(font_size::BODY).color(Color32::WHITE))
                        .fill(palette.primary)
                        .corner_radius(egui::CornerRadius::same(radius::MEDIUM));

                    if ui.add(save_btn).clicked() {
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
                            action = Some(SettingsAction::Reset);
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
        games.dota2.opendota_api_key = if self.dota2_opendota_key.is_empty() {
            None
        } else {
            Some(self.dota2_opendota_key.clone())
        };

        if changed || config.tracking != tracking || config.games != games {
            configs::update(|cfg| {
                cfg.general = general;
                cfg.notification = notification;
                cfg.appearance = appearance;
                cfg.tracking = tracking;
                cfg.games = games;
            });
        }

        action
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
            .stroke(egui::Stroke::new(1.0, palette.border))
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
                    for (pref, label_key) in [
                        (ThemePreference::Dark, "settings-theme-dark"),
                        (ThemePreference::Light, "settings-theme-light"),
                        (ThemePreference::System, "settings-theme-system"),
                    ] {
                        let is_active = general.theme == pref;
                        let btn = egui::Button::new(egui::RichText::new(i18n::message(label_key)).size(font_size::SMALL))
                            .fill(if is_active { palette.sidebar_item_active } else { Color32::TRANSPARENT })
                            .stroke(if is_active { egui::Stroke::new(2.0, palette.primary) } else { egui::Stroke::NONE })
                            .corner_radius(egui::CornerRadius::same(radius::MEDIUM));

                        if ui.add(btn).clicked() {
                            general.theme = pref;
                            action = Some(SettingsAction::ThemeChanged(pref));
                        }
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
                    if ui.checkbox(&mut tracking.background_tracking, "").changed() {
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
                    if ui.checkbox(&mut games.dota2.enabled, "").changed() {
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

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(i18n::message("settings-dota2-opendota-key")).size(font_size::BODY).color(palette.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.dota2_opendota_key)
                            .desired_width(200.0)
                            .password(true)
                            .font(egui::FontId::proportional(font_size::BODY)),
                    );
                });
            });
        });
        changed
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
                    if ui.checkbox(&mut notification.desktop.enabled, "").changed() {
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
                    if ui.checkbox(&mut notification.desktop.sound, "").changed() {
                        changed = true;
                    }
                });
            });

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(i18n::message("settings-notify-new-match")).size(font_size::BODY).color(palette.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.checkbox(&mut notification.desktop.notify_new_match, "").changed() {
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
                    if ui.checkbox(&mut general.check_updates, "").changed() {
                        changed = true;
                    }
                });
            });

            ui.add_space(spacing::SMALL);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(i18n::message("settings-update-channel")).size(font_size::BODY).color(palette.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    for (channel, label_key) in [(UpdateChannel::Beta, "settings-update-beta"), (UpdateChannel::Stable, "settings-update-stable")] {
                        let is_active = general.update_channel == channel;
                        let btn = egui::Button::new(egui::RichText::new(i18n::message(label_key)).size(font_size::SMALL))
                            .fill(if is_active { palette.sidebar_item_active } else { Color32::TRANSPARENT })
                            .stroke(if is_active { egui::Stroke::new(2.0, palette.primary) } else { egui::Stroke::NONE })
                            .corner_radius(egui::CornerRadius::same(radius::MEDIUM));

                        if ui.add(btn).clicked() {
                            general.update_channel = channel;
                            changed = true;
                        }
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
