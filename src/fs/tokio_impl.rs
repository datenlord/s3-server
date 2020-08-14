use crate::byte_stream::ByteStream;
use crate::error::S3Error;
use crate::s3_storage::S3Storage;
use crate::utils::BoxStdError;

use async_trait::async_trait;
use path_absolutize::Absolutize;
use std::convert::TryInto;
use std::env;
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};

use tokio::fs::File;
use tokio::stream::StreamExt as _;

use rusoto_s3::{
    CreateBucketError, CreateBucketOutput, CreateBucketRequest, DeleteBucketError,
    DeleteBucketRequest, DeleteObjectError, DeleteObjectOutput, DeleteObjectRequest,
    GetObjectError, GetObjectOutput, GetObjectRequest, HeadBucketError, HeadBucketRequest,
    ListBucketsError, ListBucketsOutput, PutObjectError, PutObjectOutput, PutObjectRequest,
};

#[derive(Debug)]
pub struct FileSystem {
    root: PathBuf,
}

impl FileSystem {
    pub fn new(root: impl AsRef<Path>) -> io::Result<Self> {
        let cwd = env::current_dir()?;
        let root = cwd.join(root).canonicalize()?;
        Ok(Self { root })
    }

    fn get_object_path(&self, bucket: &str, key: &str) -> io::Result<PathBuf> {
        let dir = Path::new(&bucket);
        let file_path = Path::new(&key);
        let ans = dir
            .join(&file_path)
            .absolutize_virtually(&self.root)?
            .into();
        Ok(ans)
    }

    fn get_bucket_path(&self, bucket: &str) -> io::Result<PathBuf> {
        let dir = Path::new(&bucket);
        let ans = dir.absolutize_virtually(&self.root)?.into();
        Ok(ans)
    }
}

async fn wrap_storage<T, E, Fut>(f: Fut) -> Result<T, S3Error<E>>
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
    ) -> Result<GetObjectOutput, S3Error<GetObjectError>> {
        wrap_storage(async move {
            let path = self.get_object_path(&input.bucket, &input.key)?;
            let file = File::open(&path).await?;
            let content_length = file.metadata().await?.len();
            let stream = ByteStream::new(file, 4096);

            let output: GetObjectOutput = GetObjectOutput {
                body: Some(rusoto_core::ByteStream::new(stream)),
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
    ) -> Result<PutObjectOutput, S3Error<PutObjectError>> {
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
    ) -> Result<DeleteObjectOutput, S3Error<DeleteObjectError>> {
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
    ) -> Result<CreateBucketOutput, S3Error<CreateBucketError>> {
        wrap_storage(async move {
            let path = self.get_bucket_path(&input.bucket)?;

            tokio::fs::create_dir(&path).await?;

            let output = CreateBucketOutput::default(); // TODO: handle other fields
            Ok(Ok(output))
        })
        .await
    }
    async fn delete_bucket(
        &self,
        input: DeleteBucketRequest,
    ) -> Result<(), S3Error<DeleteBucketError>> {
        wrap_storage(async move {
            let path = self.get_bucket_path(&input.bucket)?;
            tokio::fs::remove_dir_all(path).await?;
            Ok(Ok(()))
        })
        .await
    }

    async fn head_bucket(&self, input: HeadBucketRequest) -> Result<(), S3Error<HeadBucketError>> {
        wrap_storage(async move {
            let path = self.get_bucket_path(&input.bucket)?;
            let ans = if path.exists() {
                Ok(())
            } else {
                Err(HeadBucketError::NoSuchBucket(input.bucket))
            };
            Ok(ans)
        })
        .await
    }
    async fn list_buckets(&self) -> Result<ListBucketsOutput, S3Error<ListBucketsError>> {
        wrap_storage(async move {
            let mut buckets = Vec::new();

            let mut iter = tokio::fs::read_dir(&self.root).await?;
            while let Some(entry) = iter.next().await {
                let entry = entry?;
                if entry.file_type().await?.is_dir() {
                    let name: String = entry.file_name().to_string_lossy().into();
                    buckets.push(rusoto_s3::Bucket {
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
