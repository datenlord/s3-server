#![forbid(unsafe_code)]
#![deny(
    // The following are allowed by default lints according to
    // https://doc.rust-lang.org/rustc/lints/listing/allowed-by-default.html
    anonymous_parameters,
    bare_trait_objects,
    // box_pointers,
    elided_lifetimes_in_paths,
    missing_copy_implementations,
    missing_debug_implementations,
    // missing_docs, // TODO: add documents
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unstable_features,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    unused_results,
    variant_size_differences,

    // Deny all Clippy lints even Clippy allow some by default
    // https://rust-lang.github.io/rust-clippy/master/
    clippy::all,
    clippy::restriction,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo,
)]
#![allow(
    // Some explicitly allowed Clippy lints, must have clear reason to allow
    clippy::implicit_return, // actually omitting the return keyword is idiomatic Rust code
    clippy::missing_inline_in_public_items, // In general, it is not bad
    clippy::module_name_repetitions, // Allowed by default
)]
#![cfg_attr(test, allow(
    clippy::panic, // Panic when fatal failures occur
))]
#![allow(
    // TODO: Deny lints below
    missing_docs,
    clippy::missing_docs_in_private_items,
    clippy::missing_errors_doc,
    clippy::multiple_crate_versions
)]

#[macro_use]
mod macros;

mod byte_stream;
mod error;
mod utils;

mod s3_error_code;
mod s3_output;
mod s3_path;
mod s3_service;
mod s3_storage;

pub mod compat;
pub mod fs;

pub use error::{S3Error, S3Result};
pub use s3_path::{ParseS3PathError, S3Path, S3PathErrorKind};
pub use s3_service::S3Service;
pub use s3_storage::S3Storage;
