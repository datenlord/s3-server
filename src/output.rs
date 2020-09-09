//! Types which can be converted into a response

#![allow(clippy::wildcard_imports)] // for `use super::*`

use super::{
    error::{S3Error, S3Result},
    error_code::S3ErrorCode,
};

use crate::{
    utils::{time, Apply, ResponseExt, XmlWriterExt},
    BoxStdError, Response,
};

use hyper::{header, Body, StatusCode};
use xml::{
    common::XmlVersion,
    writer::{EventWriter, XmlEvent},
};

/// Types which can be converted into a response
pub trait S3Output {
    /// Try to convert into a response
    ///
    /// # Errors
    /// Returns an `Err` if the output can not be converted into a response
    fn try_into_response(self) -> S3Result<Response>;
}

impl<T, E> S3Output for S3Result<T, E>
where
    T: S3Output,
    E: S3Output,
{
    fn try_into_response(self) -> S3Result<Response> {
        match self {
            Ok(output) => output.try_into_response(),
            Err(err) => match err {
                S3Error::Operation(e) => e.try_into_response(),
                S3Error::InvalidRequest(e) => Err(<S3Error>::InvalidRequest(e)),
                S3Error::InvalidOutput(e) => Err(<S3Error>::InvalidOutput(e)),
                S3Error::Storage(e) => Err(<S3Error>::Storage(e)),
                S3Error::NotSupported => Err(S3Error::NotSupported),
            },
        }
    }
}

/// helper function for error converting
fn wrap_output(f: impl FnOnce(&mut Response) -> Result<(), BoxStdError>) -> S3Result<Response> {
    let mut res = Response::new(Body::empty());
    match f(&mut res) {
        Ok(()) => Ok(res),
        Err(e) => Err(<S3Error>::InvalidOutput(e)),
    }
}

/// set xml body
fn set_xml_body<F>(res: &mut Response, cap: usize, f: F) -> Result<(), BoxStdError>
where
    F: FnOnce(&mut EventWriter<&mut Vec<u8>>) -> Result<(), xml::writer::Error>,
{
    let mut body = Vec::with_capacity(cap);
    {
        let mut w = EventWriter::new(&mut body);
        w.write(XmlEvent::StartDocument {
            version: XmlVersion::Version10,
            encoding: Some("UTF-8"),
            standalone: None,
        })?;

        f(&mut w)?;
    }

    *res.body_mut() = Body::from(body);
    res.set_mime(&mime::TEXT_XML)?;
    Ok(())
}

/// Type representing an error response
#[derive(Debug)]
struct XmlErrorResponse {
    /// code
    code: S3ErrorCode,
    /// message
    message: Option<String>,
    /// resource
    resource: Option<String>,
    /// request_id
    request_id: Option<String>,
}

impl XmlErrorResponse {
    /// Constructs a `XmlErrorResponse`
    const fn from_code_msg(code: S3ErrorCode, message: String) -> Self {
        Self {
            code,
            message: Some(message),
            resource: None,
            request_id: None,
        }
    }
}

impl S3Output for XmlErrorResponse {
    fn try_into_response(self) -> S3Result<Response> {
        wrap_output(|res| {
            let status = self
                .code
                .as_status_code()
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

            *res.status_mut() = status;

            set_xml_body(res, 64, |w| {
                w.stack("Error", |w| {
                    w.element("Code", self.code.as_static_str())?;
                    w.opt_element("Message", self.message)?;
                    w.opt_element("Resource", self.resource)?;
                    w.opt_element("RequestId", self.request_id)?;
                    Ok(())
                })
            })?;

            Ok(())
        })
    }
}

mod copy_object {
    //! [`CopyObject`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_CopyObject.html)

    use super::*;
    use crate::dto::{CopyObjectError, CopyObjectOutput};
    use crate::header::names::*;

