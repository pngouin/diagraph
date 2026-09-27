# Changelog
## [0.2.0] - 2026-09-27

### Features

- *(manifest)* [**breaking**] Rename diagram.toml to diagraph.toml
- *(manifest)* [**breaking**] Reject unknown keys
- *(manifest)* Accept bare-string edges
- *(manifest)* [**breaking**] Key parts by name
- *(validate)* Prefix problems with their manifest path
- *(validate)* Suggest the closest Component for dangling targets
- *(validate)* Hint at declared Parts for unknown Part names
- *(cli)* Add init subcommand

### Bug Fixes

- *(viewer)* Keep part-anchored edges visible and on their part while zooming
- *(viewer)* Stop dimming from showing parts and edges hidden by zoom
- *(viewer)* Reveal parts once their component fills the screen

### Documentation

- *(changelog)* Update CHANGELOG.md for v0.1.1

### Refactor

- *(validate)* Drop duplicate Part name check
- *(discover)* Expose project-file Name lookup with its source
## [0.1.1] - 2026-09-27

### Features

- *(html)* Place edge labels by scoring candidates around nodes and edges
- *(html)* Bow edges around nodes and frames they don't connect to
- *(html)* Pan the view with right-click drag

### Bug Fixes

- *(html)* Keep long node labels inside their boxes
- *(html)* Keep the zoomed-in component at full opacity
- *(html)* Keep resized nodes inside their parent frame
- *(html)* Drop the zoom-driven ambient fade
- *(html)* Spread parallel edges between the same nodes into lanes
- *(html)* Order components by grid distance, not list position

### Documentation

- *(changelog)* Update CHANGELOG.md for v0.1.0

### Performance

- *(html)* Measure edge labels once instead of every tick

### Testing

- *(frontend)* Add property-based e2e fuzzer for the viewer
- *(frontend)* Check that nothing fades without a selection or search

### Build

- *(deps)* Update dtolnay/rust-toolchain requirement to 6bed0761d98439e5a578e2877258200ad565ba87
- *(deps)* Bump actions/checkout from 4.4.0 to 7.0.1
- *(deps)* Bump actions/checkout from 4.4.0 to 7.0.1
- *(deps)* Bump softprops/action-gh-release from 2.6.2 to 3.0.3
- *(deps)* Bump softprops/action-gh-release from 2.6.2 to 3.0.3
- *(deps)* Bump Swatinem/rust-cache
- *(deps)* Bump Swatinem/rust-cache from 49a0bdc70d2e1b713ca9e2869b211fcce03d3c1c to 6323deb102c322ba6fcbdcafc7e3dddab59af2b6
- *(deps)* Update dtolnay/rust-toolchain requirement to 6bed0761d98439e5a578e2877258200ad565ba87
- *(deps)* Bump actions/setup-node from 4.4.0 to 7.0.0
- *(deps)* Bump actions/setup-node from 4.4.0 to 7.0.0
- *(deps)* Update dtolnay/rust-toolchain requirement to 6bed0761d98439e5a578e2877258200ad565ba87
- *(deps)* Update dtolnay/rust-toolchain requirement to 6bed0761d98439e5a578e2877258200ad565ba87
- *(deps)* Bump thiserror from 2.0.20 to 2.0.21
- *(deps)* Bump thiserror from 2.0.20 to 2.0.21
## [0.1.0] - 2026-09-19

### Features

- *(model)* Add component graph model
- *(manifest)* Add diagram.toml schema
- *(discover)* Add monorepo scanning and name resolution
- *(validate)* Add manifest validation
- *(render)* Add view builder
- *(render)* Add mermaid renderer
- *(render)* Add dot renderer
- *(render)* Add interactive html viewer
- *(cli)* Wire up check/render/view commands
- *(manifest)* Add parts and part attribution to schema
- *(model)* Add Part to the graph model
- *(discover)* Map parts and part attribution into the graph
- *(validate)* Validate parts and part attribution
- *(render)* Add zoomed view builder and Part shapes
- *(cli)* Add --zoom flag
- *(render)* Embed zoomed views and add zoom to the html viewer
- *(html)* Add drag-to-reposition for viewer nodes
- *(html)* Render edge via/data labels on the lines
- *(cli)* Add view serve to serve the diagram over HTTP
- *(layout)* Order environments and components by connectivity

### Bug Fixes

- *(ci)* Correct MSRV to 1.88

### Documentation

- *(readme)* Add readme
- *(readme)* Document parts, from_part/to_part, --zoom, and viewer zoom
- *(readme)* Add interactive viewer screenshots
- *(cliff)* Fix stale hook reference in comment

### Refactor

- *(errors)* Use thiserror for library error types
- *(html)* Rewrite interactive viewer frontend in TypeScript

### Testing

- *(fixture)* Add example monorepo
- *(fixture)* Add parts to the example monorepo
- *(fixture)* Add retail-platform example

### Build

- Add Makefile wrapping frontend build and cargo

### CI

- Add GitHub Actions workflows and dependabot
- *(workflows)* Pin actions to commit SHAs
- *(release)* Pin actions and gate publish on fmt/clippy/test

### Miscellaneous

- *(cargo)* Scaffold project
- *(hooks)* Add pre-commit hook
- Ignore local tooling directories
- *(package)* Add dual MIT/Apache-2.0 license and crates.io metadata
- Stop tracking CONTEXT.md and docs/adr

