use std::path::PathBuf;

use egui::ViewportCommand;
use tokio::sync::mpsc;

use crate::{
    async_bridge::{AsyncBridge, TaskResult},
    components::{exit_modal, menu_bar, sidebar, toast},
    screens::{self, Screen},
    theme::{CourierTheme, ThemeMode},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Route {
    #[default]
    Dashboard,
    Friends,
    Matches,
    Heroes,
    Items,
    Settings,
}

impl Route {
    pub fn title(&self) -> String {
        match self {
            Route::Dashboard => i18n::message("nav-dashboard"),
            Route::Friends => i18n::message("nav-friends"),
            Route::Matches => i18n::message("nav-matches"),
            Route::Heroes => i18n::message("nav-heroes"),
            Route::Items => i18n::message("nav-items"),
            Route::Settings => i18n::message("nav-settings"),
        }
    }

    pub fn all() -> &'static [Route] {
        &[Route::Dashboard, Route::Friends, Route::Matches, Route::Heroes, Route::Items, Route::Settings]
    }

    pub fn main_routes() -> &'static [Route] {
        &[Route::Dashboard, Route::Friends, Route::Matches, Route::Heroes, Route::Items]
    }
}

enum AppState {
    Setup(Box<screens::setup::SetupScreen>),
    Main(Box<MainState>),
}

struct MainState {
    route: Route,
    sidebar_collapsed: bool,
    _bridge: AsyncBridge,

    home: screens::dashboard::DashboardScreen,
    friends: screens::friend::FriendScreen,
    matches: screens::matches::MatchScreen,
    heroes: screens::hero::HeroScreen,
    items: screens::item::ItemScreen,
    settings: screens::setting::SettingScreen,

    toasts: toast::ToastManager,
    exit_modal: exit_modal::ExitModal,

    #[cfg(feature = "inspect")]
    inspect: crate::components::inspect::InspectPanel,
}

impl MainState {
    fn new(bridge: AsyncBridge) -> Self {
        Self {
            route: Route::Dashboard,
            sidebar_collapsed: false,
            _bridge: bridge,
            home: screens::dashboard::DashboardScreen::new(),
            friends: screens::friend::FriendScreen::new(),
            matches: screens::matches::MatchScreen::new(),
            heroes: screens::hero::HeroScreen::new(),
            items: screens::item::ItemScreen::new(),
            settings: screens::setting::SettingScreen::new(),
            toasts: toast::ToastManager::new(),
            exit_modal: exit_modal::ExitModal::new(),
            #[cfg(feature = "inspect")]
            inspect: crate::components::inspect::InspectPanel::new(),
        }
    }
}