    impl S3Output for CopyObjectOutput {
        fn try_into_response(self) -> S3Result<Response> {
            wrap_output(|res| {
                res.set_opt_header(X_AMZ_EXPIRATION.clone(), self.expiration)?;
                res.set_opt_header(
                    X_AMZ_COPY_SOURCE_VERSION_ID.clone(),
                    self.copy_source_version_id,
                )?;
                res.set_opt_header(X_AMZ_VERSION_ID.clone(), self.version_id)?;
                res.set_opt_header(
                    X_AMZ_SERVER_SIDE_ENCRYPTION.clone(),
                    self.server_side_encryption,
                )?;
                res.set_opt_header(
                    X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM.clone(),
                    self.sse_customer_algorithm,
                )?;
                res.set_opt_header(
                    X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5.clone(),
                    self.sse_customer_key_md5,
                )?;
                res.set_opt_header(
                    X_AMZ_SERVER_SIDE_ENCRYPTION_AWS_KMS_KEY_ID.clone(),
                    self.ssekms_key_id,
                )?;
                res.set_opt_header(
                    X_AMZ_SERVER_SIDE_ENCRYPTION_CONTEXT.clone(),
                    self.ssekms_encryption_context,
                )?;
                res.set_opt_header(X_AMZ_REQUEST_CHARGED.clone(), self.request_charged)?;

                let copy_object_result = self.copy_object_result;

                set_xml_body(res, 64, |w| {
                    w.opt_stack("CopyObjectResult", copy_object_result, |w, result| {
                        w.opt_element("ETag", result.e_tag)?;
                        w.opt_element("LastModified", result.last_modified)
                    })
                })?;

                Ok(())
            })
        }
    }

    impl S3Output for CopyObjectError {
        fn try_into_response(self) -> S3Result<Response> {
            match self {
                Self::ObjectNotInActiveTierError(msg) => {
                    XmlErrorResponse::from_code_msg(S3ErrorCode::ObjectNotInActiveTierError, msg)
                }
            }
            .try_into_response()
        }
    }
}

mod create_bucket {
    //! [`CreateBucket`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_CreateBucket.html)

    use super::*;
    use crate::dto::{CreateBucketError, CreateBucketOutput};

    impl S3Output for CreateBucketOutput {
        fn try_into_response(self) -> S3Result<Response> {
            wrap_output(|res| {
                res.set_opt_header(header::LOCATION, self.location)?;
                Ok(())
            })
        }
    }

    impl S3Output for CreateBucketError {
        fn try_into_response(self) -> S3Result<Response> {
            let resp = match self {
                Self::BucketAlreadyExists(msg) => {
                    XmlErrorResponse::from_code_msg(S3ErrorCode::BucketAlreadyExists, msg)
                }
                Self::BucketAlreadyOwnedByYou(msg) => {
                    XmlErrorResponse::from_code_msg(S3ErrorCode::BucketAlreadyOwnedByYou, msg)
                }
            };
            resp.try_into_response()
        }
    }
}

mod delete_bucket {
    //! [`DeleteBucket`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_DeleteBucket.html)

    use super::*;
    use crate::dto::{DeleteBucketError, DeleteBucketOutput};
    use crate::utils::Apply;

    impl S3Output for DeleteBucketOutput {
        fn try_into_response(self) -> S3Result<Response> {
            Response::new_with_status(Body::empty(), StatusCode::NO_CONTENT).apply(Ok)
        }
    }

    impl S3Output for DeleteBucketError {
        fn try_into_response(self) -> S3Result<Response> {
            match self {}
        }
    }
}

mod delete_object {
    //! [`DeleteObject`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_DeleteObject.html)

    use super::*;
    use crate::dto::{DeleteObjectError, DeleteObjectOutput};

    impl S3Output for DeleteObjectOutput {
        fn try_into_response(self) -> S3Result<Response> {
            let res = Response::new(Body::empty());
            // TODO: handle other fields
            Ok(res)
        }
    }

    impl S3Output for DeleteObjectError {
        fn try_into_response(self) -> S3Result<Response> {
            match self {}
        }
    }
}

mod delete_objects {
    //! [`DeleteObjects`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_DeleteObjects.html)

