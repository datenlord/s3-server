//! S3 Authentication

use crate::errors::S3AuthError;

use std::collections::HashMap;

use async_trait::async_trait;

/// S3 Authentication Provider
#[async_trait]
pub trait S3Auth {
    /// lookup `secret_access_key` by `access_key_id`
    async fn get_secret_access_key(&self, access_key_id: &str) -> Result<String, S3AuthError>;
}

/// A simple authentication provider
#[derive(Debug, Default)]
pub struct SimpleAuth {
    /// key map
    map: HashMap<String, String>,
}

impl SimpleAuth {
    /// Constructs a new `SimpleAuth`
    #[must_use]
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// register a credential
    pub fn register(&mut self, access_key: String, secret_key: String) {
        let _prev = self.map.insert(access_key, secret_key);
    }

    /// lookup a credential
    #[must_use]
    pub fn lookup(&self, access_key: &str) -> Option<&str> {
        Some(self.map.get(access_key)?.as_str())
    }
}

#[async_trait]
impl S3Auth for SimpleAuth {
    async fn get_secret_access_key(&self, access_key_id: &str) -> Result<String, S3AuthError> {
        match self.lookup(access_key_id) {
            None => Err(S3AuthError::NotSignedUp),
            Some(s) => Ok(s.to_owned()),
        }
    }
}
