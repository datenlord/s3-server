//! utils

mod also;
mod apply;
mod response;
mod xml;

pub use self::also::Also;
pub use self::apply::Apply;
pub use self::response::ResponseExt;
pub use self::xml::XmlWriterExt;

pub mod crypto;
pub mod time;

use crate::{Body, BoxStdError};

use serde::de::DeserializeOwned;

/// deserialize xml body
pub async fn deserialize_xml_body<T: DeserializeOwned>(body: Body) -> Result<T, BoxStdError> {
    let bytes = hyper::body::to_bytes(body).await?;
    let ans: T = quick_xml::de::from_reader(&*bytes)?;
    Ok(ans)
}

macro_rules! static_regex {
    ($re: literal) => {{
        use once_cell::sync::Lazy;
        use regex::Regex;

        // compile-time verified regex
        const RE: &'static str = const_str::verified_regex!($re);

        static PATTERN: Lazy<Regex> = Lazy::new(|| {
            Regex::new(RE).unwrap_or_else(|e| panic!("Invalid static regex pattern: {}", e))
        });

        &*PATTERN
    }};
}

/// extracts the error of a result in a function returning `Result<T, E>`
///
/// returns `Ok(r)` to terminate the control flow
///
macro_rules! try_err {
    ($ret:expr) => {
        match $ret {
            Ok(r) => return Ok(r),
            Err(e) => e,
        }
    };
}

/// extracts the value of a option in a function returning `Result<Option<T>, E>`
///
/// returns `Ok(None)` to terminate the control flow
///
macro_rules! try_some {
    ($opt:expr) => {
        match $opt {
            Some(r) => r,
            None => return Ok(None),
        }
    };
}

/// asserts a predicate is true in a function returning `bool`
///
/// returns `false` to terminate the control flow
///
macro_rules! bool_try {
    ($pred:expr) => {
        if !$pred {
            return false;
        }
    };
}

/// extracts the value of a option in a function returning `bool`
///
/// returns `false` to terminate the control flow
///
macro_rules! bool_try_some {
    ($opt:expr) => {
        match $opt {
            Some(r) => r,
            None => return false,
        }
    };
}

/// Create a `S3Error` with code and message
macro_rules! code_error {
    ($code:ident, $msg:expr $(, $source:expr)?) => {
        code_error!(code = $crate::errors::S3ErrorCode::$code, $msg $(, $source)?)
    };
    (code = $code:expr, $msg:expr $(, $source:expr)?) => {{
        let code = $code;
        let err = $crate::errors::S3Error::from_code(code).message($msg);

        $(let err = err.source($source);)?

        #[cfg(debug_assertions)]
        let err = err.capture_span_trace();

        #[cfg(debug_assertions)]
        let err = err.capture_backtrace();

        let err = err.finish();

        const LOCATION: &str = concat!(file!(), ":", line!());

        tracing::error!(
            location = LOCATION,
            "generated s3 error: {}", err
        );

        if let Some(t) = err.span_trace(){
            if t.status() == tracing_error::SpanTraceStatus::CAPTURED {
                tracing::error!(
                    "location: {}, error: {}, span trace:\n{}",
                    LOCATION, err, t
                );
            }
        }

        if let Some(t) = err.backtrace(){
            tracing::error!(
                "location: {}, error: {}, backtrace:\n{:?}",
                LOCATION, err, t
            );
        }

        err
    }};
}

/// Create a `NotSupported` error
macro_rules! not_supported {
    () => {{
        code_error!(NotSupported, "The operation is not supported.")
    }};
}

/// Create a `InvalidRequest` error
macro_rules! invalid_request {
    ($msg:expr $(, $source:expr)?) => {{
        code_error!(InvalidRequest, $msg $(, $source)?)
    }};
}

macro_rules! signature_mismatch {
    () => {{
        code_error!(
            SignatureDoesNotMatch,
            "The request signature we calculated does not match the signature you provided."
        )
    }};
}

macro_rules! internal_error {
    ($e:expr) => {{
        let code = $crate::errors::S3ErrorCode::InternalError;
        let err = $crate::errors::S3Error::from_code(code)
            .message("We encountered an internal error. Please try again.")
            .source($e)
            .capture_backtrace()
            .capture_span_trace()
            .finish();

        tracing::debug!(
            location = concat!(file!(), ":", line!()),
            "generated internal error: {}",
            err
        );

        err
    }};
}
