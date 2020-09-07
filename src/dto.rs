//! S3 data transfer objects

pub use rusoto_core::ByteStream;
pub use rusoto_s3::{
    Bucket, CreateBucketError, CreateBucketOutput, CreateBucketRequest, DeleteBucketError,
    DeleteBucketRequest, DeleteObjectError, DeleteObjectOutput, DeleteObjectRequest,
    GetBucketLocationError, GetBucketLocationOutput, GetBucketLocationRequest, GetObjectError,
    GetObjectOutput, GetObjectRequest, HeadBucketError, HeadBucketRequest, HeadObjectError,
    HeadObjectOutput, HeadObjectRequest, ListBucketsError, ListBucketsOutput, ListObjectsError,
    ListObjectsOutput, ListObjectsRequest, ListObjectsV2Error, ListObjectsV2Output,
    ListObjectsV2Request, Object, PutObjectError, PutObjectOutput, PutObjectRequest,
};

/// `DeleteBucketOutput`
#[derive(Debug, Clone, Copy)]
pub struct DeleteBucketOutput;

/// `HeadBucketOutput`
#[derive(Debug, Clone, Copy)]
pub struct HeadBucketOutput;
