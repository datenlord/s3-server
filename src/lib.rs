//! S3 service

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
    missing_docs,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    // unreachable_pub,
    unstable_features,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    unused_results,
    variant_size_differences,

    // Treat warnings as errors
    warnings,

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
    unreachable_pub, // Conflicts with root-controlled visibility
    clippy::redundant_pub_crate, // Conflicts with root-controlled visibility
)]
#![cfg_attr(test, allow(
    clippy::panic, // Panic when fatal failures occur
))]
#![allow(
    // TODO: Deny lints below
    clippy::multiple_crate_versions
)]

#[macro_use]
pub(crate) mod utils;

mod error;
mod error_code;
mod output;
mod service;
mod storage;

pub use self::error::{S3Error, S3Result};
pub use self::error_code::S3ErrorCode;
pub use self::output::S3Output;
pub use self::service::S3Service;
pub use self::storage::S3Storage;

pub mod dto;
pub mod header;
pub mod path;
pub mod query;

pub mod compat;
pub mod fs;
