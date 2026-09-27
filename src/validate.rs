use std::collections::HashMap;
use std::path::PathBuf;

use crate::manifest::MANIFEST_FILE_NAME;
#[cfg(test)]
use crate::model::Edge;
use crate::model::{Component, EdgeTarget, Graph};

#[derive(Debug)]
pub struct Problem {
    pub manifest: PathBuf,
    pub kind: ProblemKind,
}

#[derive(Debug)]
pub enum ProblemKind {
    DuplicateName {
        name: String,
        first: PathBuf,
        second: PathBuf,
    },
    DanglingReference {
        from: String,
        target: String,
        suggestion: Option<String>,
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

impl std::fmt::Display for ProblemKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProblemKind::DuplicateName {
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
            ProblemKind::DanglingReference {
                from,
                target,
                suggestion: Some(suggestion),
            } => write!(
                f,
                "{from} declares an edge to \"{target}\", which is not a known Component — \
                 did you mean \"{suggestion}\"?"
            ),
            ProblemKind::DanglingReference {
                from,
                target,
                suggestion: None,
            } => write!(
                f,
                "{from} declares an edge to \"{target}\", which is not a known Component. \
                 If this points outside the monorepo, mark it `external = true`."
            ),
            ProblemKind::UnknownFromPart { component, part } => write!(
                f,
                "{component} declares an edge with from_part \"{part}\", which is not a Part \
                 it declares"
            ),
            ProblemKind::UnknownToPart {
                from,
                to_component,
                part,
            } => write!(
                f,
                "{from} declares an edge to \"{to_component}\" with to_part \"{part}\", which \
                 is not a Part {to_component} declares"
            ),
            ProblemKind::ToPartOnExternalEdge { from, part } => write!(
                f,
                "{from} declares an edge with to_part \"{part}\" but the target is marked \
                 external — external targets have no Parts"
            ),
            ProblemKind::UnknownPartEdgeTarget {
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

fn closest<'a>(needle: &str, candidates: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    let max_distance = (needle.chars().count() / 3).max(1);
    candidates
        .into_iter()
        .map(|candidate| (levenshtein(needle, candidate), candidate))
        .filter(|&(distance, _)| distance <= max_distance)
        .min()
        .map(|(_, candidate)| candidate)
}

fn levenshtein(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = (above + 1)
                .min(row[j] + 1)
                .min(diagonal + usize::from(ca != cb));
            diagonal = above;
        }
    }
    row[b.len()]
}

fn problem(component: &Component, kind: ProblemKind) -> Problem {
    Problem {
        manifest: component.dir.join(MANIFEST_FILE_NAME),
        kind,
    }
}

pub fn validate(graph: &Graph) -> Vec<Problem> {
    let mut problems = Vec::new();

    let mut seen: HashMap<&str, &PathBuf> = HashMap::new();
    for component in &graph.components {
        if let Some(&first) = seen.get(component.name.as_str()) {
            problems.push(problem(
                component,
                ProblemKind::DuplicateName {
                    name: component.name.clone(),
                    first: first.clone(),
                    second: component.dir.clone(),
                },
            ));
        } else {
            seen.insert(&component.name, &component.dir);
        }
    }

    for component in &graph.components {
        for edge in &component.edges {
            if let EdgeTarget::Component(name) = &edge.target
                && graph.find(name).is_none()
            {
                problems.push(problem(
                    component,
                    ProblemKind::DanglingReference {
                        from: component.name.clone(),
                        target: name.clone(),
                        suggestion: closest(name, graph.components.iter().map(|c| c.name.as_str()))
                            .map(str::to_owned),
                    },
                ));
            }
        }
    }

    for component in &graph.components {
        for part in &component.parts {
            for part_edge in &part.edges {
                if component.find_part(&part_edge.target).is_none() {
                    problems.push(problem(
                        component,
                        ProblemKind::UnknownPartEdgeTarget {
                            component: component.name.clone(),
                            part: part.name.clone(),
                            target: part_edge.target.clone(),
                        },
                    ));
                }
            }
        }

        for edge in &component.edges {
            if let Some(from_part) = &edge.from_part
                && component.find_part(from_part).is_none()
            {
                problems.push(problem(
                    component,
                    ProblemKind::UnknownFromPart {
                        component: component.name.clone(),
                        part: from_part.clone(),
                    },
                ));
            }

            match (&edge.target, &edge.to_part) {
                (EdgeTarget::External(_), Some(to_part)) => {
                    problems.push(problem(
                        component,
                        ProblemKind::ToPartOnExternalEdge {
                            from: component.name.clone(),
                            part: to_part.clone(),
                        },
                    ));
                }
                (EdgeTarget::Component(target_name), Some(to_part)) => {
                    if let Some(target) = graph.find(target_name)
                        && target.find_part(to_part).is_none()
                    {
                        problems.push(problem(
                            component,
                            ProblemKind::UnknownToPart {
                                from: component.name.clone(),
                                to_component: target_name.clone(),
                                part: to_part.clone(),
                            },
                        ));
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
    use PathBuf;

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
        assert!(matches!(
            problems[0].kind,
            ProblemKind::DuplicateName { .. }
        ));
        assert_eq!(problems[0].manifest, PathBuf::from("dir-b/diagraph.toml"));
    }

    #[test]
    fn levenshtein_counts_edits() {
        assert_eq!(levenshtein("", ""), 0);
        assert_eq!(levenshtein("abc", ""), 3);
        assert_eq!(levenshtein("user-servcie", "user-service"), 2);
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("héllo", "hello"), 1);
    }

    #[test]
    fn closest_respects_the_distance_threshold() {
        assert_eq!(
            closest("user-servcie", ["user-service"]),
            Some("user-service")
        );
        assert_eq!(closest("ab", ["ax"]), Some("ax"));
        assert_eq!(closest("ab", ["xy"]), None);
        assert_eq!(closest("billing", ["user-service"]), None);
    }

    #[test]
    fn closest_breaks_ties_alphabetically() {
        assert_eq!(closest("cat", ["cut", "bat"]), Some("bat"));
    }

    #[test]
    fn dangling_reference_suggests_the_closest_component() {
        let graph = Graph {
            components: vec![
                component(
                    "api-gateway",
                    "api-gateway",
                    vec![edge(EdgeTarget::Component("user-servcie".to_string()))],
                ),
                component("user-service", "user-service", vec![]),
            ],
        };
        let problems = validate(&graph);
        assert_eq!(problems.len(), 1);
        let message = problems[0].kind.to_string();
        assert!(message.contains(r#"did you mean "user-service"?"#));
        assert!(!message.contains("external = true"));
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
            &problems[0].kind,
            ProblemKind::DanglingReference { from, target, suggestion: None }
                if from == "api-gateway" && target == "missing-service"
        ));
        assert!(problems[0].kind.to_string().contains("external = true"));
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
            &problems[0].kind,
            ProblemKind::UnknownPartEdgeTarget { target, .. } if target == "does-not-exist"
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
            &problems[0].kind,
            ProblemKind::UnknownFromPart { part, .. } if part == "does-not-exist"
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
            &problems[0].kind,
            ProblemKind::UnknownToPart { part, .. } if part == "does-not-exist"
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
            &problems[0].kind,
            ProblemKind::ToPartOnExternalEdge { part, .. } if part == "worker"
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
        assert!(matches!(
            problems[0].kind,
            ProblemKind::DanglingReference { .. }
        ));
    }
}
