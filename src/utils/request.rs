//! request util

use crate::{path::ParseS3PathError, path::S3Path, Body, BoxStdError, Request};

use std::{mem, str::FromStr};

use hyper::header::{AsHeaderName, HeaderValue, ToStrError};
use serde::de::DeserializeOwned;

/// `RequestExt`
pub trait RequestExt {
    /// get header value as `&str`
    fn get_header_str(&self, name: impl AsHeaderName) -> Result<Option<&str>, ToStrError>;

    /// take request body
    fn take_body(&mut self) -> Body;

    /// extract url query
    fn extract_query<Q: DeserializeOwned>(&self) -> Result<Option<Q>, serde_urlencoded::de::Error>;

    /// extract s3 path
    fn extract_s3_path(&self) -> Result<S3Path<'_>, ParseS3PathError>;

    /// assign opt header
    fn assign_from_optional_header<T>(
        &self,
        name: impl AsHeaderName,
        opt: &mut Option<T>,
    ) -> Result<(), BoxStdError>
    where
        T: FromStr,
        T::Err: std::error::Error + Send + Sync + 'static;
}

impl RequestExt for Request {
    fn get_header_str(&self, name: impl AsHeaderName) -> Result<Option<&str>, ToStrError> {
        self.headers()
            .get(name)
            .map(HeaderValue::to_str)
            .transpose()
    }

    fn take_body(&mut self) -> Body {
        mem::replace(self.body_mut(), Body::empty())
    }

    fn extract_query<Q: DeserializeOwned>(&self) -> Result<Option<Q>, serde_urlencoded::de::Error> {
        self.uri()
            .query()
            .map(|s| serde_urlencoded::from_str::<Q>(s))
            .transpose()
    }

    fn extract_s3_path(&self) -> Result<S3Path<'_>, ParseS3PathError> {
        S3Path::try_from_path(self.uri().path())
    }

    fn assign_from_optional_header<T>(
        &self,
        name: impl AsHeaderName,
        opt: &mut Option<T>,
    ) -> Result<(), BoxStdError>
    where
        T: FromStr,
        T::Err: std::error::Error + Send + Sync + 'static,
    {
        if let Some(s) = self.get_header_str(name)? {
            let v = s.parse()?;
            *opt = Some(v);
        }
        Ok(())
    }
}
