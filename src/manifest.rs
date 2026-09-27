use std::fmt;
use std::marker::PhantomData;

use serde::de::{self, MapAccess, value::MapAccessDeserializer};
use serde::{Deserialize, Deserializer};

pub const MANIFEST_FILE_NAME: &str = "diagraph.toml";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestFile {
    /// Fallback Name, used only when no co-located language-native project
    /// file (Cargo.toml, package.json, pyproject.toml) provides one.
    pub name: Option<String>,
    pub environment: Option<String>,
    #[serde(default, deserialize_with = "shorthand_edges")]
    pub edges: Vec<EdgeDecl>,
    #[serde(default, deserialize_with = "parts_by_name")]
    pub parts: Vec<PartDecl>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeDecl {
    pub target: String,
    pub via: Option<String>,
    pub data: Option<String>,
    #[serde(default)]
    pub external: bool,
    pub from_part: Option<String>,
    pub to_part: Option<String>,
}

impl From<String> for EdgeDecl {
    fn from(target: String) -> Self {
        EdgeDecl {
            target,
            ..EdgeDecl::default()
        }
    }
}

#[derive(Debug)]
pub struct PartDecl {
    pub name: String,
    pub edges: Vec<PartEdgeDecl>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PartBody {
    #[serde(default, deserialize_with = "shorthand_edges")]
    edges: Vec<PartEdgeDecl>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartEdgeDecl {
    pub target: String,
    pub via: Option<String>,
    pub data: Option<String>,
}

impl From<String> for PartEdgeDecl {
    fn from(target: String) -> Self {
        PartEdgeDecl {
            target,
            ..PartEdgeDecl::default()
        }
    }
}

// Hand-written rather than `#[serde(untagged)]`, which would replace table errors
// (e.g. unknown keys) with "data did not match any variant".
struct Shorthand<T>(T);

impl<'de, T> Deserialize<'de> for Shorthand<T>
where
    T: Deserialize<'de> + From<String>,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor<T>(PhantomData<T>);

        impl<'de, T> de::Visitor<'de> for Visitor<T>
        where
            T: Deserialize<'de> + From<String>,
        {
            type Value = Shorthand<T>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a target name or an edge table")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(Shorthand(T::from(v.to_owned())))
            }

            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                T::deserialize(MapAccessDeserializer::new(map)).map(Shorthand)
            }
        }

        deserializer.deserialize_any(Visitor(PhantomData))
    }
}

fn shorthand_edges<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + From<String>,
{
    let edges = Vec::<Shorthand<T>>::deserialize(deserializer)?;
    Ok(edges.into_iter().map(|Shorthand(edge)| edge).collect())
}

