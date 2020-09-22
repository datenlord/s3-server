//! fs implementation based on `tokio`

use crate::{
    dto::{
        Bucket, CopyObjectError, CopyObjectOutput, CopyObjectRequest, CopyObjectResult,
        CreateBucketError, CreateBucketOutput, CreateBucketRequest, DeleteBucketError,
        DeleteBucketOutput, DeleteBucketRequest, DeleteObjectError, DeleteObjectOutput,
        DeleteObjectRequest, DeleteObjectsError, DeleteObjectsOutput, DeleteObjectsRequest,
        DeletedObject, GetBucketLocationError, GetBucketLocationOutput, GetBucketLocationRequest,
        GetObjectError, GetObjectOutput, GetObjectRequest, HeadBucketError, HeadBucketOutput,
        HeadBucketRequest, HeadObjectError, HeadObjectOutput, HeadObjectRequest, ListBucketsError,
        ListBucketsOutput, ListBucketsRequest, ListObjectsError, ListObjectsOutput,
        ListObjectsRequest, ListObjectsV2Error, ListObjectsV2Output, ListObjectsV2Request, Object,
        PutObjectError, PutObjectOutput, PutObjectRequest,
    },
    S3ErrorCode, XmlErrorResponse,
};

use crate::{
    error::{S3Error, S3Result},
    path::check_bucket_name,
    storage::S3Storage,
    utils::{time, Apply, ByteStream},
    BoxStdError,
};

