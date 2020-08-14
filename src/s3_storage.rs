use crate::error::S3Result;

use async_trait::async_trait;

use rusoto_s3::{
    CreateBucketError, CreateBucketOutput, CreateBucketRequest, DeleteBucketError,
    DeleteBucketRequest, DeleteObjectError, DeleteObjectOutput, DeleteObjectRequest,
    GetObjectError, GetObjectOutput, GetObjectRequest, HeadBucketError, HeadBucketRequest,
    ListBucketsError, ListBucketsOutput, PutObjectError, PutObjectOutput, PutObjectRequest,
};

#[async_trait]
pub trait S3Storage {
    async fn get_object(
        &self,
        input: GetObjectRequest,
    ) -> S3Result<GetObjectOutput, GetObjectError>;

    async fn put_object(
        &self,
        input: PutObjectRequest,
    ) -> S3Result<PutObjectOutput, PutObjectError>;

    async fn delete_object(
        &self,
        input: DeleteObjectRequest,
    ) -> S3Result<DeleteObjectOutput, DeleteObjectError>;

    async fn create_bucket(
        &self,
        input: CreateBucketRequest,
    ) -> S3Result<CreateBucketOutput, CreateBucketError>;
    async fn delete_bucket(&self, input: DeleteBucketRequest) -> S3Result<(), DeleteBucketError>;

    async fn head_bucket(&self, input: HeadBucketRequest) -> S3Result<(), HeadBucketError>;

    async fn list_buckets(&self) -> S3Result<ListBucketsOutput, ListBucketsError>;
}
