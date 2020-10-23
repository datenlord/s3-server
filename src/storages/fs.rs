//! fs implementation based on `tokio`

use crate::data_structures::BytesStream;
use crate::dto::{
    Bucket, CompleteMultipartUploadError, CompleteMultipartUploadOutput,
    CompleteMultipartUploadRequest, CopyObjectError, CopyObjectOutput, CopyObjectRequest,
    CopyObjectResult, CreateBucketError, CreateBucketOutput, CreateBucketRequest,
    CreateMultipartUploadError, CreateMultipartUploadOutput, CreateMultipartUploadRequest,
    DeleteBucketError, DeleteBucketOutput, DeleteBucketRequest, DeleteObjectError,
    DeleteObjectOutput, DeleteObjectRequest, DeleteObjectsError, DeleteObjectsOutput,
    DeleteObjectsRequest, DeletedObject, GetBucketLocationError, GetBucketLocationOutput,
    GetBucketLocationRequest, GetObjectError, GetObjectOutput, GetObjectRequest, HeadBucketError,
    HeadBucketOutput, HeadBucketRequest, HeadObjectError, HeadObjectOutput, HeadObjectRequest,
    ListBucketsError, ListBucketsOutput, ListBucketsRequest, ListObjectsError, ListObjectsOutput,
    ListObjectsRequest, ListObjectsV2Error, ListObjectsV2Output, ListObjectsV2Request, Object,
    PutObjectError, PutObjectOutput, PutObjectRequest, UploadPartError, UploadPartOutput,
    UploadPartRequest,
};
use crate::errors::{S3StorageError, S3StorageResult};
use crate::headers::AmzCopySource;
use crate::path::S3Path;
use crate::storage::S3Storage;
use crate::utils::{crypto, time, Apply};
use crate::{async_trait, BoxStdError};

use std::collections::{HashMap, VecDeque};
use std::convert::TryInto;
use std::env;
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};

use futures::TryStreamExt;
use md5::{Digest, Md5};
use path_absolutize::Absolutize;
use tracing::{debug, error};
use uuid::Uuid;

use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::stream::StreamExt;

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
        let root = env::current_dir()?.join(root).canonicalize()?;
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

    /// get md5 sum
    async fn get_md5_sum(&self, bucket: &str, key: &str) -> io::Result<String> {
        let object_path = self.get_object_path(bucket, key)?;
        let mut file = File::open(&object_path).await?;
        let mut buf = vec![0; 4_usize.wrapping_mul(1024).wrapping_mul(1024)];
        let mut md5_hash = Md5::new();
        loop {
            let nread = file.read(&mut buf).await?;
            if nread == 0 {
                break;
            }
            md5_hash.update(buf.get(..nread).unwrap_or_else(|| {
                panic!(
                    "nread is larger than buffer size: nread = {}, size = {}",
                    nread,
                    buf.len()
                )
            }));
        }
        md5_hash.finalize().apply(crypto::to_hex_string).apply(Ok)
    }
}

/// wrap any error as an internal error
async fn wrap_internal_error<T, E, F>(f: F) -> S3StorageResult<T, E>
where
    F: Future<Output = Result<T, BoxStdError>> + Send,
{
    let e = try_err!(f.await);
    Err(S3StorageError::Other(internal_error!(e)))
}

/// wrap operation error
const fn operation_error<E>(e: E) -> S3StorageError<E> {
    S3StorageError::Operation(e)
}

#[async_trait]
impl S3Storage for FileSystem {
    async fn create_bucket(
        &self,
        input: CreateBucketRequest,
    ) -> S3StorageResult<CreateBucketOutput, CreateBucketError> {
        let path = self
            .get_bucket_path(&input.bucket)
            .map_err(|e| internal_error!(e))?;

        if path.exists() {
            let err = CreateBucketError::BucketAlreadyExists(String::from(
                "The requested bucket name is not available. \
                    The bucket namespace is shared by all users of the system. \
                    Please select a different name and try again.",
            ));
            return Err(operation_error(err));
        }

        tokio::fs::create_dir(&path)
            .await
            .map_err(|e| internal_error!(e))?;

        let output = CreateBucketOutput::default(); // TODO: handle other fields
        Ok(output)
    }

