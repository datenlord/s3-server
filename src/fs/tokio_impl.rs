use crate::error::S3Error;
use crate::s3_storage::S3Storage;

use async_trait::async_trait;
use std::path::PathBuf;

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

#[async_trait]
impl S3Storage for FileSystem {
    async fn get_object(
        &self,
        input: GetObjectRequest,
    ) -> Result<GetObjectOutput, S3Error<GetObjectError>> {
        todo!()
    }
    async fn put_object(
        &self,
        input: PutObjectRequest,
    ) -> Result<PutObjectOutput, S3Error<PutObjectError>> {
        todo!()
    }
    async fn delete_object(
        &self,
        input: DeleteObjectRequest,
    ) -> Result<DeleteObjectOutput, S3Error<DeleteObjectError>> {
        todo!()
    }
    async fn create_bucket(
        &self,
        input: CreateBucketRequest,
    ) -> Result<CreateBucketOutput, S3Error<CreateBucketError>> {
        todo!()
    }
    async fn delete_bucket(
        &self,
        input: DeleteBucketRequest,
    ) -> Result<(), S3Error<DeleteBucketError>> {
        todo!()
    }
    async fn head_bucket(&self, input: HeadBucketRequest) -> Result<(), S3Error<HeadBucketError>> {
        todo!()
    }
    async fn list_buckets(&self) -> Result<ListBucketsOutput, S3Error<ListBucketsError>> {
        todo!()
    }
}
