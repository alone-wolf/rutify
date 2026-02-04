pub mod client;
pub mod error;

pub use client::RutifyClient;
pub use error::SdkError;
pub use rutify_core::*;

pub type SdkResult<T> = Result<T, SdkError>;
