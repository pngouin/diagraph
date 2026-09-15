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
}

#[derive(Debug, Deserialize)]
pub struct EdgeDecl {
    pub target: String,
    pub via: Option<String>,
    pub data: Option<String>,
    #[serde(default)]
    pub external: bool,
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
}
