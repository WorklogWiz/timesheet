# Rust Baseline Repository

This is a baseline for new rust repositories. It has a set of standardized tools setup up in a generic way that would help most rust projects get started quickly. It also integrates with the tooling AutoStore has chosen such as SonarCube.

## Add packages

To add packages, simply run `cargo new <name of package>`. Then add this as a member of the root `Cargo.toml` file.

## Build and test the entire workspace

To build the workspace, run `cargo build` and to then run the tests, `cargo test`
