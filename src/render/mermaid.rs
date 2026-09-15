use super::{NodeKind, RenderView};

pub fn render(view: &RenderView) -> String {
    let mut out = String::from("flowchart LR\n");
    for n in &view.nodes {
        let label = escape(&n.label);
        let shape = match n.kind {
            NodeKind::Component => format!("[\"{label}\"]"),
            NodeKind::External => format!("([\"{label}\"])"),
            NodeKind::Environment => format!("{{{{\"{label}\"}}}}"),
        };
        out.push_str(&format!("    {}{}\n", n.id, shape));
    }
    for e in &view.edges {
        let arrow = if e.cross_environment { "-.->" } else { "-->" };
        match &e.label {
            Some(l) => out.push_str(&format!(
                "    {} {}|\"{}\"| {}\n",
                e.from,
                arrow,
                escape(l),
                e.to
            )),
            None => out.push_str(&format!("    {} {} {}\n", e.from, arrow, e.to)),
        }
    }
    out
}

fn escape(s: &str) -> String {
    s.replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::EdgeView;
    use crate::render::NodeView;

    fn node(id: &str, label: &str, kind: NodeKind) -> NodeView {
        NodeView {
            id: id.to_string(),
            label: label.to_string(),
            kind,
            environment: None,
        }
    }

    #[test]
    fn renders_component_and_external_shapes() {
        let view = RenderView {
            nodes: vec![
                node("n0", "api-gateway", NodeKind::Component),
                node("n1", "s3-bucket", NodeKind::External),
            ],
            edges: vec![],
        };
        let out = render(&view);
        assert!(out.contains("n0[\"api-gateway\"]"));
        assert!(out.contains("n1([\"s3-bucket\"])"));
    }

    #[test]
    fn renders_environment_hexagon_shape() {
        let view = RenderView {
            nodes: vec![node("n0", "cloud", NodeKind::Environment)],
            edges: vec![],
        };
        let out = render(&view);
        assert!(out.contains("n0{{\"cloud\"}}"));
    }

    #[test]
    fn cross_environment_edges_use_dotted_arrow() {
        let view = RenderView {
            nodes: vec![],
            edges: vec![
                EdgeView {
                    from: "n0".to_string(),
                    to: "n1".to_string(),
                    label: None,
                    cross_environment: true,
                },
                EdgeView {
                    from: "n0".to_string(),
                    to: "n1".to_string(),
                    label: None,
                    cross_environment: false,
                },
            ],
        };
        let out = render(&view);
        assert!(out.contains("-.->"));
        assert!(out.contains("-->"));
    }

    #[test]
    fn edge_label_is_quoted_and_escaped() {
        let view = RenderView {
            nodes: vec![],
            edges: vec![EdgeView {
                from: "n0".to_string(),
                to: "n1".to_string(),
                label: Some("gRPC (\"user\" lookup)".to_string()),
                cross_environment: false,
            }],
        };
        let out = render(&view);
        assert!(out.contains("&quot;user&quot;"));
    }
}