use std::{
    collections::VecDeque,
    convert::TryInto,
    env,
    future::Future,
    io,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use path_absolutize::Absolutize;

use tokio::{fs::File, stream::StreamExt};

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

    async fn copy_object(
        &self,
        input: CopyObjectRequest,
    ) -> S3Result<CopyObjectOutput, CopyObjectError> {
        use crate::headers::AmzCopySource;

        let copy_source = AmzCopySource::from_header_str(&input.copy_source)
            .map_err(|e| S3Error::InvalidRequest(e.into()))?;

        match copy_source {
            AmzCopySource::AccessPoint { .. } => Err(S3Error::NotSupported),
            AmzCopySource::Bucket { bucket, key } => {
                wrap_storage(async {
                    let src_path = self.get_object_path(bucket, key)?;
                    let dst_path = self.get_object_path(&input.bucket, &input.key)?;

                    let metadata = tokio::fs::metadata(&src_path).await?;
                    let last_modified = time::to_rfc3339(metadata.modified()?);

                    let _ = tokio::fs::copy(src_path, dst_path).await?;

                    let output = CopyObjectOutput {
                        copy_object_result: CopyObjectResult {
                            e_tag: None,
                            last_modified: Some(last_modified),
                        }
                        .apply(Some),
                        ..CopyObjectOutput::default()
                    };

                    Ok(Ok(output))
                })
                .await
            }
        }
    }

    async fn delete_bucket(
        &self,
        input: DeleteBucketRequest,
    ) -> S3Result<DeleteBucketOutput, DeleteBucketError> {
        wrap_storage(async {
            let path = self.get_bucket_path(&input.bucket)?;
            tokio::fs::remove_dir_all(path).await?;
            Ok(Ok(DeleteBucketOutput))
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

    async fn delete_objects(
        &self,
        input: DeleteObjectsRequest,
    ) -> S3Result<DeleteObjectsOutput, DeleteObjectsError> {
        wrap_storage(async move {
            let mut objects: Vec<(PathBuf, String)> = Vec::new();
            for object in input.delete.objects {
                let path = self.get_object_path(&input.bucket, &object.key)?;
                if path.exists() {
                    objects.push((path, object.key))
                } else {
                    return Err(io::Error::new(io::ErrorKind::NotFound, "No such object").into());
                }
            }

            let mut deleted: Vec<DeletedObject> = Vec::new();
            for (path, key) in objects {
                tokio::fs::remove_file(path).await?;
                deleted.push(DeletedObject {
                    key: Some(key),
                    ..DeletedObject::default()
                });
            }
            let output = DeleteObjectsOutput {
                deleted: Some(deleted),
                ..DeleteObjectsOutput::default()
            };
            Ok(Ok(output))
        })
        .await
    }

    async fn get_bucket_location(
        &self,
        input: GetBucketLocationRequest,
    ) -> S3Result<GetBucketLocationOutput, GetBucketLocationError> {
        let path = wrap_storage(async { self.get_bucket_path(&input.bucket)?.apply(Ok).apply(Ok) })
            .await?;

        if !path.exists() {
            return Err(<S3Error<GetBucketLocationError>>::Other(
                XmlErrorResponse::from_code_msg(S3ErrorCode::NoSuchBucket, "NotFound".into()),
            ));
            // return Err(io::Error::new(io::ErrorKind::NotFound, "NotFound").into());
        }

        wrap_storage(async move {
            let output = GetBucketLocationOutput {
                location_constraint: None, // TODO: handle region
            };
            Ok(Ok(output))
        })
        .await
    }

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
            let metadata = file.metadata().await?;
            let last_modified = time::to_rfc3339(metadata.modified()?);
            let content_length = metadata.len();
            let stream = ByteStream::new(file, 4096);

            let output: GetObjectOutput = GetObjectOutput {
                body: Some(crate::dto::ByteStream::new(stream)),
                content_length: Some(content_length.try_into()?),
                last_modified: Some(last_modified),
                ..GetObjectOutput::default() // TODO: handle other fields
            };

            Ok(Ok(output))
        })
        .await
    }

    async fn head_bucket(
        &self,
        input: HeadBucketRequest,
    ) -> S3Result<HeadBucketOutput, HeadBucketError> {
        wrap_storage(async move {
            let path = self.get_bucket_path(&input.bucket)?;
            if path.exists() {
                Ok(HeadBucketOutput)
            } else {
                Err(HeadBucketError::NoSuchBucket(
                    "The specified bucket does not exist.".into(),
                ))
            }
            .apply(Ok)
        })
        .await
    }

    async fn head_object(
        &self,
        input: HeadObjectRequest,
    ) -> S3Result<HeadObjectOutput, HeadObjectError> {
        wrap_storage(async move {
            let path = self.get_object_path(&input.bucket, &input.key)?;
            if path.exists() {
                let metadata = tokio::fs::metadata(path).await?;
                let last_modified = time::to_rfc3339(metadata.modified()?);
                let size = metadata.len();
                let output: HeadObjectOutput = HeadObjectOutput {
                    content_length: Some(size.try_into()?),
                    content_type: Some(mime::APPLICATION_OCTET_STREAM.as_ref().to_owned()), // TODO: handle content type
                    last_modified: Some(last_modified),
                    ..HeadObjectOutput::default()
                };
                Ok(output)
            } else {
                Err(HeadObjectError::NoSuchKey(
                    "The specified key does not exist.".into(),
                ))
            }
            .apply(Ok)
        })
        .await
    }

    async fn list_buckets(
        &self,
        _: ListBucketsRequest,
    ) -> S3Result<ListBucketsOutput, ListBucketsError> {
        wrap_storage(async move {
            let mut buckets = Vec::new();

            let mut iter = tokio::fs::read_dir(&self.root).await?;
            while let Some(entry) = iter.next().await {
                let entry = entry?;
                if entry.file_type().await?.is_dir() {
                    let file_name = entry.file_name();
                    let name = file_name.to_string_lossy();
                    if check_bucket_name(&*name) {
                        buckets.push(Bucket {
                            creation_date: None,
                            name: Some(name.into()),
                        })
                    }
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

    async fn list_objects(
        &self,
        input: ListObjectsRequest,
    ) -> S3Result<ListObjectsOutput, ListObjectsError> {
        wrap_storage(async move {
            let path = self.get_bucket_path(&input.bucket)?;

            let mut objects = Vec::new();
            let mut dir_queue = VecDeque::new();
            dir_queue.push_back(path.clone());

            while let Some(dir) = dir_queue.pop_front() {
                let mut entries = tokio::fs::read_dir(dir).await?;
                while let Some(entry) = entries.next().await {
                    let entry = entry?;
                    if entry.file_type().await?.is_dir() {
                        dir_queue.push_back(entry.path());
                    } else {
                        let file_path = entry.path();
                        let key = file_path.strip_prefix(&path)?;
                        if let Some(ref prefix) = input.prefix {
                            if !key.to_string_lossy().as_ref().starts_with(prefix) {
                                continue;
                            }
                        }

                        let metadata = entry.metadata().await?;
                        let last_modified = time::to_rfc3339(metadata.modified()?);
                        let size = metadata.len();

                        objects.push(Object {
                            e_tag: None,
                            key: Some(key.to_string_lossy().into()),
                            last_modified: Some(last_modified),
                            owner: None,
                            size: Some(size.try_into()?),
                            storage_class: None,
                        });
                    }
                }
            }

            // TODO: handle other fields
            let output = ListObjectsOutput {
                contents: Some(objects),
                delimiter: input.delimiter,
                encoding_type: input.encoding_type,
                name: Some(input.bucket),
                common_prefixes: None,
                is_truncated: None,
                marker: None,
                max_keys: None,
                next_marker: None,
                prefix: None,
            };

            Ok(Ok(output))
        })
        .await
    }

    async fn list_objects_v2(
        &self,
        input: ListObjectsV2Request,
    ) -> S3Result<ListObjectsV2Output, ListObjectsV2Error> {
        dbg!(&input);

        wrap_storage(async move {
            let path = self.get_bucket_path(&input.bucket)?;

            let mut objects = Vec::new();
            let mut dir_queue = VecDeque::new();
            dir_queue.push_back(path.clone());

            while let Some(dir) = dir_queue.pop_front() {
                let mut entries = tokio::fs::read_dir(dir).await?;
                while let Some(entry) = entries.next().await {
                    let entry = entry?;
                    if entry.file_type().await?.is_dir() {
                        dir_queue.push_back(entry.path());
                    } else {
                        let file_path = entry.path();
                        let key = file_path.strip_prefix(&path)?;
                        if let Some(ref prefix) = input.prefix {
                            if !key.to_string_lossy().as_ref().starts_with(prefix) {
                                continue;
                            }
                        }

                        let metadata = entry.metadata().await?;
                        let last_modified = time::to_rfc3339(metadata.modified()?);
                        let size = metadata.len();

                        objects.push(Object {
                            e_tag: None,
                            key: Some(key.to_string_lossy().into()),
                            last_modified: Some(last_modified),
                            owner: None,
                            size: Some(size.try_into()?),
                            storage_class: None,
                        });
                    }
                }
            }

            // TODO: handle other fields
            let output = ListObjectsV2Output {
                key_count: Some(objects.len().try_into()?),
                contents: Some(objects),
                delimiter: input.delimiter,
                encoding_type: input.encoding_type,
                name: Some(input.bucket),
                common_prefixes: None,
                is_truncated: None,
                max_keys: None,
                prefix: None,
                continuation_token: None,
                next_continuation_token: None,
                start_after: None,
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
}
