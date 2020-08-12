use rusoto_core::RusotoError;
use std::error::Error;

#[derive(Debug, thiserror::Error)]
pub enum S3Error<E>
where
    E: Error + Send + Sync + 'static,
{
    #[error("Rusoto: {}", .0)]
    Rusoto(
        #[from]
        #[source]
        RusotoError<E>,
    ),

    #[error("InvalidRequest: {}", .0)]
    InvalidRequest(#[source] Box<dyn Error + Send + Sync + 'static>),

    #[error("InvalidOutput: {}", .0)]
    InvalidOutput(#[source] Box<dyn Error + Send + Sync + 'static>),
}
