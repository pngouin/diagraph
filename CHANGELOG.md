# Changelog
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

