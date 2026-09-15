use std::collections::{HashMap, HashSet};

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
    DuplicatePartName {
        component: String,
        part: String,
    },
    UnknownFromPart {
        component: String,
        part: String,
    },
    UnknownToPart {
        from: String,
        to_component: String,
        part: String,
    },
    ToPartOnExternalEdge {
        from: String,
        part: String,
    },
    UnknownPartEdgeTarget {
        component: String,
        part: String,
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
            Problem::DuplicatePartName { component, part } => write!(
                f,
                "Component \"{component}\" declares two Parts named \"{part}\" — Part names \
                 must be unique within their Component"
            ),
            Problem::UnknownFromPart { component, part } => write!(
                f,
                "{component} declares an edge with from_part \"{part}\", which is not a Part \
                 it declares"
            ),
            Problem::UnknownToPart {
                from,
                to_component,
                part,
            } => write!(
                f,
                "{from} declares an edge to \"{to_component}\" with to_part \"{part}\", which \
                 is not a Part {to_component} declares"
            ),
            Problem::ToPartOnExternalEdge { from, part } => write!(
                f,
                "{from} declares an edge with to_part \"{part}\" but the target is marked \
                 external — external targets have no Parts"
            ),
            Problem::UnknownPartEdgeTarget {
                component,
                part,
                target,
            } => write!(
                f,
                "Component \"{component}\"'s Part \"{part}\" declares an internal edge to \
                 \"{target}\", which is not a Part it declares"
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

    for component in &graph.components {
        let mut seen_parts: HashSet<&str> = HashSet::new();
        for part in &component.parts {
            if !seen_parts.insert(part.name.as_str()) {
                problems.push(Problem::DuplicatePartName {
                    component: component.name.clone(),
                    part: part.name.clone(),
                });
            }
        }

        for part in &component.parts {
            for part_edge in &part.edges {
                if component.find_part(&part_edge.target).is_none() {
                    problems.push(Problem::UnknownPartEdgeTarget {
                        component: component.name.clone(),
                        part: part.name.clone(),
                        target: part_edge.target.clone(),
                    });
                }
            }
        }

        for edge in &component.edges {
            if let Some(from_part) = &edge.from_part
                && component.find_part(from_part).is_none()
            {
                problems.push(Problem::UnknownFromPart {
                    component: component.name.clone(),
                    part: from_part.clone(),
                });
            }

            match (&edge.target, &edge.to_part) {
                (EdgeTarget::External(_), Some(to_part)) => {
                    problems.push(Problem::ToPartOnExternalEdge {
                        from: component.name.clone(),
                        part: to_part.clone(),
                    });
                }
                (EdgeTarget::Component(target_name), Some(to_part)) => {
                    if let Some(target) = graph.find(target_name)
                        && target.find_part(to_part).is_none()
                    {
                        problems.push(Problem::UnknownToPart {
                            from: component.name.clone(),
                            to_component: target_name.clone(),
                            part: to_part.clone(),
                        });
                    }
                }
                _ => {}
            }
        }
    }

    problems
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Part, PartEdge};
    use std::path::PathBuf;

    fn component(name: &str, dir: &str, edges: Vec<Edge>) -> Component {
        Component {
            name: name.to_string(),
            dir: PathBuf::from(dir),
            environment: None,
            edges,
            parts: vec![],
        }
    }

    fn component_with_parts(name: &str, edges: Vec<Edge>, parts: Vec<Part>) -> Component {
        Component {
            parts,
            ..component(name, name, edges)
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

    fn part_edge(target: &str) -> PartEdge {
        PartEdge {
            target: target.to_string(),
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

    #[test]
    fn valid_graph_with_parts_and_attribution_has_no_problems() {
        let graph = Graph {
            components: vec![
                component_with_parts(
                    "report-generator",
                    vec![Edge {
                        target: EdgeTarget::External("s3-reports-bucket".to_string()),
                        from_part: Some("upload-thread".to_string()),
                        ..edge(EdgeTarget::External("s3-reports-bucket".to_string()))
                    }],
                    vec![
                        part("fetch-thread", vec![part_edge("upload-thread")]),
                        part("upload-thread", vec![]),
                    ],
                ),
                component_with_parts(
                    "api-gateway",
                    vec![Edge {
                        to_part: Some("fetch-thread".to_string()),
                        ..edge(EdgeTarget::Component("report-generator".to_string()))
                    }],
                    vec![],
                ),
            ],
        };
        assert!(validate(&graph).is_empty());
    }

    #[test]
    fn duplicate_part_name_is_reported() {
        let graph = Graph {
            components: vec![component_with_parts(
                "report-generator",
                vec![],
                vec![part("worker", vec![]), part("worker", vec![])],
            )],
        };
        let problems = validate(&graph);
        assert_eq!(problems.len(), 1);
        assert!(matches!(problems[0], Problem::DuplicatePartName { .. }));
    }

    #[test]
    fn unknown_part_edge_target_is_reported() {
        let graph = Graph {
            components: vec![component_with_parts(
                "report-generator",
                vec![],
                vec![part("fetch-thread", vec![part_edge("does-not-exist")])],
            )],
        };
        let problems = validate(&graph);
        assert_eq!(problems.len(), 1);
        assert!(matches!(
            &problems[0],
            Problem::UnknownPartEdgeTarget { target, .. } if target == "does-not-exist"
        ));
    }

    #[test]
    fn unknown_from_part_is_reported() {
        let graph = Graph {
            components: vec![component_with_parts(
                "report-generator",
                vec![Edge {
                    from_part: Some("does-not-exist".to_string()),
                    ..edge(EdgeTarget::External("s3-bucket".to_string()))
                }],
                vec![],
            )],
        };
        let problems = validate(&graph);
        assert_eq!(problems.len(), 1);
        assert!(matches!(
            &problems[0],
            Problem::UnknownFromPart { part, .. } if part == "does-not-exist"
        ));
    }

    #[test]
    fn unknown_to_part_on_known_component_target_is_reported() {
        let graph = Graph {
            components: vec![
                component_with_parts(
                    "api-gateway",
                    vec![Edge {
                        to_part: Some("does-not-exist".to_string()),
                        ..edge(EdgeTarget::Component("report-generator".to_string()))
                    }],
                    vec![],
                ),
                component_with_parts("report-generator", vec![], vec![]),
            ],
        };
        let problems = validate(&graph);
        assert_eq!(problems.len(), 1);
        assert!(matches!(
            &problems[0],
            Problem::UnknownToPart { part, .. } if part == "does-not-exist"
        ));
    }

    #[test]
    fn to_part_on_external_edge_is_reported() {
        let graph = Graph {
            components: vec![component_with_parts(
                "report-generator",
                vec![Edge {
                    to_part: Some("worker".to_string()),
                    ..edge(EdgeTarget::External("s3-bucket".to_string()))
                }],
                vec![],
            )],
        };
        let problems = validate(&graph);
        assert_eq!(problems.len(), 1);
        assert!(matches!(
            &problems[0],
            Problem::ToPartOnExternalEdge { part, .. } if part == "worker"
        ));
    }

    #[test]
    fn missing_target_component_reports_only_dangling_reference_not_unknown_to_part() {
        let graph = Graph {
            components: vec![component_with_parts(
                "api-gateway",
                vec![Edge {
                    to_part: Some("worker".to_string()),
                    ..edge(EdgeTarget::Component("missing-service".to_string()))
                }],
                vec![],
            )],
        };
        let problems = validate(&graph);
        assert_eq!(problems.len(), 1);
        assert!(matches!(problems[0], Problem::DanglingReference { .. }));
    }
}
