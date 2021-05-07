//! Ordered headers

use crate::Request;

use std::str::FromStr;

use hyper::header::{AsHeaderName, ToStrError};
use smallvec::SmallVec;

/// Immutable http header container
#[derive(Debug)]
pub struct OrderedHeaders<'a> {
    /// Ascending headers (header names are lowercase)
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
        headers.sort_unstable();
        Self { headers }
    }

    /// Constructs `OrderedHeaders<'a>` from `&'a Request`
    pub fn from_req(req: &'a Request) -> Result<Self, ToStrError> {
        let mut headers: SmallVec<[(&'a str, &'a str); 16]> =
            SmallVec::with_capacity(req.headers().len());

        for (name, value) in req.headers().iter() {
            headers.push((name.as_str(), value.to_str()?));
        }
        headers.sort_unstable();

        Ok(Self { headers })
    }

    /// + Signed headers must be sorted
    pub fn map_signed_headers(&self, signed_headers: &[impl AsRef<str>]) -> Self {
        let mut headers: SmallVec<[(&'a str, &'a str); 16]> = SmallVec::new();
        for &(name, value) in self.headers.iter() {
            if signed_headers
                .binary_search_by(|probe| probe.as_ref().cmp(name))
                .is_ok()
            {
                headers.push((name, value));
            }
        }
        Self { headers }
    }

    /// Gets header value by name. Time `O(logn)`
    pub fn get(&self, name: impl AsHeaderName) -> Option<&'a str> {
        let headers = self.headers.as_slice();
        let ans = match headers.binary_search_by_key(&name.as_str(), |&(n, _)| n) {
            Ok(idx) => headers.get(idx).map(|&(_, v)| v),
            Err(_) => None,
        };
        drop(name);
        ans
    }

    /// Assigns value from optional header
    pub fn assign<T: FromStr>(
        &self,
        name: impl AsHeaderName,
        opt: &mut Option<T>,
    ) -> Result<(), T::Err> {
        if let Some(s) = self.get(name) {
            let v = s.parse()?;
            *opt = Some(v);
        }
        Ok(())
    }

    /// Assigns string from optional header
    pub fn assign_str(&self, name: impl AsHeaderName, opt: &mut Option<String>) {
        if let Some(s) = self.get(name) {
            *opt = Some(s.to_owned());
        }
    }
}

impl<'a> AsRef<[(&'a str, &'a str)]> for OrderedHeaders<'a> {
    fn as_ref(&self) -> &[(&'a str, &'a str)] {
        self.headers.as_ref()
    }
}