/// `[parts.<name>]` tables, kept in declaration order.
fn parts_by_name<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<PartDecl>, D::Error> {
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = Vec<PartDecl>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a table of Parts keyed by name")
        }

        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut parts = Vec::new();
            while let Some((name, body)) = map.next_entry::<String, PartBody>()? {
                parts.push(PartDecl {
                    name,
                    edges: body.edges,
                });
            }
            Ok(parts)
        }
    }

    deserializer.deserialize_map(Visitor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_manifest_has_no_edges_and_no_optional_fields() {
        let manifest: ManifestFile = toml::from_str("").unwrap();
        assert_eq!(manifest.name, None);
        assert_eq!(manifest.environment, None);
        assert!(manifest.edges.is_empty());
    }

    #[test]
    fn edge_external_defaults_to_false() {
        let manifest: ManifestFile = toml::from_str(
            r#"
            [[edges]]
            target = "other-service"
            "#,
        )
        .unwrap();
        let edge = &manifest.edges[0];
        assert_eq!(edge.target, "other-service");
        assert!(!edge.external);
        assert_eq!(edge.via, None);
        assert_eq!(edge.data, None);
    }

    #[test]
    fn edge_parses_all_fields() {
        let manifest: ManifestFile = toml::from_str(
            r#"
            name = "fallback-name"
            environment = "cloud"

            [[edges]]
            target = "s3-bucket"
            via = "upload"
            data = "PDF report"
            external = true
            "#,
        )
        .unwrap();
        assert_eq!(manifest.name.as_deref(), Some("fallback-name"));
        assert_eq!(manifest.environment.as_deref(), Some("cloud"));
        let edge = &manifest.edges[0];
        assert_eq!(edge.target, "s3-bucket");
        assert_eq!(edge.via.as_deref(), Some("upload"));
        assert_eq!(edge.data.as_deref(), Some("PDF report"));
        assert!(edge.external);
    }

    #[test]
    fn phase_one_manifest_has_no_parts_and_no_part_attribution() {
        let manifest: ManifestFile = toml::from_str(
            r#"
            [[edges]]
            target = "other-service"
            "#,
        )
        .unwrap();
        assert!(manifest.parts.is_empty());
        assert_eq!(manifest.edges[0].from_part, None);
        assert_eq!(manifest.edges[0].to_part, None);
    }

    #[test]
    fn edge_parses_from_part_and_to_part() {
        let manifest: ManifestFile = toml::from_str(
            r#"
            [[edges]]
            target = "report-generator"
            from_part = "upload-thread"
            to_part = "fetch-thread"
            "#,
        )
        .unwrap();
        let edge = &manifest.edges[0];
        assert_eq!(edge.from_part.as_deref(), Some("upload-thread"));
        assert_eq!(edge.to_part.as_deref(), Some("fetch-thread"));
    }

    #[test]
    fn parts_with_internal_edges_parse() {
        let manifest: ManifestFile = toml::from_str(
            r#"
            [parts.fetch-thread]
            [[parts.fetch-thread.edges]]
            target = "upload-thread"
            via = "channel"
            data = "raw report rows"

            [parts.upload-thread]
            "#,
        )
        .unwrap();
        assert_eq!(manifest.parts.len(), 2);
        assert_eq!(manifest.parts[0].name, "fetch-thread");
        let part_edge = &manifest.parts[0].edges[0];
        assert_eq!(part_edge.target, "upload-thread");
        assert_eq!(part_edge.via.as_deref(), Some("channel"));
        assert_eq!(part_edge.data.as_deref(), Some("raw report rows"));
        assert_eq!(manifest.parts[1].name, "upload-thread");
        assert!(manifest.parts[1].edges.is_empty());
    }

    #[test]
    fn parts_keep_declaration_order() {
        let manifest: ManifestFile = toml::from_str(
            r#"
            [parts.zeta]
            [parts.alpha]
            [parts.mid]
            "#,
        )
        .unwrap();
        let names: Vec<_> = manifest.parts.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["zeta", "alpha", "mid"]);
    }

    #[test]
    fn duplicate_part_name_is_a_parse_error() {
        let result = toml::from_str::<ManifestFile>(
            r#"
            [parts.worker]
            [parts.worker]
            "#,
        );
        assert!(result.is_err());
    }

    #[test]
    fn bare_string_edges_are_targets_only() {
        let manifest: ManifestFile =
            toml::from_str(r#"edges = ["user-service", "billing"]"#).unwrap();
        assert_eq!(manifest.edges.len(), 2);
        assert_eq!(manifest.edges[0].target, "user-service");
        assert_eq!(manifest.edges[1].target, "billing");
        assert_eq!(manifest.edges[0].via, None);
        assert!(!manifest.edges[0].external);
    }

    #[test]
    fn bare_strings_and_inline_tables_mix() {
        let manifest: ManifestFile = toml::from_str(
            r#"edges = ["user-service", { target = "s3-bucket", external = true, via = "upload" }]"#,
        )
        .unwrap();
        assert_eq!(manifest.edges[0].target, "user-service");
        assert_eq!(manifest.edges[1].target, "s3-bucket");
        assert!(manifest.edges[1].external);
        assert_eq!(manifest.edges[1].via.as_deref(), Some("upload"));
    }

    #[test]
    fn part_edges_accept_bare_strings() {
        let manifest: ManifestFile = toml::from_str(
            r#"
            [parts.worker]
            edges = ["listener"]

            [parts.listener]
            "#,
        )
        .unwrap();
        assert_eq!(manifest.parts[0].edges[0].target, "listener");
        assert_eq!(manifest.parts[0].edges[0].via, None);
    }

    #[test]
    fn edge_of_the_wrong_type_is_rejected() {
        let err = toml::from_str::<ManifestFile>("edges = [42]")
            .unwrap_err()
            .to_string();
        assert!(err.contains("a target name or an edge table"));
    }

    fn unknown_key_error(raw: &str) -> String {
        toml::from_str::<ManifestFile>(raw).unwrap_err().to_string()
    }

    #[test]
    fn unknown_top_level_key_is_rejected() {
        assert!(unknown_key_error("enviroment = \"cloud\"\n").contains("enviroment"));
    }

    #[test]
    fn unknown_key_in_a_mixed_edge_array_is_still_reported() {
        let err = unknown_key_error(r#"edges = ["a", { target = "b", extrenal = true }]"#);
        assert!(err.contains("unknown field `extrenal`"));
    }

    #[test]
    fn unknown_edge_key_is_rejected() {
        let err = unknown_key_error(
            r#"
            [[edges]]
            target = "s3-bucket"
            extrenal = true
            "#,
        );
        assert!(err.contains("extrenal"));
    }

    #[test]
    fn unknown_part_key_is_rejected() {
        let err = unknown_key_error(
            r#"
            [parts.worker]
            name = "worker"
            "#,
        );
        assert!(err.contains("name"));
    }

    #[test]
    fn unknown_part_edge_key_is_rejected() {
        let err = unknown_key_error(
            r#"
            [parts.worker]
            edges = [{ target = "listener", from_part = "worker" }]
            "#,
        );
        assert!(err.contains("from_part"));
    }
}
