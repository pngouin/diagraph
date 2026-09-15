use std::collections::HashMap;

use anyhow::{Context, Result};

use crate::model::{Edge, EdgeTarget, Graph};

pub mod mermaid;

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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Component,
    External,
    Environment,
}

impl NodeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            NodeKind::Component => "component",
            NodeKind::External => "external",
            NodeKind::Environment => "environment",
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
    let center = graph.find(name).with_context(|| {
        format!("no Component named \"{name}\" was found (does it match a Cargo.toml/package.json name?)")
    })?;

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

fn target_name_and_kind(target: &EdgeTarget) -> (String, NodeKind) {
    match target {
        EdgeTarget::Component(n) => (n.clone(), NodeKind::Component),
        EdgeTarget::External(n) => (n.clone(), NodeKind::External),
    }
}

fn edge_label(edge: &Edge) -> Option<String> {
    match (&edge.via, &edge.data) {
        (Some(via), Some(data)) => Some(format!("{via} ({data})")),
        (Some(via), None) => Some(via.clone()),
        (None, Some(data)) => Some(format!("data: {data}")),
        (None, None) => None,
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
    use crate::model::Component;
    use std::path::PathBuf;

    fn component(name: &str, environment: Option<&str>, edges: Vec<Edge>) -> Component {
        Component {
            name: name.to_string(),
            dir: PathBuf::from(name),
            environment: environment.map(str::to_string),
            edges,
        }
    }

    fn edge(target: EdgeTarget) -> Edge {
        Edge {
            target,
            via: None,
            data: None,
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
}
