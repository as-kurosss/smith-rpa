//! Config generation per tool type.
//!
//! Takes a captured `BestSelector` and produces the `serde_json::Value` config
//! that the corresponding smith-windows tool expects.

use serde_json::{json, Value};

use crate::types::{BestSelector, FlowNode, ToolType};

/// Parameters for generating tool configs.
pub struct GenerateParams {
    /// Auto-generated output key for the find step.
    pub output_key: String,
    /// Text for input_text / set_text.
    pub text: Option<String>,
    /// Duration for wait tool.
    pub duration_ms: Option<u64>,
}

/// Generates one or two `FlowNode`s for a captured element and chosen tool type.
///
/// - `ToolType::Find` → one node (`windows.find`)
/// - `ToolType::Click` → two nodes (`windows.find` + `windows.click`)
/// - `ToolType::InputText` → two nodes (`windows.find` + `windows.input_text`)
/// - `ToolType::SetText` → two nodes (`windows.find` + `windows.set_text`)
/// - `ToolType::Wait` → one node (`windows.wait`)
pub fn generate_nodes(
    selector: &BestSelector,
    tool: ToolType,
    params: &GenerateParams,
) -> Vec<FlowNode> {
    match tool {
        ToolType::Find => {
            vec![FlowNode {
                tool: "windows.find".into(),
                args: build_find_config(selector, &params.output_key),
            }]
        }
        ToolType::Click => {
            vec![
                FlowNode {
                    tool: "windows.find".into(),
                    args: build_find_config(selector, &params.output_key),
                },
                FlowNode {
                    tool: "windows.click".into(),
                    args: build_click_config(&params.output_key),
                },
            ]
        }
        ToolType::InputText => {
            let text = params.text.clone().unwrap_or_default();
            vec![
                FlowNode {
                    tool: "windows.find".into(),
                    args: build_find_config(selector, &params.output_key),
                },
                FlowNode {
                    tool: "windows.input_text".into(),
                    args: build_text_config(&text, &params.output_key),
                },
            ]
        }
        ToolType::SetText => {
            let text = params.text.clone().unwrap_or_default();
            vec![
                FlowNode {
                    tool: "windows.find".into(),
                    args: build_find_config(selector, &params.output_key),
                },
                FlowNode {
                    tool: "windows.set_text".into(),
                    args: build_text_config(&text, &params.output_key),
                },
            ]
        }
        ToolType::Wait => {
            let duration = params.duration_ms.unwrap_or(1000);
            vec![FlowNode {
                tool: "windows.wait".into(),
                args: build_wait_config(duration),
            }]
        }
    }
}

/// Generates ONLY the action config for clipboard use (without the find prefix).
/// Used by `single --clip` mode where find + click are separated by `---`.
pub fn generate_action_config(
    selector: &BestSelector,
    tool: ToolType,
    params: &GenerateParams,
) -> Value {
    match tool {
        ToolType::Find => build_find_config(selector, &params.output_key),
        ToolType::Click => build_click_config(&params.output_key),
        ToolType::InputText => build_text_config(
            params.text.as_deref().unwrap_or_default(),
            &params.output_key,
        ),
        ToolType::SetText => build_text_config(
            params.text.as_deref().unwrap_or_default(),
            &params.output_key,
        ),
        ToolType::Wait => build_wait_config(params.duration_ms.unwrap_or(1000)),
    }
}

// ── Config builders ──────────────────────────────────────

/// Builds a `windows.find` config from a captured selector.
///
/// Selector priority:
/// 1. `automation_id` — if present, use as the single most stable field.
/// 2. `name` + `control_type` — if `automation_id` absent.
/// 3. `class_name` + `control_type` — fallback.
///
/// `control_type` is always included when available and not `Custom`.
fn build_find_config(sel: &BestSelector, output_key: &str) -> Value {
    let mut map = serde_json::Map::new();
    map.insert("output_key".into(), Value::String(output_key.to_string()));

    // automation_id is the most stable selector
    if let Some(ref aid) = sel.automation_id {
        map.insert("automation_id".into(), Value::String(aid.clone()));
    }

    // name is descriptive and helps match
    if let Some(ref name) = sel.name {
        map.insert("name".into(), Value::String(name.clone()));
    }

    // control_type narrows the search
    if sel.control_type != "Custom" && !sel.control_type.is_empty() {
        map.insert(
            "control_type".into(),
            Value::String(sel.control_type.clone()),
        );
    }

    // class_name as additional filter
    if let Some(ref cn) = sel.class_name {
        map.insert("class_name".into(), Value::String(cn.clone()));
    }

    Value::Object(map)
}

