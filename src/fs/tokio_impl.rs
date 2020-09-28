//! fs implementation based on `tokio`

use crate::{
    dto::{
        Bucket, CompleteMultipartUploadError, CompleteMultipartUploadOutput,
        CompleteMultipartUploadRequest, CopyObjectError, CopyObjectOutput, CopyObjectRequest,
        CopyObjectResult, CreateBucketError, CreateBucketOutput, CreateBucketRequest,
        CreateMultipartUploadError, CreateMultipartUploadOutput, CreateMultipartUploadRequest,
        DeleteBucketError, DeleteBucketOutput, DeleteBucketRequest, DeleteObjectError,
        DeleteObjectOutput, DeleteObjectRequest, DeleteObjectsError, DeleteObjectsOutput,
        DeleteObjectsRequest, DeletedObject, GetBucketLocationError, GetBucketLocationOutput,
        GetBucketLocationRequest, GetObjectError, GetObjectOutput, GetObjectRequest,
        HeadBucketError, HeadBucketOutput, HeadBucketRequest, HeadObjectError, HeadObjectOutput,
        HeadObjectRequest, ListBucketsError, ListBucketsOutput, ListBucketsRequest,
        ListObjectsError, ListObjectsOutput, ListObjectsRequest, ListObjectsV2Error,
        ListObjectsV2Output, ListObjectsV2Request, Object, PutObjectError, PutObjectOutput,
        PutObjectRequest, UploadPartError, UploadPartOutput, UploadPartRequest,
    },
    error::{S3Error, S3Result},
    path::check_bucket_name,
    storage::S3Storage,
    utils::{time, Apply, ByteStream},
    BoxStdError, S3ErrorCode, XmlErrorResponse,
};

