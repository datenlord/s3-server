//! Ordered headers

use crate::Request;

use hyper::header::{AsHeaderName, ToStrError};
use smallvec::SmallVec;

/// Immutable http header container
#[derive(Debug)]
pub struct OrderedHeaders<'a> {
    /// ascending headers (header names are lowercase)
    headers: SmallVec<[(&'a str, &'a str); 16]>,
}

impl<'a> OrderedHeaders<'a> {
    /// Constructs `OrderedHeaders` from slice
    ///
    /// + header names must be lowercase
    /// + header values must be valid
    #[cfg(test)]
    pub fn from_slice_unchecked(slice: &[(&'a str, &'a str)]) -> Self {
        let mut headers = SmallVec::new();
        headers.extend_from_slice(slice);
        headers.sort();
        Self { headers }
    }

    /// Constructs `OrderedHeaders<'a>` from `&'a Request`
    pub fn from_req(req: &'a Request) -> Result<Self, ToStrError> {
        let mut headers: SmallVec<[(&'a str, &'a str); 16]> =
            SmallVec::with_capacity(req.headers().len());

        for (name, value) in req.headers().iter() {
            headers.push((name.as_str(), value.to_str()?));
        }
        headers.sort();

        Ok(Self { headers })
    }

    /// Get header value by name. Time `O(logn)`
    pub fn get(&self, name: impl AsHeaderName) -> Option<&str> {
        let headers = self.headers.as_slice();
        let ans = match headers.binary_search_by_key(&name.as_str(), |(n, _)| *n) {
            Ok(idx) => headers.get(idx).map(|(_, v)| *v),
            Err(_) => None,
        };
        drop(name);
        ans
    }
}

impl<'a> AsRef<[(&'a str, &'a str)]> for OrderedHeaders<'a> {
    fn as_ref(&self) -> &[(&'a str, &'a str)] {
        self.headers.as_ref()
    }
}
