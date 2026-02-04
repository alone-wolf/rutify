use thiserror::Error;
use rutify_core::RutifyError;

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

impl From<SdkError> for RutifyError {
    fn from(err: SdkError) -> Self {
        match err {
            SdkError::HttpError(e) => RutifyError::Network { message: e.to_string() },
            SdkError::JsonError(e) => RutifyError::Parse { message: e.to_string() },
            SdkError::ApiError { status } => RutifyError::Api { status, message: "API error".to_string() },
            SdkError::InvalidUrl(e) => RutifyError::Config { message: e.to_string() },
            SdkError::NetworkError(msg) => RutifyError::Network { message: msg },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sdk_error_creation() {
        let error = SdkError::NetworkError("test error".to_string());
        assert!(matches!(error, SdkError::NetworkError(_)));
    }

    #[test]
    fn test_sdk_error_display() {
        let error = SdkError::NetworkError("test error".to_string());
        assert_eq!(error.to_string(), "Network error: test error");
    }

    #[test]
    fn test_sdk_error_to_rutify_error() {
        let sdk_error = SdkError::NetworkError("network issue".to_string());
        let rutify_error: RutifyError = sdk_error.into();
        
        match rutify_error {
            RutifyError::Network { message } => assert_eq!(message, "network issue"),
            _ => panic!("Expected Network error"),
        }
    }

    #[test]
    fn test_sdk_error_api_to_rutify_error() {
        let sdk_error = SdkError::ApiError { status: "500".to_string() };
        let rutify_error: RutifyError = sdk_error.into();
        
        match rutify_error {
            RutifyError::Api { status, message } => {
                assert_eq!(status, "500");
                assert_eq!(message, "API error");
            },
            _ => panic!("Expected API error"),
        }
    }

    #[test]
    fn test_sdk_error_json_to_rutify_error() {
        // Create a JSON error through parsing invalid JSON
        let json_result: Result<serde_json::Value, serde_json::Error> = 
            serde_json::from_str("invalid json");
        let sdk_error = SdkError::JsonError(json_result.unwrap_err());
        let rutify_error: RutifyError = sdk_error.into();
        
        match rutify_error {
            RutifyError::Parse { message } => {
                // Check that it's a parse error, not necessarily containing "JSON"
                assert!(!message.is_empty());
            },
            _ => panic!("Expected Parse error"),
        }
    }

    #[test]
    fn test_sdk_error_url_to_rutify_error() {
        let sdk_error = SdkError::InvalidUrl(url::ParseError::EmptyHost);
        let rutify_error: RutifyError = sdk_error.into();
        
        match rutify_error {
            RutifyError::Config { message } => {
                // Check that it's a config error, not necessarily containing "URL"
                assert!(!message.is_empty());
            },
            _ => panic!("Expected Config error"),
        }
    }
}
