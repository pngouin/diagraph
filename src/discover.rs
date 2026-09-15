use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

use crate::manifest::{MANIFEST_FILE_NAME, ManifestFile};
use crate::model::{Component, Edge, EdgeTarget, Graph, Part, PartEdge};

pub type Result<T> = std::result::Result<T, DiscoverError>;

#[derive(Debug, Error)]
pub enum DiscoverError {
    #[error("walking {0}")]
    Walk(#[source] walkdir::Error),
    #[error("reading {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("parsing {path}")]
    ParseToml {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("parsing {path}")]
    ParseJson {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error(
        "resolving a Name for the Component at {dir} (no Cargo.toml, package.json, or pyproject.toml found, and no `name` field in diagram.toml)"
    )]
    NoNameSource { dir: PathBuf },
}

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|source| DiscoverError::Read {
        path: path.to_path_buf(),
        source,
    })
}

fn parse_toml<T: serde::de::DeserializeOwned>(path: &Path, raw: &str) -> Result<T> {
    toml::from_str(raw).map_err(|source| DiscoverError::ParseToml {
        path: path.to_path_buf(),
        source,
    })
}

fn parse_json<T: serde::de::DeserializeOwned>(path: &Path, raw: &str) -> Result<T> {
    serde_json::from_str(raw).map_err(|source| DiscoverError::ParseJson {
        path: path.to_path_buf(),
        source,
    })
}

const IGNORED_DIR_NAMES: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    "vendor",
    ".venv",
    "venv",
    "__pycache__",
];

pub fn scan(root: &Path) -> Result<Graph> {
    let mut components = Vec::new();

    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| !is_ignored(e))
    {
        let entry = entry.map_err(DiscoverError::Walk)?;
        if entry.file_name() != MANIFEST_FILE_NAME {
            continue;
        }
        let dir = entry
            .path()
            .parent()
            .expect("diagram.toml always has a parent directory")
            .to_path_buf();
        components.push(load_component(&dir)?);
    }

    components.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Graph { components })
}

fn is_ignored(entry: &walkdir::DirEntry) -> bool {
    entry.file_type().is_dir()
        && entry
            .file_name()
            .to_str()
            .is_some_and(|name| IGNORED_DIR_NAMES.contains(&name))
}

fn load_component(dir: &Path) -> Result<Component> {
    let manifest_path = dir.join(MANIFEST_FILE_NAME);
    let raw = read(&manifest_path)?;
    let manifest: ManifestFile = parse_toml(&manifest_path, &raw)?;

    let name = resolve_name(dir, &manifest)?;

    let edges = manifest
        .edges
        .into_iter()
        .map(|e| Edge {
            target: if e.external {
                EdgeTarget::External(e.target)
            } else {
                EdgeTarget::Component(e.target)
            },
            via: e.via,
            data: e.data,
            from_part: e.from_part,
            to_part: e.to_part,
        })
        .collect();

    let parts = manifest
        .parts
        .into_iter()
        .map(|p| Part {
            name: p.name,
            edges: p
                .edges
                .into_iter()
                .map(|pe| PartEdge {
                    target: pe.target,
                    via: pe.via,
                    data: pe.data,
                })
                .collect(),
        })
        .collect();

    Ok(Component {
        name,
        dir: dir.to_path_buf(),
        environment: manifest.environment,
        edges,
        parts,
    })
}

/// Resolves a Component's canonical Name: from a co-located language-native
/// project file first (Cargo.toml, package.json, pyproject.toml), falling
/// back to `diagram.toml`'s own `name` field only when none is found.
fn resolve_name(dir: &Path, manifest: &ManifestFile) -> Result<String> {
    if let Some(name) = read_cargo_toml_name(dir)? {
        return Ok(name);
    }
    if let Some(name) = read_package_json_name(dir)? {
        return Ok(name);
    }
    if let Some(name) = read_pyproject_toml_name(dir)? {
        return Ok(name);
    }
    if let Some(name) = &manifest.name {
        return Ok(name.clone());
    }
    Err(DiscoverError::NoNameSource {
        dir: dir.to_path_buf(),
    })
}

fn read_cargo_toml_name(dir: &Path) -> Result<Option<String>> {
    let path = dir.join("Cargo.toml");
    if !path.is_file() {
        return Ok(None);
    }
    #[derive(Deserialize)]
    struct CargoToml {
        package: Package,
    }
    #[derive(Deserialize)]
    struct Package {
        name: String,
    }
    let raw = read(&path)?;
    let parsed: CargoToml = parse_toml(&path, &raw)?;
    Ok(Some(parsed.package.name))
}

fn read_package_json_name(dir: &Path) -> Result<Option<String>> {
    let path = dir.join("package.json");
    if !path.is_file() {
        return Ok(None);
    }
    let raw = read(&path)?;
    let parsed: serde_json::Value = parse_json(&path, &raw)?;
    Ok(parsed
        .get("name")
        .and_then(|v| v.as_str())
        .map(str::to_string))
}