use std::{
    collections::HashMap,
    collections::VecDeque,
    convert::TryInto,
    env,
    fmt::{self, Debug},
    future::Future,
    io,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use futures::TryStreamExt;
use log::{debug, error};
use md5::{Digest, Md5};
use path_absolutize::Absolutize;
use tokio::{fs::File, io::AsyncReadExt, stream::StreamExt};
use uuid::Uuid;

/// A S3 storage implementation based on file system
#[derive(Debug)]
pub struct FileSystem {
    /// root path
    root: PathBuf,

    /// validators
    validators: Validators,
}

/// validators

#[derive(Default)]
struct Validators {
    /// storage class validator
    storage_class: Option<Box<dyn Fn(&str) -> bool + Send + Sync + 'static>>,
}

impl FileSystem {
    /// Constructs a file system storage located at `root`
    /// # Errors
    /// Returns an `Err` if current working directory is invalid or `root` doesn't exist
    pub fn new(root: impl AsRef<Path>) -> io::Result<Self> {
        let root = env::current_dir()?.join(root).canonicalize()?;
        let validators = Validators::default();
        Ok(Self { root, validators })
    }

    /// Set a validator for x-amz-storage-class
    pub fn set_storage_class_validator<F>(&mut self, f: F)
    where
        F: Fn(&str) -> bool + Send + Sync + 'static,
    {
        self.validators.storage_class = Some(Box::new(f));
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

    /// resolve metadata path under the virtual root (custom format)
    fn get_metadata_path(&self, bucket: &str, key: &str) -> io::Result<PathBuf> {
        let encode = |s: &str| base64::encode_config(s, base64::URL_SAFE_NO_PAD);

        let file_path_str = format!(
            ".bucket-{}.object-{}.metadata.json",
            encode(bucket),
            encode(key),
        );
        let file_path = Path::new(&file_path_str);
        let ans = file_path.absolutize_virtually(&self.root)?.into();
        Ok(ans)
    }

    /// load metadata from fs
    async fn load_metadata(
        &self,
        bucket: &str,
        key: &str,
    ) -> io::Result<Option<HashMap<String, String>>> {
        let path = self.get_metadata_path(bucket, key)?;
        if path.exists() {
            let content = tokio::fs::read(&path).await?;
            let map = serde_json::from_slice(&content)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            Ok(Some(map))
        } else {
            Ok(None)
        }
    }

    /// save metadata
    async fn save_metadata(
        &self,
        bucket: &str,
        key: &str,
        metadata: &HashMap<String, String>,
    ) -> io::Result<()> {
        let path = self.get_metadata_path(bucket, key)?;
        let content = serde_json::to_vec(metadata)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        tokio::fs::write(&path, &content).await
    }
}

impl Validators {
    /// validate storage class
    fn validate_storage_class(&self, storage_class: &str) -> bool {
        self.storage_class
            .as_ref()
            .map_or(true, |f| f(storage_class))
    }
}

impl Debug for Validators {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Validators {{...}}")
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

                    let file_metadata = tokio::fs::metadata(&src_path).await?;
                    let last_modified = time::to_rfc3339(file_metadata.modified()?);

                    let _ = tokio::fs::copy(src_path, dst_path).await?;

                    {
                        let src_metadata_path = self.get_metadata_path(bucket, key)?;
                        let dst_metadata_path =
                            self.get_metadata_path(&input.bucket, &input.key)?;
                        let _ = tokio::fs::copy(src_metadata_path, dst_metadata_path).await?;
                    }

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
                    error!("{}", e);
                    return Ok(Err(GetObjectError::NoSuchKey(
                        "The specified key does not exist.".into(),
                    )));
                }
            };
            let file_metadata = file.metadata().await?;
            let last_modified = time::to_rfc3339(file_metadata.modified()?);
            let content_length = file_metadata.len();
            let stream = ByteStream::new(file, 4096);

            let object_metadata = self.load_metadata(&input.bucket, &input.key).await?;

            let output: GetObjectOutput = GetObjectOutput {
                body: Some(crate::dto::ByteStream::new(stream)),
                content_length: Some(content_length.try_into()?),
                last_modified: Some(last_modified),
                metadata: object_metadata,
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
                let file_metadata = tokio::fs::metadata(path).await?;
                let last_modified = time::to_rfc3339(file_metadata.modified()?);
                let size = file_metadata.len();

                let object_metadata = self.load_metadata(&input.bucket, &input.key).await?;

                let output: HeadObjectOutput = HeadObjectOutput {
                    content_length: Some(size.try_into()?),
                    content_type: Some(mime::APPLICATION_OCTET_STREAM.as_ref().to_owned()), // TODO: handle content type
                    last_modified: Some(last_modified),
                    metadata: object_metadata,
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
        if let Some(ref storage_class) = input.storage_class {
            if !self.validators.validate_storage_class(storage_class) {
                return Err(S3Error::Other(XmlErrorResponse::from_code_msg(
                    S3ErrorCode::InvalidStorageClass,
                    "The storage class you specified is not valid.".into(),
                )));
            }
        }

        let PutObjectRequest {
            body,
            bucket,
            key,
            metadata,
            ..
        } = input;

        let body = body.ok_or_else(||{
            S3Error::Other(
                XmlErrorResponse::from_code_msg(
                S3ErrorCode::IncompleteBody,
                "You did not provide the number of bytes specified by the Content-Length HTTP header.".into(),))})?;

        wrap_storage(async move {
            let path = self.get_object_path(&bucket, &key)?;

            let mut md5_hash = Md5::new();
            let body = body.map_ok(|bytes| {
                md5_hash.update(&bytes);
                bytes
            });

            let mut reader = tokio::io::stream_reader(body);
            let file = File::create(&path).await?;
            let mut writer = tokio::io::BufWriter::new(file);

            let (ret, duration) =
                time::count_duration(tokio::io::copy(&mut reader, &mut writer)).await;
            let size = ret?;
            debug!(
                "PutObject: write file: path = {}, size = {}, duration = {:?}",
                path.display(),
                size,
                duration
            );
            if let Some(ref metadata) = metadata {
                self.save_metadata(&bucket, &key, metadata).await?;
            }

            let md5_sum = md5_hash
                .finalize()
                .apply(|a| faster_hex::hex_string(&a))
                .unwrap_or_else(|_| unreachable!());

            let mut output = PutObjectOutput::default(); // TODO: handle other fields
            output.e_tag = Some(format!("\"{}\"", md5_sum));

            Ok(Ok(output))
        })
        .await
    }

    async fn create_multipart_upload(
        &self,
        input: CreateMultipartUploadRequest,
    ) -> S3Result<CreateMultipartUploadOutput, CreateMultipartUploadError> {
        let upload_id = Uuid::new_v4().to_string();

        let output = CreateMultipartUploadOutput {
            bucket: Some(input.bucket),
            key: Some(input.key),
            upload_id: Some(upload_id),
            ..CreateMultipartUploadOutput::default()
        };

        Ok(output)
    }

    async fn upload_part(
        &self,
        input: UploadPartRequest,
    ) -> S3Result<UploadPartOutput, UploadPartError> {
        let UploadPartRequest {
            body,
            upload_id,
            part_number,
            ..
        } = input;

        let body = body.ok_or_else(||{
            S3Error::Other(
                XmlErrorResponse::from_code_msg(
                S3ErrorCode::IncompleteBody,
                "You did not provide the number of bytes specified by the Content-Length HTTP header.".into(),))})?;

        wrap_storage(async {
            let file_path_str = format!(".upload_id-{}.part-{}", upload_id, part_number);
            let file_path = Path::new(&file_path_str).absolutize_virtually(&self.root)?;

            let mut md5_hash = Md5::new();
            let body = body.map_ok(|bytes| {
                md5_hash.update(&bytes);
                bytes
            });

            let file = File::create(&file_path).await?;
            let mut reader = tokio::io::stream_reader(body);
            let mut writer = tokio::io::BufWriter::new(file);

            let (ret, duration) =
                time::count_duration(tokio::io::copy(&mut reader, &mut writer)).await;
            let size = ret?;
            debug!(
                "UploadPart: write file: path = {}, size = {}, duration = {:?}",
                file_path.display(),
                size,
                duration
            );

            let md5_sum = md5_hash
                .finalize()
                .apply(|a| faster_hex::hex_string(&a))
                .unwrap_or_else(|_| unreachable!());
            let e_tag = format!("\"{}\"", md5_sum);

            let mut output = UploadPartOutput::default();
            output.e_tag = Some(e_tag);

            Ok(Ok(output))
        })
        .await
    }

    async fn complete_multipart_upload(
        &self,
        input: CompleteMultipartUploadRequest,
    ) -> S3Result<CompleteMultipartUploadOutput, CompleteMultipartUploadError> {
        let CompleteMultipartUploadRequest {
            multipart_upload,
            bucket,
            key,
            upload_id,
            ..
        } = input;

        let multipart_upload = if let Some(multipart_upload) = multipart_upload {
            multipart_upload
        } else {
            return Err(S3Error::Other(XmlErrorResponse::from_code_msg(
                S3ErrorCode::InvalidPart,
                "Missing multipart_upload".into(),
            )));
        };

        wrap_storage(async {
            let object_path = self.get_object_path(&bucket, &key)?;
            let file = File::create(&object_path).await?;
            let mut writer = tokio::io::BufWriter::new(file);

            let mut cnt: i64 = 0;
            for part in multipart_upload.parts.into_iter().flatten() {
                let part_number = part.part_number.ok_or_else(|| {
                    io::Error::new(io::ErrorKind::NotFound, "Missing part_number")
                })?;
                cnt = cnt.wrapping_add(1);
                if part_number != cnt {
                    return Err(io::Error::new(io::ErrorKind::Other, "InvalidPartOrder").into());
                }
                let part_path_str = format!(".upload_id-{}.part-{}", upload_id, part_number);
                let part_path = Path::new(&part_path_str).absolutize_virtually(&self.root)?;
                let mut reader = tokio::io::BufReader::new(File::open(&part_path).await?);
                let (ret, duration) =
                    time::count_duration(tokio::io::copy(&mut reader, &mut writer)).await;
                let size = ret?;
                debug!(
                    "CompleteMultipartUpload: write file from {} to {}, size = {}, duration = {:?}",
                    part_path.display(),
                    object_path.display(),
                    size,
                    duration
                );
                tokio::fs::remove_file(&part_path).await?;
            }
            drop(writer);

            let mut file = File::open(&object_path).await?;
            let file_size = file.metadata().await?.len();

            let (md5_sum, duration) = time::count_duration(async {
                let mut buf = vec![0; 4_usize.wrapping_mul(1024).wrapping_mul(1024)];
                let mut md5_hash = Md5::new();
                while let Ok(nread) = file.read(&mut buf).await {
                    if nread == 0{
                        break
                    }
                    md5_hash.update(buf.get(..nread).unwrap_or_else(|| unreachable!()));
                }
                md5_hash
                    .finalize()
                    .apply(|a| faster_hex::hex_string(&a))
                    .unwrap_or_else(|_| unreachable!())
            })
            .await;

            debug!("CompleteMultipartUpload: calculate md5 sum: sum = {}, path = {}, size = {}, duration = {:?}",
                md5_sum,
                object_path.display(),
                file_size,
                duration
            );

            let e_tag = format!("\"{}\"", md5_sum);
            let output = CompleteMultipartUploadOutput {
                bucket: Some(bucket),
                key: Some(key),
                e_tag: Some(e_tag),
                ..CompleteMultipartUploadOutput::default()
            };
            Ok(Ok(output))
        })
        .await
    }
}
