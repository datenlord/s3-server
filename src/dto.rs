//! S3 data transfer objects

pub use rusoto_core::ByteStream;
pub use rusoto_s3::{
    Bucket, CopyObjectError, CopyObjectOutput, CopyObjectRequest, CopyObjectResult,
    CreateBucketError, CreateBucketOutput, CreateBucketRequest, Delete, DeleteBucketError,
    DeleteBucketRequest, DeleteObjectError, DeleteObjectOutput, DeleteObjectRequest,
    DeleteObjectsError, DeleteObjectsOutput, DeleteObjectsRequest, DeletedObject,
    GetBucketLocationError, GetBucketLocationOutput, GetBucketLocationRequest, GetObjectError,
    GetObjectOutput, GetObjectRequest, HeadBucketError, HeadBucketRequest, HeadObjectError,
    HeadObjectOutput, HeadObjectRequest, ListBucketsError, ListBucketsOutput, ListObjectsError,
    ListObjectsOutput, ListObjectsRequest, ListObjectsV2Error, ListObjectsV2Output,
    ListObjectsV2Request, Object, ObjectIdentifier, PutObjectError, PutObjectOutput,
    PutObjectRequest,
};

/// `DeleteBucketOutput`
#[derive(Debug, Clone, Copy)]
pub struct DeleteBucketOutput;

/// `HeadBucketOutput`
#[derive(Debug, Clone, Copy)]
pub struct HeadBucketOutput;

/// `HeadBucketOutput`
#[derive(Debug, Clone, Copy)]
pub struct ListBucketsRequest;

pub(crate) mod xml {
    //! Xml repr

    use serde::Deserialize;

    /// Object Identifier is unique value to identify objects.
    #[derive(Debug, Deserialize)]
    pub struct ObjectIdentifier {
        /// Key name of the object to delete.
        #[serde(rename = "Key")]
        pub key: String,
        /// VersionId for the specific version of the object to delete.
        #[serde(rename = "VersionId")]
        pub version_id: Option<String>,
    }

    /// Container for the objects to delete.
    #[derive(Debug, Deserialize)]
    pub struct Delete {
        /// The objects to delete.
        #[serde(rename = "Object")]
        pub objects: Vec<ObjectIdentifier>,
        /// Element to enable quiet mode for the request. When you add this element, you must set its value to true.
        #[serde(rename = "Quiet")]
        pub quiet: Option<bool>,
    }

    impl From<ObjectIdentifier> for super::ObjectIdentifier {
        fn from(ObjectIdentifier { key, version_id }: ObjectIdentifier) -> Self {
            Self { key, version_id }
        }
    }

    impl From<Delete> for super::Delete {
        fn from(delete: Delete) -> Self {
            Self {
                quiet: delete.quiet,
                objects: delete.objects.into_iter().map(Into::into).collect(),
            }
        }
    }
}
