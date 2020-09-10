//! [`DeleteObjects`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_DeleteObjects.html)

use super::*;
use crate::dto::{self, DeleteObjectsError, DeleteObjectsOutput, DeleteObjectsRequest};

/// extract operation request
pub async fn extract(
    req: &Request,
    body: Body,
    bucket: &str,
) -> Result<DeleteObjectsRequest, BoxStdError> {
    let delete: dto::xml::Delete = deserialize_xml_body(body).await?;

    let mut input: DeleteObjectsRequest = DeleteObjectsRequest {
        delete: delete.into(),
        bucket: bucket.into(),
        ..dto::DeleteObjectsRequest::default()
    };

    assign_opt!(from req to input: headers [
        (&*X_AMZ_MFA, mfa),
        (&*X_AMZ_REQUEST_PAYER, request_payer),
    ]);

    // TODO: handle "x-amz-bypass-governance-retention"
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
