use serde::Deserialize;

pub const MANIFEST_FILE_NAME: &str = "diagram.toml";

#[derive(Debug, Deserialize)]
pub struct ManifestFile {
    /// Fallback Name, used only when no co-located language-native project
    /// file (Cargo.toml, package.json, pyproject.toml) provides one.
    pub name: Option<String>,
    pub environment: Option<String>,
    #[serde(default)]
    pub edges: Vec<EdgeDecl>,
    #[serde(default)]
    pub parts: Vec<PartDecl>,
}

#[derive(Debug, Deserialize)]
pub struct EdgeDecl {
    pub target: String,
    pub via: Option<String>,
    pub data: Option<String>,
    #[serde(default)]
    pub external: bool,
    pub from_part: Option<String>,
    pub to_part: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PartDecl {
    pub name: String,
    #[serde(default)]
    pub edges: Vec<PartEdgeDecl>,
}

#[derive(Debug, Deserialize)]
pub struct PartEdgeDecl {
    pub target: String,
    pub via: Option<String>,
    pub data: Option<String>,
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
            [[parts]]
            name = "fetch-thread"
            [[parts.edges]]
            target = "upload-thread"
            via = "channel"
            data = "raw report rows"

            [[parts]]
            name = "upload-thread"
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
}
