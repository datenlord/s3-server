//! Trait representing the capabilities of the Amazon S3 API at server side

use crate::errors::S3StorageResult;

use crate::dto::{
    CompleteMultipartUploadError, CompleteMultipartUploadOutput, CompleteMultipartUploadRequest,
    CopyObjectError, CopyObjectOutput, CopyObjectRequest, CreateBucketError, CreateBucketOutput,
    CreateBucketRequest, CreateMultipartUploadError, CreateMultipartUploadOutput,
    CreateMultipartUploadRequest, DeleteBucketError, DeleteBucketOutput, DeleteBucketRequest,
    DeleteObjectError, DeleteObjectOutput, DeleteObjectRequest, DeleteObjectsError,
    DeleteObjectsOutput, DeleteObjectsRequest, GetBucketLocationError, GetBucketLocationOutput,
    GetBucketLocationRequest, GetObjectError, GetObjectOutput, GetObjectRequest, HeadBucketError,
    HeadBucketOutput, HeadBucketRequest, HeadObjectError, HeadObjectOutput, HeadObjectRequest,
    ListBucketsError, ListBucketsOutput, ListBucketsRequest, ListObjectsError, ListObjectsOutput,
    ListObjectsRequest, ListObjectsV2Error, ListObjectsV2Output, ListObjectsV2Request,
    PutObjectError, PutObjectOutput, PutObjectRequest, UploadPartError, UploadPartOutput,
    UploadPartRequest,
};

use async_trait::async_trait;

/// Trait representing the capabilities of the Amazon S3 API at server side.
///
/// See <https://docs.aws.amazon.com/AmazonS3/latest/API/API_Operations_Amazon_Simple_Storage_Service.html>
#[async_trait]
pub trait S3Storage {
    /// See [CompleteMultipartUpload](https://docs.aws.amazon.com/AmazonS3/latest/API/API_CompleteMultipartUpload.html)
    async fn complete_multipart_upload(
        &self,
        input: CompleteMultipartUploadRequest,
    ) -> S3StorageResult<CompleteMultipartUploadOutput, CompleteMultipartUploadError>;

    /// See [CopyObject](https://docs.aws.amazon.com/AmazonS3/latest/API/API_CopyObject.html)
    async fn copy_object(
        &self,
        input: CopyObjectRequest,
    ) -> S3StorageResult<CopyObjectOutput, CopyObjectError>;

    /// See [CreateMultipartUpload](https://docs.aws.amazon.com/AmazonS3/latest/API/API_CreateMultipartUpload.html)
    async fn create_multipart_upload(
        &self,
        input: CreateMultipartUploadRequest,
    ) -> S3StorageResult<CreateMultipartUploadOutput, CreateMultipartUploadError>;

    /// See [CreateBucket](https://docs.aws.amazon.com/AmazonS3/latest/API/API_CreateBucket.html)
    async fn create_bucket(
        &self,
        input: CreateBucketRequest,
    ) -> S3StorageResult<CreateBucketOutput, CreateBucketError>;

    /// See [DeleteBucket](https://docs.aws.amazon.com/AmazonS3/latest/API/API_DeleteBucket.html)
    async fn delete_bucket(
        &self,
        input: DeleteBucketRequest,
    ) -> S3StorageResult<DeleteBucketOutput, DeleteBucketError>;

    /// See [DeleteObject](https://docs.aws.amazon.com/AmazonS3/latest/API/API_DeleteObject.html)
    async fn delete_object(
        &self,
        input: DeleteObjectRequest,
    ) -> S3StorageResult<DeleteObjectOutput, DeleteObjectError>;

    /// See [DeleteObject](https://docs.aws.amazon.com/AmazonS3/latest/API/API_DeleteObject.html)
    async fn delete_objects(
        &self,
        input: DeleteObjectsRequest,
    ) -> S3StorageResult<DeleteObjectsOutput, DeleteObjectsError>;

    /// See [GetBucketLocation](https://docs.aws.amazon.com/AmazonS3/latest/API/API_GetBucketLocation.html)
    async fn get_bucket_location(
        &self,
        input: GetBucketLocationRequest,
    ) -> S3StorageResult<GetBucketLocationOutput, GetBucketLocationError>;

    /// See [GetObject](https://docs.aws.amazon.com/AmazonS3/latest/API/API_GetObject.html)
    async fn get_object(
        &self,
        input: GetObjectRequest,
    ) -> S3StorageResult<GetObjectOutput, GetObjectError>;

    /// See [HeadBucket](https://docs.aws.amazon.com/AmazonS3/latest/API/API_HeadBucket.html)
    async fn head_bucket(
        &self,
        input: HeadBucketRequest,
    ) -> S3StorageResult<HeadBucketOutput, HeadBucketError>;

    /// See [HeadObject](https://docs.aws.amazon.com/AmazonS3/latest/API/API_HeadObject.html)
    async fn head_object(
        &self,
        input: HeadObjectRequest,
    ) -> S3StorageResult<HeadObjectOutput, HeadObjectError>;

    /// See [ListBuckets](https://docs.aws.amazon.com/AmazonS3/latest/API/API_ListBuckets.html)
    async fn list_buckets(
        &self,
        input: ListBucketsRequest,
    ) -> S3StorageResult<ListBucketsOutput, ListBucketsError>;

    /// See [ListObjects](https://docs.aws.amazon.com/AmazonS3/latest/API/API_ListObjects.html)
    async fn list_objects(
        &self,
        input: ListObjectsRequest,
    ) -> S3StorageResult<ListObjectsOutput, ListObjectsError>;

    /// See [ListObjectsV2](https://docs.aws.amazon.com/AmazonS3/latest/API/API_ListObjectsV2.html)
    async fn list_objects_v2(
        &self,
        input: ListObjectsV2Request,
    ) -> S3StorageResult<ListObjectsV2Output, ListObjectsV2Error>;

    /// See [PutObject](https://docs.aws.amazon.com/AmazonS3/latest/API/API_PutObject.html)
    async fn put_object(
        &self,
        input: PutObjectRequest,
    ) -> S3StorageResult<PutObjectOutput, PutObjectError>;

    /// See [UploadPart](https://docs.aws.amazon.com/AmazonS3/latest/API/API_UploadPart.html)
    async fn upload_part(
        &self,
        input: UploadPartRequest,
    ) -> S3StorageResult<UploadPartOutput, UploadPartError>;
}
