//! [`DeleteObjects`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_DeleteObjects.html)

use super::{wrap_internal_error, ReqContext, S3Handler};

use crate::dto::{
    Delete, DeleteObjectsError, DeleteObjectsOutput, DeleteObjectsRequest, ObjectIdentifier,
};
use crate::errors::{S3Error, S3Result};
use crate::headers::{
    X_AMZ_BYPASS_GOVERNANCE_RETENTION, X_AMZ_MFA, X_AMZ_REQUEST_CHARGED, X_AMZ_REQUEST_PAYER,
};
use crate::output::S3Output;
use crate::storage::S3Storage;
use crate::utils::{deserialize_xml_body, ResponseExt, XmlWriterExt};
use crate::{async_trait, Method, Response};

/// `DeleteObject` handler
pub struct Handler;

#[async_trait]
impl S3Handler for Handler {
    fn is_match(&self, ctx: &'_ ReqContext<'_>) -> bool {
        bool_try!(ctx.req.method() == Method::POST);
        bool_try!(ctx.path.is_bucket());
        let qs = bool_try_some!(ctx.query_strings.as_ref());
        qs.get("delete").is_some()
    }

    async fn handle(
        &self,
        ctx: &mut ReqContext<'_>,
        storage: &(dyn S3Storage + Send + Sync),
    ) -> S3Result<Response> {
        let input = extract(ctx).await?;
        let output = storage.delete_objects(input).await;
        output.try_into_response()
    }
}

/// extract operation request
pub async fn extract(ctx: &mut ReqContext<'_>) -> S3Result<DeleteObjectsRequest> {
    let bucket = ctx.unwrap_bucket_path();
    let delete: self::xml::Delete = deserialize_xml_body(ctx.take_body())
        .await
        .map_err(|err| invalid_request!("Invalid xml format", err))?;

    let mut input: DeleteObjectsRequest = DeleteObjectsRequest {
        delete: delete.into(),
        bucket: bucket.into(),
        ..DeleteObjectsRequest::default()
    };

    let h = &ctx.headers;
    h.assign_str(&*X_AMZ_MFA, &mut input.mfa);
    h.assign_str(&*X_AMZ_REQUEST_PAYER, &mut input.request_payer);
    h.assign(
        &*X_AMZ_BYPASS_GOVERNANCE_RETENTION,
        &mut input.bypass_governance_retention,
    )
    .map_err(|err| invalid_request!("Invalid header: x-amz-bypass-governance-retention", err))?;

    Ok(input)
}

impl S3Output for DeleteObjectsOutput {
    fn try_into_response(self) -> S3Result<Response> {
        wrap_internal_error(|res| {
            res.set_optional_header(&*X_AMZ_REQUEST_CHARGED, self.request_charged)?;

            let deleted = self.deleted;
            let errors = self.errors;

            res.set_xml_body(4096, |w| {
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

impl From<DeleteObjectsError> for S3Error {
    fn from(e: DeleteObjectsError) -> Self {
        match e {}
    }
}

mod xml {
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
