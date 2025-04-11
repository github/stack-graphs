# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Fork Management Guidelines
- This is a fork of the original stack-graphs repository with our own adjustments
- Preserve all original license terms (dual Apache 2.0/MIT license)
- Maintain original CONTRIBUTING.md and CODE_OF_CONDUCT.md as-is
- Clearly document our changes in commit messages and code comments
- Consider prefixing our custom features or modifications with a consistent marker
- When making changes, isolate them from original code where possible for easier upstream merges

## Build/Test Commands
- `cargo test` - Run all tests
- `cargo test -- TESTNAME` - Run a specific test (e.g., `cargo test -- can_jump_to_definition`)
- `cargo test --no-run` - Build tests without running them
- `cargo run --features cli -- test TESTFILES...` - Run specific test files
- `cargo run --features cli -- parse FILES...` - Parse files
- `cargo run --features cli -- index SOURCE_DIR` - Index a source directory

## Formatting/Linting
- `cargo fmt` - Format code using rustfmt
- `cargo clippy` - Run the Rust linter
- `cargo clippy --fix --all-features --fix --allow-dirty` - Apply automated fixes for clippy warnings
- Fix common clippy warnings including:
  - Redundant field names (`field: field` → `field`)
  - Unnecessary closures and patterns
  - Needless returns and borrows
  - Redundant pattern matching

## Code Style Guidelines
- Follow standard Rust naming conventions (snake_case for variables/functions, CamelCase for types)
- Use meaningful variable names that reflect their purpose
- Structure code with clear error handling using Result types
- Add documentation comments (///) for public APIs
- Write tests for new functionality
- Keep changes focused and maintain existing code organization
- Use proper imports organization (std first, then external crates, then local modules)