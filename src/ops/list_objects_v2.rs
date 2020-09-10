//! [`ListObjectsV2`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_ListObjectsV2.html)

use super::*;
use crate::dto::{ListObjectsV2Error, ListObjectsV2Output, ListObjectsV2Request};

/// extract operation request
pub fn extract(
    req: &Request,
    query: GetQuery,
    bucket: &str,
) -> Result<ListObjectsV2Request, BoxStdError> {
    let mut input = ListObjectsV2Request {
        bucket: bucket.into(),
        ..ListObjectsV2Request::default()
    };

    assign_opt!(from query to input fields [
        continuation_token,
        delimiter,
        encoding_type,
        fetch_owner,
        max_keys,
        prefix,
        start_after,
    ]);

    assign_opt!(from req to input headers [
        &*X_AMZ_REQUEST_PAYER => request_payer,
    ]);

    Ok(input)
}

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
            res.set_xml_body(4096, |w| {
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
