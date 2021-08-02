//! cargo test --test basic -- --test-threads=1

#[macro_use]
mod common;

use common::{Request, ResultExt};

use s3_server::headers::X_AMZ_CONTENT_SHA256;
use s3_server::path::S3Path;
use s3_server::storages::fs::FileSystem;
use s3_server::S3Service;

use std::io;
use std::path::{Path, PathBuf};

use anyhow::Result;
use hyper::header::HeaderValue;
use hyper::{Body, Method, StatusCode};
use tracing::{debug_span, error};

use tokio::fs;

fn setup_service() -> Result<(PathBuf, S3Service)> {
    common::setup_tracing();

    let root = common::setup_fs_root(true).unwrap();

    enter_sync!(debug_span!("setup service", root = %root.display()));

    let fs =
        FileSystem::new(&root).inspect_err(|err| error!(%err, "failed to create filesystem"))?;

    let service = S3Service::new(fs);

    Ok((root, service))
}

#[tracing::instrument(
    skip(root),
    fields(root = %root.as_ref().display()),
)]
pub async fn helper_write_object(
    root: impl AsRef<Path>,
    bucket: &str,
    key: &str,
    content: &str,
) -> io::Result<()> {
    let dir_path = common::generate_path(&root, S3Path::Bucket { bucket });
    if !dir_path.exists() {
        fs::create_dir(dir_path).await?;
    }
    let file_path = common::generate_path(root, S3Path::Object { bucket, key });
    fs::write(file_path, content).await
}

mod success {

    use super::*;
    #[tokio::test]
    async fn get_object() {
        let (root, service) = setup_service().unwrap();

        let bucket = "asd";
        let key = "qwe";
        let content = "Hello World!";

        helper_write_object(root, bucket, key, content)
            .await
            .unwrap();

        let mut req = Request::new(Body::empty());
        *req.method_mut() = Method::GET;
        *req.uri_mut() = format!("http://localhost/{}/{}", bucket, key)
            .parse()
            .unwrap();
        req.headers_mut().insert(
            X_AMZ_CONTENT_SHA256.clone(),
            HeaderValue::from_static("UNSIGNED-PAYLOAD"),
        );

        let mut res = service.hyper_call(req).await.unwrap();
        let body = common::recv_body_string(&mut res).await.unwrap();

        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(body, content);
    }

    #[tokio::test]
    async fn put_object() -> Result<()> {
        let (root, service) = setup_service().unwrap();

        let bucket = "asd";
        let key = "qwe";
        let content = "Hello World!";

        let dir_path = common::generate_path(&root, S3Path::Bucket { bucket });
        fs::create_dir(dir_path).await.unwrap();

        let mut req = Request::new(Body::from(content));
        *req.method_mut() = Method::PUT;
        *req.uri_mut() = format!("http://localhost/{}/{}", bucket, key)
            .parse()
            .unwrap();
        req.headers_mut().insert(
            X_AMZ_CONTENT_SHA256.clone(),
            HeaderValue::from_static("UNSIGNED-PAYLOAD"),
        );

        let mut res = service.hyper_call(req).await.unwrap();
        let body = common::recv_body_string(&mut res).await.unwrap();

        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(body, "");

        let file_path = common::generate_path(root, S3Path::Object { bucket, key });
        let file_content = fs::read_to_string(file_path).await.unwrap();

        assert_eq!(file_content, content);

        Ok(())
    }

    #[tokio::test]
    async fn delete_object() -> Result<()> {
        let (root, service) = setup_service().unwrap();

        let bucket = "asd";
        let key = "qwe";
        let content = "Hello World!";

        helper_write_object(&root, bucket, key, content)
            .await
            .unwrap();

        let mut req = Request::new(Body::empty());
        *req.method_mut() = Method::DELETE;
        *req.uri_mut() = format!("http://localhost/{}/{}", bucket, key)
            .parse()
            .unwrap();
        req.headers_mut().insert(
            X_AMZ_CONTENT_SHA256.clone(),
            HeaderValue::from_static("UNSIGNED-PAYLOAD"),
        );

        let mut res = service.hyper_call(req).await.unwrap();
        let body = common::recv_body_string(&mut res).await.unwrap();

        assert_eq!(res.status(), StatusCode::NO_CONTENT);
        assert_eq!(body, "");

        let file_path = common::generate_path(&root, S3Path::Object { bucket, key });
        assert!(!file_path.exists());

        Ok(())
    }

