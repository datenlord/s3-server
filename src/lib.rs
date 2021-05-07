//! An experimental generic S3 server
//!
//! ## Public API
//!
//! ### Type: `S3Service`, `SharedS3Service`
//!
//! [`S3Service`] is the actual S3 service owner. [`SharedS3Service`] is like `Arc<S3Service>`.
//!
//! [`S3Service`] contains internal handlers, a storage and a optional auth provider.
//!
//! [`hyper::service::Service<Request>`] is implemented for [`SharedS3Service`].
//!
//! When a new tcp stream connects, the server should clone [`SharedS3Service`], then handle each http request by `S3Service::hyper_call` or `S3Service::handle`.
//!
//! An [`S3Service`] instance can be integrated into a [`hyper`] application.
//!
//! See `src/bin/s3-server.rs` for how to setup an [`S3Service`].
//!
//! ### Trait: `S3Storage`
//!
//! [`S3Storage`] is an async trait.
//!
//! Each method of [`S3Storage`] represents a server-side S3 API.
//!
//! [`S3Service`] extracts DTO from http request, dispatches requests to the storage and converts the output into http response.
//!
//! ### Trait: `S3Auth`
//!
//! [`S3Auth`] is an async trait.
//!
//! An [`S3Auth`] instance provides AK/SK management.
//!
//! [`S3Service`] looks up secret access keys from an auth provider and checks http signature (if any) by the AK and SK.
//!
//! ## Internal API
//!
//! ### Type: `S3Error`, `S3StorageError<E>`, `S3AuthError`
//!
//! `S3Error` contains error code, message, error source, span trace and backtrace.
//!
//! `S3StorageError<E>` and `S3AuthError` are special kinds of `S3Error`. They can be converted into `S3Error`.
//!
//! See `src/internal_macros.rs` for how to create an `S3Error` by macros.
//!
//! ### Trait: `S3Handler`
//!
//! An `S3Handler` determines whether the http request matches it.
//!
//! If the http request matches the handler,
//! then the handler will be called with two arguments:
//! `&mut ReqContext<'_>` and `&(dyn S3Storage + Send + Sync)`.
//!
//! ### Trait: `S3Output`
//!
//! `S3Output` represents types which can be converted into a response.
//!
//! `S3Output` is implemented for DTOs and results.
//!
//! ### S3 types
//!
//! `S3Path` represents a path in the S3 storage.
//!
//! All types in `src/dto.rs` are data transfer objects which represent the input or output of S3 APIs.
//!
//! All types in `src/headers` are http headers which may occur in an S3 http request.
//!
//! All types in `src/streams` are http body streams which may occur in an S3 http request.

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
    clippy::unwrap_used, // Tests need [`unwrap`]
    clippy::indexing_slicing, // Fail fast
))]

#[macro_use]
mod internal_macros;

pub(crate) mod utils;

mod data_structures;
mod ops;
mod output;
mod signature_v4;
mod streams;

mod auth;
mod service;
mod storage;

pub use self::auth::{S3Auth, SimpleAuth};
pub use self::service::{S3Service, SharedS3Service};
pub use self::storage::S3Storage;

pub mod dto;
pub mod errors;
pub mod headers;
pub mod path;
pub mod storages;

/// Request type
pub(crate) type Request = hyper::Request<Body>;

/// Response type
pub(crate) type Response = hyper::Response<Body>;

/// `Box<dyn std::error::Error + Send + Sync + 'static>`
pub(crate) type BoxStdError = Box<dyn std::error::Error + Send + Sync + 'static>;

pub(crate) use async_trait::async_trait;
pub(crate) use hyper::{Body, Method, StatusCode};
pub(crate) use mime::Mime;
