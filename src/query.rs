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
}
