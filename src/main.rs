use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
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
        /// Render the zoomed (Part-level) view of --component instead of just its neighbors.
        #[arg(long, requires = "component")]
        zoom: bool,
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
        #[command(subcommand)]
        action: Option<ViewAction>,
    },
}

#[derive(Subcommand)]
enum ViewAction {
    /// Serve the diagram over HTTP instead of writing it to a file.
    Serve {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Interface to bind to.
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value_t = 4000)]
        port: u16,
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
            zoom,
            format,
            output,
        } => cmd_render(&root, component, environment, zoom, format, output),
        Command::View {
            root,
            output,
            action,
        } => match action {
            None => cmd_view(&root, &output),
            Some(ViewAction::Serve { root, host, port }) => cmd_view_serve(&root, &host, port),
        },
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
    zoom: bool,
    format: Format,
    output: Option<PathBuf>,
) -> Result<()> {
    let graph = discover::scan(root)?;
    let view = match (&component, zoom, environment) {
        (Some(name), true, _) => render::zoomed_view(&graph, name)?,
        (Some(name), false, _) => render::component_view(&graph, name)?,
        (None, _, true) => render::environment_view(&graph),
        (None, _, false) => render::global_view(&graph),
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

fn cmd_view_serve(root: &Path, host: &str, port: u16) -> Result<()> {
    let graph = discover::scan(root)?;
    let body = render::html::render(&graph)?.into_bytes();

    let listener =
        TcpListener::bind((host, port)).with_context(|| format!("binding to {host}:{port}"))?;
    println!(
        "Serving diagraph at http://{}  (Ctrl+C to stop)",
        listener.local_addr()?
    );

    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        serve_once(&mut stream, &body);
    }
    Ok(())
}

/// One-shot, single-page response: the whole diagram is static for the life
/// of the process, so there's nothing to route and no need for a real HTTP
/// server crate — just enough of the protocol for a browser to render it.
fn serve_once(stream: &mut std::net::TcpStream, body: &[u8]) {
    let mut request = Vec::new();
    let mut chunk = [0u8; 512];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                request.extend_from_slice(&chunk[..n]);
                if request.windows(4).any(|w| w == b"\r\n\r\n") || request.len() > 8192 {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
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
            false,
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
    fn render_zoom_produces_the_zoomed_view() {
        let dir = scratch_dir();
        let output = dir.join("zoomed.mmd");
        cmd_render(
            &fixture_root(),
            Some("report-generator".to_string()),
            false,
            true,
            Format::Mermaid,
            Some(output.clone()),
        )
        .unwrap();
        let text = std::fs::read_to_string(&output).unwrap();
        assert!(text.starts_with("flowchart LR"));
        assert!(text.contains("report-generator"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn zoom_flag_requires_component_flag() {
        let result = Cli::try_parse_from(["diagraph", "render", "--zoom"]);
        assert!(result.is_err());
    }

    #[test]
    fn view_serve_defaults_to_localhost() {
        let cli = Cli::try_parse_from(["diagraph", "view", "serve"]).unwrap();
        let Command::View { action, .. } = cli.command else {
            panic!("expected the view command")
        };
        match action {
            Some(ViewAction::Serve { host, port, .. }) => {
                assert_eq!(host, "127.0.0.1");
                assert_eq!(port, 4000);
            }
            _ => panic!("expected the serve action"),
        }
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
