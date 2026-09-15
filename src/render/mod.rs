use std::collections::HashMap;

use thiserror::Error;

use crate::model::{Edge, EdgeTarget, Graph, PartEdge};

pub mod dot;
pub mod html;
pub mod mermaid;

pub type Result<T> = std::result::Result<T, RenderError>;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("no Component named \"{0}\" was found (does it match a Cargo.toml/package.json name?)")]
    UnknownComponent(String),
}

pub struct RenderView {
    pub nodes: Vec<NodeView>,
    pub edges: Vec<EdgeView>,
}

pub struct NodeView {
    pub id: String,
    pub label: String,
    pub kind: NodeKind,
    pub environment: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Component,
    External,
    Environment,
    Part,
}

impl NodeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            NodeKind::Component => "component",
            NodeKind::External => "external",
            NodeKind::Environment => "environment",
            NodeKind::Part => "part",
        }
    }
}

pub struct EdgeView {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub cross_environment: bool,
}

/// Every declared Component as a node, every Edge as an edge; External
/// targets get their own deduped nodes (the same External name used from two
/// Components converges on one node).
pub fn global_view(graph: &Graph) -> RenderView {
    let mut nodes = Vec::new();
    let mut ids = HashMap::new();
    for c in &graph.components {
        resolve_node(
            &mut nodes,
            &mut ids,
            &c.name,
            NodeKind::Component,
            c.environment.clone(),
        );
    }

    let mut edges = Vec::new();
    for c in &graph.components {
        let from_id = ids[&c.name].clone();
        for e in &c.edges {
            let (name, kind) = target_name_and_kind(&e.target);
            let to_id = resolve_node(&mut nodes, &mut ids, &name, kind, None);
            edges.push(EdgeView {
                from: from_id.clone(),
                to: to_id,
                label: edge_label(e),
                cross_environment: graph.crosses_environment(c, e),
            });
        }
    }

    RenderView { nodes, edges }
}

/// One named Component plus its direct neighbors: its own outgoing edges
/// (including External targets) and every other Component's edge that
/// targets it (incoming, derived by scanning — Edges are outgoing-only).
pub fn component_view(graph: &Graph, name: &str) -> Result<RenderView> {
    let center = graph
        .find(name)
        .ok_or_else(|| RenderError::UnknownComponent(name.to_string()))?;

    let mut nodes = Vec::new();
    let mut ids = HashMap::new();
    let center_id = resolve_node(
        &mut nodes,
        &mut ids,
        &center.name,
        NodeKind::Component,
        center.environment.clone(),
    );

    let mut edges = Vec::new();
    for e in &center.edges {
        let (target_name, kind) = target_name_and_kind(&e.target);
        let env = graph.environment_of(&e.target).map(str::to_string);
        let to_id = resolve_node(&mut nodes, &mut ids, &target_name, kind, env);
        edges.push(EdgeView {
            from: center_id.clone(),
            to: to_id,
            label: edge_label(e),
            cross_environment: graph.crosses_environment(center, e),
        });
    }

    for other in &graph.components {
        if other.name == center.name {
            continue;
        }
        for e in &other.edges {
            if matches!(&e.target, EdgeTarget::Component(n) if n == &center.name) {
                let from_id = resolve_node(
                    &mut nodes,
                    &mut ids,
                    &other.name,
                    NodeKind::Component,
                    other.environment.clone(),
                );
                edges.push(EdgeView {
                    from: from_id,
                    to: center_id.clone(),
                    label: edge_label(e),
                    cross_environment: graph.crosses_environment(other, e),
                });
            }
        }
    }

    Ok(RenderView { nodes, edges })
}

/// Nodes = `Graph::environments()`, edges = `Graph::environment_edges()`
/// (already deduped, already excludes same-environment and external-target
/// edges).
pub fn environment_view(graph: &Graph) -> RenderView {
    let mut nodes = Vec::new();
    let mut ids = HashMap::new();
    for env in graph.environments() {
        resolve_node(
            &mut nodes,
            &mut ids,
            &env,
            NodeKind::Environment,
            Some(env.clone()),
        );
    }

    let edges = graph
        .environment_edges()
        .into_iter()
        .map(|(from_env, to_env)| EdgeView {
            from: ids[&from_env].clone(),
            to: ids[&to_env].clone(),
            label: None,
            cross_environment: true,
        })
        .collect();

    RenderView { nodes, edges }
}

