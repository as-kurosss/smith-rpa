//! Output formatters for generated flow nodes.
//!
//! Supports two output formats:
//! - **JSON** — structured `FlowGraphOutput` with a `nodes` array.
//! - **Rust** — `FlowGraph::builder(...)` code ready to paste into a Rust project.

use crate::types::{FlowGraphOutput, FlowNode, FormatType};

/// Formats a list of `FlowNode`s into the specified output format.
pub fn format_output(nodes: &[FlowNode], format: FormatType) -> String {
    match format {
        FormatType::Json => format_json(nodes),
        FormatType::Rust => format_rust(nodes),
    }
}

/// Formats as compact JSON.
fn format_json(nodes: &[FlowNode]) -> String {
    let output = FlowGraphOutput {
        tool: "selector-capture".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        nodes: nodes.to_vec(),
    };
    serde_json::to_string_pretty(&output).unwrap_or_else(|_| "[]".into())
}

/// Formats as Rust code using `FlowGraph::builder(...)`.
///
/// Each `FlowNode` becomes a `.add_node(Node::rpa(...))` call, consecutive
/// nodes are connected with `.connect(n_i, EdgeKind::Success, n_i+1)`.
fn format_rust(nodes: &[FlowNode]) -> String {
    if nodes.is_empty() {
        return "// No nodes recorded\nlet graph = FlowGraph::builder(\"recorded_flow\").build().expect(\"valid graph\");\n".into();
    }

    let mut out = String::new();

    // Imports
    out.push_str("use serde_json::json;\n");
    out.push_str("use smith_core::RetryPolicy;\n");
    out.push_str("use smith_workflow::{EdgeKind, FlowGraph, Node};\n\n");

    // Builder creation
    out.push_str("let mut g = FlowGraph::builder(\"recorded_flow\");\n");

    // Add nodes
    for (i, node) in nodes.iter().enumerate() {
        let args_json = serde_json::to_string(&node.args).unwrap_or_else(|_| "{}".into());
        out.push_str(&format!(
            "let n{i} = g.add_node(Node::rpa(\"{tool}\", json!({args})));\n",
            i = i,
            tool = node.tool,
            args = args_json,
        ));
    }

    // Connect consecutive nodes with EdgeKind::Success
    for i in 0..nodes.len().saturating_sub(1) {
        out.push_str(&format!(
            "g.connect(n{i}, EdgeKind::Success, n{j});\n",
            i = i,
            j = i + 1
        ));
    }

    // Final build
    out.push_str("let graph = g.build().expect(\"valid graph\");\n");

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_nodes() -> Vec<FlowNode> {
        vec![
            FlowNode {
                tool: "windows.find".into(),
                args: json!({"output_key": "el_01", "name": "Submit", "control_type": "Button"}),
            },
            FlowNode {
                tool: "windows.click".into(),
                args: json!({"element_key": "el_01"}),
            },
        ]
    }

    #[test]
    fn test_format_json_contains_nodes() {
        let nodes = sample_nodes();
        let output = format_json(&nodes);
        assert!(output.contains("windows.find"));
        assert!(output.contains("windows.click"));
        assert!(output.contains("el_01"));
        assert!(output.contains("selector-capture"));
    }

    #[test]
    fn test_format_json_empty() {
        let output = format_json(&[]);
        assert!(output.contains("\"nodes\": []"));
    }

    #[test]
    fn test_format_rust_contains_imports() {
        let nodes = sample_nodes();
        let output = format_rust(&nodes);
        assert!(output.contains("use serde_json::json;"));
        assert!(output.contains("use smith_workflow::{EdgeKind, FlowGraph, Node};"));
    }

    #[test]
    fn test_format_rust_adds_nodes() {
        let nodes = sample_nodes();
        let output = format_rust(&nodes);
        assert!(output.contains("let n0 = g.add_node("));
        assert!(output.contains("let n1 = g.add_node("));
        assert!(output.contains("windows.find"));
        assert!(output.contains("windows.click"));
    }

    #[test]
    fn test_format_rust_connects_consecutive() {
        let nodes = sample_nodes();
        let output = format_rust(&nodes);
        assert!(output.contains("g.connect(n0, EdgeKind::Success, n1);"));
    }

    #[test]
    fn test_format_rust_empty() {
        let output = format_rust(&[]);
        assert!(output.contains("No nodes recorded"));
    }

    #[test]
    fn test_format_rust_single_node() {
        let nodes = vec![FlowNode {
            tool: "windows.wait".into(),
            args: json!({"duration_ms": 1000}),
        }];
        let output = format_rust(&nodes);
        assert!(output.contains("n0 = g.add_node("));
        // No connections for a single node
        assert!(!output.contains("g.connect"));
        assert!(output.contains("windows.wait"));
    }

    #[test]
    fn test_format_output_dispatches() {
        let nodes = sample_nodes();
        let json = format_output(&nodes, FormatType::Json);
        let rust = format_output(&nodes, FormatType::Rust);
        assert!(json.contains("\"tool\""));
        assert!(rust.contains("FlowGraph::builder"));
    }
}
