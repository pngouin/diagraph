# Diagram

A tool that scans a monorepo and renders architecture diagrams (static and interactive) showing which Components talk to which, based on manifests declared inside each Component's directory.

## Language

**Component**:
A directory identified solely by the presence of a component manifest file at its root. Independent of whatever a package/library boundary means to its own build tooling — a Component exists because it declares itself as one, not because it happens to be a Cargo/npm/Python package.
_Avoid_: lib, package, service, module (unless quoting the user's own words)

**Name**:
The canonical, globally-unique identifier for a Component, used to reference it as an edge target. Read from the Component's co-located language-native project file (`Cargo.toml` `[package].name`, `package.json` `"name"`, a Python project file's project name) rather than retyped in the component manifest.

**Edge**:
A declared, one-directional statement of runtime/architectural communication: "this Component calls that Component." Declared only by the caller (outgoing-only) — a target Component never separately declares that it is called. The full graph's incoming edges for a Component are derived by inverting all outgoing edges across the monorepo. Carries a free-text `via` label describing *how* the communication happens (e.g. "REST", "gRPC", "file upload") rather than a fixed protocol enum, and an optional free-text `data` label describing *what* is exchanged (e.g. "user profile", "video frame"), independent of `via`. May optionally carry `from_part` and `to_part`, attributing the edge to specific Parts on either end (see Part).
_Avoid_: dependency, communicates_with (as loosely used before this was pinned down), build dependency

**Environment**:
A free-text label on a Component (e.g. "cloud", "iot", "mobile") describing where it's deployed. Exactly one per Component, not a set. Drives two things: automatic visual distinction of any Edge whose source and target Components have different Environment values, and the Environment view (see below).
_Avoid_: platform, deployment target (unless quoting the user's own words)

**Part**:
An internal subdivision of a Component worth diagramming individually — a thread, actor, worker, or module. A Part's name is unique only within its own Component, never globally. Declared in the same `diagram.toml` as its Component. A Part may declare purely-internal edges to other Parts in the same Component (its own `via` label), and may be referenced by another Component's outgoing Edge via `to_part` (validated against the target Component's declared Parts — an unrecognized `to_part` is a validation error, same as any other dangling reference).
_Avoid_: thread, actor, worker, module (unless quoting the user's own words for a specific case) — "Part" is the umbrella term regardless of what the subdivision actually is at runtime.

**Environment view / global view / zoomed view**:
Three renderings of the same underlying graph of Environments, Components, Edges, and Parts — never separately declared graphs, only different altitudes over one set of facts.
- **Environment view** (most zoomed out): nodes are the distinct Environment values in use; an edge appears between two Environments only if some Edge crosses between a Component in each. Same-environment Edges are omitted — this view exists specifically to show cross-environment boundaries, and internal same-environment traffic is noise at this altitude.
- **Global view**: nodes are Components, edges are Edges. Parts collapse away entirely even when `from_part`/`to_part` are set.
- **Zoomed view**: scoped to one Component, renders its Parts and internal Part-to-Part edges, and redraws any Edge touching that Component at the specific Part named by `from_part`/`to_part` instead of at the Component boundary.

**Component manifest**:
The file named `diagram.toml`, present at a Component's root, whose presence is what makes that directory a Component. Declares its outgoing Edges and, only when no co-located language-native project file exists, an explicit `name` fallback.

**External target**:
An Edge target that is not a Component in this monorepo — e.g. an S3 bucket, a third-party API, a database — with no manifest of its own. Lets an intermediary hop (like "Tooling produces a file, uploads it, Backend-X consumes it later") appear as its own node on the diagram (Tooling → S3 → Backend-X) instead of being collapsed into a direct edge. Must be marked `external = true` on the Edge; any target name that is neither a discovered Component's Name nor marked external is a validation error, not a silently-accepted new node.

## Explicitly out of scope

**Build/compile-time dependency**: importing another Component as a library at compile time. Already covered by existing tooling (Cargo/npm/Nx dependency graphs) and is a different question from "what calls what at runtime." Not modeled as an Edge.
