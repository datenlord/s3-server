#[allow(dead_code)] // TODO: remove it
#[derive(Debug)]
pub(super) enum S3Path<'a> {
    Root,
    Bucket { bucket: &'a str },
    Object { bucket: &'a str, key: &'a str },
}

#[derive(Debug, thiserror::Error)]
#[error("ParseS3PathError")]
pub(super) struct ParseS3PathError {
    _priv: (),
}

impl<'a> S3Path<'a> {
    #[allow(dead_code)] // TODO: remove it
    pub(super) fn from_path(path: &'a str) -> Result<Self, ParseS3PathError> {
        if !path.starts_with('/') {
            return Err(ParseS3PathError { _priv: () });
        }

        let mut iter = path.split('/');
        let _ = iter.next().ok_or_else(|| ParseS3PathError { _priv: () })?;

        let bucket = match iter.next() {
            None => return Err(ParseS3PathError { _priv: () }),
            Some("") => return Ok(S3Path::Root),
            Some(s) => s,
        };

        let key = match iter.next() {
            None | Some("") => return Ok(S3Path::Bucket { bucket }),

            // here can not panic, because `split` ensures `path` has enough length
            Some(_) => path.get(bucket.len().saturating_add(2)..).unwrap(),
        };

        Ok(Self::Object { bucket, key })
    }
}

#[test]
fn test_s3_path() {
    assert!(matches!(S3Path::from_path("/"), Ok(S3Path::Root)));

    assert!(matches!(
        S3Path::from_path("/bucket"),
        Ok(S3Path::Bucket { bucket: "bucket" })
    ));

    assert!(matches!(
        S3Path::from_path("/bucket/"),
        Ok(S3Path::Bucket { bucket: "bucket" })
    ));

    assert!(matches!(
        S3Path::from_path("/bucket/dir/object"),
        Ok(S3Path::Object {
            bucket: "bucket",
            key: "dir/object"
        })
    ));

    assert!(S3Path::from_path("asd").is_err());

    assert!(S3Path::from_path("a/").is_err());
}