    use super::*;
    use crate::dto::{DeleteObjectsError, DeleteObjectsOutput};
    use crate::header::names::X_AMZ_REQUEST_CHARGED;

    impl S3Output for DeleteObjectsOutput {
        fn try_into_response(self) -> S3Result<Response> {
            wrap_output(|res| {
                res.set_opt_header(X_AMZ_REQUEST_CHARGED.clone(), self.request_charged)?;

                let deleted = self.deleted;
                let errors = self.errors;

                set_xml_body(res, 4096, |w| {
                    w.stack("DeleteResult", |w| {
                        if let Some(deleted) = deleted {
                            w.iter_element(deleted.into_iter(), |w, deleted_object| {
                                w.stack("Deleted", |w| {
                                    w.opt_element(
                                        "DeleteMarker",
                                        deleted_object.delete_marker.map(|b| b.to_string()),
                                    )?;
                                    w.opt_element(
                                        "DeleteMarkerVersionId",
                                        deleted_object.delete_marker_version_id,
                                    )?;
                                    w.opt_element("Key", deleted_object.key)?;
                                    w.opt_element("VersionId", deleted_object.version_id)?;
                                    Ok(())
                                })
                            })?;
                        }
                        if let Some(errors) = errors {
                            w.iter_element(errors.into_iter(), |w, error| {
                                w.stack("Error", |w| {
                                    w.opt_element("Code", error.code)?;
                                    w.opt_element("Key", error.key)?;
                                    w.opt_element("Message", error.message)?;
                                    w.opt_element("VersionId", error.version_id)?;
                                    Ok(())
                                })
                            })?;
                        }
                        Ok(())
                    })
                })?;

                Ok(())
            })
        }
    }

    impl S3Output for DeleteObjectsError {
        fn try_into_response(self) -> S3Result<Response> {
            match self {}
        }
    }
}

mod get_object {
    //! [`GetObject`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_GetObject.html)

    use super::*;
    use crate::dto::{GetObjectError, GetObjectOutput};

    impl S3Output for GetObjectOutput {
        fn try_into_response(self) -> S3Result<Response> {
            wrap_output(|res| {
                if let Some(body) = self.body {
                    *res.body_mut() = Body::wrap_stream(body);
                }
                res.set_opt_header(
                    header::CONTENT_LENGTH,
                    self.content_length.map(|l| format!("{}", l)),
                )?;
                res.set_opt_header(header::CONTENT_TYPE, self.content_type)?;

                res.set_opt_header(
                    header::LAST_MODIFIED,
                    time::map_opt_rfc3339_to_last_modified(self.last_modified)?,
                )?;
                // TODO: handle other fields
                Ok(())
            })
        }
    }

    impl S3Output for GetObjectError {
        fn try_into_response(self) -> S3Result<Response> {
            let resp = match self {
                Self::NoSuchKey(msg) => {
                    XmlErrorResponse::from_code_msg(S3ErrorCode::NoSuchKey, msg)
                }
            };
            resp.try_into_response()
        }
    }
}

mod get_bucket_location {
    //! [`GetBucketLocation`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_GetBucketLocation.html)

    use super::*;
    use crate::dto::{GetBucketLocationError, GetBucketLocationOutput};

    impl S3Output for GetBucketLocationOutput {
        fn try_into_response(self) -> S3Result<Response> {
            wrap_output(|res| {
                set_xml_body(res, 4096, |w| {
                    w.element(
                        "LocationConstraint",
                        self.location_constraint.as_deref().unwrap_or(""),
                    )
                })
            })
        }
    }

    impl S3Output for GetBucketLocationError {
        fn try_into_response(self) -> S3Result<Response> {
            match self {}
        }
    }
}

mod head_bucket {
    //! [`HeadBucket`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_HeadBucket.html)

    use super::*;
    use crate::dto::{HeadBucketError, HeadBucketOutput};

    impl S3Output for HeadBucketOutput {
        fn try_into_response(self) -> S3Result<Response> {
            Response::new(Body::empty()).apply(Ok)
        }
    }

