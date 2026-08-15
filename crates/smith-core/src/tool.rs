// crates/smith-core/src/tool.rs
use std::time::SystemTime;

use async_trait::async_trait;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use thiserror::Error;
use tokio_util::sync::CancellationToken;

use crate::context::ExecutionContext;

// ---------------------------------------------------------------------------
// ToolError — structured error reporting for tool execution (§4.2)
// ---------------------------------------------------------------------------

/// Structured error type for tool execution with contextual accountability.
#[derive(Error, Debug)]
pub enum ToolError {
    /// Invalid or missing input parameters.
    #[error("Invalid input: {message}")]
    InvalidInput {
        message: String,
        /// Parameter name if the error is field-specific.
        param: Option<String>,
        /// The input value that caused the error.
        input: Option<Value>,
        /// When the error occurred.
        timestamp: SystemTime,
    },

    /// UI element not found or inaccessible.
    #[error("Element not found: {message}")]
    ElementNotFound {
        message: String,
        /// Selector parameters used for the search.
        selector: Option<Value>,
        timestamp: SystemTime,
    },

    /// Operation cancelled by user/authority (§5.4).
    #[error("Operation cancelled")]
    Cancelled { timestamp: SystemTime },

    /// Platform or UIA error with underlying cause.
    #[error("Platform error: {message}")]
    PlatformError {
        message: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
        /// Input parameters at the time of failure.
        input: Option<Value>,
        timestamp: SystemTime,
    },

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Other errors forwarded from underlying infrastructure.
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl ToolError {
    fn timestamp() -> SystemTime {
        SystemTime::now()
    }

    /// Creates an `InvalidInput` error.
    pub fn invalid_input(
        message: impl Into<String>,
        param: Option<String>,
        input: Option<Value>,
    ) -> Self {
        Self::InvalidInput {
            message: message.into(),
            param,
            input,
            timestamp: Self::timestamp(),
        }
    }

    /// Creates an `ElementNotFound` error.
    pub fn element_not_found(message: impl Into<String>, selector: Option<Value>) -> Self {
        Self::ElementNotFound {
            message: message.into(),
            selector,
            timestamp: Self::timestamp(),
        }
    }

    /// Creates a `Cancelled` error.
    pub fn cancelled() -> Self {
        Self::Cancelled {
            timestamp: Self::timestamp(),
        }
    }

    /// Creates a `PlatformError` error.
    pub fn platform_error(
        message: impl Into<String>,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
        input: Option<Value>,
    ) -> Self {
        Self::PlatformError {
            message: message.into(),
            source: source.into(),
            input,
            timestamp: Self::timestamp(),
        }
    }
}

// ---------------------------------------------------------------------------
// Typed Tool trait with associated types (§3.5)
// ---------------------------------------------------------------------------

/// Base trait for all automation tools with compile-time type contracts.
///
/// # Requirements
/// - `Send + Sync`: Tools may be executed in Tokio's multi-thread runtime.
/// - Stateless: The tool itself does not store execution state, only configuration.
/// - `Input` and `Output` define a strict, self-documenting contract for each tool.
///
/// # Errors
/// Returns `ToolError` on failure with contextual information.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Typed input parameters for this tool.
    type Input: DeserializeOwned + Serialize + Send;

    /// Typed output produced by this tool on success.
    type Output: Serialize + Send;

    /// Unique tool name (e.g., `windows.click`).
    fn name(&self) -> &'static str;

    /// Description for documentation and LLM agents.
    fn description(&self) -> &'static str;

    /// JSON Schema for input validation.
    fn schema(&self) -> Value;

    /// Asynchronous tool execution with typed input/output.
    ///
    /// # Arguments
    /// * `input` - Typed call parameters (satisfying `Self::Input`)
    /// * `ctx` - Execution context (read/write variables)
    /// * `token` - Cancellation token for graceful shutdown (§5.4)
    async fn execute(
        &self,
        input: Self::Input,
        ctx: &mut ExecutionContext,
        token: CancellationToken,
    ) -> Result<Self::Output, ToolError>;
}

// ---------------------------------------------------------------------------
// DynTool — object-safe wrapper for dynamic dispatch (used by Registry)
// ---------------------------------------------------------------------------

/// Object-safe dynamic dispatch trait for tools.
///
/// The `Registry` stores `Box<dyn DynTool>` and uses JSON Value for
/// parameter passing at runtime. A blanket impl covers all `T: Tool`.
#[async_trait]
pub trait DynTool: Send + Sync {
    /// Unique tool name.
    fn name(&self) -> &'static str;

    /// Description for documentation.
    fn description(&self) -> &'static str;

    /// JSON Schema for input validation.
    fn schema(&self) -> Value;

    /// Execute the tool with JSON-serialized parameters.
    ///
    /// # Arguments
    /// * `config` - JSON Value parameters
    /// * `ctx` - Execution context
    /// * `token` - Cancellation token
    async fn execute(
        &self,
        config: Value,
        ctx: &mut ExecutionContext,
        token: CancellationToken,
    ) -> Result<Value, ToolError>;
}

#[async_trait]
impl<T> DynTool for T
where
    T: Tool + Send + Sync,
{
    fn name(&self) -> &'static str {
        self.name()
    }

    fn description(&self) -> &'static str {
        self.description()
    }

    fn schema(&self) -> Value {
        self.schema()
    }

    async fn execute(
        &self,
        config: Value,
        ctx: &mut ExecutionContext,
        token: CancellationToken,
    ) -> Result<Value, ToolError> {
        let input: T::Input = serde_json::from_value(config)?;
        let output = <Self as Tool>::execute(self, input, ctx, token).await?;
        Ok(serde_json::to_value(output)?)
    }
}
