# HACKING

Developer onboarding for `sd2`.

* For end-user CLI usage, see `README.md`.
* For internal architecture and module layout, see `DESIGN.md`.

## Project Overview

`sd2` is a next-generation stream-oriented text processor written in Rust. It is designed for two primary audiences:
1.  **Humans:** Offering a safer, clearer alternative to `sed` and `awk` with atomic writes and explicit operations.
2.  **AI Agents:** Providing a formal, machine-readable API (via JSON schemas and manifests) for deterministic refactoring without shell injection risks.

**Key Design Principles:**
*   **Atomic Transactions:** Writes are atomic; files are never left partially modified.
*   **No Implicit Traversal:** Does not walk directories by default; delegates to tools like `ripgrep` or `fd`.
*   **Structured I/O:** Supports JSON manifests for defining operations and JSON output for reporting.
*   **Pipeline-First:** Designed to work effectively in Unix pipelines.

## Architecture

`sd2`'s internal architecture, module map, and data-flow notes live in `DESIGN.md`.

## Building and Running

### Prerequisites
*   Rust toolchain (v1.86.0+)
*   `rg` (ripgrep) is required for some integration tests (`tests/ripgrep_workflow.rs` currently invokes `/usr/bin/rg`).

### Commands

*   **Build:**
    ```bash
    cargo build
    ```

*   **Run:**
    ```bash
    cargo run -- <args>
    ```

*   **Test:**
    ```bash
    cargo test
    ```

*   **Format:**
    ```bash
    cargo fmt --all
    ```

*   **Lint:**
    ```bash
    cargo clippy --all-targets --all-features -- -D warnings
    ```

*   **Install (Local):**
    ```bash
    cargo install --path .
    ```

### Usage Examples

*   **Basic Replacement:**
    ```bash
    cargo run -- "find_pattern" "replace_pattern" src/main.rs
    ```

*   **Dry Run:**
    ```bash
    cargo run -- "foo" "bar" src/main.rs --dry-run
    ```

*   **Schema Dump (for Agents):**
    ```bash
    cargo run -- schema
    ```

*   **Apply Manifest:**
    ```bash
    cargo run -- apply --manifest examples/manifest.json
    ```

*   **Stdin Paths (from fd):**
    ```bash
    fd -e rs | cargo run -- "foo" "bar"
    ```

*   **Ripgrep JSON input:**
    ```bash
    rg --json "TODO" | cargo run -- --rg-json "TODO" "FIXME"
    ```

## Development Conventions

*   **Code Style:** Follow standard Rust formatting (`cargo fmt`) and clippy advice (`cargo clippy`).
*   **Safety:** Prioritize atomic operations. Never modify a file in-place without a strategy to prevent data loss on crash.
*   **Testing:**
    *   Unit tests are co-located in source files (e.g., `src/replacer/mod.rs`).
    *   Integration tests likely exist (implied by `assert_cmd` in dev-dependencies) or should be added to `tests/`.
*   **Agent Interaction:** When adding features, consider how an AI agent would invoke them. Ensure schemas are updated (`model.rs`) and CLI flags have corresponding JSON manifest fields.

## Contribution Flow

1. Create a focused change (small PRs are preferred).
2. Add/update tests when behavior changes (`cargo test`).
3. Run formatting and linting (`cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`).
4. Submit a PR with a clear description, including any user-facing CLI or JSON schema changes.
