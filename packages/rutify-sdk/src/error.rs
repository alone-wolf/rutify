use thiserror::Error;

#[derive(Debug, Error)]
pub enum SdkError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("JSON serialization/deserialization failed: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("API returned error status: {status}")]
    ApiError { status: String },
    
    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    
    #[error("Network error: {0}")]
    NetworkError(String),
}

pub type SdkResult<T> = Result<T, SdkError>;
