use std::path::PathBuf;

use egui::ViewportCommand;
use snafu::ResultExt;
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
    runtime: tokio::runtime::Handle,
    storage: storage::Storage,
    client: plugin::Client,
    client_proxy: Option<String>,
    rx: mpsc::UnboundedReceiver<TaskResult>,
    toast_rx: mpsc::UnboundedReceiver<toast::ToastEvent>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, runtime: tokio::runtime::Handle, storage: storage::Storage, client: plugin::Client) -> Self {
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
        egui_extras::install_image_loaders(&cc.egui_ctx);

        let mut style = (*cc.egui_ctx.global_style()).clone();
        style.spacing.button_padding = egui::vec2(12.0, 6.0);
        cc.egui_ctx.set_global_style(style);

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
        let bridge = AsyncBridge::new(runtime.clone(), tx, cc.egui_ctx.clone(), toast_sender);

        let state = if configs::is_first_launch() {
            AppState::Setup(Box::default())
        } else {
            // Populate the watch list from the last-synced data without a network call.
            Self::spawn_load_friends_from_db(&bridge, &storage);
            AppState::Main(Box::new(MainState::new(bridge.clone())))
        };

        let client_proxy = config.network.proxy.clone();

        Self {
            state,
            theme: CourierTheme::new(theme_mode),
            current_title: String::new(),
            bridge,
            runtime,
            storage,
            client,
            client_proxy,
            rx,
            toast_rx,
        }
    }

    /// Load the stored friend list off the UI thread (no network).
    fn spawn_load_friends_from_db(bridge: &AsyncBridge, storage: &storage::Storage) {
        let storage = storage.clone();
        bridge.spawn(async move {
            let friends = storage.list_friends().await.whatever_context("Failed to load friends from storage")?;
            Ok(TaskResult::FriendsLoaded(friends))
        });
    }

    /// Sync the full friend list from Steam (friend list + player summaries),
    /// picking up any newly added or removed friends, then store and hand it back.
    fn spawn_sync_friends(bridge: &AsyncBridge, client: &plugin::Client, storage: &storage::Storage) {
        let (steam_id, key) = {
            let config = configs::read();
            (config.games.dota2.steam_id.clone(), config.secrets.steam_web_api_key.clone())
        };
        let (Some(steam_id), Some(key)) = (steam_id, key) else {
            bridge.toasts().error(i18n::message("friends-error-title"), i18n::message("friends-error-credentials"));
            return;
        };

        let client = client.clone();
        let storage = storage.clone();
        bridge.spawn(async move {
            let friends = client.steam(key).get_friends(&steam_id).await.whatever_context("Failed to fetch friends from Steam")?;
            storage.replace_friends(&friends).await.whatever_context("Failed to save friends")?;
            Ok(TaskResult::FriendsLoaded(friends))
        });
    }

    /// Refresh the online state and current game of the already-known `friends`
    /// (player summaries only; no friend-list fetch), then store and hand it back.
    fn spawn_refresh_statuses(bridge: &AsyncBridge, client: &plugin::Client, storage: &storage::Storage, friends: Vec<shared::Friend>) {
        let key = configs::read().secrets.steam_web_api_key.clone();
        let Some(key) = key else {
            bridge.toasts().error(i18n::message("friends-error-title"), i18n::message("friends-error-credentials"));
            return;
        };

        let client = client.clone();
        let storage = storage.clone();
        bridge.spawn(async move {
            let friends = client.steam(key).refresh_statuses(&friends).await.whatever_context("Failed to refresh friend statuses")?;
            storage.replace_friends(&friends).await.whatever_context("Failed to save friends")?;
            Ok(TaskResult::FriendsLoaded(friends))
        });
    }

    /// Sync static reference data: heroes from Steam, items from Stratz, for every
    /// supported locale, then report the stored row counts.
    fn spawn_sync_static_data(bridge: &AsyncBridge, client: &plugin::Client, storage: &storage::Storage) {
        let (key, token) = {
            let config = configs::read();
            (config.secrets.steam_web_api_key.clone(), config.secrets.stratz_api_token.clone())
        };
        let (Some(key), Some(token)) = (key, token) else {
            bridge.toasts().error(i18n::message("data-sync-error-title"), i18n::message("data-sync-error-credentials"));
            return;
        };

        let client = client.clone();
        let storage = storage.clone();
        bridge.spawn(async move {
            for &locale in i18n::SUPPORTED_LOCALES {
                let heroes = client.steam(key.clone()).get_heroes(locale).await.whatever_context("Failed to fetch heroes")?;
                storage.upsert_heroes(locale, &heroes).await.whatever_context("Failed to save heroes")?;
                let items = client.stratz(token.clone()).get_items(locale).await.whatever_context("Failed to fetch items")?;
                storage.upsert_items(locale, &items).await.whatever_context("Failed to save items")?;
            }

            // OpenDota constants fill the gaps Steam/Stratz leave (every neutral
            // artifact and the `enhancement_*` active enchantments). They are
            // English-only, so they land under the `en` locale; localized names
            // from the loop above are left untouched.
            let od_heroes = client.opendota(None).get_heroes().await.whatever_context("Failed to fetch heroes from OpenDota")?;
            storage.upsert_heroes("en", &od_heroes).await.whatever_context("Failed to save OpenDota heroes")?;
            let od_items = client.opendota(None).get_items().await.whatever_context("Failed to fetch items from OpenDota")?;
            storage.upsert_items("en", &od_items).await.whatever_context("Failed to save OpenDota items")?;

            let heroes = storage.hero_count().await.whatever_context("Failed to count heroes")?;
            let items = storage.item_count().await.whatever_context("Failed to count items")?;
            Ok(TaskResult::StaticDataSynced { heroes, items })
        });
    }

    /// Load the localized hero/item name maps used to render matches as text.
    fn spawn_load_match_names(bridge: &AsyncBridge, storage: &storage::Storage) {
        let locale = i18n::current_locale();
        let storage = storage.clone();
        bridge.spawn(async move {
            let heroes = storage.hero_names(&locale).await.whatever_context("Failed to load hero names")?;
            let items = storage.item_names(&locale).await.whatever_context("Failed to load item names")?;
            let hero_slugs = storage.hero_slugs().await.whatever_context("Failed to load hero slugs")?;
            let item_slugs = storage.item_slugs().await.whatever_context("Failed to load item slugs")?;
            Ok(TaskResult::MatchNamesLoaded {
                heroes,
                items,
                hero_slugs,
                item_slugs,
            })
        });
    }

    /// Load a friend's stored match summaries (no network).
    fn spawn_load_matches_from_db(bridge: &AsyncBridge, storage: &storage::Storage, steam_id: String) {
        let storage = storage.clone();
        bridge.spawn(async move {
            let matches = storage.list_match_summaries(&steam_id).await.whatever_context("Failed to load matches from storage")?;
            Ok(TaskResult::MatchSummariesLoaded { steam_id, matches })
        });
    }

    /// Fetch a friend's recent matches from OpenDota (capped at `max_match_history`),
    /// store them, then hand them back.
    fn spawn_fetch_matches(bridge: &AsyncBridge, client: &plugin::Client, storage: &storage::Storage, steam_id: String) {
        let (api_key, limit) = {
            let config = configs::read();
            (config.secrets.opendota_api_key.clone(), config.matches.max_match_history)
        };

        let client = client.clone();
        let storage = storage.clone();
        bridge.spawn(async move {
            let matches = client
                .opendota(api_key)
                .recent_matches(&steam_id, limit)
                .await
                .whatever_context("Failed to fetch matches from OpenDota")?;
            storage.replace_match_summaries(&steam_id, &matches).await.whatever_context("Failed to save matches")?;
            Ok(TaskResult::MatchSummariesLoaded { steam_id, matches })
        });
    }

    /// Open one match's full detail: serve it from the DB if present, otherwise
    /// fetch it from OpenDota and store it.
    fn spawn_fetch_match_detail(bridge: &AsyncBridge, client: &plugin::Client, storage: &storage::Storage, match_id: i64) {
        let api_key = configs::read().secrets.opendota_api_key.clone();
        let client = client.clone();
        let storage = storage.clone();
        bridge.spawn(async move {
            if let Some(detail) = storage.get_match_detail(match_id).await.whatever_context("Failed to load match detail from storage")? {
                return Ok(TaskResult::MatchDetailLoaded(Box::new(detail)));
            }
            let detail = client
                .opendota(api_key)
                .match_detail(match_id)
                .await
                .whatever_context("Failed to fetch match detail from OpenDota")?;
            storage.upsert_match_detail(&detail).await.whatever_context("Failed to save match detail")?;
            Ok(TaskResult::MatchDetailLoaded(Box::new(detail)))
        });
    }

    /// Move the on-disk storage to `new_path`: close the pool so the database
    /// file is released, migrate the directory contents, then reopen at the new
    /// location.
    fn migrate_storage(runtime: &tokio::runtime::Handle, storage: &mut storage::Storage, new_path: PathBuf) {
        runtime.block_on(storage.close());

        if let Err(e) = configs::migrate_storage_path(new_path) {
            tracing::error!("Failed to migrate storage data: {e}");
        }

        match runtime.block_on(storage::Storage::open(configs::storage_path())) {
            Ok(reopened) => *storage = reopened,
            Err(e) => tracing::error!("Failed to reopen storage after migration: {e}"),
        }
    }

    /// Rebuild the HTTP client when the proxy setting changes so the new proxy
    /// takes effect without restarting the app. No-ops when the proxy is
    /// unchanged; on an invalid proxy the existing client is kept.
    fn reload_client(client: &mut plugin::Client, current_proxy: &mut Option<String>, proxy: Option<String>, toasts: &mut toast::ToastManager) {
        if *current_proxy == proxy {
            return;
        }
        match plugin::Client::new(proxy.as_deref()) {
            Ok(new_client) => {
                *client = new_client;
                *current_proxy = proxy;
                toasts.push_success(i18n::message("network-proxy-updated-title"), i18n::message("network-proxy-updated-message"));
            }
            Err(e) => toasts.push_error(i18n::message("network-proxy-error-title"), e.to_string()),
        }
    }

    fn title(&self) -> String {
        match &self.state {
            AppState::Setup(_) => "Courier - Setup".to_owned(),
            AppState::Main(main) => format!("Courier - {}", main.route.title()),
        }
    }

    fn handle_task_result(&mut self, result: TaskResult) {
        let AppState::Main(main) = &mut self.state else {
            return;
        };

        match result {
            TaskResult::FriendsLoaded(friends) => main.friends.set_friends(friends),
            TaskResult::StaticDataSynced { heroes, items } => {
                main.settings.set_syncing(false);
                let summary = format!("{heroes} {} · {items} {}", i18n::message("nav-heroes"), i18n::message("nav-items"));
                main.toasts.push_success(i18n::message("data-sync-done-title"), summary);
            }
            TaskResult::MatchSummariesLoaded { steam_id, matches } => main.matches.set_matches(steam_id, matches),
            TaskResult::MatchDetailLoaded(detail) => main.matches.set_detail(*detail),
            TaskResult::MatchNamesLoaded {
                heroes,
                items,
                hero_slugs,
                item_slugs,
            } => main.matches.set_names(heroes, items, hero_slugs, item_slugs),
            TaskResult::TaskFailed(error) => {
                // A failure clears whichever in-flight indicator was set; the
                // unaffected ones are already idle, so resetting them all is harmless.
                main.friends.set_loading(false);
                main.matches.set_loading(false);
                main.settings.set_syncing(false);
                main.toasts.push_error(i18n::message("common-error"), error);
            }
        }
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
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Drain all completed background task results (non-blocking)
        while let Ok(result) = self.rx.try_recv() {
            self.handle_task_result(result);
        }

        self.theme.apply_to_ctx(&ctx);

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
                egui::CentralPanel::default().show_inside(ui, |ui| {
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
                main.toasts.drain(&mut self.toast_rx);

                egui::Panel::top("menu_bar").show_inside(ui, |ui| {
                    if let Some(action) = menu_bar::show(ui) {
                        Self::handle_menu_action(main, action);
                    }
                });

                let target_width = if main.sidebar_collapsed {
                    crate::theme::sidebar::collapsed_width() + crate::theme::spacing::SMALL * 2.0
                } else {
                    crate::theme::sidebar::width() + crate::theme::spacing::MEDIUM * 2.0
                };
                let animated_width = ctx.animate_value_with_time(egui::Id::new("sidebar_width"), target_width, 1.0 / crate::theme::animation::sidebar_speed());

                let sidebar_frame = egui::Frame::NONE.fill(crate::theme::colors().sidebar_bg);
                egui::Panel::left("sidebar")
                    .resizable(false)
                    .show_separator_line(false)
                    .exact_size(animated_width)
                    .frame(sidebar_frame)
                    .show_inside(ui, |ui| {
                        sidebar::show(ui, &mut main.route, &mut main.sidebar_collapsed);
                    });

                egui::CentralPanel::default().show_inside(ui, |ui| {
                    let settings_actions = match main.route {
                        Route::Dashboard => {
                            main.home.show(ui);
                            Vec::new()
                        }
                        Route::Friends => {
                            match main.friends.show(ui) {
                                Some(screens::friend::FriendAction::SyncFriends) => {
                                    main.friends.set_loading(true);
                                    Self::spawn_sync_friends(&self.bridge, &self.client, &self.storage);
                                }
                                Some(screens::friend::FriendAction::RefreshStatuses) => {
                                    let friends = main.friends.friends().to_vec();
                                    main.friends.set_loading(true);
                                    Self::spawn_refresh_statuses(&self.bridge, &self.client, &self.storage, friends);
                                }
                                None => {}
                            }
                            Vec::new()
                        }
                        Route::Matches => {
                            main.matches.set_friends(main.friends.friends());
                            if main.matches.needs_names() {
                                main.matches.mark_names_requested();
                                Self::spawn_load_match_names(&self.bridge, &self.storage);
                            }
                            match main.matches.show(ui) {
                                Some(screens::matches::MatchAction::SelectFriend(steam_id)) => {
                                    main.matches.set_loading(true);
                                    Self::spawn_load_matches_from_db(&self.bridge, &self.storage, steam_id);
                                }
                                Some(screens::matches::MatchAction::FetchMatches(steam_id)) => {
                                    main.matches.set_loading(true);
                                    Self::spawn_fetch_matches(&self.bridge, &self.client, &self.storage, steam_id);
                                }
                                Some(screens::matches::MatchAction::OpenMatch(match_id)) => {
                                    main.matches.set_loading(true);
                                    Self::spawn_fetch_match_detail(&self.bridge, &self.client, &self.storage, match_id);
                                }
                                Some(screens::matches::MatchAction::CloseDetail) => main.matches.close_detail(),
                                None => {}
                            }
                            Vec::new()
                        }
                        Route::Heroes => {
                            main.heroes.show(ui);
                            Vec::new()
                        }
                        Route::Items => {
                            main.items.show(ui);
                            Vec::new()
                        }
                        Route::Settings => main.settings.show(ui),
                    };

                    for action in settings_actions {
                        match action {
                            screens::setting::SettingsAction::ThemeChanged(pref) => {
                                let new_mode = match pref {
                                    configs::ThemePreference::Light => ThemeMode::Light,
                                    configs::ThemePreference::Dark => ThemeMode::Dark,
                                    configs::ThemePreference::System => ThemeMode::Dark,
                                };
                                self.theme.set_mode(new_mode);
                            }
                            screens::setting::SettingsAction::StoragePathChanged(new_path) => {
                                Self::migrate_storage(&self.runtime, &mut self.storage, new_path);
                            }
                            screens::setting::SettingsAction::ProxyChanged(proxy) => {
                                Self::reload_client(&mut self.client, &mut self.client_proxy, proxy, &mut main.toasts);
                            }
                            screens::setting::SettingsAction::SyncStaticData => {
                                main.settings.set_syncing(true);
                                Self::spawn_sync_static_data(&self.bridge, &self.client, &self.storage);
                            }
                            screens::setting::SettingsAction::Reset => {
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
                                Self::reload_client(&mut self.client, &mut self.client_proxy, None, &mut main.toasts);
                                // Rebuild the settings screen to pick up default values
                                main.settings = screens::setting::SettingScreen::new();
                            }
                        }
                    }
                });

                // Toasts overlay
                main.toasts.show(&ctx);

                // Inspect panel (dev only)
                #[cfg(feature = "inspect")]
                main.inspect.show(&ctx);

                // Exit modal overlay
                if let Some(exit_action) = main.exit_modal.ui(&ctx) {
                    match exit_action {
                        exit_modal::ExitAction::Confirm => {
                            if let Err(e) = configs::save() {
                                tracing::error!("Failed to save config on exit: {e}");
                            }
                            self.runtime.block_on(self.storage.close());
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
