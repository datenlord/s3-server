//! Common Request Headers

mod amz_content_sha256;
mod amz_copy_source;
mod amz_date;
mod authorization_v4;

pub use self::amz_content_sha256::AmzContentSha256;
pub use self::amz_copy_source::AmzCopySource;
pub use self::amz_date::AmzDate;
pub use self::authorization_v4::{AuthorizationV4, CredentialV4};

pub use hyper::header::*;

// FIXME: declare const headers, see <https://github.com/hyperium/http/issues/264>

use once_cell::sync::Lazy;

macro_rules! declare_header_name{
    {$($(#[$docs:meta])* $n:ident: $s:expr;)+} => {
        $(
            $(#[$docs])*
            pub static $n: Lazy<HeaderName> = Lazy::new(||HeaderName::from_static($s));
        )+

        #[test]
        fn check_headers(){
            $(
                assert_eq!($n.as_str(), $s);
            )+
        }
    }
}

declare_header_name! {
    /// x-amz-mfa
    X_AMZ_MFA: "x-amz-mfa";

    /// x-amz-content-sha256
    X_AMZ_CONTENT_SHA_256: "x-amz-content-sha256";

    /// x-amz-expiration
    X_AMZ_EXPIRATION: "x-amz-expiration";

    /// x-amz-copy-source-version-id
    X_AMZ_COPY_SOURCE_VERSION_ID: "x-amz-copy-source-version-id";

    /// x-amz-version-id
    X_AMZ_VERSION_ID: "x-amz-version-id";

    /// x-amz-request-charged
    X_AMZ_REQUEST_CHARGED: "x-amz-request-charged";

    /// x-amz-acl
    X_AMZ_ACL: "x-amz-acl";

    /// x-amz-copy-source
    X_AMZ_COPY_SOURCE: "x-amz-copy-source";

    /// x-amz-copy-source-if-match
    X_AMZ_COPY_SOURCE_IF_MATCH: "x-amz-copy-source-if-match";

    /// x-amz-copy-source-if-modified-since
    X_AMZ_COPY_SOURCE_IF_MODIFIED_SINCE: "x-amz-copy-source-if-modified-since";

    /// x-amz-copy-source-if-none-match
    X_AMZ_COPY_SOURCE_IF_NONE_MATCH: "x-amz-copy-source-if-none-match";

    /// x-amz-copy-source-if-unmodified-since
    X_AMZ_COPY_SOURCE_IF_UNMODIFIED_SINCE: "x-amz-copy-source-if-unmodified-since";

    /// x-amz-grant-full-control
    X_AMZ_GRANT_FULL_CONTROL: "x-amz-grant-full-control";

    /// x-amz-grant-read
    X_AMZ_GRANT_READ: "x-amz-grant-read";

    /// x-amz-grant-write
    X_AMZ_GRANT_WRITE: "x-amz-grant-write";

    /// x-amz-grant-read-acp
    X_AMZ_GRANT_READ_ACP: "x-amz-grant-read-acp";

    /// x-amz-grant-write-acp
    X_AMZ_GRANT_WRITE_ACP: "x-amz-grant-write-acp";

    /// x-amz-metadata-directive
    X_AMZ_METADATA_DIRECTIVE: "x-amz-metadata-directive";

    /// x-amz-tagging-directive
    X_AMZ_TAGGING_DIRECTIVE: "x-amz-tagging-directive";

    /// x-amz-server-side-encryption
    X_AMZ_SERVER_SIDE_ENCRYPTION: "x-amz-server-side-encryption";

    /// x-amz-storage-class
    X_AMZ_STORAGE_CLASS: "x-amz-storage-class";

    /// x-amz-website-redirect-location
    X_AMZ_WEBSITE_REDIRECT_LOCATION: "x-amz-website-redirect-location";

    /// x-amz-server-side-encryption-customer-algorithm
    X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM: "x-amz-server-side-encryption-customer-algorithm";

    /// x-amz-server-side-encryption-customer-key
    X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY: "x-amz-server-side-encryption-customer-key";

    /// x-amz-server-side-encryption-customer-key-MD5
    X_AMZ_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5: "x-amz-server-side-encryption-customer-key-md5";

    /// x-amz-server-side-encryption-aws-kms-key-id
    X_AMZ_SERVER_SIDE_ENCRYPTION_AWS_KMS_KEY_ID: "x-amz-server-side-encryption-aws-kms-key-id";

    /// x-amz-server-side-encryption-context
    X_AMZ_SERVER_SIDE_ENCRYPTION_CONTEXT: "x-amz-server-side-encryption-context";

    /// x-amz-copy-source-server-side-encryption-customer-algorithm
    X_AMZ_COPY_SOURCE_SERVER_SIDE_ENCRYPTION_CUSTOMER_ALGORITHM: "x-amz-copy-source-server-side-encryption-customer-algorithm";

    /// x-amz-copy-source-server-side-encryption-customer-key
    X_AMZ_COPY_SOURCE_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY: "x-amz-copy-source-server-side-encryption-customer-key";

    /// x-amz-copy-source-server-side-encryption-customer-key-MD5
    X_AMZ_COPY_SOURCE_SERVER_SIDE_ENCRYPTION_CUSTOMER_KEY_MD5: "x-amz-copy-source-server-side-encryption-customer-key-md5";

    /// x-amz-request-payer
    X_AMZ_REQUEST_PAYER: "x-amz-request-payer";

    /// x-amz-tagging
    X_AMZ_TAGGING: "x-amz-tagging";

    /// x-amz-object-lock-mode
    X_AMZ_OBJECT_LOCK_MODE: "x-amz-object-lock-mode";

    /// x-amz-object-lock-retain-until-date
    X_AMZ_OBJECT_LOCK_RETAIN_UNTIL_DATE: "x-amz-object-lock-retain-until-date";

    /// x-amz-object-lock-legal-hold
    X_AMZ_OBJECT_LOCK_LEGAL_HOLD: "x-amz-object-lock-legal-hold";

    /// x-amz-delete-marker
    X_AMZ_DELETE_MARKER: "x-amz-delete-marker";

    /// x-amz-restore
    X_AMZ_RESTORE: "x-amz-restore";

    /// x-amz-missing-meta
    X_AMZ_MISSING_META: "x-amz-missing-meta";

    /// x-amz-replication-status
    X_AMZ_REPLICATION_STATUS: "x-amz-replication-status";

    /// x-amz-mp-parts-count
    X_AMZ_MP_PARTS_COUNT: "x-amz-mp-parts-count";

    /// x-amz-tagging-count
    X_AMZ_TAGGING_COUNT: "x-amz-tagging-count";

    /// Content-MD5
    CONTENT_MD5: "content-md5";

    /// x-amz-bucket-object-lock-enabled
    X_AMZ_BUCKET_OBJECT_LOCK_ENABLED: "x-amz-bucket-object-lock-enabled";

    /// x-amz-bypass-governance-retention
    X_AMZ_BYPASS_GOVERNANCE_RETENTION: "x-amz-bypass-governance-retention";

    /// x-amz-date
    X_AMZ_DATE: "x-amz-date";

    /// x-amz-content-sha256
    X_AMZ_CONTENT_SHA256: "x-amz-content-sha256";

    /// x-amz-abort-date
    X_AMZ_ABORT_DATE: "x-amz-abort-date";

    /// x-amz-abort-rule-id
    X_AMZ_ABORT_RULE_ID: "x-amz-abort-rule-id";
}