    impl S3Output for HeadBucketError {
        fn try_into_response(self) -> S3Result<Response> {
            let resp = match self {
                Self::NoSuchBucket(msg) => {
                    XmlErrorResponse::from_code_msg(S3ErrorCode::NoSuchBucket, msg)
                }
            };
            resp.try_into_response()
        }
    }
}

mod head_object {
    //! [`HeadObject`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_HeadObject.html)

    use super::*;
    use crate::dto::{HeadObjectError, HeadObjectOutput};

    impl S3Output for HeadObjectOutput {
        fn try_into_response(self) -> S3Result<Response> {
            wrap_output(|res| {
                res.set_opt_header(header::CONTENT_TYPE, self.content_type)?;
                res.set_opt_header(
                    header::CONTENT_LENGTH,
                    self.content_length.map(|l| l.to_string()),
                )?;
                res.set_opt_header(
                    header::LAST_MODIFIED,
                    time::map_opt_rfc3339_to_last_modified(self.last_modified)?,
                )?;
                res.set_opt_header(header::ETAG, self.e_tag)?;
                res.set_opt_header(header::EXPIRES, self.expires)?;
                // TODO: handle other fields
                Ok(())
            })
        }
    }

    impl S3Output for HeadObjectError {
        fn try_into_response(self) -> S3Result<Response> {
            let resp = match self {
                Self::NoSuchKey(msg) => {
                    XmlErrorResponse::from_code_msg(S3ErrorCode::NoSuchKey, msg)
                }
            };
            resp.try_into_response()
        }
    }
}

mod list_buckets {
    //! [`ListBuckets`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_ListBuckets.html)

    use super::*;
    use crate::dto::{ListBucketsError, ListBucketsOutput};

    impl S3Output for ListBucketsOutput {
        fn try_into_response(self) -> S3Result<Response> {
            wrap_output(|res| {
                set_xml_body(res, 4096, |w| {
                    w.stack("ListBucketsOutput", |w| {
                        w.opt_stack("Buckets", self.buckets, |w, buckets| {
                            for bucket in buckets {
                                w.stack("Bucket", |w| {
                                    w.opt_element("CreationDate", bucket.creation_date)?;
                                    w.opt_element("Name", bucket.name)
                                })?;
                            }
                            Ok(())
                        })?;

                        w.opt_stack("Owner", self.owner, |w, owner| {
                            w.opt_element("DisplayName", owner.display_name)?;
                            w.opt_element("ID", owner.id)
                        })?;
                        Ok(())
                    })
                })
            })
        }
    }

    impl S3Output for ListBucketsError {
        fn try_into_response(self) -> S3Result<Response> {
            match self {}
        }
    }
}

mod list_objects {
    //! [`ListObjects`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_ListObjects.html)

    use super::*;
    use crate::dto::{ListObjectsError, ListObjectsOutput};

    impl S3Output for ListObjectsError {
        fn try_into_response(self) -> S3Result<Response> {
            let resp = match self {
                Self::NoSuchBucket(msg) => {
                    XmlErrorResponse::from_code_msg(S3ErrorCode::NoSuchBucket, msg)
                }
            };
            resp.try_into_response()
        }
    }

