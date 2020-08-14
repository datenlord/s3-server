pub(super) type Request = hyper::Request<hyper::Body>;
pub(super) type Response = hyper::Response<hyper::Body>;
pub(super) type BoxStdError = Box<dyn std::error::Error + Send + Sync + 'static>;
pub(super) type StdResult<T> = Result<T, BoxStdError>;
