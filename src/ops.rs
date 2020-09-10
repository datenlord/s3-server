//! S3 operations

use crate::{
    header::names::*,
    output::{wrap_output, XmlErrorResponse},
    query::GetQuery,
    utils::{deserialize_xml_body, time, Apply, RequestExt, ResponseExt, XmlWriterExt},
    BoxStdError, Request, Response, S3ErrorCode, S3Output, S3Result,
};

use hyper::{header::*, Body, StatusCode};

pub mod copy_object;
pub mod create_bucket;
pub mod delete_bucket;
pub mod delete_object;
pub mod delete_objects;
pub mod get_bucket_location;
pub mod get_object;
pub mod head_bucket;
pub mod head_object;
pub mod list_buckets;
pub mod list_objects;
pub mod list_objects_v2;
pub mod put_object;
