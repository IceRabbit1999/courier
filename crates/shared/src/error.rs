use std::io;

use snafu::{Location, Snafu};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Configuration error: {message}"))]
    Config {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("I18n error: {message}"))]
    I18n {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("API error: {message}"))]
    Api {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Storage error: {message}"))]
    Storage {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Plugin error: {message}"))]
    Plugin {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("IO error: {source}"))]
    Io {
        source: io::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Serialization error: {source}"))]
    Serialization {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("{message}"))]
    Other {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
#[snafu(display("Expected {expected} but found None"))]
pub struct NotFound {
    expected: String,
    #[snafu(implicit)]
    location: Location,
}