fn read_pyproject_toml_name(dir: &Path) -> Result<Option<String>> {
    let path = dir.join("pyproject.toml");
    if !path.is_file() {
        return Ok(None);
    }
    let raw = read(&path)?;
    let parsed: toml::Value = parse_toml(&path, &raw)?;
    let name = parsed
        .get("project")
        .and_then(|v| v.get("name"))
        .or_else(|| {
            parsed
                .get("tool")
                .and_then(|v| v.get("poetry"))
                .and_then(|v| v.get("name"))
        })
        .and_then(|v| v.as_str())
        .map(str::to_string);
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "diagraph-test-{}-{}-{}",
                std::process::id(),
                n,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir_all(&path).unwrap();
            TempDir(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }

        fn write(&self, rel: &str, contents: &str) {
            let path = self.0.join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn name_sourced_from_cargo_toml() {
        let dir = TempDir::new();
        dir.write("Cargo.toml", "[package]\nname = \"cargo-service\"\n");
        dir.write("diagram.toml", "");
        let graph = scan(dir.path()).unwrap();
        assert_eq!(graph.components.len(), 1);
        assert_eq!(graph.components[0].name, "cargo-service");
    }

    #[test]
    fn name_sourced_from_package_json() {
        let dir = TempDir::new();
        dir.write("package.json", r#"{"name": "node-service"}"#);
        dir.write("diagram.toml", "");
        let graph = scan(dir.path()).unwrap();
        assert_eq!(graph.components[0].name, "node-service");
    }

    #[test]
    fn name_sourced_from_pyproject_project_table() {
        let dir = TempDir::new();
        dir.write("pyproject.toml", "[project]\nname = \"py-service\"\n");
        dir.write("diagram.toml", "");
        let graph = scan(dir.path()).unwrap();
        assert_eq!(graph.components[0].name, "py-service");
    }

    #[test]
    fn name_sourced_from_pyproject_poetry_table() {
        let dir = TempDir::new();
        dir.write(
            "pyproject.toml",
            "[tool.poetry]\nname = \"poetry-service\"\n",
        );
        dir.write("diagram.toml", "");
        let graph = scan(dir.path()).unwrap();
        assert_eq!(graph.components[0].name, "poetry-service");
    }

    #[test]
    fn name_falls_back_to_diagram_toml_when_no_language_file() {
        let dir = TempDir::new();
        dir.write("diagram.toml", "name = \"fallback-service\"\n");
        let graph = scan(dir.path()).unwrap();
        assert_eq!(graph.components[0].name, "fallback-service");
    }

    #[test]
    fn cargo_toml_takes_precedence_over_package_json() {
        let dir = TempDir::new();
        dir.write("Cargo.toml", "[package]\nname = \"cargo-wins\"\n");
        dir.write("package.json", r#"{"name": "package-json-loses"}"#);
        dir.write("diagram.toml", "");
        let graph = scan(dir.path()).unwrap();
        assert_eq!(graph.components[0].name, "cargo-wins");
    }

    #[test]
    fn errors_when_no_name_source_is_available() {
        let dir = TempDir::new();
        dir.write("diagram.toml", "");
        assert!(scan(dir.path()).is_err());
    }

    #[test]
    fn parts_and_part_attribution_are_mapped_into_the_graph() {
        let dir = TempDir::new();
        dir.write(
            "diagram.toml",
            r#"
            name = "report-generator"

            [[parts]]
            name = "fetch-thread"
            [[parts.edges]]
            target = "upload-thread"
            via = "channel"

            [[parts]]
            name = "upload-thread"

            [[edges]]
            target = "s3-reports-bucket"
            external = true
            from_part = "upload-thread"
            "#,
        );
        let graph = scan(dir.path()).unwrap();
        let component = &graph.components[0];
        assert_eq!(component.parts.len(), 2);
        assert_eq!(component.parts[0].edges[0].target, "upload-thread");
        assert_eq!(
            component.edges[0].from_part.as_deref(),
            Some("upload-thread")
        );
    }

    #[test]
    fn ignored_directories_are_not_scanned() {
        let dir = TempDir::new();
        dir.write(
            "real-service/Cargo.toml",
            "[package]\nname = \"real-service\"\n",
        );
        dir.write("real-service/diagram.toml", "");
        dir.write(
            "node_modules/some-lib/package.json",
            r#"{"name": "some-lib"}"#,
        );
        dir.write("node_modules/some-lib/diagram.toml", "");
        let graph = scan(dir.path()).unwrap();
        assert_eq!(graph.components.len(), 1);
        assert_eq!(graph.components[0].name, "real-service");
    }
}
