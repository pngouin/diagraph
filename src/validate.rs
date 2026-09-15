use std::collections::HashMap;

#[cfg(test)]
use crate::model::{Component, Edge};
use crate::model::{EdgeTarget, Graph};

#[derive(Debug)]
pub enum Problem {
    DuplicateName {
        name: String,
        first: std::path::PathBuf,
        second: std::path::PathBuf,
    },
    DanglingReference {
        from: String,
        target: String,
    },
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Problem::DuplicateName {
                name,
                first,
                second,
            } => write!(
                f,
                "Component name \"{name}\" is declared by both {} and {} — Names must be \
                 globally unique",
                first.display(),
                second.display()
            ),
            Problem::DanglingReference { from, target } => write!(
                f,
                "{from} declares an edge to \"{target}\", which is not a known Component. \
                 If this points outside the monorepo, mark it `external = true`."
            ),
        }
    }
}

pub fn validate(graph: &Graph) -> Vec<Problem> {
    let mut problems = Vec::new();

    let mut seen: HashMap<&str, &std::path::PathBuf> = HashMap::new();
    for component in &graph.components {
        if let Some(&first) = seen.get(component.name.as_str()) {
            problems.push(Problem::DuplicateName {
                name: component.name.clone(),
                first: first.clone(),
                second: component.dir.clone(),
            });
        } else {
            seen.insert(&component.name, &component.dir);
        }
    }

    for component in &graph.components {
        for edge in &component.edges {
            if let EdgeTarget::Component(name) = &edge.target
                && graph.find(name).is_none()
            {
                problems.push(Problem::DanglingReference {
                    from: component.name.clone(),
                    target: name.clone(),
                });
            }
        }
    }

    problems
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn component(name: &str, dir: &str, edges: Vec<Edge>) -> Component {
        Component {
            name: name.to_string(),
            dir: PathBuf::from(dir),
            environment: None,
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

    #[test]
    fn valid_graph_with_external_edge_has_no_problems() {
        let graph = Graph {
            components: vec![
                component(
                    "api-gateway",
                    "api-gateway",
                    vec![edge(EdgeTarget::Component("user-service".to_string()))],
                ),
                component(
                    "user-service",
                    "user-service",
                    vec![edge(EdgeTarget::External("s3-bucket".to_string()))],
                ),
            ],
        };
        assert!(validate(&graph).is_empty());
    }

    #[test]
    fn duplicate_component_names_are_reported() {
        let graph = Graph {
            components: vec![
                component("shared-name", "dir-a", vec![]),
                component("shared-name", "dir-b", vec![]),
            ],
        };
        let problems = validate(&graph);
        assert_eq!(problems.len(), 1);
        assert!(matches!(problems[0], Problem::DuplicateName { .. }));
    }

    #[test]
    fn dangling_reference_is_reported() {
        let graph = Graph {
            components: vec![component(
                "api-gateway",
                "api-gateway",
                vec![edge(EdgeTarget::Component("missing-service".to_string()))],
            )],
        };
        let problems = validate(&graph);
        assert_eq!(problems.len(), 1);
        assert!(matches!(
            &problems[0],
            Problem::DanglingReference { from, target }
                if from == "api-gateway" && target == "missing-service"
        ));
    }
}
