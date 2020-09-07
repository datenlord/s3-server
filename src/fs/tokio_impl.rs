//! fs implementation based on `tokio`

use crate::error::{S3Error, S3Result};
use crate::storage::S3Storage;
use crate::utils::ByteStream;
use crate::BoxStdError;

use crate::dto::{
    Bucket, CreateBucketError, CreateBucketOutput, CreateBucketRequest, DeleteBucketError,
    DeleteBucketOutput, DeleteBucketRequest, DeleteObjectError, DeleteObjectOutput,
    DeleteObjectRequest, GetObjectError, GetObjectOutput, GetObjectRequest, HeadBucketError,
    HeadBucketOutput, HeadBucketRequest, ListBucketsError, ListBucketsOutput, PutObjectError,
    PutObjectOutput, PutObjectRequest,
};

use async_trait::async_trait;
use path_absolutize::Absolutize;
use std::convert::TryInto;
use std::env;
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};

use tokio::fs::File;
use tokio::stream::StreamExt as _;

/// A S3 storage implementation based on file system
#[derive(Debug)]
pub struct FileSystem {
    /// root path
    root: PathBuf,
}

impl FileSystem {
    /// Constructs a file system storage located at `root`
    /// # Errors
    /// Returns an `Err` if current working directory is invalid or `root` doesn't exist
    pub fn new(root: impl AsRef<Path>) -> io::Result<Self> {
        let cwd = env::current_dir()?;
        let root = cwd.join(root).canonicalize()?;
        Ok(Self { root })
    }

    /// resolve object path under the virtual root
    fn get_object_path(&self, bucket: &str, key: &str) -> io::Result<PathBuf> {
        let dir = Path::new(&bucket);
        let file_path = Path::new(&key);
        let ans = dir
            .join(&file_path)
            .absolutize_virtually(&self.root)?
            .into();
        Ok(ans)
    }

    /// resolve bucket path under the virtual root
    fn get_bucket_path(&self, bucket: &str) -> io::Result<PathBuf> {
        let dir = Path::new(&bucket);
        let ans = dir.absolutize_virtually(&self.root)?.into();
        Ok(ans)
    }
}

/// helper function for error converting
async fn wrap_storage<T, E, Fut>(f: Fut) -> S3Result<T, E>
where
    Fut: Future<Output = Result<Result<T, E>, BoxStdError>> + Send,
{
    match f.await {
        Ok(Ok(res)) => Ok(res),
        Ok(Err(err)) => Err(<S3Error<E>>::Operation(err)),
        Err(err) => Err(<S3Error<E>>::Storage(err)),
    }
}

#[async_trait]
impl S3Storage for FileSystem {
    async fn get_object(
        &self,
        input: GetObjectRequest,
    ) -> S3Result<GetObjectOutput, GetObjectError> {
        wrap_storage(async move {
            let path = self.get_object_path(&input.bucket, &input.key)?;
            let file = match File::open(&path).await {
                Ok(file) => file,
                Err(e) => {
                    log::error!("{}", e);
                    return Ok(Err(GetObjectError::NoSuchKey(
                        "The specified key does not exist.".into(),
                    )));
                }
            };
            let content_length = file.metadata().await?.len();
            let stream = ByteStream::new(file, 4096);

            let output: GetObjectOutput = GetObjectOutput {
                body: Some(crate::dto::ByteStream::new(stream)),
                content_length: Some(content_length.try_into()?),
                ..GetObjectOutput::default() // TODO: handle other fields
            };

            Ok(Ok(output))
        })
        .await
    }
    async fn put_object(
        &self,
        input: PutObjectRequest,
    ) -> S3Result<PutObjectOutput, PutObjectError> {
        wrap_storage(async move {
            let path = self.get_object_path(&input.bucket, &input.key)?;

            if let Some(body) = input.body {
                let mut reader = tokio::io::stream_reader(body);
                let file = File::create(&path).await?;
                let mut writer = tokio::io::BufWriter::new(file);
                let _ = tokio::io::copy(&mut reader, &mut writer).await?;
            }

            let output = PutObjectOutput::default(); // TODO: handle other fields

            Ok(Ok(output))
        })
        .await
    }
    async fn delete_object(
        &self,
        input: DeleteObjectRequest,
    ) -> S3Result<DeleteObjectOutput, DeleteObjectError> {
        wrap_storage(async move {
            let path = self.get_object_path(&input.bucket, &input.key)?;

            tokio::fs::remove_file(path).await?;

            let output = DeleteObjectOutput::default(); // TODO: handle other fields
            Ok(Ok(output))
        })
        .await
    }
    async fn create_bucket(
        &self,
        input: CreateBucketRequest,
    ) -> S3Result<CreateBucketOutput, CreateBucketError> {
        wrap_storage(async move {
            let path = self.get_bucket_path(&input.bucket)?;
            if path.exists() {
                return Ok(Err(CreateBucketError::BucketAlreadyExists(
                    concat!(
                        "The requested bucket name is not available. ",
                        "The bucket namespace is shared by all users of the system. ",
                        "Please select a different name and try again."
                    )
                    .into(),
                )));
            }

            tokio::fs::create_dir(&path).await?;

            let output = CreateBucketOutput::default(); // TODO: handle other fields
            Ok(Ok(output))
        })
        .await
    }
    async fn delete_bucket(
        &self,
        input: DeleteBucketRequest,
    ) -> S3Result<DeleteBucketOutput, DeleteBucketError> {
        wrap_storage(async move {
            let path = self.get_bucket_path(&input.bucket)?;
            tokio::fs::remove_dir_all(path).await?;
            Ok(Ok(DeleteBucketOutput))
        })
        .await
    }

    async fn head_bucket(
        &self,
        input: HeadBucketRequest,
    ) -> S3Result<HeadBucketOutput, HeadBucketError> {
        wrap_storage(async move {
            let path = self.get_bucket_path(&input.bucket)?;
            let ans = if path.exists() {
                Ok(HeadBucketOutput)
            } else {
                Err(HeadBucketError::NoSuchBucket(
                    "The specified bucket does not exist.".into(),
                ))
            };
            Ok(ans)
        })
        .await
    }
    async fn list_buckets(&self) -> S3Result<ListBucketsOutput, ListBucketsError> {
        wrap_storage(async move {
            let mut buckets = Vec::new();

            let mut iter = tokio::fs::read_dir(&self.root).await?;
            while let Some(entry) = iter.next().await {
                let entry = entry?;
                if entry.file_type().await?.is_dir() {
                    let name: String = entry.file_name().to_string_lossy().into();
                    buckets.push(Bucket {
                        creation_date: None,
                        name: Some(name),
                    })
                }
            }
            let output = ListBucketsOutput {
                buckets: Some(buckets),
                owner: None, // TODO: handle owner
            };
            Ok(Ok(output))
        })
        .await
    }
}
