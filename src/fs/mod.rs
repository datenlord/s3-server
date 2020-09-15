//! S3 storage implementations based on file system

#[cfg(feature = "rt-tokio")]
mod tokio_impl;

#[cfg(feature = "rt-tokio")]
pub use tokio_impl::FileSystem as TokioFileSystem;
