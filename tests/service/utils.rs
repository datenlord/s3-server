use s3_server::path::S3Path;

use std::fs;
use std::io;
use std::mem;
use std::path::{Path, PathBuf};

use anyhow::Result;
use hyper::header;
use mime::Mime;

pub type Request = hyper::Request<hyper::Body>;
pub type Response = hyper::Response<hyper::Body>;

pub trait ResultExt<T, E> {
    /// See <https://github.com/rust-lang/rust/issues/91345>
    fn unstable_inspect_err(self, f: impl FnOnce(&mut E)) -> Self;
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
    fn unstable_inspect_err(mut self, f: impl FnOnce(&mut E)) -> Self {
        if let Err(ref mut e) = self {
            f(e)
        }
        self
    }
}

pub fn parse_mime(res: &Response) -> Result<Mime> {
    match res.headers().get(header::CONTENT_TYPE) {
        None => anyhow::bail!("No Content-Type"),
        Some(v) => Ok(v.to_str()?.parse::<Mime>()?),
    }
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

#[tracing::instrument(
    skip(root),
    fields(root = %root.as_ref().display()),
)]
pub fn fs_write_object(
    root: impl AsRef<Path>,
    bucket: &str,
    key: &str,
    content: &str,
) -> io::Result<()> {
    let dir_path = generate_path(&root, S3Path::Bucket { bucket });
    if !dir_path.exists() {
        fs::create_dir(dir_path)?;
    }
    let file_path = generate_path(root, S3Path::Object { bucket, key });
    fs::write(file_path, content)
}
