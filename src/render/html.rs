use serde::Serialize;
use thiserror::Error;

use super::{RenderView, environment_view, global_view};
use crate::model::Graph;

pub const DEFAULT_FILENAME: &str = "diagraph.html";

const TEMPLATE: &str = include_str!("html/template.html");
const PLACEHOLDER: &str = "__DIAGRAPH_DATA__";

#[derive(Debug, Error)]
pub enum HtmlError {
    #[error("serializing graph data to JSON")]
    Serialize(#[source] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, HtmlError>;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonNode {
    id: String,
    label: String,
    kind: &'static str,
    environment: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsonEdge {
    from: String,
    to: String,
    label: Option<String>,
    cross_environment: bool,
}

#[derive(Serialize)]
struct JsonView {
    nodes: Vec<JsonNode>,
    edges: Vec<JsonEdge>,
}

#[derive(Serialize)]
struct Views {
    global: JsonView,
    environment: JsonView,
}

#[derive(Serialize)]
struct Payload {
    environments: Vec<String>,
    views: Views,
}

fn to_json_view(v: RenderView) -> JsonView {
    JsonView {
        nodes: v
            .nodes
            .into_iter()
            .map(|n| JsonNode {
                id: n.id,
                label: n.label,
                kind: n.kind.as_str(),
                environment: n.environment,
            })
            .collect(),
        edges: v
            .edges
            .into_iter()
            .map(|e| JsonEdge {
                from: e.from,
                to: e.to,
                label: e.label,
                cross_environment: e.cross_environment,
            })
            .collect(),
    }
}

/// Escapes every `<` in a JSON payload to its numeric unicode escape, so a
/// Component/External name containing `</script>` can never prematurely
/// close the `<script>` tag it's embedded in.
fn escape_less_than(json: &str) -> String {
    let mut out = String::with_capacity(json.len());
    for c in json.chars() {
        if c == '<' {
            out.push_str(&format!("\\u{:04x}", c as u32));
        } else {
            out.push(c);
        }
    }
    out
}

pub fn render(graph: &Graph) -> Result<String> {
    let payload = Payload {
        environments: graph.environments(),
        views: Views {
            global: to_json_view(global_view(graph)),
            environment: to_json_view(environment_view(graph)),
        },
    };
    let json = serde_json::to_string(&payload).map_err(HtmlError::Serialize)?;
    let json_safe = escape_less_than(&json);
    Ok(TEMPLATE.replacen(PLACEHOLDER, &json_safe, 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Component, Edge, EdgeTarget};
    use std::path::PathBuf;

    fn sample_graph() -> Graph {
        Graph {
            components: vec![
                Component {
                    name: "api-gateway".to_string(),
                    dir: PathBuf::from("api-gateway"),
                    environment: Some("cloud".to_string()),
                    edges: vec![Edge {
                        target: EdgeTarget::Component("user-service".to_string()),
                        via: Some("gRPC".to_string()),
                        data: None,
                        from_part: None,
                        to_part: None,
                    }],
                    parts: vec![],
                },
                Component {
                    name: "user-service".to_string(),
                    dir: PathBuf::from("user-service"),
                    environment: Some("cloud".to_string()),
                    edges: vec![],
                    parts: vec![],
                },
            ],
        }
    }

    fn extract_payload(html: &str) -> serde_json::Value {
        let needle = "id=\"diagraph-data\">";
        let marker_start = html.find(needle).unwrap() + needle.len();
        let marker_end = html[marker_start..].find("</script>").unwrap() + marker_start;
        serde_json::from_str(&html[marker_start..marker_end]).unwrap()
    }

    #[test]
    fn embedded_payload_round_trips_as_json() {
        let html = render(&sample_graph()).unwrap();
        let payload = extract_payload(&html);
        assert_eq!(payload["environments"][0], "cloud");
        assert!(payload["views"]["global"]["nodes"].is_array());
        assert!(payload["views"]["environment"]["nodes"].is_array());
    }

    #[test]
    fn label_containing_script_close_tag_does_not_break_out() {
        let mut graph = sample_graph();
        graph.components[0].name = "</script><script>alert(1)".to_string();
        let html = render(&graph).unwrap();
        assert!(!html.contains("</script><script>alert(1)"));
        let payload = extract_payload(&html);
        let label = payload["views"]["global"]["nodes"][0]["label"]
            .as_str()
            .unwrap();
        assert!(label.contains("</script>"));
    }

    #[test]
    fn output_is_self_contained_html() {
        let html = render(&sample_graph()).unwrap();
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(!html.contains("cdn."));
        assert!(!html.contains("<script src="));
        assert!(!html.contains("<link"));
    }
}