    #[tokio::test]
    async fn create_bucket() -> Result<()> {
        let (root, service) = setup_service().unwrap();

        let bucket = "asd";
        let dir_path = common::generate_path(root, S3Path::Bucket { bucket });

        let mut req = Request::new(Body::empty());
        *req.method_mut() = Method::PUT;
        *req.uri_mut() = format!("http://localhost/{}", bucket).parse().unwrap();
        req.headers_mut().insert(
            X_AMZ_CONTENT_SHA256.clone(),
            HeaderValue::from_static("UNSIGNED-PAYLOAD"),
        );

        let mut res = service.hyper_call(req).await.unwrap();
        let body = common::recv_body_string(&mut res).await.unwrap();

        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(body, "");

        assert!(dir_path.exists());

        Ok(())
    }

    #[tokio::test]
    async fn delete_bucket() -> Result<()> {
        let (root, service) = setup_service().unwrap();

        let bucket = "asd";
        let dir_path = common::generate_path(root, S3Path::Bucket { bucket });
        fs::create_dir(&dir_path).await.unwrap();

        let mut req = Request::new(Body::empty());
        *req.method_mut() = Method::DELETE;
        *req.uri_mut() = format!("http://localhost/{}", bucket).parse().unwrap();
        req.headers_mut().insert(
            X_AMZ_CONTENT_SHA256.clone(),
            HeaderValue::from_static("UNSIGNED-PAYLOAD"),
        );

        let mut res = service.hyper_call(req).await.unwrap();
        let body = common::recv_body_string(&mut res).await.unwrap();

        assert_eq!(res.status(), StatusCode::NO_CONTENT);
        assert_eq!(body, "");

        assert!(!dir_path.exists());

        Ok(())
    }

    #[tokio::test]
    async fn head_bucket() -> Result<()> {
        let (root, service) = setup_service().unwrap();

        let bucket = "asd";
        let dir_path = common::generate_path(root, S3Path::Bucket { bucket });
        fs::create_dir(&dir_path).await.unwrap();

        let mut req = Request::new(Body::empty());
        *req.method_mut() = Method::HEAD;
        *req.uri_mut() = format!("http://localhost/{}", bucket).parse().unwrap();
        req.headers_mut().insert(
            X_AMZ_CONTENT_SHA256.clone(),
            HeaderValue::from_static("UNSIGNED-PAYLOAD"),
        );

        let mut res = service.hyper_call(req).await.unwrap();
        let body = common::recv_body_string(&mut res).await.unwrap();

        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(body, "");

        Ok(())
    }

