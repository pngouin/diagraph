use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::discover::{self, DiscoverError, NameSource};
use crate::manifest::MANIFEST_FILE_NAME;

const SCHEMA_URL: &str = "https://github.com/pngouin/diagraph#the-diagraphtoml-schema";

#[derive(Debug, Error)]
pub enum InitError {
    #[error("{0} already exists")]
    AlreadyExists(PathBuf),
    #[error("--name can't be used here: the Component at {dir} already takes its Name from {file}")]
    NameFromProjectFile { dir: PathBuf, file: NameSource },
    #[error("can't guess a Name from {0}; pass --name")]
    NoDirName(PathBuf),
    #[error("accessing {path}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error(transparent)]
    Discover(#[from] DiscoverError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameOrigin {
    ProjectFile(NameSource),
    Flag,
    DirName,
}

#[derive(Debug)]
pub struct Initialized {
    pub manifest: PathBuf,
    pub name: String,
    pub origin: NameOrigin,
}

pub fn init(dir: &Path, name: Option<&str>) -> Result<Initialized, InitError> {
    let (name, origin) = match (discover::project_name(dir)?, name) {
        (Some((_, file)), Some(_)) => {
            return Err(InitError::NameFromProjectFile {
                dir: dir.to_path_buf(),
                file,
            });
        }
        (Some((found, file)), None) => (found, NameOrigin::ProjectFile(file)),
        (None, Some(given)) => (given.to_owned(), NameOrigin::Flag),
        (None, None) => (dir_name(dir)?, NameOrigin::DirName),
    };

    let fallback = match origin {
        NameOrigin::ProjectFile(_) => None,
        NameOrigin::Flag | NameOrigin::DirName => Some(name.as_str()),
    };
    let manifest = dir.join(MANIFEST_FILE_NAME);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&manifest)
        .map_err(|source| match source.kind() {
            io::ErrorKind::AlreadyExists => InitError::AlreadyExists(manifest.clone()),
            _ => InitError::Io {
                path: manifest.clone(),
                source,
            },
        })?;
    file.write_all(template(fallback).as_bytes())
        .map_err(|source| InitError::Io {
            path: manifest.clone(),
            source,
        })?;

    Ok(Initialized {
        manifest,
        name,
        origin,
    })
}

fn dir_name(dir: &Path) -> Result<String, InitError> {
    let absolute = fs::canonicalize(dir).map_err(|source| InitError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    absolute
        .file_name()
        .and_then(|n| n.to_str())
        .map(str::to_owned)
        .ok_or_else(|| InitError::NoDirName(dir.to_path_buf()))
}

fn template(fallback_name: Option<&str>) -> String {
    let mut out = format!("# Schema: {SCHEMA_URL}\n\n");
    if let Some(name) = fallback_name {
        out.push_str(&format!("name = {}\n", toml::Value::from(name)));
    }
    out.push_str("# environment = \"cloud\"\n# edges = [\"other-component\"]\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::ManifestFile;
    use crate::{discover, validate};
    use std::sync::atomic::{AtomicU32, Ordering};

    fn scratch_dir(label: &str) -> PathBuf {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir()
            .join(format!("diagraph-init-test-{}-{n}", std::process::id()))
            .join(label);
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn template_without_fallback_is_an_empty_manifest() {
        let manifest: ManifestFile = toml::from_str(&template(None)).unwrap();
        assert_eq!(manifest.name, None);
        assert_eq!(manifest.environment, None);
        assert!(manifest.edges.is_empty());
    }

    #[test]
    fn template_fallback_name_is_escaped() {
        let manifest: ManifestFile = toml::from_str(&template(Some(r#"we"ird"#))).unwrap();
        assert_eq!(manifest.name.as_deref(), Some(r#"we"ird"#));
    }

    #[test]
    fn name_from_project_file_is_not_written() {
        let dir = scratch_dir("api");
        fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"api-gateway\"\n",
        )
        .unwrap();

        let done = init(&dir, None).unwrap();

        assert_eq!(done.name, "api-gateway");
        assert_eq!(done.origin, NameOrigin::ProjectFile(NameSource::CargoToml));
        assert!(
            !fs::read_to_string(&done.manifest)
                .unwrap()
                .contains("name =")
        );
        let graph = discover::scan(&dir).unwrap();
        assert_eq!(graph.components[0].name, "api-gateway");
        assert!(validate::validate(&graph).is_empty());
    }

    #[test]
    fn name_is_guessed_from_the_directory_without_a_project_file() {
        let dir = scratch_dir("billing");

        let done = init(&dir, None).unwrap();

        assert_eq!(done.name, "billing");
        assert_eq!(done.origin, NameOrigin::DirName);
        assert_eq!(discover::scan(&dir).unwrap().components[0].name, "billing");
    }

    #[test]
    fn name_flag_is_written_without_a_project_file() {
        let dir = scratch_dir("svc");

        let done = init(&dir, Some("billing-service")).unwrap();

        assert_eq!(done.origin, NameOrigin::Flag);
        assert_eq!(
            discover::scan(&dir).unwrap().components[0].name,
            "billing-service"
        );
    }

    #[test]
    fn name_flag_conflicts_with_a_project_file() {
        let dir = scratch_dir("web");
        fs::write(dir.join("package.json"), r#"{"name": "web-dashboard"}"#).unwrap();

        let err = init(&dir, Some("other")).unwrap_err();

        assert!(matches!(
            err,
            InitError::NameFromProjectFile {
                file: NameSource::PackageJson,
                ..
            }
        ));
        assert!(!dir.join(MANIFEST_FILE_NAME).exists());
    }

    #[test]
    fn existing_manifest_is_left_untouched() {
        let dir = scratch_dir("taken");
        fs::write(dir.join(MANIFEST_FILE_NAME), "name = \"keep-me\"\n").unwrap();

        let err = init(&dir, None).unwrap_err();

        assert!(matches!(err, InitError::AlreadyExists(_)));
        assert_eq!(
            fs::read_to_string(dir.join(MANIFEST_FILE_NAME)).unwrap(),
            "name = \"keep-me\"\n"
        );
    }
}
