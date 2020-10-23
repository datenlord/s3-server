//! S3 server

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
    clippy::panic, // Panic when fatal failures occur
    clippy::blanket_clippy_restriction_lints,
)]
#![cfg_attr(test, allow(
    clippy::unwrap_used, // Tests need `unwrap`
    clippy::indexing_slicing, // Fail fast
))]
#![allow(
    // FIXME: Deny lints below
    clippy::multiple_crate_versions,

    dead_code, // TODO: remove it
    clippy::todo, // TODO: remove it
)]

#[macro_use]
pub(crate) mod utils;

mod data_structures;
mod ops;
mod output;
mod signature_v4;

mod auth;
mod service;
mod storage;

pub use self::auth::{S3Auth, SimpleAuth};
pub use self::service::S3Service;
pub use self::storage::S3Storage;

pub mod dto;
pub mod errors;
pub mod headers;
pub mod path;
pub mod storages;
pub mod streams;

/// Request type
pub(crate) type Request = hyper::Request<Body>;

/// Response type
pub(crate) type Response = hyper::Response<Body>;

/// `Box<dyn std::error::Error + Send + Sync + 'static>`
pub(crate) type BoxStdError = Box<dyn std::error::Error + Send + Sync + 'static>;

pub(crate) use async_trait::async_trait;
pub(crate) use hyper::{Body, Method, StatusCode};
pub(crate) use mime::Mime;
