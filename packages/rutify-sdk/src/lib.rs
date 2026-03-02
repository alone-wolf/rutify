pub mod auth;
pub mod client;
pub mod error;

pub use auth::{
    CreateTokenRequest, CreateTokenResponse, LoginRequest, LoginResponse, RegisterRequest,
    TokenInfo,
};
pub use client::RutifyClient;
pub use error::SdkError;
pub use rutify_core::*;

pub type SdkResult<T> = Result<T, SdkError>;
