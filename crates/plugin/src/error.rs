use snafu::{Location, Snafu};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Failed to build the http client"))]
    #[snafu(context(name(BuildClientSnafu)))]
    BuildClient {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Request failed"))]
    #[snafu(context(name(RequestSnafu)))]
    Request {
        source: reqwest::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Failed to deserialize response: {source}"))]
    #[snafu(context(name(DeserializeSnafu)))]
    Deserialize {
        source: serde_json::Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("GraphQL query failed: {message}"))]
    #[snafu(context(name(GraphQlSnafu)))]
    GraphQl {
        message: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("GraphQL response carried no data"))]
    #[snafu(context(name(EmptyDataSnafu)))]
    EmptyData {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("'{steam_id}' is not a valid Steam64 id"))]
    #[snafu(context(name(InvalidSteamIdSnafu)))]
    InvalidSteamId {
        steam_id: String,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("Could not resolve a Steam profile for '{input}'"))]
    #[snafu(context(name(UnresolvedProfileSnafu)))]
    UnresolvedProfile {
        input: String,
        #[snafu(implicit)]
        location: Location,
    },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
