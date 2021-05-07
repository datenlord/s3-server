//! Ordered query strings

use crate::utils::{Also, Apply};

use std::str::FromStr;

use smallvec::SmallVec;

/// Immutable query string container
#[derive(Debug)]
pub struct OrderedQs {
    /// Ascending query strings
    qs: SmallVec<[(String, String); 16]>,
}

impl OrderedQs {
    /// Constructs `OrderedQs` from vec
    ///
    /// + strings must be url-decoded
    #[cfg(test)]
    pub fn from_vec_unchecked(v: Vec<(String, String)>) -> Self {
        Self {
            qs: v.also(|v| v.sort()).into(),
        }
    }

    /// Parses `OrderedQs` from query
    pub fn from_query(query: &str) -> Result<Self, serde_urlencoded::de::Error> {
        serde_urlencoded::from_str::<Vec<(String, String)>>(query)?
            .also(|v| v.sort())
            .apply(|qs| Ok(Self { qs: qs.into() }))
    }

    /// Gets query value by name. Time `O(logn)`
    pub fn get(&self, name: &str) -> Option<&str> {
        let qs = self.qs.as_ref();
        match qs.binary_search_by_key(&name, |&(ref n, _)| n.as_str()) {
            Ok(idx) => qs.get(idx).map(|&(_, ref v)| v.as_str()),
            Err(_) => None,
        }
    }

    /// Assigns value from optional query
    pub fn assign<T: FromStr>(&self, name: &str, opt: &mut Option<T>) -> Result<(), T::Err> {
        if let Some(s) = self.get(name) {
            let v = s.parse()?;
            *opt = Some(v);
        }
        Ok(())
    }

    /// Assigns string from optional query
    pub fn assign_str(&self, name: &str, opt: &mut Option<String>) {
        if let Some(s) = self.get(name) {
            *opt = Some(s.to_owned());
        }
    }
}

impl AsRef<[(String, String)]> for OrderedQs {
    fn as_ref(&self) -> &[(String, String)] {
        self.qs.as_ref()
    }
}
