//! time format

use super::Apply;

use std::future::Future;
use std::time::{Duration, Instant, SystemTime};

use chrono::{DateTime, Local, Utc};
use futures::FutureExt;

/// See <https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Last-Modified>
///
/// See <https://docs.rs/chrono/0.4.15/chrono/format/strftime/index.html#specifiers>
const LAST_MODIFIED_TIME_FORMAT: &str = "%a, %d %b %Y %T GMT";

/// convert `SystemTime` to rfc3339
pub fn to_rfc3339(time: SystemTime) -> String {
    let time: DateTime<Local> = time.into();
    time.to_rfc3339()
}

/// convert rfc3339 to `last_modified`
pub fn rfc3339_to_last_modified(s: &str) -> Result<String, chrono::ParseError> {
    let time: DateTime<Utc> = DateTime::parse_from_rfc3339(s)?.into();
    time.format(LAST_MODIFIED_TIME_FORMAT).to_string().apply(Ok)
}

/// convert optional rfc3339 to optional `last_modified`
pub fn map_opt_rfc3339_to_last_modified(
    s: Option<String>,
) -> Result<Option<String>, chrono::ParseError> {
    s.map(|ref s| rfc3339_to_last_modified(s)).transpose()
}

/// Returns the output of a future and elapsed time
pub fn count_duration<F>(f: F) -> impl Future<Output = (F::Output, Duration)> + Send
where
    F: Future + Send,
{
    let t0 = Instant::now();
    f.map(move |ans| {
        let dur = t0.elapsed();
        (ans, dur)
    })
}