    async fn copy_object(
        &self,
        input: CopyObjectRequest,
    ) -> S3StorageResult<CopyObjectOutput, CopyObjectError> {
        let copy_source = AmzCopySource::from_header_str(&input.copy_source)
            .map_err(|err| invalid_request!("Invalid header: x-amz-copy-source", err))?;

        let (bucket, key) = match copy_source {
            AmzCopySource::AccessPoint { .. } => return Err(not_supported!().into()),
            AmzCopySource::Bucket { bucket, key } => (bucket, key),
        };

        wrap_internal_error(async {
            let src_path = self.get_object_path(bucket, key)?;
            let dst_path = self.get_object_path(&input.bucket, &input.key)?;

            let file_metadata = tokio::fs::metadata(&src_path).await?;
            let last_modified = time::to_rfc3339(file_metadata.modified()?);

            let _ = tokio::fs::copy(&src_path, &dst_path).await?;

            debug!(
                from = %src_path.display(),
                to = %dst_path.display(),
                "CopyObject: copy file",
            );

            let src_metadata_path = self.get_metadata_path(bucket, key)?;
            if src_metadata_path.exists() {
                let dst_metadata_path = self.get_metadata_path(&input.bucket, &input.key)?;
                let _ = tokio::fs::copy(src_metadata_path, dst_metadata_path).await?;
            }

            let md5_sum = self.get_md5_sum(bucket, key).await?;

            let output = CopyObjectOutput {
                copy_object_result: CopyObjectResult {
                    e_tag: Some(format!("\"{}\"", md5_sum)),
                    last_modified: Some(last_modified),
                }
                .apply(Some),
                ..CopyObjectOutput::default()
            };

            Ok(output)
        })
        .await
    }

    async fn delete_bucket(
        &self,
        input: DeleteBucketRequest,
    ) -> S3StorageResult<DeleteBucketOutput, DeleteBucketError> {
        wrap_internal_error(async {
            let path = self.get_bucket_path(&input.bucket)?;
            tokio::fs::remove_dir_all(path).await?;
            Ok(DeleteBucketOutput)
        })
        .await
    }

    async fn delete_object(
        &self,
        input: DeleteObjectRequest,
    ) -> S3StorageResult<DeleteObjectOutput, DeleteObjectError> {
        wrap_internal_error(async move {
            let path = self.get_object_path(&input.bucket, &input.key)?;
            if input.key.ends_with('/') {
                let mut dir = tokio::fs::read_dir(&path).await?;
                let is_empty = dir.next().await.is_none();
                if is_empty {
                    tokio::fs::remove_dir(&path).await?;
                }
            } else {
                tokio::fs::remove_file(path).await?;
            }
            let output = DeleteObjectOutput::default(); // TODO: handle other fields
            Ok(output)
        })
        .await
    }

