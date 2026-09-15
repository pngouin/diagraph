use super::{NodeKind, RenderView};

pub fn render(view: &RenderView) -> String {
    let mut out = String::from("digraph diagraph {\n    rankdir=LR;\n");
    for n in &view.nodes {
        let shape = match n.kind {
            NodeKind::Component => "box",
            NodeKind::External => "ellipse",
            NodeKind::Environment => "hexagon",
            NodeKind::Part => "component",
        };
        let dashed = matches!(n.kind, NodeKind::External);
        out.push_str(&format!(
            "    {} [label=\"{}\", shape={}{}];\n",
            n.id,
            escape(&n.label),
            shape,
            if dashed { ", style=dashed" } else { "" }
        ));
    }
    for e in &view.edges {
        let mut attrs = Vec::new();
        if let Some(l) = &e.label {
            attrs.push(format!("label=\"{}\"", escape(l)));
        }
        if e.cross_environment {
            attrs.push("style=dashed".to_string());
        }
        if attrs.is_empty() {
            out.push_str(&format!("    {} -> {};\n", e.from, e.to));
        } else {
            out.push_str(&format!(
                "    {} -> {} [{}];\n",
                e.from,
                e.to,
                attrs.join(", ")
            ));
        }
    }
    out.push_str("}\n");
    out
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::{EdgeView, NodeView};

    fn node(id: &str, label: &str, kind: NodeKind) -> NodeView {
        NodeView {
            id: id.to_string(),
            label: label.to_string(),
            kind,
            environment: None,
        }
    }

    #[test]
    fn external_nodes_get_ellipse_and_dashed_style() {
        let view = RenderView {
            nodes: vec![node("n0", "s3-bucket", NodeKind::External)],
            edges: vec![],
        };
        let out = render(&view);
        assert!(out.contains("shape=ellipse"));
        assert!(out.contains("style=dashed"));
    }

    #[test]
    fn part_nodes_get_component_shape() {
        let view = RenderView {
            nodes: vec![node("n0", "fetch-thread", NodeKind::Part)],
            edges: vec![],
        };
        let out = render(&view);
        assert!(out.contains("shape=component"));
    }

    #[test]
    fn component_nodes_get_box_shape_without_dashed_style() {
        let view = RenderView {
            nodes: vec![node("n0", "api-gateway", NodeKind::Component)],
            edges: vec![],
        };
        let out = render(&view);
        assert!(out.contains("shape=box"));
        assert!(!out.contains("style=dashed"));
    }

    #[test]
    fn cross_environment_edges_get_dashed_style() {
        let view = RenderView {
            nodes: vec![],
            edges: vec![EdgeView {
                from: "n0".to_string(),
                to: "n1".to_string(),
                label: None,
                cross_environment: true,
            }],
        };
        let out = render(&view);
        assert!(out.contains("n0 -> n1 [style=dashed];"));
    }

    #[test]
    fn labels_are_escaped() {
        let view = RenderView {
            nodes: vec![node("n0", "say \"hi\"", NodeKind::Component)],
            edges: vec![],
        };
        let out = render(&view);
        assert!(out.contains("say \\\"hi\\\""));
    }
}
