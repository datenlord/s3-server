//! [`ListObjectsV2`](https://docs.aws.amazon.com/AmazonS3/latest/API/API_ListObjectsV2.html)

use super::{wrap_internal_error, ReqContext, S3Handler};

use crate::dto::{ListObjectsV2Error, ListObjectsV2Output, ListObjectsV2Request};
use crate::errors::{S3Error, S3ErrorCode, S3Result};
use crate::headers::X_AMZ_REQUEST_PAYER;
use crate::output::S3Output;
use crate::storage::S3Storage;
use crate::utils::{ResponseExt, XmlWriterExt};
use crate::{async_trait, Method, Response};

/// `ListObjectsV2` handler
pub struct Handler;

#[async_trait]
impl S3Handler for Handler {
    fn is_match(&self, ctx: &'_ ReqContext<'_>) -> bool {
        bool_try!(ctx.req.method() == Method::GET);
        bool_try!(ctx.path.is_bucket());
        let qs = bool_try_some!(ctx.query_strings.as_ref());
        let list_type = bool_try_some!(qs.get("list-type"));
        list_type == "2"
    }

    async fn handle(
        &self,
        ctx: &mut ReqContext<'_>,
        storage: &(dyn S3Storage + Send + Sync),
    ) -> S3Result<Response> {
        let input = extract(ctx)?;
        let output = storage.list_objects_v2(input).await;
        output.try_into_response()
    }
}

/// extract operation request
fn extract(
    ctx: &mut ReqContext<'_>,
    // req: &Request,
    // query: GetQuery,
    // bucket: &str,
) -> S3Result<ListObjectsV2Request> {
    let bucket = ctx.unwrap_bucket_path();

    let mut input = ListObjectsV2Request {
        bucket: bucket.into(),

        ..ListObjectsV2Request::default()
    };

    if let Some(ref q) = ctx.query_strings {
        q.assign_str("continuation-token", &mut input.continuation_token);
        q.assign_str("delimiter", &mut input.delimiter);
        q.assign_str("encoding-type", &mut input.encoding_type);
        q.assign("fetch-owner", &mut input.fetch_owner)
            .map_err(|err| invalid_request!("Invalid query: fetch-owner", err))?;
        q.assign("max-keys", &mut input.max_keys)
            .map_err(|err| invalid_request!("Invalid query: max-keys", err))?;
        q.assign_str("prefix", &mut input.prefix);
        q.assign_str("start-after", &mut input.start_after);
    }

    ctx.headers
        .assign_str(&*X_AMZ_REQUEST_PAYER, &mut input.request_payer);

    Ok(input)
}

impl S3Output for ListObjectsV2Output {
    fn try_into_response(self) -> S3Result<Response> {
        wrap_internal_error(|res| {
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

impl From<ListObjectsV2Error> for S3Error {
    fn from(e: ListObjectsV2Error) -> Self {
        match e {
            ListObjectsV2Error::NoSuchBucket(msg) => Self::new(S3ErrorCode::NoSuchBucket, msg),
        }
    }
}
