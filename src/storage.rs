//! Trait representing the capabilities of the Amazon S3 API at server side

use crate::dto::{
    CopyObjectError, CopyObjectOutput, CopyObjectRequest, CreateBucketError, CreateBucketOutput,
    CreateBucketRequest, DeleteBucketError, DeleteBucketOutput, DeleteBucketRequest,
    DeleteObjectError, DeleteObjectOutput, DeleteObjectRequest, DeleteObjectsError,
    DeleteObjectsOutput, DeleteObjectsRequest, GetBucketLocationError, GetBucketLocationOutput,
    GetBucketLocationRequest, GetObjectError, GetObjectOutput, GetObjectRequest, HeadBucketError,
    HeadBucketOutput, HeadBucketRequest, HeadObjectError, HeadObjectOutput, HeadObjectRequest,
    ListBucketsError, ListBucketsOutput, ListBucketsRequest, ListObjectsError, ListObjectsOutput,
    ListObjectsRequest, ListObjectsV2Error, ListObjectsV2Output, ListObjectsV2Request,
    PutObjectError, PutObjectOutput, PutObjectRequest,
};
use crate::error::S3Result;

use async_trait::async_trait;

/// Trait representing the capabilities of the Amazon S3 API at server side
#[async_trait]
pub trait S3Storage {
    /// [CreateBucket](https://docs.aws.amazon.com/AmazonS3/latest/API/API_CreateBucket.html)
    async fn create_bucket(
        &self,
        input: CreateBucketRequest,
    ) -> S3Result<CreateBucketOutput, CreateBucketError>;

    /// [CopyObject](https://docs.aws.amazon.com/AmazonS3/latest/API/API_CopyObject.html)
    async fn copy_object(
        &self,
        input: CopyObjectRequest,
    ) -> S3Result<CopyObjectOutput, CopyObjectError>;

    /// [DeleteBucket](https://docs.aws.amazon.com/AmazonS3/latest/API/API_DeleteBucket.html)
    async fn delete_bucket(
        &self,
        input: DeleteBucketRequest,
    ) -> S3Result<DeleteBucketOutput, DeleteBucketError>;

    /// [DeleteObject](https://docs.aws.amazon.com/AmazonS3/latest/API/API_DeleteObject.html)
    async fn delete_object(
        &self,
        input: DeleteObjectRequest,
    ) -> S3Result<DeleteObjectOutput, DeleteObjectError>;

    /// [DeleteObject](https://docs.aws.amazon.com/AmazonS3/latest/API/API_DeleteObject.html)
    async fn delete_objects(
        &self,
        input: DeleteObjectsRequest,
    ) -> S3Result<DeleteObjectsOutput, DeleteObjectsError>;

    /// [GetBucketLocation](https://docs.aws.amazon.com/AmazonS3/latest/API/API_GetBucketLocation.html)
    async fn get_bucket_location(
        &self,
        input: GetBucketLocationRequest,
    ) -> S3Result<GetBucketLocationOutput, GetBucketLocationError>;

    /// [GetObject](https://docs.aws.amazon.com/AmazonS3/latest/API/API_GetObject.html)
    async fn get_object(
        &self,
        input: GetObjectRequest,
    ) -> S3Result<GetObjectOutput, GetObjectError>;

    /// [HeadBucket](https://docs.aws.amazon.com/AmazonS3/latest/API/API_HeadBucket.html)
    async fn head_bucket(
        &self,
        input: HeadBucketRequest,
    ) -> S3Result<HeadBucketOutput, HeadBucketError>;

    /// [HeadObject](https://docs.aws.amazon.com/AmazonS3/latest/API/API_HeadObject.html)
    async fn head_object(
        &self,
        input: HeadObjectRequest,
    ) -> S3Result<HeadObjectOutput, HeadObjectError>;

    /// [ListBuckets](https://docs.aws.amazon.com/AmazonS3/latest/API/API_ListBuckets.html)
    async fn list_buckets(
        &self,
        input: ListBucketsRequest,
    ) -> S3Result<ListBucketsOutput, ListBucketsError>;

    /// [ListObjects](https://docs.aws.amazon.com/AmazonS3/latest/API/API_ListObjects.html)
    async fn list_objects(
        &self,
        input: ListObjectsRequest,
    ) -> S3Result<ListObjectsOutput, ListObjectsError>;

    /// [ListObjectsV2](https://docs.aws.amazon.com/AmazonS3/latest/API/API_ListObjectsV2.html)
    async fn list_objects_v2(
        &self,
        input: ListObjectsV2Request,
    ) -> S3Result<ListObjectsV2Output, ListObjectsV2Error>;

    /// [PutObject](https://docs.aws.amazon.com/AmazonS3/latest/API/API_PutObject.html)
    async fn put_object(
        &self,
        input: PutObjectRequest,
    ) -> S3Result<PutObjectOutput, PutObjectError>;
}
