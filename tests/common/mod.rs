use s3_server::path::S3Path;

use std::env;
use std::fs;
use std::mem;
use std::path::{Path, PathBuf};

use anyhow::Result;
use hyper::{header, Body};
use mime::Mime;
use tracing::debug_span;
use tracing_subscriber::EnvFilter;

#[macro_export]
macro_rules! enter {
    ($span:expr) => {
        let __span = $span;
        let __enter = __span.enter();
    };
}

#[macro_export]
macro_rules! trace_ctx{
    ($result:expr, $fmt:literal $($args:tt)*) => {
        match $result {
            Ok(r) => Ok(r),
            Err(e) => Err({
                tracing::error!($fmt, err = e $($args)*);
                let ctx = format!($fmt, err = e $($args)*);
                anyhow::Error::new(e).context(ctx)
            }),
        }
    }
}

#[macro_export]
macro_rules! trace_anyhow{
    ($fmt:literal $($args:tt)*) => {{
        tracing::error!($fmt $($args)*);
        anyhow::anyhow!($fmt $($args)*)
    }}
}

pub type Request = hyper::Request<Body>;
pub type Response = hyper::Response<Body>;

pub fn setup_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_new("s3_server=debug").unwrap())
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::rfc3339())
        .with_test_writer()
        .try_init();
}

pub fn setup_fs_root(clear: bool) -> Result<PathBuf> {
    let root: PathBuf = env::var("S3_TEST_FS_ROOT")
        .unwrap_or_else(|_| "target/s3-test".into())
        .into();

    enter!(debug_span!("setup fs root", ?clear, root = %root.display()));

    let exists = root.exists();
    if exists && clear {
        trace_ctx!(
            fs::remove_dir_all(&root),
            "failed to remove root directory: {err}",
        )?;
    }

    if !exists || clear {
        trace_ctx!(
            fs::create_dir_all(&root),
            "failed to create directory: {err}",
        )?;
    }

    if !root.exists() {
        return Err(trace_anyhow!("root does not exist"));
    }

    Ok(root)
}

pub fn generate_path(root: impl AsRef<Path>, path: S3Path) -> PathBuf {
    match path {
        S3Path::Root => root.as_ref().to_owned(),
        S3Path::Bucket { bucket } => root.as_ref().join(bucket),
        S3Path::Object { bucket, key } => root.as_ref().join(bucket).join(key),
    }
}

pub async fn recv_body_string(res: &mut Response) -> Result<String> {
    let body = mem::take(res.body_mut());
    let bytes = hyper::body::to_bytes(body).await?;
    let ans = String::from_utf8(bytes.to_vec())?;
    Ok(ans)
}

pub fn parse_mime(res: &Response) -> Result<Mime> {
    match res.headers().get(header::CONTENT_TYPE) {
        None => anyhow::bail!("No Content-Type"),
        Some(v) => Ok(v.to_str()?.parse::<Mime>()?),
    }
}