/// Scoped to one Component: its Parts, the internal Part-to-Part edges
/// between them, and every Edge touching the Component redrawn at the
/// specific Part named by `from_part`/`to_part` when known. An Edge with no
/// such attribution attaches to the Component's own node instead of being
/// dropped, so a Component with no declared Parts still renders a sensible
/// (Component-view-like) diagram.
pub fn zoomed_view(graph: &Graph, name: &str) -> Result<RenderView> {
    let center = graph
        .find(name)
        .ok_or_else(|| RenderError::UnknownComponent(name.to_string()))?;

    let mut nodes = Vec::new();
    let mut ids = HashMap::new();
    let center_id = resolve_node(
        &mut nodes,
        &mut ids,
        &center.name,
        NodeKind::Component,
        center.environment.clone(),
    );

    // A Part's name is only unique within its own Component, so it must never
    // share a namespace with `ids` (real Component/External Names) — a Part
    // could otherwise collide with an unrelated neighbor of the same name.
    let mut part_ids: HashMap<&str, String> = HashMap::new();
    for part in &center.parts {
        let id = format!("n{}", nodes.len());
        nodes.push(NodeView {
            id: id.clone(),
            label: part.name.clone(),
            kind: NodeKind::Part,
            environment: center.environment.clone(),
        });
        part_ids.insert(&part.name, id);
    }

    let mut edges = Vec::new();

    for part in &center.parts {
        let from_id = part_ids[part.name.as_str()].clone();
        for part_edge in &part.edges {
            let Some(to_id) = part_ids.get(part_edge.target.as_str()).cloned() else {
                continue; // validate() reports this as UnknownPartEdgeTarget
            };
            edges.push(EdgeView {
                from: from_id.clone(),
                to: to_id,
                label: part_edge_label(part_edge),
                cross_environment: false,
            });
        }
    }

    for e in &center.edges {
        let origin = e
            .from_part
            .as_deref()
            .and_then(|p| part_ids.get(p))
            .cloned()
            .unwrap_or_else(|| center_id.clone());
        let (target_name, kind) = target_name_and_kind(&e.target);
        let env = graph.environment_of(&e.target).map(str::to_string);
        let to_id = resolve_node(&mut nodes, &mut ids, &target_name, kind, env);
        edges.push(EdgeView {
            from: origin,
            to: to_id,
            label: with_remote_part_hint(edge_label(e), e.to_part.as_deref()),
            cross_environment: graph.crosses_environment(center, e),
        });
    }

    for other in &graph.components {
        if other.name == center.name {
            continue;
        }
        for e in &other.edges {
            if matches!(&e.target, EdgeTarget::Component(n) if n == &center.name) {
                let from_id = resolve_node(
                    &mut nodes,
                    &mut ids,
                    &other.name,
                    NodeKind::Component,
                    other.environment.clone(),
                );
                let dest = e
                    .to_part
                    .as_deref()
                    .and_then(|p| part_ids.get(p))
                    .cloned()
                    .unwrap_or_else(|| center_id.clone());
                edges.push(EdgeView {
                    from: from_id,
                    to: dest,
                    label: with_remote_part_hint(edge_label(e), e.from_part.as_deref()),
                    cross_environment: graph.crosses_environment(other, e),
                });
            }
        }
    }

    Ok(RenderView { nodes, edges })
}

fn target_name_and_kind(target: &EdgeTarget) -> (String, NodeKind) {
    match target {
        EdgeTarget::Component(n) => (n.clone(), NodeKind::Component),
        EdgeTarget::External(n) => (n.clone(), NodeKind::External),
    }
}

fn combine_via_data(via: &Option<String>, data: &Option<String>) -> Option<String> {
    match (via, data) {
        (Some(via), Some(data)) => Some(format!("{via} ({data})")),
        (Some(via), None) => Some(via.clone()),
        (None, Some(data)) => Some(format!("data: {data}")),
        (None, None) => None,
    }
}

fn edge_label(edge: &Edge) -> Option<String> {
    combine_via_data(&edge.via, &edge.data)
}

fn part_edge_label(edge: &PartEdge) -> Option<String> {
    combine_via_data(&edge.via, &edge.data)
}

/// Appends the remote side's Part attribution to a label, when known — the
/// remote Component isn't itself zoomed in here, so its internal detail can
/// only surface as a hint on the edge.
fn with_remote_part_hint(base: Option<String>, remote_part: Option<&str>) -> Option<String> {
    match (base, remote_part) {
        (Some(b), Some(p)) => Some(format!("{b} → {p}")),
        (None, Some(p)) => Some(format!("→ {p}")),
        (base, None) => base,
    }
}

