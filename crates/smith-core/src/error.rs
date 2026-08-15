use std::error::Error;

use thiserror::Error;

use crate::tool::ToolError;

#[derive(Error, Debug)]
pub enum SmithError {
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),

    #[error("UI element not found or selector invalid")]
    ElementNotFound,

    #[error("Operation cancelled by user")]
    Cancelled,

    #[error("Context error: {0}")]
    ContextError(String),

    #[error("Platform error: {message}")]
    PlatformError {
        message: String,
        #[source]
        source: Box<dyn Error + Send + Sync>,
    },

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Converts `ToolError` into `SmithError`.
///
/// This bridges the typed tool error domain with the older,
/// general-purpose error type used by the graph executor.
impl From<ToolError> for SmithError {
    fn from(err: ToolError) -> Self {
        match err {
            ToolError::InvalidInput { message, .. } => {
                SmithError::InvalidParams(message.to_string())
            }
            ToolError::ElementNotFound { .. } => SmithError::ElementNotFound,
            ToolError::Cancelled { .. } => SmithError::Cancelled,
            ToolError::PlatformError {
                message, source, ..
            } => SmithError::PlatformError { message, source },
            ToolError::JsonError(e) => SmithError::Other(anyhow::anyhow!("JSON error: {e}")),
            ToolError::Other(e) => SmithError::Other(anyhow::anyhow!("{e}")),
        }
    }
}

pub type SmithResult<T> = Result<T, SmithError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_params_display() {
        let err = SmithError::InvalidParams("test message".into());
        assert_eq!(err.to_string(), "Invalid parameters: test message");
    }

    #[test]
    fn test_element_not_found_display() {
        let err = SmithError::ElementNotFound;
        assert_eq!(err.to_string(), "UI element not found or selector invalid");
    }

    #[test]
    fn test_cancelled_display() {
        let err = SmithError::Cancelled;
        assert_eq!(err.to_string(), "Operation cancelled by user");
    }

    #[test]
    fn test_context_error_display() {
        let err = SmithError::ContextError("something went wrong".into());
        assert_eq!(err.to_string(), "Context error: something went wrong");
    }

    #[test]
    fn test_platform_error_display() {
        let err = SmithError::PlatformError {
            message: "access denied".into(),
            source: Box::new(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "os error",
            )),
        };
        assert_eq!(err.to_string(), "Platform error: access denied");
    }

    #[test]
    fn test_conversion_from_anyhow_error() {
        let underlying = anyhow::anyhow!("oops");
        let err: SmithError = underlying.into();
        assert!(matches!(err, SmithError::Other(_)));
        assert_eq!(err.to_string(), "oops");
    }
}
