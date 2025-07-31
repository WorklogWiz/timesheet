#
# This justfile contains the various commands that we use in this project
# Build the project in debug mode
build-all:
    cargo build --workspace --all-targets

# Install the `timesheet` binary to your ~/.cargo/bin directory
install: release
    cargo install --path cli

# Build in release mode
release: test
    cargo build --release

# Run the 'timesheet' CLI with optional arguments
run args="":
    cargo run --bin timesheet -- {{args}}

# Run in release mode
run-release args="":
    cargo run --release --bin timesheet -- {{args}}

# Run all tests
test:
    cargo test --workspace --all-targets

# Make sure we follow the rules we have agreed upon before committing your changes
prepare-commit: fmt clippy
    @echo "You are ready to commit, when all warnings have been dealt with"

# Format code and fail if formatting is needed, print line numbers to make it easy to jump directly
fmt:
    cargo fmt --all -- --check -l

# Apply formatting automatically
fmt-fix:
    cargo fmt --all

# Run clippy linting for warnings
clippy:
    cargo clippy --all-targets --all-features --release -- -D clippy::pedantic


# Clean build artifacts
clean:
    cargo clean

# Update dependencies
update:
    cargo update

# Show dependency tree (requires cargo-tree)
tree:
    cargo tree

# Generate and export dependency graph as SVG (requires dot/Graphviz and cargo-depgraph)
depgraph:
    cargo depgraph --workspace-only | dot -Tsvg -o docs/assets/deps.svg


# Add, delete, sync, start, stop, view status, or modify config using the CLI
add args="":
    cargo run --bin timesheet -- add {{args}}

# Removes a timesheet entry
del args="":
    cargo run --bin timesheet -- del {{args}}

# Shows the current status of timesheet registering for the last 4 weeks, including timer state
status args="":
    cargo run --bin timesheet -- status {{args}}

# Supposed to list all available issue codes, but not implemented
codes:
    cargo run --bin timesheet -- codes

# Start a new timer for an issue
start issue:
    cargo run --bin timesheet -- start -i {{issue}}

# Stop the current timer
stop:
    cargo run --bin timesheet -- stop

# Sync local timesheet database with remote Jira installation
sync args="":
    cargo run --bin timesheet -- sync {{args}}

# Dumps the location and contents of the configuration file
config args="":
    cargo run --bin timesheet -- config list {{args}}