    #[tokio::test]
    async fn list_bucket() -> Result<()> {
        let (root, service) = setup_service().unwrap();

        let buckets = ["asd", "qwe"];
        for &bucket in buckets.iter() {
            let dir_path = common::generate_path(&root, S3Path::Bucket { bucket });
            fs::create_dir(&dir_path).await.unwrap();
        }

        let mut req = Request::new(Body::empty());
        *req.method_mut() = Method::GET;
        *req.uri_mut() = "http://localhost/".parse().unwrap();
        req.headers_mut().insert(
            X_AMZ_CONTENT_SHA256.clone(),
            HeaderValue::from_static("UNSIGNED-PAYLOAD"),
        );

        let mut res = service.hyper_call(req).await.unwrap();
        let body = common::recv_body_string(&mut res).await.unwrap();

        assert_eq!(res.status(), StatusCode::OK);

        let parser = xml::EventReader::new(io::Cursor::new(body.as_bytes()));
        let mut in_name = false;
        for e in parser {
            let e = e.unwrap();
            match e {
                xml::reader::XmlEvent::StartElement { name, .. } => {
                    if name.local_name == "Name" {
                        in_name = true;
                    }
                }
                xml::reader::XmlEvent::EndElement { name } => {
                    if name.local_name == "Name" {
                        in_name = false;
                    }
                }
                xml::reader::XmlEvent::Characters(s) => {
                    if in_name {
                        assert!(["asd", "qwe"].contains(&s.as_str()));
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}

mod error {

    use super::*;

    #[tokio::test]
    async fn get_object() {
        let (_, service) = setup_service().unwrap();

        let bucket = "asd";
        let key = "qwe";

        let mut req = Request::new(Body::empty());
        *req.method_mut() = Method::GET;
        *req.uri_mut() = format!("http://localhost/{}/{}", bucket, key)
            .parse()
            .unwrap();
        req.headers_mut().insert(
            X_AMZ_CONTENT_SHA256.clone(),
            HeaderValue::from_static("UNSIGNED-PAYLOAD"),
        );

        let mut res = service.hyper_call(req).await.unwrap();
        let body = common::recv_body_string(&mut res).await.unwrap();
        let mime = common::parse_mime(&res).unwrap();

        assert_eq!(res.status(), StatusCode::NOT_FOUND);
        assert_eq!(mime, mime::TEXT_XML);
        assert_eq!(
            body,
            concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>",
                "<Error>",
                "<Code>NoSuchKey</Code>",
                "<Message>The specified key does not exist.</Message>",
                "</Error>"
            )
        );
    }

    #[tokio::test]
    async fn head_bucket() -> Result<()> {
        let (_, service) = setup_service().unwrap();

        let bucket = "asd";

        let mut req = Request::new(Body::empty());
        *req.method_mut() = Method::HEAD;
        *req.uri_mut() = format!("http://localhost/{}", bucket).parse().unwrap();
        req.headers_mut().insert(
            X_AMZ_CONTENT_SHA256.clone(),
            HeaderValue::from_static("UNSIGNED-PAYLOAD"),
        );

        let mut res = service.hyper_call(req).await.unwrap();
        let body = common::recv_body_string(&mut res).await.unwrap();
        let mime = common::parse_mime(&res).unwrap();

        assert_eq!(res.status(), StatusCode::NOT_FOUND);
        assert_eq!(mime, mime::TEXT_XML);
        assert_eq!(
            body,
            concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>",
                "<Error>",
                "<Code>NoSuchBucket</Code>",
                "<Message>The specified bucket does not exist.</Message>",
                "</Error>"
            )
        );

        Ok(())
    }

    #[tokio::test]
    async fn create_bucket() -> Result<()> {
        let (root, service) = setup_service().unwrap();

        let bucket = "asd";
        let dir_path = common::generate_path(root, S3Path::Bucket { bucket });
        fs::create_dir(dir_path).await?;

        let mut req = Request::new(Body::empty());
        *req.method_mut() = Method::PUT;
        *req.uri_mut() = format!("http://localhost/{}", bucket).parse().unwrap();
        req.headers_mut().insert(
            X_AMZ_CONTENT_SHA256.clone(),
            HeaderValue::from_static("UNSIGNED-PAYLOAD"),
        );

        let mut res = service.hyper_call(req).await.unwrap();
        let body = common::recv_body_string(&mut res).await.unwrap();
        let mime = common::parse_mime(&res).unwrap();

        assert_eq!(res.status(), StatusCode::CONFLICT);
        assert_eq!(mime, mime::TEXT_XML);
        assert_eq!(
            body,
            concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>",
                "<Error>",
                "<Code>BucketAlreadyExists</Code>",
                "<Message>",
                "The requested bucket name is not available. ",
                "The bucket namespace is shared by all users of the system. ",
                "Please select a different name and try again.",
                "</Message>",
                "</Error>"
            )
        );

        Ok(())
    }
}