pub struct App {
    state: AppState,
    theme: CourierTheme,
    current_title: String,
    bridge: AsyncBridge,
    rx: mpsc::UnboundedReceiver<TaskResult>,
    toast_rx: mpsc::UnboundedReceiver<toast::ToastEvent>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, runtime: tokio::runtime::Handle) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

        #[cfg(feature = "cjk")]
        {
            fonts.font_data.insert(
                "noto_sans_sc".to_owned(),
                egui::FontData::from_static(include_bytes!("../assets/fonts/NotoSansSC-Regular.ttf")).into(),
            );
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                family.push("noto_sans_sc".to_owned());
            }
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                family.push("noto_sans_sc".to_owned());
            }
        }

        cc.egui_ctx.set_fonts(fonts);

        let config = configs::read();

        let locale = if config.general.language == "auto" {
            i18n::detect_system_locale()
        } else {
            Some(config.general.language.clone())
        };
        i18n::init(locale.as_deref());

        let theme_mode = match config.general.theme {
            configs::ThemePreference::Light => ThemeMode::Light,
            configs::ThemePreference::Dark => ThemeMode::Dark,
            configs::ThemePreference::System => ThemeMode::Dark,
        };

        let (tx, rx) = mpsc::unbounded_channel::<TaskResult>();
        let (toast_tx, toast_rx) = mpsc::unbounded_channel::<toast::ToastEvent>();
        let toast_sender = toast::ToastSender::new(toast_tx);
        let bridge = AsyncBridge::new(runtime, tx, cc.egui_ctx.clone(), toast_sender);

        let state = if configs::is_first_launch() {
            AppState::Setup(Box::default())
        } else {
            AppState::Main(Box::new(MainState::new(bridge.clone())))
        };

        Self {
            state,
            theme: CourierTheme::new(theme_mode),
            current_title: String::new(),
            bridge,
            rx,
            toast_rx,
        }
    }

    fn title(&self) -> String {
        match &self.state {
            AppState::Setup(_) => "Courier - Setup".to_owned(),
            AppState::Main(main) => format!("Courier - {}", main.route.title()),
        }
    }

    fn handle_task_result(&mut self, _result: TaskResult) {
        // Dispatch results to the appropriate screen/state.
        // match result {
        //     TaskResult::MatchesLoaded(matches) => { ... }
        //     TaskResult::MatchesFailed(err) => { ... }
        // }
    }

    fn handle_menu_action(main: &mut MainState, action: menu_bar::MenuAction) {
        match action {
            menu_bar::MenuAction::Navigate(route) => {
                main.route = route;
            }
            menu_bar::MenuAction::OpenFeedback => {
                main.toasts.push_info(i18n::message("feedback-title"), i18n::message("feedback-coming-soon"));
            }
            menu_bar::MenuAction::ShowAbout => {
                main.toasts.push_info("Courier", format!("Version {}", env!("CARGO_PKG_VERSION")));
            }
            menu_bar::MenuAction::CheckUpdates => {
                main.toasts.push_info(i18n::message("updates-title"), i18n::message("updates-check-coming-soon"));
            }
            menu_bar::MenuAction::Exit => {
                main.exit_modal.show();
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Drain all completed background task results (non-blocking)
        while let Ok(result) = self.rx.try_recv() {
            self.handle_task_result(result);
        }

        self.theme.apply_to_ctx(ctx);

        let title = self.title();
        if title != self.current_title {
            self.current_title = title.clone();
            ctx.send_viewport_cmd(ViewportCommand::Title(title));
        }

        // Handle close request
        if ctx.input(|i| i.viewport().close_requested()) {
            match &mut self.state {
                AppState::Setup(_) => {
                    // Allow close during setup
                }
                AppState::Main(main) => {
                    if !main.exit_modal.visible {
                        ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                        main.exit_modal.show();
                    }
                }
            }
        }

        match &mut self.state {
            AppState::Setup(setup) => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    setup.show(ui);
                });

                // Check if setup completed (must check after show() to avoid borrow issues)
                let should_transition = {
                    let AppState::Setup(ref setup) = self.state else { unreachable!() };
                    setup.completed
                };
                if should_transition {
                    let AppState::Setup(ref setup) = self.state else { unreachable!() };
                    // Clone the result before mutating self
                    let result = setup.result();
                    let app_path = PathBuf::from(&result.app_path);
                    if let Err(e) = configs::set_app_path(app_path) {
                        tracing::error!("Failed to set app path: {e:?}");
                    }

                    configs::update(|cfg| {
                        cfg.general.language = result.language.clone();
                    });

                    if let Err(e) = configs::save() {
                        tracing::error!("Failed to save config after setup: {e}");
                    }

                    let locale = if result.language == "auto" {
                        i18n::detect_system_locale()
                    } else {
                        Some(result.language)
                    };
                    i18n::init(locale.as_deref());

                    let mut main = MainState::new(self.bridge.clone());
                    if result.open_settings {
                        main.route = Route::Settings;
                    }
                    self.state = AppState::Main(Box::new(main));
                }
            }

            AppState::Main(main) => {
                // Drain toast events from the channel into ToastManager
                main.toasts.drain(&mut self.toast_rx);

                // Menu bar
                egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
                    if let Some(action) = menu_bar::show(ui) {
                        Self::handle_menu_action(main, action);
                    }
                });

                // Sidebar
                let target_width = if main.sidebar_collapsed {
                    crate::theme::sidebar::collapsed_width() + crate::theme::spacing::SMALL * 2.0
                } else {
                    crate::theme::sidebar::width() + crate::theme::spacing::MEDIUM * 2.0
                };
                let animated_width = ctx.animate_value_with_time(egui::Id::new("sidebar_width"), target_width, 1.0 / crate::theme::animation::sidebar_speed());

                let sidebar_frame = egui::Frame::NONE.fill(crate::theme::colors().sidebar_bg);
                egui::SidePanel::left("sidebar")
                    .resizable(false)
                    .show_separator_line(false)
                    .exact_width(animated_width)
                    .frame(sidebar_frame)
                    .show(ctx, |ui| {
                        sidebar::show(ui, &mut main.route, &mut main.sidebar_collapsed);
                    });

                // Main content
                egui::CentralPanel::default().show(ctx, |ui| {
                    let settings_action = match main.route {
                        Route::Dashboard => {
                            main.home.show(ui);
                            None
                        }
                        Route::Friends => {
                            main.friends.show(ui);
                            None
                        }
                        Route::Matches => {
                            main.matches.show(ui);
                            None
                        }
                        Route::Heroes => {
                            main.heroes.show(ui);
                            None
                        }
                        Route::Items => {
                            main.items.show(ui);
                            None
                        }
                        Route::Settings => main.settings.show(ui),
                    };

                    match settings_action {
                        Some(screens::setting::SettingsAction::ThemeChanged(pref)) => {
                            let new_mode = match pref {
                                configs::ThemePreference::Light => ThemeMode::Light,
                                configs::ThemePreference::Dark => ThemeMode::Dark,
                                configs::ThemePreference::System => ThemeMode::Dark,
                            };
                            self.theme.set_mode(new_mode);
                        }
                        Some(screens::setting::SettingsAction::Reset) => {
                            configs::update(|cfg| {
                                *cfg = configs::AppConfig::default();
                            });
                            if let Err(e) = configs::save() {
                                tracing::error!("Failed to save config after reset: {e}");
                            }
                            let default_theme = configs::AppConfig::default().general.theme;
                            let new_mode = match default_theme {
                                configs::ThemePreference::Light => ThemeMode::Light,
                                configs::ThemePreference::Dark => ThemeMode::Dark,
                                configs::ThemePreference::System => ThemeMode::Dark,
                            };
                            self.theme.set_mode(new_mode);
                            // Rebuild the settings screen to pick up default values
                            main.settings = screens::setting::SettingScreen::new();
                        }
                        None => {}
                    }
                });

                // Toasts overlay
                main.toasts.show(ctx);

                // Inspect panel (dev only)
                #[cfg(feature = "inspect")]
                main.inspect.show(ctx);

                // Exit modal overlay
                if let Some(exit_action) = main.exit_modal.ui(ctx) {
                    match exit_action {
                        exit_modal::ExitAction::Confirm => {
                            if let Err(e) = configs::save() {
                                tracing::error!("Failed to save config on exit: {e}");
                            }
                            ctx.send_viewport_cmd(ViewportCommand::Close);
                        }
                        exit_modal::ExitAction::Cancel => {
                            main.exit_modal.hide();
                        }
                    }
                }
            }
        }
    }
}
