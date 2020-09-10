//! S3 operations

use crate::{
    header::names::*,
    output::{wrap_output, XmlErrorResponse},
    utils::{time, Apply, ResponseExt, XmlWriterExt},
    Response, S3ErrorCode, S3Output, S3Result,
};

use hyper::{header::*, Body, StatusCode};

mod copy_object;
mod create_bucket;
mod delete_bucket;
mod delete_object;
mod delete_objects;
mod get_bucket_location;
mod get_object;
mod head_bucket;
mod head_object;
mod list_buckets;
mod list_objects;
mod list_objects_v2;
mod put_object;
