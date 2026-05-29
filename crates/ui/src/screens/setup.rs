use egui::Color32;

use crate::theme::{colors, font_size, radius, spacing};

pub struct SetupResult {
    pub language: String,
    pub app_path: String,
    pub open_settings: bool,
}

pub struct SetupScreen {
    language: &'static str,
    app_path: String,
    open_settings: bool,
    pub completed: bool,
}

impl Default for SetupScreen {
    fn default() -> Self {
        Self {
            language: "auto",
            app_path: configs::default_app_path().display().to_string(),
            open_settings: false,
            completed: false,
        }
    }
}

impl SetupScreen {
    pub fn result(&self) -> SetupResult {
        SetupResult {
            language: self.language.to_owned(),
            app_path: self.app_path.clone(),
            open_settings: self.open_settings,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let palette = colors();

        let available = ui.available_size();

        ui.vertical_centered(|ui| {
            ui.set_max_width(600.0);
            ui.add_space((available.y * 0.1).max(spacing::XLARGE));

            ui.label(egui::RichText::new(i18n::message("setup-title")).size(font_size::TITLE).color(palette.text));
            ui.add_space(spacing::SMALL);
            ui.label(egui::RichText::new(i18n::message("setup-subtitle")).size(font_size::LARGE).color(palette.text_secondary));
            ui.add_space(spacing::SMALL);
            ui.label(egui::RichText::new(i18n::message("setup-description")).size(font_size::BODY).color(palette.text_muted));

            ui.add_space(spacing::LARGE);

            // Language section
            egui::Frame::NONE
                .fill(palette.surface)
                .stroke(egui::Stroke::new(1.0, palette.border))
                .corner_radius(egui::CornerRadius::same(radius::MEDIUM))
                .inner_margin(spacing::MEDIUM)
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(i18n::message("setup-language-label")).size(font_size::BODY).color(palette.text));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let options = ["auto", "en", "zh-CN"];
                            egui::ComboBox::from_id_salt("setup_language").selected_text(self.language).show_ui(ui, |ui| {
                                for opt in &options {
                                    if ui.selectable_value(&mut self.language, opt, *opt).changed() && *opt != "auto" {
                                        i18n::set_locale(opt);
                                    }
                                }
                            });
                        });
                    });
                });

            ui.add_space(spacing::MEDIUM);

            // App path section
            egui::Frame::NONE
                .fill(palette.surface)
                .stroke(egui::Stroke::new(1.0, palette.border))
                .corner_radius(egui::CornerRadius::same(radius::MEDIUM))
                .inner_margin(spacing::MEDIUM)
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.label(egui::RichText::new(i18n::message("setup-app-path")).size(font_size::BODY).color(palette.text));
                    ui.label(
                        egui::RichText::new(i18n::message("setup-app-path-description"))
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
                        if ui.button(i18n::message("setup-browse")).clicked()
                            && let Some(path) = rfd::FileDialog::new().set_title("Choose App Data Directory").pick_folder()
                        {
                            self.app_path = path.display().to_string();
                        }
                    });
                });

            ui.add_space(spacing::LARGE);

            // Buttons
            ui.horizontal(|ui| {
                let settings_btn = egui::Button::new(
                    egui::RichText::new(i18n::message("setup-open-settings"))
                        .size(font_size::BODY)
                        .color(palette.text_secondary),
                )
                .stroke(egui::Stroke::new(1.0, palette.border))
                .fill(Color32::TRANSPARENT)
                .corner_radius(egui::CornerRadius::same(radius::MEDIUM));

                if ui.add(settings_btn).clicked() {
                    self.open_settings = true;
                    self.completed = true;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let continue_btn = egui::Button::new(egui::RichText::new(i18n::message("setup-confirm")).size(font_size::BODY).color(Color32::WHITE))
                        .fill(palette.primary)
                        .corner_radius(egui::CornerRadius::same(radius::MEDIUM));

                    if ui.add(continue_btn).clicked() {
                        self.completed = true;
                    }
                });
            });
        });
    }
}
