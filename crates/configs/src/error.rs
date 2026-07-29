use snafu::{Location, Snafu};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Failed to load configuration"))]
    #[snafu(context(name(ConfigInitSnafu)))]
    ConfigInit {
        source: config::ConfigError,
        #[snafu(implicit, provide)]
        location: Location,
    },
    #[snafu(display("Failed to save configuration"))]
    #[snafu(context(name(ConfigSaveSnafu)))]
    ConfigSave {
        source: std::io::Error,
        #[snafu(implicit, provide)]
        location: Location,
    },
    #[snafu(display("TOML serialization error"))]
    #[snafu(context(name(TomlSerializeSnafu)))]
    TomlSerialize {
        source: toml::ser::Error,
        #[snafu(implicit, provide)]
        location: Location,
    },
    #[snafu(display("Failed to migrate app data"))]
    #[snafu(context(name(MigrationSnafu)))]
    Migration {
        source: std::io::Error,
        #[snafu(implicit, provide)]
        location: Location,
    },
    #[snafu(display("App path not initialized"))]
    AppPath {
        #[snafu(implicit, provide)]
        location: Location,
    },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
