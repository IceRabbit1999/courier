use crate::{app::Route, theme::colors};

pub enum MenuAction {
    Navigate(Route),
    OpenFeedback,
    ShowAbout,
    CheckUpdates,
    Exit,
}

pub fn show(ui: &mut egui::Ui) -> Option<MenuAction> {
    let palette = colors();
    let mut action = None;

    egui::Frame::NONE.fill(palette.surface).show(ui, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button(i18n::message("menu-file"), |ui| {
                if ui.button(i18n::message("menu-settings")).clicked() {
                    action = Some(MenuAction::Navigate(Route::Settings));
                    ui.close();
                }
                ui.separator();
                if ui.button(i18n::message("menu-exit")).clicked() {
                    action = Some(MenuAction::Exit);
                    ui.close();
                }
            });

            ui.menu_button(i18n::message("menu-help"), |ui| {
                if ui.button(i18n::message("menu-feedback")).clicked() {
                    action = Some(MenuAction::OpenFeedback);
                    ui.close();
                }
                if ui.button(i18n::message("menu-check-updates")).clicked() {
                    action = Some(MenuAction::CheckUpdates);
                    ui.close();
                }
                ui.separator();
                if ui.button(i18n::message("menu-about")).clicked() {
                    action = Some(MenuAction::ShowAbout);
                    ui.close();
                }
            });
        });
    });

    action
}
