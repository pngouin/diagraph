use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use clap::{Parser, Subcommand, ValueEnum};

use diagraph::{discover, render, validate};

#[derive(Parser)]
#[command(
    name = "diagraph",
    version,
    about = "Declare which components in your monorepo talk to which, and render architecture diagrams."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scan the monorepo and validate every diagram.toml.
    Check {
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Render one view as Mermaid or Graphviz DOT text.
    Render {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Render only this Component and its direct neighbors.
        #[arg(long, conflicts_with = "environment")]
        component: Option<String>,
        /// Render the Environment view instead of the Global view.
        #[arg(long)]
        environment: bool,
        #[arg(long, value_enum, default_value = "mermaid")]
        format: Format,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Write a self-contained, offline interactive HTML diagram.
    View {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(short, long, default_value = "diagraph.html")]
        output: PathBuf,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Format {
    Mermaid,
    Dot,
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Check { root } => cmd_check(&root),
        Command::Render {
            root,
            component,
            environment,
            format,
            output,
        } => cmd_render(&root, component, environment, format, output),
        Command::View { root, output } => cmd_view(&root, &output),
    }
}

fn cmd_check(root: &Path) -> Result<()> {
    let graph = discover::scan(root)?;
    let problems = validate::validate(&graph);
    if problems.is_empty() {
        println!(
            "diagraph check: {} Component(s), no problems found",
            graph.components.len()
        );
        return Ok(());
    }
    for p in &problems {
        eprintln!("{p}");
    }
    bail!("{} problem(s) found", problems.len());
}

fn cmd_render(
    root: &Path,
    component: Option<String>,
    environment: bool,
    format: Format,
    output: Option<PathBuf>,
) -> Result<()> {
    let graph = discover::scan(root)?;
    let view = match (&component, environment) {
        (Some(name), _) => render::component_view(&graph, name)?,
        (None, true) => render::environment_view(&graph),
        (None, false) => render::global_view(&graph),
    };
    let text = match format {
        Format::Mermaid => render::mermaid::render(&view),
        Format::Dot => render::dot::render(&view),
    };
    match output {
        Some(path) => std::fs::write(&path, text)?,
        None => print!("{text}"),
    }
    Ok(())
}

fn cmd_view(root: &Path, output: &Path) -> Result<()> {
    let graph = discover::scan(root)?;
    let html = render::html::render(&graph)?;
    std::fs::write(output, html)?;
    println!("{}", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_root() -> PathBuf {
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/monorepo"))
    }

    fn scratch_dir() -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("diagraph-cli-test-{}-{}", std::process::id(), n));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn check_succeeds_on_valid_fixture() {
        assert!(cmd_check(&fixture_root()).is_ok());
    }

    #[test]
    fn render_global_view_produces_mermaid_output() {
        let dir = scratch_dir();
        let output = dir.join("global.mmd");
        cmd_render(
            &fixture_root(),
            None,
            false,
            Format::Mermaid,
            Some(output.clone()),
        )
        .unwrap();
        let text = std::fs::read_to_string(&output).unwrap();
        assert!(text.starts_with("flowchart LR"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn render_component_view_produces_dot_output() {
        let dir = scratch_dir();
        let output = dir.join("component.dot");
        cmd_render(
            &fixture_root(),
            Some("report-generator".to_string()),
            false,
            Format::Dot,
            Some(output.clone()),
        )
        .unwrap();
        let text = std::fs::read_to_string(&output).unwrap();
        assert!(text.starts_with("digraph diagraph"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn render_environment_view_shows_both_environments() {
        let dir = scratch_dir();
        let output = dir.join("environment.mmd");
        cmd_render(
            &fixture_root(),
            None,
            true,
            Format::Mermaid,
            Some(output.clone()),
        )
        .unwrap();
        let text = std::fs::read_to_string(&output).unwrap();
        assert!(text.contains("cloud"));
        assert!(text.contains("iot"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn view_writes_self_contained_html() {
        let dir = scratch_dir();
        let output = dir.join("out.html");
        cmd_view(&fixture_root(), &output).unwrap();
        let text = std::fs::read_to_string(&output).unwrap();
        assert!(text.starts_with("<!DOCTYPE html>"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
