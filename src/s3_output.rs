use crate::error::{InvalidOutputError, S3Error, S3Result};

use hyper::{
    header::{self, HeaderValue},
    Body,
};
use rusoto_s3::{
    CreateBucketOutput, DeleteObjectOutput, GetBucketLocationOutput, GetObjectOutput,
    ListBucketsOutput, PutObjectOutput,
};
use std::convert::TryFrom;
use xml::{
    common::XmlVersion,
    writer::{EventWriter, XmlEvent},
};

type Response = hyper::Response<Body>;

pub(super) trait S3Output {
    fn try_into_response(self) -> S3Result<Response>;
}

impl<T: S3Output> S3Output for S3Result<T> {
    fn try_into_response(self) -> S3Result<Response> {
        match self {
            Ok(output) => output.try_into_response(),
            Err(e) => Err(e),
        }
    }
}

fn warp_output(f: impl FnOnce() -> Result<Response, InvalidOutputError>) -> S3Result<Response> {
    match f() {
        Ok(res) => Ok(res),
        Err(e) => Err(<S3Error>::InvalidOutput(e)),
    }
}

impl S3Output for GetObjectOutput {
    fn try_into_response(self) -> S3Result<Response> {
        warp_output(|| {
            let mut res = Response::new(Body::empty());
            if let Some(body) = self.body {
                *res.body_mut() = Body::wrap_stream(body);
            }
            if let Some(content_length) = self.content_length {
                let val = HeaderValue::try_from(format!("{}", content_length))?;
                let _ = res.headers_mut().insert(header::CONTENT_LENGTH, val);
            }
            if let Some(content_type) = self.content_type {
                let val = HeaderValue::try_from(content_type)?;
                let _ = res.headers_mut().insert(header::CONTENT_TYPE, val);
            }
            // TODO: handle other fields
            Ok(res)
        })
    }
}

impl S3Output for CreateBucketOutput {
    fn try_into_response(self) -> S3Result<Response> {
        warp_output(|| {
            let mut res = Response::new(Body::empty());
            if let Some(location) = self.location {
                let val = HeaderValue::try_from(location)?;
                let _ = res.headers_mut().insert(header::LOCATION, val);
            }
            Ok(res)
        })
    }
}

impl S3Output for PutObjectOutput {
    fn try_into_response(self) -> S3Result<Response> {
        dbg!(self);
        let res = Response::new(Body::empty());
        // TODO: handle other fields
        Ok(res)
    }
}

impl S3Output for () {
    fn try_into_response(self) -> S3Result<Response> {
        let res = Response::new(Body::empty());
        Ok(res)
    }
}

impl S3Output for DeleteObjectOutput {
    fn try_into_response(self) -> S3Result<Response> {
        dbg!(self);
        let res = Response::new(Body::empty());
        // TODO: handle other fields
        Ok(res)
    }
}

impl S3Output for ListBucketsOutput {
    fn try_into_response(self) -> S3Result<Response> {
        warp_output(|| {
            let mut body = Vec::with_capacity(4096);
            {
                let mut w = EventWriter::new(&mut body);
                w.write(XmlEvent::StartDocument {
                    version: XmlVersion::Version10,
                    encoding: Some("UTF-8"),
                    standalone: None,
                })?;

                w.write(XmlEvent::start_element("ListBucketsOutput"))?;

                if let Some(buckets) = self.buckets {
                    w.write(XmlEvent::start_element("Buckets"))?;

                    for bucket in buckets {
                        w.write(XmlEvent::start_element("Bucket"))?;
                        if let Some(creation_date) = bucket.creation_date {
                            w.write(XmlEvent::start_element("CreationDate"))?;
                            w.write(XmlEvent::characters(&creation_date))?;
                            w.write(XmlEvent::end_element())?;
                        }

                        if let Some(name) = bucket.name {
                            w.write(XmlEvent::start_element("Name"))?;
                            w.write(XmlEvent::characters(&name))?;
                            w.write(XmlEvent::end_element())?;
                        }
                        w.write(XmlEvent::end_element())?;
                    }

                    w.write(XmlEvent::end_element())?;
                }

                if let Some(owner) = self.owner {
                    w.write(XmlEvent::start_element("Owner"))?;
                    if let Some(display_name) = owner.display_name {
                        w.write(XmlEvent::start_element("DisplayName"))?;
                        w.write(XmlEvent::characters(&display_name))?;
                        w.write(XmlEvent::end_element())?;
                    }
                    if let Some(id) = owner.id {
                        w.write(XmlEvent::start_element("ID"))?;
                        w.write(XmlEvent::characters(&id))?;
                        w.write(XmlEvent::end_element())?;
                    }
                }

                w.write(XmlEvent::end_element())?;
            }

            let mut res = Response::new(Body::from(body));
            let val = HeaderValue::try_from(mime::TEXT_XML.as_ref())?;
            let _ = res.headers_mut().insert(header::CONTENT_TYPE, val);

            // TODO: handle other fields

            Ok(res)
        })
    }
}

impl S3Output for GetBucketLocationOutput {
    fn try_into_response(self) -> S3Result<Response> {
        warp_output(|| {
            let mut body = Vec::with_capacity(64);
            let mut w = EventWriter::new(&mut body);
            w.write(XmlEvent::StartDocument {
                version: XmlVersion::Version10,
                encoding: Some("UTF-8"),
                standalone: None,
            })?;

            w.write(XmlEvent::start_element("LocationConstraint"))?;
            if let Some(location_constraint) = self.location_constraint {
                w.write(XmlEvent::characters(&location_constraint))?;
            }
            w.write(XmlEvent::end_element())?;

            let res = Response::new(Body::from(body));

            // TODO: handle other fields

            Ok(res)
        })
    }
}
