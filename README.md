# diagraph

Declare which components in your monorepo talk to which, and render architecture diagrams — static (Mermaid/DOT) and interactive (a self-contained HTML file).

## What it is

Each component in your monorepo — a Rust crate, a Node package, whatever — gets a small `diagram.toml` declaring the other components it calls at runtime. `diagraph` scans the whole tree, checks those declarations for typos and dangling references, and renders the result as a diagram: the whole monorepo's architecture, or just one component's direct neighbors.

This covers **Components** and **Environments** (where a component is deployed — `cloud`, `iot`, `mobile`, ...). A future phase adds **Parts** — internal subdivisions of a component (threads, actors, modules) — and a zoomed-in view of a single component's internals. That's not built yet; the schema below is shaped so it can be added without breaking existing manifests.

## Quick start

```sh
cargo run -- check --root examples/monorepo
cargo run -- render --root examples/monorepo
cargo run -- view --root examples/monorepo -o diagraph.html
```

## The `diagram.toml` schema

A directory is a Component purely because it contains a `diagram.toml` — nothing else about it matters (it doesn't need to be a Cargo/npm package boundary).

| Field | Type | Required | Meaning |
|---|---|---|---|
| `name` | string | only as fallback | The Component's Name, used only when no co-located language-native project file provides one (see below). |
| `environment` | string | no | Free-text deployment label, e.g. `"cloud"`, `"iot"`, `"mobile"`. At most one per Component. |
| `edges` | array of tables | no | This Component's outgoing calls. See below. |

Each entry under `[[edges]]`:

| Field | Type | Required | Meaning |
|---|---|---|---|
| `target` | string | yes | The Name of the Component (or, if `external`, the name of the thing) being called. |
| `via` | string | no | Free-text: *how* — `"REST"`, `"gRPC"`, `"file upload"`. |
| `data` | string | no | Free-text: *what* is exchanged — `"user profile"`, `"PDF report"`. |
| `external` | bool | no, default `false` | Set to `true` when `target` is outside this monorepo (an S3 bucket, a third-party API) and has no `diagram.toml` of its own. |

**A Component's Name is never retyped in `diagram.toml`.** It's read, in order, from:

1. `Cargo.toml`'s `[package].name`
2. `package.json`'s `"name"`
3. a Python project file's `[project].name`, or `[tool.poetry].name`
4. `diagram.toml`'s own `name` field — only as a last resort, when none of the above exist

(Why: [docs/adr/0002](docs/adr/0002-name-sourced-from-language-manifest.md) — two names for one thing drift apart over time.)

Two examples from `examples/monorepo/`:

```toml
# report-generator/diagram.toml — an edge to something outside the monorepo
environment = "cloud"

[[edges]]
target = "s3-reports-bucket"
external = true
via = "upload"
data = "PDF report"
```

```toml
# api-gateway/diagram.toml — two ordinary edges to other Components
environment = "cloud"

[[edges]]
target = "user-service"
via = "gRPC"
data = "user lookup"

[[edges]]
target = "report-generator"
via = "job trigger"
data = "report request"
```

### Validation

`diagraph check` reports two kinds of problem:

- **Duplicate Name** — two Components resolved to the same Name.
- **Dangling reference** — an edge's `target` matches no known Component and isn't marked `external = true`.

Edges are declared **only by the caller** — a Component never declares that it's called by someone else; incoming edges are derived by inverting the graph. (Why: [docs/adr/0001](docs/adr/0001-outgoing-only-edges.md).) Unrecognized targets are errors, not silently-accepted new nodes. (Why: [docs/adr/0003](docs/adr/0003-strict-validation-on-unrecognized-targets.md).)

## The three views

- **Global view** — every Component as a node, every Edge as an edge. External targets get their own (deduped) node.
- **Component view** — one named Component plus only its direct neighbors, incoming and outgoing.
- **Environment view** — nodes are the distinct Environment values in use; an edge appears between two Environments only if some Edge crosses between a Component in each. Same-environment edges are omitted — this view is specifically about crossing boundaries.

## CLI reference

```sh
# Validate every diagram.toml under --root (default: current directory)
diagraph check [--root PATH]

# Render a view as Mermaid (default) or DOT text
diagraph render [--root PATH] [--component NAME | --environment] [--format mermaid|dot] [-o FILE]

# Examples:
diagraph render --root examples/monorepo
diagraph render --root examples/monorepo --component report-generator --format dot
diagraph render --root examples/monorepo --environment -o environments.mmd

# Write a self-contained, offline interactive HTML diagram (never opens a browser)
diagraph view [--root PATH] [-o FILE]
diagraph view --root examples/monorepo -o diagraph.html
```

## Try it on the bundled example

```sh
cargo run -- view --root examples/monorepo -o diagraph.html
```

Then open `diagraph.html` in a browser. It's fully self-contained — no network requests, works offline.

## Contributing

Git hooks live in `.githooks/`, not `.git/hooks/`. Run once per clone:

```sh
git config core.hooksPath .githooks
```

## Status

Phase 1 (this repo, right now): Components, Environments, the three views above.
Phase 2 (not yet built): Parts, `from_part`/`to_part` edge attribution, the zoomed-into-one-Component view.