fn build_click_config(element_key: &str) -> Value {
    json!({ "element_key": element_key })
}

fn build_text_config(text: &str, element_key: &str) -> Value {
    // input_text supports element_key OR inline selector fields
    // set_text also supports element_key
    json!({
        "text": text,
        "element_key": element_key
    })
}

fn build_wait_config(duration_ms: u64) -> Value {
    json!({ "duration_ms": duration_ms })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_selector() -> BestSelector {
        BestSelector {
            control_type: "Button".into(),
            name: Some("Submit".into()),
            class_name: Some("ButtonClass".into()),
            automation_id: Some("btnSubmit".into()),
        }
    }

    #[test]
    fn test_build_find_config_includes_output_key() {
        let sel = sample_selector();
        let config = build_find_config(&sel, "el_01");
        assert_eq!(config["output_key"], "el_01");
        assert_eq!(config["name"], "Submit");
        assert_eq!(config["automation_id"], "btnSubmit");
        assert_eq!(config["control_type"], "Button");
        assert_eq!(config["class_name"], "ButtonClass");
    }

    #[test]
    fn test_build_find_config_minimal() {
        let sel = BestSelector {
            control_type: "Pane".into(),
            name: None,
            class_name: None,
            automation_id: None,
        };
        let config = build_find_config(&sel, "el_01");
        assert_eq!(config["output_key"], "el_01");
        assert!(config.get("name").is_none());
        assert!(config.get("automation_id").is_none());
        // Pane is not "Custom" so control_type should be included
        assert_eq!(config["control_type"], "Pane");
    }

    #[test]
    fn test_build_find_config_skips_custom_control_type() {
        let sel = BestSelector {
            control_type: "Custom".into(),
            name: Some("MyElement".into()),
            class_name: None,
            automation_id: None,
        };
        let config = build_find_config(&sel, "el_01");
        assert_eq!(config["name"], "MyElement");
        // "Custom" control_type should be excluded
        assert!(config.get("control_type").is_none());
    }

    #[test]
    fn test_generate_nodes_find() {
        let sel = sample_selector();
        let params = GenerateParams {
            output_key: "el_01".into(),
            text: None,
            duration_ms: None,
        };
        let nodes = generate_nodes(&sel, ToolType::Find, &params);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].tool, "windows.find");
        assert_eq!(nodes[0].args["output_key"], "el_01");
    }

    #[test]
    fn test_generate_nodes_click() {
        let sel = sample_selector();
        let params = GenerateParams {
            output_key: "el_01".into(),
            text: None,
            duration_ms: None,
        };
        let nodes = generate_nodes(&sel, ToolType::Click, &params);
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].tool, "windows.find");
        assert_eq!(nodes[1].tool, "windows.click");
        assert_eq!(nodes[1].args["element_key"], "el_01");
    }

    #[test]
    fn test_generate_nodes_input_text() {
        let sel = sample_selector();
        let params = GenerateParams {
            output_key: "el_01".into(),
            text: Some("Hello".into()),
            duration_ms: None,
        };
        let nodes = generate_nodes(&sel, ToolType::InputText, &params);
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].tool, "windows.find");
        assert_eq!(nodes[1].tool, "windows.input_text");
        assert_eq!(nodes[1].args["text"], "Hello");
        assert_eq!(nodes[1].args["element_key"], "el_01");
    }

    #[test]
    fn test_generate_nodes_set_text() {
        let sel = sample_selector();
        let params = GenerateParams {
            output_key: "el_01".into(),
            text: Some("test".into()),
            duration_ms: None,
        };
        let nodes = generate_nodes(&sel, ToolType::SetText, &params);
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[1].tool, "windows.set_text");
        assert_eq!(nodes[1].args["text"], "test");
    }

    #[test]
    fn test_generate_nodes_wait() {
        let sel = sample_selector();
        let params = GenerateParams {
            output_key: "el_01".into(),
            text: None,
            duration_ms: Some(2000),
        };
        let nodes = generate_nodes(&sel, ToolType::Wait, &params);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].tool, "windows.wait");
        assert_eq!(nodes[0].args["duration_ms"], 2000);
    }

    #[test]
    fn test_generate_action_config_click() {
        let sel = sample_selector();
        let params = GenerateParams {
            output_key: "el_01".into(),
            text: None,
            duration_ms: None,
        };
        let cfg = generate_action_config(&sel, ToolType::Click, &params);
        assert_eq!(cfg["element_key"], "el_01");
        assert!(cfg.get("output_key").is_none());
    }
}