    async fn delete_objects(
        &self,
        input: DeleteObjectsRequest,
    ) -> S3StorageResult<DeleteObjectsOutput, DeleteObjectsError> {
        wrap_internal_error(async move {
            let mut objects: Vec<(PathBuf, String)> = Vec::new();
            for object in input.delete.objects {
                let path = self.get_object_path(&input.bucket, &object.key)?;
                if path.exists() {
                    objects.push((path, object.key))
                }
                // else {
                // return Err(io::Error::new(io::ErrorKind::NotFound, "No such object").into());
                // }
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
            Ok(output)
        })
        .await
    }

    async fn get_bucket_location(
        &self,
        input: GetBucketLocationRequest,
    ) -> S3StorageResult<GetBucketLocationOutput, GetBucketLocationError> {
        let path = self
            .get_bucket_path(&input.bucket)
            .map_err(|e| internal_error!(e))?;

        if !path.exists() {
            let err = code_error!(NoSuchBucket, "NotFound");
            return Err(err.into());
        }

        let output = GetBucketLocationOutput {
            location_constraint: None, // TODO: handle region
        };

        Ok(output)
    }

    async fn get_object(
        &self,
        input: GetObjectRequest,
    ) -> S3StorageResult<GetObjectOutput, GetObjectError> {
        let object_path = self
            .get_object_path(&input.bucket, &input.key)
            .map_err(|e| internal_error!(e))?;

        let file = match File::open(&object_path).await {
            Ok(file) => file,
            Err(e) => {
                error!(error = %e, "GetObject: open file");
                let err = code_error!(NoSuchKey, "The specified key does not exist.");
                return Err(err.into());
            }
        };

        wrap_internal_error(async {
            let file_metadata = file.metadata().await?;
            let last_modified = time::to_rfc3339(file_metadata.modified()?);
            let content_length = file_metadata.len();
            let stream = BytesStream::new(file, 4096);

            let object_metadata = self.load_metadata(&input.bucket, &input.key).await?;

            let (md5_sum, duration) = {
                let (ret, duration) =
                    time::count_duration(self.get_md5_sum(&input.bucket, &input.key)).await;
                let md5_sum = ret?;
                (md5_sum, duration)
            };

            debug!(
                sum = ?md5_sum,
                path = %object_path.display(),
                size = ?content_length,
                ?duration,
                "GetObject: calculate md5 sum",
            );

            let output: GetObjectOutput = GetObjectOutput {
                body: Some(crate::dto::ByteStream::new(stream)),
                content_length: Some(content_length.try_into()?),
                last_modified: Some(last_modified),
                metadata: object_metadata,
                e_tag: Some(format!("\"{}\"", md5_sum)),
                ..GetObjectOutput::default() // TODO: handle other fields
            };

            Ok(output)
        })
        .await
    }

    async fn head_bucket(
        &self,
        input: HeadBucketRequest,
    ) -> S3StorageResult<HeadBucketOutput, HeadBucketError> {
        let path = self
            .get_bucket_path(&input.bucket)
            .map_err(|e| internal_error!(e))?;

        if !path.exists() {
            let err = code_error!(NoSuchBucket, "The specified bucket does not exist.");
            return Err(err.into());
        }

        Ok(HeadBucketOutput)
    }

    async fn head_object(
        &self,
        input: HeadObjectRequest,
    ) -> S3StorageResult<HeadObjectOutput, HeadObjectError> {
        let path = self
            .get_object_path(&input.bucket, &input.key)
            .map_err(|e| internal_error!(e))?;

        if !path.exists() {
            let err = code_error!(NoSuchKey, "The specified key does not exist.");
            return Err(err.into());
        }

        wrap_internal_error(async move {
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
        })
        .await
    }

    async fn list_buckets(
        &self,
        _: ListBucketsRequest,
    ) -> S3StorageResult<ListBucketsOutput, ListBucketsError> {
        wrap_internal_error(async move {
            let mut buckets = Vec::new();

            let mut iter = tokio::fs::read_dir(&self.root).await?;
            while let Some(entry) = iter.next().await {
                let entry = entry?;
                if entry.file_type().await?.is_dir() {
                    let file_name = entry.file_name();
                    let name = file_name.to_string_lossy();
                    if S3Path::check_bucket_name(&*name) {
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
            Ok(output)
        })
        .await
    }

    async fn list_objects(
        &self,
        input: ListObjectsRequest,
    ) -> S3StorageResult<ListObjectsOutput, ListObjectsError> {
        wrap_internal_error(async move {
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

            objects.sort_by(|lhs, rhs| {
                let lhs_key = lhs.key.as_deref().unwrap_or("");
                let rhs_key = rhs.key.as_deref().unwrap_or("");
                lhs_key.cmp(rhs_key)
            });

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

            Ok(output)
        })
        .await
    }

    async fn list_objects_v2(
        &self,
        input: ListObjectsV2Request,
    ) -> S3StorageResult<ListObjectsV2Output, ListObjectsV2Error> {
        wrap_internal_error(async move {
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

            objects.sort_by(|lhs, rhs| {
                let lhs_key = lhs.key.as_deref().unwrap_or("");
                let rhs_key = rhs.key.as_deref().unwrap_or("");
                lhs_key.cmp(rhs_key)
            });

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

            Ok(output)
        })
        .await
    }

    async fn put_object(
        &self,
        input: PutObjectRequest,
    ) -> S3StorageResult<PutObjectOutput, PutObjectError> {
        if let Some(ref storage_class) = input.storage_class {
            let is_valid = ["STANDARD", "REDUCED_REDUNDANCY"].contains(&storage_class.as_str());
            if !is_valid {
                let err = code_error!(
                    InvalidStorageClass,
                    "The storage class you specified is not valid."
                );
                return Err(err.into());
            }
        }

        let PutObjectRequest {
            body,
            bucket,
            key,
            metadata,
            content_length,
            ..
        } = input;

        let body = body.ok_or_else(||{
            code_error!(IncompleteBody,"You did not provide the number of bytes specified by the Content-Length HTTP header.")
        })?;

        if key.ends_with('/') {
            if content_length == Some(0) {
                return wrap_internal_error(async move {
                    let object_path = self.get_object_path(&bucket, &key)?;
                    tokio::fs::create_dir_all(&object_path).await?;
                    let output = PutObjectOutput::default();
                    Ok(output)
                })
                .await;
            } else {
                let err = not_supported!();
                return Err(err.into());
            }
        }

        wrap_internal_error(async move {
            let object_path = self.get_object_path(&bucket, &key)?;

            let mut md5_hash = Md5::new();
            let body = body.map_ok(|bytes| {
                md5_hash.update(&bytes);
                bytes
            });

            let file = File::create(&object_path).await?;
            let mut reader = tokio::io::stream_reader(body);
            let mut writer = tokio::io::BufWriter::new(file);

            let (ret, duration) =
                time::count_duration(tokio::io::copy(&mut reader, &mut writer)).await;
            let size = ret?;
            debug!(
                path = %object_path.display(),
                ?size,
                ?duration,
                "PutObject: write file",
            );
            if let Some(ref metadata) = metadata {
                self.save_metadata(&bucket, &key, metadata).await?;
            }

            let md5_sum = md5_hash.finalize().apply(crypto::to_hex_string);

            let mut output = PutObjectOutput::default(); // TODO: handle other fields
            output.e_tag = Some(format!("\"{}\"", md5_sum));

            Ok(output)
        })
        .await
    }

    async fn create_multipart_upload(
        &self,
        input: CreateMultipartUploadRequest,
    ) -> S3StorageResult<CreateMultipartUploadOutput, CreateMultipartUploadError> {
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
    ) -> S3StorageResult<UploadPartOutput, UploadPartError> {
        let UploadPartRequest {
            body,
            upload_id,
            part_number,
            ..
        } = input;

        let body = body.ok_or_else(||{
            code_error!(IncompleteBody, "You did not provide the number of bytes specified by the Content-Length HTTP header.")
        })?;

        wrap_internal_error(async {
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
                path = %file_path.display(),
                ?size,
                ?duration,
                "UploadPart: write file",
            );

            let md5_sum = md5_hash.finalize().apply(crypto::to_hex_string);
            let e_tag = format!("\"{}\"", md5_sum);

            let mut output = UploadPartOutput::default();
            output.e_tag = Some(e_tag);

            Ok(output)
        })
        .await
    }

    async fn complete_multipart_upload(
        &self,
        input: CompleteMultipartUploadRequest,
    ) -> S3StorageResult<CompleteMultipartUploadOutput, CompleteMultipartUploadError> {
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
            let err = code_error!(InvalidPart, "Missing multipart_upload");
            return Err(err.into());
        };

        wrap_internal_error(async {
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
                    from = %part_path.display(),
                    to = %object_path.display(),
                    ?size,
                    ?duration,
                    "CompleteMultipartUpload: write file",
                );
                tokio::fs::remove_file(&part_path).await?;
            }
            drop(writer);

            let file_size = tokio::fs::metadata(&object_path).await?.len();

            let (md5_sum, duration) = {
                let (ret, duration) = time::count_duration(self.get_md5_sum(&bucket, &key)).await;
                let md5_sum = ret?;
                (md5_sum, duration)
            };

            debug!(
                sum = ?md5_sum,
                path = %object_path.display(),
                size = ?file_size,
                ?duration,
                "CompleteMultipartUpload: calculate md5 sum",
            );

            let e_tag = format!("\"{}\"", md5_sum);
            let output = CompleteMultipartUploadOutput {
                bucket: Some(bucket),
                key: Some(key),
                e_tag: Some(e_tag),
                ..CompleteMultipartUploadOutput::default()
            };
            Ok(output)
        })
        .await
    }
}
