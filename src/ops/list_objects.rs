//! [`ListObjects`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_ListObjects.html)

use super::*;
use crate::dto::{ListObjectsError, ListObjectsOutput, ListObjectsRequest};

/// extract operation request
pub fn extract(
    req: &Request,
    query: Option<GetQuery>,
    bucket: &str,
) -> Result<ListObjectsRequest, BoxStdError> {
    let mut input = ListObjectsRequest {
        bucket: bucket.into(),
        ..ListObjectsRequest::default()
    };

    if let Some(query) = query {
        assign_opt!(from query to input: fields [
            delimiter,
            encoding_type,
            marker,
            max_keys,
            prefix,
        ]);
    }

    assign_opt!(from req to input: headers [
        (&*X_AMZ_REQUEST_PAYER, request_payer),
    ]);

    Ok(input)
}

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
            res.set_xml_body(4096, |w| {
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
