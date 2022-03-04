//! internal macros

/// lazy-initialized regex
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

/// extracts the ok value of a result in a function returning `Result<T, E>` where E: From<S3Error>
///
/// returns an wrapped internal error to terminate the control flow
///
macro_rules! trace_try {
    ($ret:expr) => {
        match $ret {
            Ok(r) => r,
            Err(e) => return Err(internal_error!(e).into()),
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

        let err = err.finish();

        tracing::debug!(
            "generated s3 error: {}", err
        );

        err
    }};
}

/// Create a `NotSupported` error
macro_rules! not_supported {
    ($msg:expr) => {{
        code_error!(NotSupported, $msg)
    }};
}

/// Create a `InvalidRequest` error
macro_rules! invalid_request {
    ($msg:expr $(, $source:expr)?) => {{
        code_error!(InvalidRequest, $msg $(, $source)?)
    }};
}

/// Create a `SignatureDoesNotMatch` error
macro_rules! signature_mismatch {
    () => {{
        code_error!(
            SignatureDoesNotMatch,
            "The request signature we calculated does not match the signature you provided."
        )
    }};
}

/// Create an internal error
macro_rules! internal_error {
    ($e:expr) => {{
        let code = $crate::errors::S3ErrorCode::InternalError;
        let err = $crate::errors::S3Error::from_code(code)
            .message("We encountered an internal error. Please try again.")
            .source($e)
            .capture_backtrace()
            .capture_span_trace()
            .finish();

        tracing::error!("generated internal error: {}", err);

        err
    }};
}
