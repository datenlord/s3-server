//! Trait representing the capabilities of the Amazon S3 API at server side

use crate::{
    dto::{
        CreateBucketError, CreateBucketOutput, CreateBucketRequest, DeleteBucketError,
        DeleteBucketOutput, DeleteBucketRequest, DeleteObjectError, DeleteObjectOutput,
        DeleteObjectRequest, GetObjectError, GetObjectOutput, GetObjectRequest, HeadBucketError,
        HeadBucketOutput, HeadBucketRequest, ListBucketsError, ListBucketsOutput, PutObjectError,
        PutObjectOutput, PutObjectRequest,
    },
    error::S3Result,
};

use async_trait::async_trait;

/// Trait representing the capabilities of the Amazon S3 API at server side
#[async_trait]
pub trait S3Storage {
    /// [GetObject](https://docs.aws.amazon.com/AmazonS3/latest/API/API_GetObject.html)
    async fn get_object(
        &self,
        input: GetObjectRequest,
    ) -> S3Result<GetObjectOutput, GetObjectError>;

    /// [PutObject](https://docs.aws.amazon.com/AmazonS3/latest/API/API_PutObject.html)
    async fn put_object(
        &self,
        input: PutObjectRequest,
    ) -> S3Result<PutObjectOutput, PutObjectError>;

    /// [DeleteObject](https://docs.aws.amazon.com/AmazonS3/latest/API/API_DeleteObject.html)
    async fn delete_object(
        &self,
        input: DeleteObjectRequest,
    ) -> S3Result<DeleteObjectOutput, DeleteObjectError>;

    /// [CreateBucket](https://docs.aws.amazon.com/AmazonS3/latest/API/API_CreateBucket.html)
    async fn create_bucket(
        &self,
        input: CreateBucketRequest,
    ) -> S3Result<CreateBucketOutput, CreateBucketError>;

    /// [DeleteBucket](https://docs.aws.amazon.com/AmazonS3/latest/API/API_DeleteBucket.html)
    async fn delete_bucket(
        &self,
        input: DeleteBucketRequest,
    ) -> S3Result<DeleteBucketOutput, DeleteBucketError>;

    /// [HeadBucket](https://docs.aws.amazon.com/AmazonS3/latest/API/API_HeadBucket.html)
    async fn head_bucket(
        &self,
        input: HeadBucketRequest,
    ) -> S3Result<HeadBucketOutput, HeadBucketError>;

    /// [ListBuckets](https://docs.aws.amazon.com/AmazonS3/latest/API/API_ListBuckets.html)
    async fn list_buckets(&self) -> S3Result<ListBucketsOutput, ListBucketsError>;
}
