//! [`DeleteObjects`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_DeleteObjects.html)

use super::*;
use crate::dto::{
    Delete, DeleteObjectsError, DeleteObjectsOutput, DeleteObjectsRequest, ObjectIdentifier,
};

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

/// extract operation request
pub async fn extract(
    req: &Request,
    body: Body,
    bucket: &str,
) -> Result<DeleteObjectsRequest, BoxStdError> {
    let delete: self::xml::Delete = deserialize_xml_body(body).await?;

    let mut input: DeleteObjectsRequest = DeleteObjectsRequest {
        delete: delete.into(),
        bucket: bucket.into(),
        ..DeleteObjectsRequest::default()
    };

    assign_opt!(from req to input headers [
        &*X_AMZ_MFA => mfa,
        &*X_AMZ_REQUEST_PAYER => request_payer,
        &*X_AMZ_BYPASS_GOVERNANCE_RETENTION => bypass_governance_retention,
    ]);

    Ok(input)
}

impl S3Output for DeleteObjectsOutput {
    fn try_into_response(self) -> S3Result<Response> {
        wrap_output(|res| {
            res.set_opt_header(|| X_AMZ_REQUEST_CHARGED.clone(), self.request_charged)?;

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

impl S3Output for DeleteObjectsError {
    fn try_into_response(self) -> S3Result<Response> {
        match self {}
    }
}
