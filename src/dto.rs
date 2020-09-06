//! S3 data transfer objects

pub use rusoto_core::ByteStream;
pub use rusoto_s3::{
    Bucket, CreateBucketError, CreateBucketOutput, CreateBucketRequest, DeleteBucketError,
    DeleteBucketRequest, DeleteObjectError, DeleteObjectOutput, DeleteObjectRequest,
    GetBucketLocationError, GetBucketLocationOutput, GetBucketLocationRequest, GetObjectError,
    GetObjectOutput, GetObjectRequest, HeadBucketError, HeadBucketRequest, HeadObjectError,
    HeadObjectOutput, HeadObjectRequest, ListBucketsError, ListBucketsOutput, ListObjectsError,
    ListObjectsOutput, ListObjectsRequest, Object, PutObjectError, PutObjectOutput,
    PutObjectRequest,
};
