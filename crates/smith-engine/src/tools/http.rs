//! Универсальный HTTP-инструмент: любой запрос (method/url/headers/body).
//!
//! Используется в том числе для вызова LLM API — без привязки к конкретному
//! провайдеру: разработчик сам формирует запрос как обычный HTTP POST.

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use smith_core::{ExecutionContext, Tool, ToolError};
use tokio_util::sync::CancellationToken;

/// Входные параметры `http.request`.
#[derive(Debug, Serialize, Deserialize)]
pub struct HttpInput {
    /// Полный URL запроса.
    pub url: String,
    /// HTTP-метод (GET, POST, PUT, ...); по умолчанию GET.
    #[serde(default = "default_method")]
    pub method: String,
    /// Заголовки запроса.
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
    /// Тело запроса (JSON).
    #[serde(default)]
    pub body: Option<Value>,
    /// Таймаут в миллисекундах; по умолчанию 30 000.
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

/// Результат HTTP-запроса.
#[derive(Debug, Serialize)]
pub struct HttpOutput {
    pub status: u16,
    /// Тело ответа: JSON, если ответ — JSON, иначе строка.
    pub body: Value,
}

fn default_method() -> String {
    "GET".to_string()
}

/// Выполняет произвольный HTTP-запрос.
pub struct HttpTool {
    client: reqwest::Client,
}

impl HttpTool {
    /// Создаёт новый инструмент.
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for HttpTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for HttpTool {
    type Input = HttpInput;
    type Output = HttpOutput;

    fn name(&self) -> &'static str {
        "http.request"
    }

    fn description(&self) -> &'static str {
        "Performs an HTTP request (method, url, headers, body) and returns status and response body"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "Full request URL" },
                "method": { "type": "string", "description": "HTTP method (GET, POST, ...)" },
                "headers": { "type": "object", "description": "Request headers" },
                "body": { "type": "object", "description": "JSON request body" },
                "timeout_ms": { "type": "integer", "minimum": 1, "description": "Timeout in milliseconds" }
            },
            "required": ["url"]
        })
    }

    async fn execute(
        &self,
        input: HttpInput,
        _ctx: &mut ExecutionContext,
        token: CancellationToken,
    ) -> Result<HttpOutput, ToolError> {
        if token.is_cancelled() {
            return Err(ToolError::cancelled());
        }

        let method =
            reqwest::Method::from_bytes(input.method.to_uppercase().as_bytes()).map_err(|e| {
                ToolError::invalid_input(
                    format!("Invalid HTTP method '{}': {e}", input.method),
                    Some("method".into()),
                    None,
                )
            })?;

        let mut request = self.client.request(method, &input.url);
        if let Some(headers) = &input.headers {
            for (name, value) in headers {
                request = request.header(name, value);
            }
        }
        if let Some(body) = &input.body {
            request = request.json(body);
        }

        let timeout = Duration::from_millis(input.timeout_ms.unwrap_or(30_000));
        let response = tokio::time::timeout(timeout, request.send())
            .await
            .map_err(|_| {
                ToolError::platform_error(
                    "HTTP request timed out",
                    std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout"),
                    None,
                )
            })?
            .map_err(|e| ToolError::platform_error("HTTP request failed", e, None))?;

        let status = response.status().as_u16();
        let text = response
            .text()
            .await
            .map_err(|e| ToolError::platform_error("Failed to read response body", e, None))?;

        let body = serde_json::from_str(&text).unwrap_or(Value::String(text));
        Ok(HttpOutput { status, body })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use smith_core::{Ready, Unvalidated};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn ctx() -> ExecutionContext<Ready> {
        ExecutionContext::<Unvalidated>::new().validate()
    }

    #[tokio::test]
    async fn test_http_get_returns_json_body() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
            .mount(&server)
            .await;

        let tool = HttpTool::new();
        let output = tool
            .execute(
                HttpInput {
                    url: format!("{}/api", server.uri()),
                    method: "GET".into(),
                    headers: None,
                    body: None,
                    timeout_ms: Some(5_000),
                },
                &mut ctx(),
                CancellationToken::new(),
            )
            .await
            .unwrap();

        assert_eq!(output.status, 200);
        assert_eq!(output.body, json!({"ok": true}));
    }

    #[tokio::test]
    async fn test_http_post_sends_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/echo"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({"received": true})))
            .mount(&server)
            .await;

        let tool = HttpTool::new();
        let output = tool
            .execute(
                HttpInput {
                    url: format!("{}/echo", server.uri()),
                    method: "POST".into(),
                    headers: None,
                    body: Some(json!({"prompt": "hello"})),
                    timeout_ms: Some(5_000),
                },
                &mut ctx(),
                CancellationToken::new(),
            )
            .await
            .unwrap();

        assert_eq!(output.status, 201);
        assert_eq!(output.body["received"], true);
    }

    #[tokio::test]
    async fn test_http_non_json_body_becomes_string() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/text"))
            .respond_with(ResponseTemplate::new(200).set_body_string("plain text"))
            .mount(&server)
            .await;

        let tool = HttpTool::new();
        let output = tool
            .execute(
                HttpInput {
                    url: format!("{}/text", server.uri()),
                    method: "GET".into(),
                    headers: None,
                    body: None,
                    timeout_ms: Some(5_000),
                },
                &mut ctx(),
                CancellationToken::new(),
            )
            .await
            .unwrap();

        assert_eq!(output.body, Value::String("plain text".into()));
    }

    #[tokio::test]
    async fn test_http_invalid_method_returns_error() {
        let tool = HttpTool::new();
        let result = tool
            .execute(
                HttpInput {
                    url: "http://example.com".into(),
                    method: "NOT A METHOD".into(),
                    headers: None,
                    body: None,
                    timeout_ms: Some(1_000),
                },
                &mut ctx(),
                CancellationToken::new(),
            )
            .await;

        assert!(result.is_err());
    }
}
