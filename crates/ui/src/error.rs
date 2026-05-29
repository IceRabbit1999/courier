use snafu::{Location, Snafu};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Asset error: {message}"))]
    Asset {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Theme error: {message}"))]
    Theme {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Navigation error: {message}"))]
    Navigation {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("Screen error: {message}"))]
    Screen {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
