// crates/smith-core/src/ai.rs
//! Trait AiHandler — an abstraction over LLM agents.
//!
//! Allows GraphExecutor and WorkflowExecutor to not depend on
//! a specific agent implementation (SmithAgent).

use async_trait::async_trait;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::context::ExecutionContext;
use crate::error::SmithResult;

/// Trait for handling AI steps.
///
/// Implemented by SmithAgent (or any other LLM agent).
/// All methods accept `ExecutionContext` for reading/writing context.
#[async_trait]
pub trait AiHandler: Send + Sync {
    /// Execute a prompt with tools (ReAct loop).
    ///
    /// * `prompt` — instruction for the agent
    /// * `tools` — list of available tools (names from ToolRegistry)
    /// * `max_steps` — maximum number of tool calls
    /// * `ctx` — execution context
    /// * `token` — cancellation token
    async fn agent_run(
        &self,
        prompt: &str,
        tools: &[String],
        max_steps: usize,
        ctx: &mut ExecutionContext,
        token: &CancellationToken,
    ) -> SmithResult<Value>;

    /// Execute think (LLM without tools, data generation).
    async fn think(
        &self,
        prompt: &str,
        schema: &Value,
        ctx: &mut ExecutionContext,
        token: &CancellationToken,
    ) -> SmithResult<Value>;

    /// Execute decide (LLM selects an option from a list).
    async fn decide(
        &self,
        prompt: &str,
        options: &[String],
        ctx: &mut ExecutionContext,
        token: &CancellationToken,
    ) -> SmithResult<String>;
}
