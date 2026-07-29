use snafu::{Location, Snafu};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Failed to create storage directory"))]
    #[snafu(context(name(CreateDirSnafu)))]
    CreateDir {
        source: std::io::Error,
        #[snafu(implicit, provide)]
        location: Location,
    },
    #[snafu(display("Failed to connect to the database"))]
    #[snafu(context(name(ConnectSnafu)))]
    Connect {
        source: sqlx::Error,
        #[snafu(implicit, provide)]
        location: Location,
    },
    #[snafu(display("Failed to run database migrations"))]
    #[snafu(context(name(MigrateSnafu)))]
    Migrate {
        source: sqlx::migrate::MigrateError,
        #[snafu(implicit, provide)]
        location: Location,
    },
    #[snafu(display("Database query failed"))]
    #[snafu(context(name(QuerySnafu)))]
    Query {
        source: sqlx::Error,
        #[snafu(implicit, provide)]
        location: Location,
    },
    #[snafu(display("Expected {expected} but got None"))]
    #[snafu(context(name(UnexpectedNoneSnafu)))]
    UnexpectedNone {
        expected: String,
        #[snafu(implicit, provide)]
        location: Location,
    },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
