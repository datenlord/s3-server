use datenlord_s3::S3Path;

use anyhow::{Context, Result};
use hyper::{header, Body};
use mime::Mime;
use std::env;
use std::fs;
use std::mem;
use std::path::{Path, PathBuf};

pub type Request = hyper::Request<Body>;
pub type Response = hyper::Response<Body>;

pub fn setup_fs_root(clear: bool) -> Result<PathBuf> {
    let root: PathBuf = env::var("S3_TEST_FS_ROOT")
        .unwrap_or_else(|_| "target/s3-test".into())
        .into();

    let exists = root.exists();
    if exists && clear {
        fs::remove_dir_all(&root)
            .with_context(|| format!("Failed to remove directory: {}", root.display()))?;
    }

    if !exists || clear {
        fs::create_dir_all(&root)
            .with_context(|| format!("Failed to create directory: {}", root.display()))?;
    }

    assert!(
        root.exists(),
        "root does not exist. root = {}",
        root.display()
    );

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
