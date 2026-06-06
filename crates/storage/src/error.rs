use snafu::{Location, Snafu};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Failed to create storage directory"))]
    #[snafu(context(name(CreateDirSnafu)))]
    CreateDir {
        source: std::io::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to connect to the database"))]
    #[snafu(context(name(ConnectSnafu)))]
    Connect {
        source: sqlx::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to run database migrations"))]
    #[snafu(context(name(MigrateSnafu)))]
    Migrate {
        source: sqlx::migrate::MigrateError,
        #[snafu(implicit)]
        location: Location,
    },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
