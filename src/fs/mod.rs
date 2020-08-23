//! S3 storage implementations based on file system

cfg_rt_tokio! {
    mod tokio_impl;
    pub use tokio_impl::FileSystem as TokioFileSystem;
}