/// Looks up or lazily creates a node, keyed by its display name. A Component
/// named identically to an External target string is treated as the same
/// node — an accepted simplification, since Names and External target
/// strings share one namespace in the rendered graph.
fn resolve_node(
    nodes: &mut Vec<NodeView>,
    ids: &mut HashMap<String, String>,
    name: &str,
    kind: NodeKind,
    environment: Option<String>,
) -> String {
    ids.entry(name.to_string())
        .or_insert_with(|| {
            let id = format!("n{}", nodes.len());
            nodes.push(NodeView {
                id: id.clone(),
                label: name.to_string(),
                kind,
                environment,
            });
            id
        })
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Component, Part};
    use std::path::PathBuf;

    fn component(name: &str, environment: Option<&str>, edges: Vec<Edge>) -> Component {
        Component {
            name: name.to_string(),
            dir: PathBuf::from(name),
            environment: environment.map(str::to_string),
            edges,
            parts: vec![],
        }
    }

    fn edge(target: EdgeTarget) -> Edge {
        Edge {
            target,
            via: None,
            data: None,
            from_part: None,
            to_part: None,
        }
    }

    fn part(name: &str, edges: Vec<PartEdge>) -> Part {
        Part {
            name: name.to_string(),
            edges,
        }
    }

    fn sample_graph() -> Graph {
        Graph {
            components: vec![
                component(
                    "api-gateway",
                    Some("cloud"),
                    vec![edge(EdgeTarget::Component("user-service".to_string()))],
                ),
                component(
                    "user-service",
                    Some("cloud"),
                    vec![edge(EdgeTarget::External("s3-bucket".to_string()))],
                ),
                component(
                    "field-agent",
                    Some("iot"),
                    vec![edge(EdgeTarget::Component("api-gateway".to_string()))],
                ),
            ],
        }
    }

    #[test]
    fn global_view_has_a_node_per_component_and_external_target() {
        let graph = sample_graph();
        let view = global_view(&graph);
        assert_eq!(view.nodes.len(), 4); // 3 components + 1 external
        assert_eq!(view.edges.len(), 3);
        let external_count = view
            .nodes
            .iter()
            .filter(|n| n.kind == NodeKind::External)
            .count();
        assert_eq!(external_count, 1);
    }

    #[test]
    fn global_view_flags_only_the_cross_environment_edge() {
        let graph = sample_graph();
        let view = global_view(&graph);
        let cross_count = view.edges.iter().filter(|e| e.cross_environment).count();
        assert_eq!(cross_count, 1);
    }

    #[test]
    fn external_targets_with_the_same_name_converge_on_one_node() {
        let graph = Graph {
            components: vec![
                component(
                    "report-generator",
                    Some("cloud"),
                    vec![edge(EdgeTarget::External("s3-reports-bucket".to_string()))],
                ),
                component(
                    "backup-service",
                    Some("cloud"),
                    vec![edge(EdgeTarget::External("s3-reports-bucket".to_string()))],
                ),
            ],
        };
        let view = global_view(&graph);
        assert_eq!(view.nodes.len(), 3); // 2 components + 1 shared external
        assert_eq!(view.edges.len(), 2);
    }

    #[test]
    fn component_view_includes_incoming_and_outgoing_edges() {
        let graph = sample_graph();
        let view = component_view(&graph, "api-gateway").unwrap();
        // api-gateway -> user-service (outgoing), field-agent -> api-gateway (incoming)
        assert_eq!(view.edges.len(), 2);
        assert_eq!(view.nodes.len(), 3);
    }

    #[test]
    fn component_view_errors_on_unknown_name() {
        let graph = sample_graph();
        assert!(component_view(&graph, "does-not-exist").is_err());
    }

    #[test]
    fn environment_view_matches_graph_environment_edges() {
        let graph = sample_graph();
        let view = environment_view(&graph);
        assert_eq!(view.nodes.len(), 2);
        assert_eq!(view.edges.len(), graph.environment_edges().len());
        assert!(view.edges.iter().all(|e| e.cross_environment));
    }

    fn report_generator_with_parts() -> Component {
        Component {
            parts: vec![
                part(
                    "fetch-thread",
                    vec![PartEdge {
                        target: "upload-thread".to_string(),
                        via: Some("channel".to_string()),
                        data: None,
                    }],
                ),
                part("upload-thread", vec![]),
            ],
            edges: vec![Edge {
                from_part: Some("upload-thread".to_string()),
                ..edge(EdgeTarget::External("s3-reports-bucket".to_string()))
            }],
            ..component("report-generator", Some("cloud"), vec![])
        }
    }

    #[test]
    fn zoomed_view_includes_parts_and_their_internal_edges() {
        let graph = Graph {
            components: vec![report_generator_with_parts()],
        };
        let view = zoomed_view(&graph, "report-generator").unwrap();
        let part_nodes = view
            .nodes
            .iter()
            .filter(|n| n.kind == NodeKind::Part)
            .count();
        assert_eq!(part_nodes, 2);
        let internal_edge = view
            .edges
            .iter()
            .find(|e| e.label.as_deref() == Some("channel"));
        assert!(internal_edge.is_some());
    }

    #[test]
    fn zoomed_view_attributes_outgoing_edge_to_its_from_part() {
        let graph = Graph {
            components: vec![report_generator_with_parts()],
        };
        let view = zoomed_view(&graph, "report-generator").unwrap();
        let upload_thread_id = view
            .nodes
            .iter()
            .find(|n| n.label == "upload-thread")
            .unwrap()
            .id
            .clone();
        let external_edge = view
            .edges
            .iter()
            .find(|e| e.from == upload_thread_id)
            .unwrap();
        let external_node = view
            .nodes
            .iter()
            .find(|n| n.id == external_edge.to)
            .unwrap();
        assert_eq!(external_node.kind, NodeKind::External);
    }

    #[test]
    fn zoomed_view_attaches_unattributed_edge_to_the_center_node() {
        let graph = Graph {
            components: vec![
                component(
                    "api-gateway",
                    Some("cloud"),
                    vec![edge(EdgeTarget::Component("report-generator".to_string()))],
                ),
                report_generator_with_parts(),
            ],
        };
        let view = zoomed_view(&graph, "report-generator").unwrap();
        let center_id = view
            .nodes
            .iter()
            .find(|n| n.label == "report-generator" && n.kind == NodeKind::Component)
            .unwrap()
            .id
            .clone();
        let incoming = view.edges.iter().find(|e| e.to == center_id).unwrap();
        let sender = view.nodes.iter().find(|n| n.id == incoming.from).unwrap();
        assert_eq!(sender.label, "api-gateway");
    }

    #[test]
    fn zoomed_view_attributes_incoming_edge_to_its_to_part() {
        let graph = Graph {
            components: vec![
                component(
                    "api-gateway",
                    Some("cloud"),
                    vec![Edge {
                        to_part: Some("fetch-thread".to_string()),
                        ..edge(EdgeTarget::Component("report-generator".to_string()))
                    }],
                ),
                report_generator_with_parts(),
            ],
        };
        let view = zoomed_view(&graph, "report-generator").unwrap();
        let fetch_thread_id = view
            .nodes
            .iter()
            .find(|n| n.label == "fetch-thread")
            .unwrap()
            .id
            .clone();
        let incoming = view
            .edges
            .iter()
            .find(|e| e.to == fetch_thread_id && e.from != fetch_thread_id)
            .unwrap();
        let sender = view.nodes.iter().find(|n| n.id == incoming.from).unwrap();
        assert_eq!(sender.label, "api-gateway");
    }

    #[test]
    fn zoomed_view_part_named_like_a_real_component_does_not_collide() {
        let mut center = report_generator_with_parts();
        center.parts.push(part("user-service", vec![]));
        let graph = Graph {
            components: vec![
                center,
                component(
                    "user-service",
                    Some("cloud"),
                    vec![edge(EdgeTarget::Component("report-generator".to_string()))],
                ),
            ],
        };
        let view = zoomed_view(&graph, "report-generator").unwrap();
        let matching_user_service_nodes: Vec<_> = view
            .nodes
            .iter()
            .filter(|n| n.label == "user-service")
            .collect();
        assert_eq!(matching_user_service_nodes.len(), 2);
        assert!(
            matching_user_service_nodes
                .iter()
                .any(|n| n.kind == NodeKind::Part)
        );
        assert!(
            matching_user_service_nodes
                .iter()
                .any(|n| n.kind == NodeKind::Component)
        );
    }

    #[test]
    fn zoomed_view_errors_on_unknown_name() {
        let graph = sample_graph();
        assert!(zoomed_view(&graph, "does-not-exist").is_err());
    }
}
