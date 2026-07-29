#![feature(error_generic_member_access)]

mod loader;

use std::{collections::HashMap, sync::LazyLock};

use fluent_bundle::{FluentArgs, FluentResource, bundle::FluentBundle};
use intl_memoizer::concurrent::IntlLangMemoizer;
pub use loader::load_locale;
use parking_lot::RwLock;
use snafu::{Snafu, whatever};
use tracing::{error, warn};
use unic_langid::LanguageIdentifier;

/// Supported locales
pub const SUPPORTED_LOCALES: &[&str] = &["en", "zh-CN"];
pub const DEFAULT_LOCALE: &str = "en";

/// I18n errors. Bundle loading is the only fallible path and its one caller just
/// logs and skips the locale, so a single catch-all carries every failure rather
/// than a variant nobody matches on.
#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(whatever, display("{message}"))]
    Whatever {
        message: String,
        #[snafu(source(from(Box<dyn std::error::Error + Send + Sync>, Some)))]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

type Bundle = FluentBundle<FluentResource, IntlLangMemoizer>;

static I18N: LazyLock<RwLock<I18nState>> = LazyLock::new(|| RwLock::new(I18nState::new()));

struct I18nState {
    current_locale: String,
    bundles: HashMap<String, Bundle>,
}

impl I18nState {
    fn new() -> Self {
        let mut state = Self {
            current_locale: DEFAULT_LOCALE.to_string(),
            bundles: HashMap::new(),
        };

        for locale in SUPPORTED_LOCALES {
            let Ok(bundle) = Self::load_bundle(locale).inspect_err(|err| error!("Failed to load locale '{}': {:?}", locale, err)) else {
                continue;
            };
            state.bundles.insert(locale.to_string(), bundle);
        }

        state
    }

    fn load_bundle(locale: &str) -> Result<Bundle> {
        let Ok(langid) = locale.parse::<LanguageIdentifier>() else {
            whatever!("Invalid locale '{locale}': failed to parse language identifier");
        };

        let source = load_locale(locale);
        let resource = match FluentResource::try_new(source) {
            Ok(resource) => resource,
            Err((_, errs)) => whatever!("Failed to parse locale '{locale}' resource: {errs:?}"),
        };

        let mut bundle = FluentBundle::new_concurrent(vec![langid]);
        bundle.set_use_isolating(false);
        if let Err(errs) = bundle.add_resource(resource) {
            whatever!("Failed to add resource for locale '{locale}': {errs:?}");
        }

        Ok(bundle)
    }

    fn get_message(&self, locale: &str, key: &str, args: Option<&FluentArgs>) -> String {
        if let Some(bundle) = self.bundles.get(locale)
            && let Some(msg) = bundle.get_message(key)
            && let Some(pattern) = msg.value()
        {
            let mut errors = vec![];
            let result = bundle.format_pattern(pattern, args, &mut errors);
            if errors.is_empty() {
                return result.to_string();
            }
        }

        // Fallback to default locale
        if locale != DEFAULT_LOCALE
            && let Some(bundle) = self.bundles.get(DEFAULT_LOCALE)
            && let Some(msg) = bundle.get_message(key)
            && let Some(pattern) = msg.value()
        {
            let mut errors = vec![];
            let result = bundle.format_pattern(pattern, args, &mut errors);
            if errors.is_empty() {
                return result.to_string();
            }
        }

        warn!("Message not found for key: '{}', return original key directly", key);
        key.to_string()
    }
}

pub fn init(locale: Option<&str>) {
    let locale = locale.map(String::from).or_else(detect_system_locale).unwrap_or_else(|| DEFAULT_LOCALE.to_string());

    set_locale(&locale);
}

pub fn set_locale(locale: &str) {
    let mut state = I18N.write();

    let matched = SUPPORTED_LOCALES
        .iter()
        .find(|&&l| l == locale)
        .or_else(|| {
            // Try matching just the language part
            let lang = locale.split('-').next().unwrap_or(locale);
            SUPPORTED_LOCALES.iter().find(|&&l| l.starts_with(lang))
        })
        .unwrap_or(&DEFAULT_LOCALE);

    state.current_locale = matched.to_string();
}

pub fn detect_system_locale() -> Option<String> {
    sys_locale::get_locale()
}

pub fn message(key: &str) -> String {
    let state = I18N.read();
    state.get_message(&state.current_locale, key, None)
}

/// Look up `key` in a specific locale, ignoring the process-wide current locale.
/// Used by the hub, which renders one message per subscriber language; an
/// unknown locale falls back to [`DEFAULT_LOCALE`].
pub fn message_in(locale: &str, key: &str) -> String {
    I18N.read().get_message(locale, key, None)
}

/// The active locale (one of [`SUPPORTED_LOCALES`]). Used to pick the right
/// localized rows when querying stored reference data.
pub fn current_locale() -> String {
    I18N.read().current_locale.clone()
}
