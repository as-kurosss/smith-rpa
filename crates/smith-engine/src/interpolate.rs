//! Подстановка переменных вида `{{var}}` из `ExecutionContext`.
//!
//! Применяется к строковым параметрам шагов до вызова инструмента.

use serde_json::Value;
use smith_core::{ContextValue, ExecutionContext};

/// Заменяет все вхождения `{{key}}` значениями из контекста.
///
/// Неизвестные ключи остаются в строке как есть (безопасное поведение:
/// пользователь сразу видит неразрешённую переменную).
pub fn interpolate(template: &str, ctx: &ExecutionContext) -> String {
    let mut result = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(start) = rest.find("{{") {
        result.push_str(&rest[..start]);
        let after_open = &rest[start + 2..];
        if let Some(end) = after_open.find("}}") {
            let key = &after_open[..end];
            match ctx.get(key).and_then(context_value_to_string) {
                Some(value) => result.push_str(&value),
                None => {
                    // Ключ не найден — сохраняем плейсхолдер
                    result.push_str("{{");
                    result.push_str(key);
                    result.push_str("}}");
                }
            }
            rest = &after_open[end + 2..];
        } else {
            // Нет закрывающей скобки — остаток как есть (без дублирования префикса)
            result.push_str(&rest[start..]);
            return result;
        }
    }

    result.push_str(rest);
    result
}

/// Рекурсивно интерполирует все строки внутри JSON-значения.
#[must_use]
pub fn interpolate_value(value: &Value, ctx: &ExecutionContext) -> Value {
    match value {
        Value::String(s) => Value::String(interpolate(s, ctx)),
        Value::Array(items) => {
            Value::Array(items.iter().map(|v| interpolate_value(v, ctx)).collect())
        }
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), interpolate_value(v, ctx)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Преобразует простое значение контекста в строку для подстановки.
fn context_value_to_string(value: &ContextValue) -> Option<String> {
    match value {
        ContextValue::String(s) => Some(s.clone()),
        ContextValue::Number(n) => Some(n.to_string()),
        ContextValue::Boolean(b) => Some(b.to_string()),
        ContextValue::Null => Some(String::new()),
        ContextValue::List(_) | ContextValue::Bytes(_) | ContextValue::Custom(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smith_core::{ExecutionContext, Ready, Unvalidated};

    fn ready_ctx() -> ExecutionContext<Ready> {
        let mut ctx = ExecutionContext::<Unvalidated>::new();
        ctx.set("name", ContextValue::String("Invoice".into()));
        ctx.set("count", ContextValue::Number(3.0));
        ctx.set("enabled", ContextValue::Boolean(true));
        ctx.validate()
    }

    #[test]
    fn test_interpolate_string_variable() {
        let ctx = ready_ctx();
        assert_eq!(interpolate("Hello {{name}}!", &ctx), "Hello Invoice!");
    }

    #[test]
    fn test_interpolate_number_and_boolean() {
        let ctx = ready_ctx();
        assert_eq!(interpolate("{{count}}/{{enabled}}", &ctx), "3/true");
    }

    #[test]
    fn test_interpolate_missing_key_keeps_placeholder() {
        let ctx = ready_ctx();
        assert_eq!(interpolate("{{missing}}", &ctx), "{{missing}}");
    }

    #[test]
    fn test_interpolate_no_placeholders_unchanged() {
        let ctx = ready_ctx();
        assert_eq!(interpolate("plain text", &ctx), "plain text");
    }

    #[test]
    fn test_interpolate_unclosed_placeholder_kept() {
        let ctx = ready_ctx();
        assert_eq!(interpolate("a {{name", &ctx), "a {{name");
    }

    #[test]
    fn test_interpolate_value_recurses_into_json() {
        let ctx = ready_ctx();
        let value = serde_json::json!({
            "url": "https://api.example.com/{{name}}",
            "items": ["{{count}}"],
            "nested": { "flag": "{{enabled}}" }
        });
        let result = interpolate_value(&value, &ctx);
        assert_eq!(result["url"], "https://api.example.com/Invoice");
        assert_eq!(result["items"][0], "3");
        assert_eq!(result["nested"]["flag"], "true");
    }

    #[test]
    fn test_interpolate_value_non_string_unchanged() {
        let ctx = ready_ctx();
        let value = serde_json::json!({ "num": 42, "arr": [1, 2] });
        assert_eq!(interpolate_value(&value, &ctx), value);
    }
}
