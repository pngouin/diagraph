use std::path::PathBuf;

/// A directory identified solely by the presence of a `diagram.toml` at its root.
#[derive(Debug, Clone)]
pub struct Component {
    pub name: String,
    pub dir: PathBuf,
    pub environment: Option<String>,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub target: EdgeTarget,
    pub via: Option<String>,
    pub data: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EdgeTarget {
    Component(String),
    External(String),
}

impl EdgeTarget {
    pub fn name(&self) -> &str {
        match self {
            EdgeTarget::Component(n) => n,
            EdgeTarget::External(n) => n,
        }
    }
}

pub struct Graph {
    pub components: Vec<Component>,
}

impl Graph {
    pub fn find(&self, name: &str) -> Option<&Component> {
        self.components.iter().find(|c| c.name == name)
    }

    pub fn environment_of(&self, target: &EdgeTarget) -> Option<&str> {
        match target {
            EdgeTarget::Component(name) => self.find(name).and_then(|c| c.environment.as_deref()),
            EdgeTarget::External(_) => None,
        }
    }

    /// Edges between two Components whose Environment values are both known and differ.
    pub fn crosses_environment(&self, from: &Component, edge: &Edge) -> bool {
        match (&from.environment, self.environment_of(&edge.target)) {
            (Some(a), Some(b)) => a != b,
            _ => false,
        }
    }

    /// Distinct (from_env, to_env) pairs where a Component in `from_env` has an Edge
    /// to a Component in `to_env`. External targets are excluded (they have no
    /// Environment). Same-environment pairs are omitted by design.
    pub fn environment_edges(&self) -> Vec<(String, String)> {
        let mut pairs = std::collections::BTreeSet::new();
        for component in &self.components {
            let Some(from_env) = component.environment.as_deref() else {
                continue;
            };
            for edge in &component.edges {
                if let Some(to_env) = self.environment_of(&edge.target)
                    && to_env != from_env
                {
                    pairs.insert((from_env.to_string(), to_env.to_string()));
                }
            }
        }
        pairs.into_iter().collect()
    }

    /// Distinct Environment values in use across all Components.
    pub fn environments(&self) -> Vec<String> {
        let envs: std::collections::BTreeSet<String> = self
            .components
            .iter()
            .filter_map(|c| c.environment.clone())
            .collect();
        envs.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn find_returns_matching_component() {
        let graph = Graph {
            components: vec![component("a", None, vec![]), component("b", None, vec![])],
        };
        assert_eq!(graph.find("b").unwrap().name, "b");
        assert!(graph.find("c").is_none());
    }

    #[test]
    fn environment_of_looks_up_component_environment() {
        let graph = Graph {
            components: vec![component("a", Some("cloud"), vec![])],
        };
        assert_eq!(
            graph.environment_of(&EdgeTarget::Component("a".to_string())),
            Some("cloud")
        );
        assert_eq!(
            graph.environment_of(&EdgeTarget::Component("missing".to_string())),
            None
        );
        assert_eq!(
            graph.environment_of(&EdgeTarget::External("bucket".to_string())),
            None
        );
    }

    #[test]
    fn crosses_environment_only_when_both_known_and_differ() {
        let graph = Graph {
            components: vec![
                component("cloud-svc", Some("cloud"), vec![]),
                component("iot-svc", Some("iot"), vec![]),
                component("no-env-svc", None, vec![]),
            ],
        };
        let cloud_svc = graph.find("cloud-svc").unwrap();
        assert!(graph.crosses_environment(
            cloud_svc,
            &edge(EdgeTarget::Component("iot-svc".to_string()))
        ));
        assert!(!graph.crosses_environment(
            cloud_svc,
            &edge(EdgeTarget::Component("cloud-svc".to_string()))
        ));
        assert!(!graph.crosses_environment(
            cloud_svc,
            &edge(EdgeTarget::Component("no-env-svc".to_string()))
        ));
        assert!(
            !graph
                .crosses_environment(cloud_svc, &edge(EdgeTarget::External("bucket".to_string())))
        );
    }

    #[test]
    fn environment_edges_dedups_omits_same_env_and_excludes_external() {
        let graph = Graph {
            components: vec![
                component(
                    "iot-svc",
                    Some("iot"),
                    vec![edge(EdgeTarget::Component("cloud-svc".to_string()))],
                ),
                component(
                    "cloud-svc",
                    Some("cloud"),
                    vec![
                        edge(EdgeTarget::Component("cloud-svc-2".to_string())),
                        edge(EdgeTarget::Component("iot-svc".to_string())),
                        edge(EdgeTarget::External("bucket".to_string())),
                    ],
                ),
                component("cloud-svc-2", Some("cloud"), vec![]),
            ],
        };
        assert_eq!(
            graph.environment_edges(),
            vec![
                ("cloud".to_string(), "iot".to_string()),
                ("iot".to_string(), "cloud".to_string())
            ]
        );
    }

    #[test]
    fn environments_returns_distinct_sorted_values() {
        let graph = Graph {
            components: vec![
                component("a", Some("cloud"), vec![]),
                component("b", Some("iot"), vec![]),
                component("c", Some("cloud"), vec![]),
                component("d", None, vec![]),
            ],
        };
        assert_eq!(
            graph.environments(),
            vec!["cloud".to_string(), "iot".to_string()]
        );
    }
}
