//! Url query

use serde::Deserialize;

/// Url query of a GET S3 request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GetQuery {
    /// location
    pub location: Option<String>,
    /// delimiter
    pub delimiter: Option<String>,
    /// encoding-type
    pub encoding_type: Option<String>,
    /// marker
    pub marker: Option<String>,
    /// max-keys
    pub max_keys: Option<i64>,
    /// prefix
    pub prefix: Option<String>,
    /// list-type
    pub list_type: Option<u8>,
    /// continuation-token
    pub continuation_token: Option<String>,
    /// fetch-owner
    pub fetch_owner: Option<bool>,
    /// start-after
    pub start_after: Option<String>,
}

/// Url query of a POST S3 request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PostQuery {
    /// delete
    pub delete: Option<String>,
}
