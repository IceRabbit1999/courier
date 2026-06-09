use graphql_client::GraphQLQuery;
use serde::{Deserialize, Serialize};
use shared::ItemEntry;

use crate::client::Client;

const STRATZ_URL: &str = "https://api.stratz.com/graphql";

/// Stratz custom scalar. Hero/item ids comfortably fit an `i32`.
type Short = i32;

/// Subset of Stratz' `LanguageEnum` we request. Supplied to the codegen via
/// `extern_enums` so we control the variants and their wire spelling, rather than
/// generating all 27 languages from the schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LanguageEnum {
    #[serde(rename = "ENGLISH")]
    English,
    #[serde(rename = "S_CHINESE")]
    SChinese,
}

impl LanguageEnum {
    /// Map an app locale (see [`i18n::SUPPORTED_LOCALES`]) to the Stratz language.
    fn from_locale(locale: &str) -> Self {
        match locale {
            "zh-CN" => Self::SChinese,
            _ => Self::English,
        }
    }
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/client/stratz/schema.json",
    query_path = "src/client/stratz/queries/get_items.graphql",
    response_derives = "Debug, Clone",
    extern_enums("LanguageEnum")
)]
struct GetItems;

/// Typed access to the Stratz GraphQL API, sharing the one [`Client`].
///
/// Docs: <https://docs.stratz.com/>
pub struct Stratz<'a> {
    client: &'a Client,
    token: String,
}

impl Client {
    pub fn stratz(&self, token: impl Into<String>) -> Stratz<'_> {
        Stratz {
            client: self,
            token: token.into(),
        }
    }
}

impl Stratz<'_> {
    /// `constants.items` — the full item list for one locale. The Steam
    /// `GetGameItems` endpoint is gone (404), so item data comes from here. The
    /// localized name lives under the `language` sub-object (the top-level
    /// `displayName` stays English regardless of the `language` argument).
    pub async fn get_items(&self, locale: &str) -> crate::Result<Vec<ItemEntry>> {
        let variables = get_items::Variables {
            language: LanguageEnum::from_locale(locale),
        };
        let data = self.client.graphql::<GetItems>(STRATZ_URL, &self.token, variables).await?;

        let items = data.constants.and_then(|constants| constants.items).unwrap_or_default();
        Ok(items
            .into_iter()
            .flatten()
            .filter_map(|item| {
                Some(ItemEntry {
                    id: item.id?,
                    short_name: item.short_name?,
                    display_name: item.language?.display_name?,
                })
            })
            .collect())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test environment")]
mod tests {
    #[tokio::test]
    async fn test_get_items() {
        dotenvy::dotenv().ok();
        let token = std::env::var("STRATZ_API_TOKEN").unwrap();
        let client = crate::Client::new(None).unwrap();
        let stratz = client.stratz(token);
        let items = stratz.get_items("zh-CN").await.unwrap();
        println!("{:#?}", items);
    }
}
