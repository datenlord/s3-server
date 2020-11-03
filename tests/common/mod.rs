use s3_server::path::S3Path;

use std::env;
use std::fs;
use std::mem;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use hyper::{header, Body};
use mime::Mime;
use tracing::{debug_span, error};

pub trait ResultExt<T, E> {
    fn inspect_err(self, f: impl FnOnce(&mut E)) -> Self;
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
    fn inspect_err(mut self, f: impl FnOnce(&mut E)) -> Self {
        if let Err(ref mut e) = self {
            f(e)
        }
        self
    }
}

macro_rules! enter_sync {
    ($span:expr) => {
        let __span = $span;
        let __enter = __span.enter();
    };
}

pub type Request = hyper::Request<Body>;
pub type Response = hyper::Response<Body>;

pub fn setup_tracing() {
    use tracing_error::ErrorSubscriber;
    use tracing_subscriber::subscribe::CollectorExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::{fmt, EnvFilter};

    let _ = tracing_subscriber::fmt()
        .event_format(fmt::format::Format::default().pretty())
        .with_env_filter(EnvFilter::try_new("s3_server=debug").unwrap())
        .with_timer(fmt::time::ChronoLocal::rfc3339())
        .with_test_writer()
        .finish()
        .with(ErrorSubscriber::default())
        .try_init();
}

pub fn setup_fs_root(clear: bool) -> Result<PathBuf> {
    let root: PathBuf = env::var("S3_TEST_FS_ROOT")
        .unwrap_or_else(|_| "target/s3-test".into())
        .into();

    enter_sync!(debug_span!("setup fs root", ?clear, root = %root.display()));

    let exists = root.exists();
    if exists && clear {
        fs::remove_dir_all(&root)
            .inspect_err(|err| error!(%err,"failed to remove root directory"))?;
    }

    if !exists || clear {
        fs::create_dir_all(&root).inspect_err(|err| error!(%err, "failed to create directory"))?;
    }

    if !root.exists() {
        let err = anyhow!("root does not exist");
        error!(%err);
        return Err(err);
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
