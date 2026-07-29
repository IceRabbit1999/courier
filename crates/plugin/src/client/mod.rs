mod opendota;
mod steam;
mod stratz;

use graphql_client::GraphQLQuery;
use reqwest::Method;
use serde::de::DeserializeOwned;
use snafu::{OptionExt, ResultExt};
pub use steam::Steam;
pub use stratz::Stratz;
use tracing::{error, info};

use crate::error::{BuildClientSnafu, DeserializeSnafu, EmptyDataSnafu, GraphQlSnafu, RequestSnafu};

/// A single HTTP request against some target, fully described so [`Client::execute`]
/// can build, send and deserialize it. REST `GET`s only need [`Endpoint::url`] and
/// [`Endpoint::query`]; a GraphQL/Claude call overrides [`Endpoint::method`] and
/// [`Endpoint::body`].
pub trait Endpoint {
    type Response: DeserializeOwned;

    fn method(&self) -> Method {
        Method::GET
    }

    fn url(&self) -> String;

    fn query(&self) -> Vec<(&'static str, String)> {
        Vec::new()
    }

    fn body(&self) -> Option<serde_json::Value> {
        None
    }

    /// A bearer token to authenticate the request, if the endpoint needs one
    /// (e.g. the hub's `/notify`). Applied as an `Authorization: Bearer` header.
    fn bearer(&self) -> Option<String> {
        None
    }
}

#[derive(Debug, Clone)]
pub struct Client {
    internal: reqwest::Client,
}

impl Client {
    pub fn new(proxy: Option<&str>) -> crate::Result<Self> {
        let mut builder = reqwest::Client::builder();
        if let Some(proxy) = proxy {
            builder = builder.proxy(reqwest::Proxy::all(proxy).context(BuildClientSnafu)?);
        }
        let internal = builder.build().context(BuildClientSnafu)?;
        Ok(Self { internal })
    }

    pub async fn execute<E>(&self, endpoint: E) -> crate::Result<E::Response>
    where
        E: Endpoint,
    {
        let url = endpoint.url();
        let mut request = self.internal.request(endpoint.method(), &url).query(&endpoint.query());
        if let Some(body) = endpoint.body() {
            request = request.json(&body);
        }
        if let Some(token) = endpoint.bearer() {
            request = request.bearer_auth(token);
        }

        info!(request = ?request, "Spawn a request");
        let response = request.send().await.context(RequestSnafu)?;
        let text = response.text().await.context(RequestSnafu)?;

        serde_json::from_str::<E::Response>(&text)
            .inspect_err(|_| error!("Failed to deserialize response get: {}", text))
            .context(DeserializeSnafu)
    }

    /// Execute a typed GraphQL query `Q` against `url`, authenticating with a
    /// bearer `token`. We build the request body via the `graphql_client` codegen
    /// but POST it through this shared client (rather than `post_graphql`) so the
    /// whole app uses one reqwest. GraphQL replies are HTTP 200 even on failure,
    /// so the `errors` array is surfaced explicitly.
    pub async fn graphql<Q>(&self, url: &str, token: &str, variables: Q::Variables) -> crate::Result<Q::ResponseData>
    where
        Q: GraphQLQuery,
    {
        let body = Q::build_query(variables);
        let response = self
            .internal
            .post(url)
            .bearer_auth(token)
            .header(reqwest::header::USER_AGENT, "STRATZ_API")
            .json(&body)
            .send()
            .await
            .context(RequestSnafu)?
            .error_for_status()
            .context(RequestSnafu)?;

        let payload = response.json::<graphql_client::Response<Q::ResponseData>>().await.context(RequestSnafu)?;

        if let Some(errors) = payload.errors
            && !errors.is_empty()
        {
            let message = errors.into_iter().map(|e| e.message).collect::<Vec<_>>().join("; ");
            return GraphQlSnafu { message }.fail();
        }

        payload.data.context(EmptyDataSnafu)
    }
}