    impl S3Output for ListObjectsOutput {
        fn try_into_response(self) -> S3Result<Response> {
            wrap_output(|res| {
                set_xml_body(res, 4096, |w| {
                    w.stack("ListBucketResult", |w| {
                        w.opt_element("IsTruncated", self.is_truncated.map(|b| b.to_string()))?;
                        w.opt_element("Marker", self.marker)?;
                        w.opt_element("NextMarker", self.next_marker)?;
                        if let Some(contents) = self.contents {
                            for content in contents {
                                w.stack("Contents", |w| {
                                    w.opt_element("Key", content.key)?;
                                    w.opt_element("LastModified", content.last_modified)?;
                                    w.opt_element("ETag", content.e_tag)?;
                                    w.opt_element("Size", content.size.map(|s| s.to_string()))?;
                                    w.opt_element("StorageClass", content.storage_class)?;
                                    w.opt_stack("Owner", content.owner, |w, owner| {
                                        w.opt_element("ID", owner.id)?;
                                        w.opt_element("DisplayName", owner.display_name)?;
                                        Ok(())
                                    })
                                })?;
                            }
                        }
                        w.opt_element("Name", self.name)?;
                        w.opt_element("Prefix", self.prefix)?;
                        w.opt_element("Delimiter", self.delimiter)?;
                        w.opt_element("MaxKeys", self.max_keys.map(|k| k.to_string()))?;
                        w.opt_stack("CommonPrefixes", self.common_prefixes, |w, prefixes| {
                            w.iter_element(prefixes.into_iter(), |w, common_prefix| {
                                w.opt_element("Prefix", common_prefix.prefix)
                            })
                        })?;
                        w.opt_element("EncodingType", self.encoding_type)?;
                        Ok(())
                    })
                })
            })
        }
    }
}

mod list_objects_v2 {
    //! [`ListObjectsV2`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_ListObjectsV2.html)

    use super::*;
    use crate::dto::{ListObjectsV2Error, ListObjectsV2Output};

    impl S3Output for ListObjectsV2Error {
        fn try_into_response(self) -> S3Result<Response> {
            let resp = match self {
                Self::NoSuchBucket(msg) => {
                    XmlErrorResponse::from_code_msg(S3ErrorCode::NoSuchBucket, msg)
                }
            };
            resp.try_into_response()
        }
    }

    impl S3Output for ListObjectsV2Output {
        fn try_into_response(self) -> S3Result<Response> {
            wrap_output(|res| {
                set_xml_body(res, 4096, |w| {
                    w.stack("ListBucketResult", |w| {
                        w.opt_element("IsTruncated", self.is_truncated.map(|b| b.to_string()))?;
                        if let Some(contents) = self.contents {
                            for content in contents {
                                w.stack("Contents", |w| {
                                    w.opt_element("Key", content.key)?;
                                    w.opt_element("LastModified", content.last_modified)?;
                                    w.opt_element("ETag", content.e_tag)?;
                                    w.opt_element("Size", content.size.map(|s| s.to_string()))?;
                                    w.opt_element("StorageClass", content.storage_class)?;
                                    w.opt_stack("Owner", content.owner, |w, owner| {
                                        w.opt_element("ID", owner.id)?;
                                        w.opt_element("DisplayName", owner.display_name)?;
                                        Ok(())
                                    })
                                })?;
                            }
                        }
                        w.opt_element("Name", self.name)?;
                        w.opt_element("Prefix", self.prefix)?;
                        w.opt_element("Delimiter", self.delimiter)?;
                        w.opt_element("MaxKeys", self.max_keys.map(|k| k.to_string()))?;
                        w.opt_stack("CommonPrefixes", self.common_prefixes, |w, prefixes| {
                            w.iter_element(prefixes.into_iter(), |w, common_prefix| {
                                w.opt_element("Prefix", common_prefix.prefix)
                            })
                        })?;
                        w.opt_element("EncodingType", self.encoding_type)?;
                        w.opt_element("KeyCount", self.max_keys.map(|k| k.to_string()))?;
                        w.opt_element("ContinuationToken", self.continuation_token)?;
                        w.opt_element("NextContinuationToken", self.next_continuation_token)?;
                        w.opt_element("StartAfter", self.start_after)?;
                        Ok(())
                    })
                })
            })
        }
    }
}

mod put_object {
    //! [`PutObject`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_PutObject.html)

    use super::*;
    use crate::dto::{PutObjectError, PutObjectOutput};

    impl S3Output for PutObjectOutput {
        fn try_into_response(self) -> S3Result<Response> {
            let res = Response::new(Body::empty());
            // TODO: handle other fields
            Ok(res)
        }
    }

    impl S3Output for PutObjectError {
        fn try_into_response(self) -> S3Result<Response> {
            match self {}
        }
    }
}
