//! Модель робота: `Robot` — список шагов, каждый шаг — вызов инструмента.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Робот: именованный список линейных шагов.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Robot {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub steps: Vec<Step>,
}

/// Один шаг: имя инструмента (`action`) + параметры (`params`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub action: String,
    #[serde(default)]
    pub params: Value,
}

fn default_version() -> String {
    "1.0".to_string()
}

impl Robot {
    /// Парсит робота из JSON-строки.
    ///
    /// # Errors
    ///
    /// Возвращает `serde_json::Error` при невалидном JSON.
    pub fn from_json_str(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Сериализует робота в pretty-printed JSON.
    ///
    /// # Errors
    ///
    /// Возвращает `serde_json::Error` при ошибке сериализации.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "name": "Invoice Processor",
        "version": "1.0",
        "steps": [
            { "type": "rpa", "action": "windows.click", "params": { "element_key": "btn" } },
            { "type": "ai", "action": "http.request", "params": { "url": "https://example.com" } }
        ]
    }"#;

    #[test]
    fn test_parse_valid_robot_ignores_unknown_fields() {
        let robot = Robot::from_json_str(SAMPLE).unwrap();
        assert_eq!(robot.name, "Invoice Processor");
        assert_eq!(robot.version, "1.0");
        assert_eq!(robot.steps.len(), 2);
        assert_eq!(robot.steps[0].action, "windows.click");
        // Поле "type" из исходного формата игнорируется, но не ломает парсинг
        assert_eq!(robot.steps[0].params["element_key"], "btn");
    }

    #[test]
    fn test_parse_missing_optional_fields() {
        let robot = Robot::from_json_str(r#"{"name": "Minimal"}"#).unwrap();
        assert_eq!(robot.version, "1.0");
        assert!(robot.steps.is_empty());
    }

    #[test]
    fn test_parse_invalid_json_returns_error() {
        assert!(Robot::from_json_str("{not json}").is_err());
    }

    #[test]
    fn test_serialize_round_trip() {
        let robot: Robot = Robot::from_json_str(SAMPLE).unwrap();
        let json = robot.to_json().unwrap();
        let back: Robot = Robot::from_json_str(&json).unwrap();
        assert_eq!(back.name, robot.name);
        assert_eq!(back.steps.len(), robot.steps.len());
        assert_eq!(back.steps[0].action, robot.steps[0].action);
    }
}
