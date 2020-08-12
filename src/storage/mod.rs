use crate::error::S3Error;

use async_trait::async_trait;

use rusoto_s3::{
    CreateBucketError, CreateBucketOutput, CreateBucketRequest, DeleteBucketError,
    DeleteBucketRequest, DeleteObjectError, DeleteObjectOutput, DeleteObjectRequest,
    GetObjectError, GetObjectOutput, GetObjectRequest, HeadBucketError, HeadBucketRequest,
    ListBucketsError, ListBucketsOutput, PutObjectError, PutObjectOutput, PutObjectRequest,
};

type HeadBucketOutput = ();
type DeleteBucketOutput = ();
type ListBucketRequest = ();

#[async_trait]
pub trait S3Storage {
    async fn get_object(
        &self,
        input: GetObjectRequest,
    ) -> Result<GetObjectOutput, S3Error<GetObjectError>>;

    async fn put_object(
        &self,
        input: PutObjectRequest,
    ) -> Result<PutObjectOutput, S3Error<PutObjectError>>;

    async fn delete_object(
        &self,
        input: DeleteObjectRequest,
    ) -> Result<DeleteObjectOutput, S3Error<DeleteObjectError>>;

    async fn create_bucket(
        &self,
        input: CreateBucketRequest,
    ) -> Result<CreateBucketOutput, S3Error<CreateBucketError>>;
    async fn delete_bucket(
        &self,
        input: DeleteBucketRequest,
    ) -> Result<DeleteBucketOutput, S3Error<DeleteBucketError>>;

    async fn head_bucket(
        &self,
        input: HeadBucketRequest,
    ) -> Result<HeadBucketOutput, S3Error<HeadBucketError>>;

    async fn list_buckets(
        &self,
        input: ListBucketRequest,
    ) -> Result<ListBucketsOutput, S3Error<ListBucketsError>>;
}
