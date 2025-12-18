# DESIGN

This document explains how `sd2` works internally. For how to use the CLI, see `README.md`. For contributor setup and workflows, see `HACKING.md`.

## High-Level Architecture

At a high level, `sd2`:

1. Parses CLI arguments into an execution configuration (`Pipeline`).
2. Resolves an input mode (explicit file args, stdin paths, stdin text, or `rg --json`).
3. Executes a stream-oriented replacement pipeline over each input item.
4. Produces a `Report`, enforces policies, and commits writes atomically.

```mermaid
flowchart TD
  CLI[CLI args\nsrc/cli.rs] --> MAIN[src/main.rs]
  MAIN --> INPUT[Input collection\nsrc/input.rs + src/rgjson.rs]
  MAIN --> MODEL[Pipeline/Operations\nsrc/model.rs]
  MAIN --> ENGINE[Executor\nsrc/engine.rs]
  ENGINE --> REPLACER[Regex/Literal replacer\nsrc/replacer/]
  ENGINE --> POLICY[Policy checks\nsrc/policy.rs]
  ENGINE --> WRITE[Atomic staging/writes\nsrc/write.rs]
  WRITE --> TXN[Commit/rollback\nsrc/transaction.rs]
  ENGINE --> REPORT[Reporting/events\nsrc/reporter.rs + src/events.rs]
  REPORT --> OUT[stdout/stderr\n(diff/summary/json/agent)]
```

## Core Components

## Codebase Map (From `HACKING.md`)

The project previously kept a short “module map” in `HACKING.md`. It is preserved here as a quick orientation aid:

*   **Language:** Rust (Edition 2024)
*   **Entry Point:** `src/main.rs` - Handles CLI parsing (via `clap`) and dispatches either the default command (FIND REPLACE) or specific subcommands (`schema`, `apply`).
*   **Core Logic:**
    *   `src/engine.rs`: Orchestrates the execution of the replacement pipeline.
    *   `src/replacer/mod.rs`: Encapsulates the regex replacement logic using the `regex` crate.
    *   `src/model.rs`: Defines the data structures for the Pipeline and Operations (serialized via `serde`).
    *   `src/validate.rs` (under `replacer`): Validates replacement strings and capture groups.
*   **Dependencies:**
    *   `clap`: CLI argument parsing.
    *   `regex`: The regex engine.
    *   `serde`/`serde_json`: JSON serialization/deserialization for manifests and reports.
    *   `schemars`: JSON Schema generation.
    *   `tempfile`: Managing atomic writes.
    *   `ignore` (optional): For directory walking (feature gated).

Note: the validator currently lives at `src/replacer/validate.rs` (the older `src/validate.rs` reference was a shorthand).

### Entry Point and CLI

- `src/main.rs`: Program entry point; parses CLI, resolves input mode, builds a `Pipeline`, and dispatches execution.
- `src/cli.rs`: `clap` configuration for flags, defaults, and subcommands (`schema`, `apply`).

### Data Model (Manifests and Execution Config)

- `src/model.rs`: Serializable types (`Pipeline`, `Operation`, policy and filesystem modes) used for both CLI execution and `apply --manifest …`.
- `sd2 schema`: Emits a JSON Schema for the `Pipeline` type via `schemars`.

### Input Collection

- `src/input.rs`: Determines how stdin should be interpreted (paths vs text) and reads inputs accordingly.
- `src/rgjson.rs`: Parses `rg --json` and converts matches into deterministic replacement spans (`InputItem::RipgrepMatch`).

### Execution Engine

- `src/engine.rs`: Orchestrates end-to-end execution:
  - validates the pipeline and inputs
  - applies glob include/exclude filters
  - enforces pre/post policies
  - processes each input (file, stdin text, or `rg` match)
  - stages or writes outputs and aggregates a report

### Replacement Logic

- `src/replacer/mod.rs`: Builds a configured `Replacer` over literal or regex patterns and applies operations with replacement counting.
- `src/replacer/validate.rs`: Validates replacement strings and (when enabled) capture expansion rules.

### Atomic Writes and Transactions

- `src/write.rs`: Implements atomic write primitives:
  - `write_file`: write to a temp file + atomic rename.
  - `stage_file`: prepare a staged temp entry for transaction-wide commits.
- `src/transaction.rs`: `TransactionManager` used for `transaction=all` to commit staged files only if every file succeeds.

### Reporting and Exit Codes

- `src/reporter.rs`: Accumulates per-file results into a `Report` and prints in multiple formats.
- `src/events.rs`: JSON event types for newline-delimited streaming output.
- `src/exit_codes.rs`: Centralized exit code mapping for success, policy failures, and transactional failures.
- `src/error.rs`: Error types and machine-readable codes used by reporting and JSON output.

## Data Flow

### CLI Mode (`sd2 FIND REPLACE …`)

1. `src/main.rs` parses arguments and resolves `InputMode`.
2. Inputs are collected into `Vec<InputItem>` (paths, stdin text, or `rg` matches).
3. A `Pipeline` is built from CLI args (or loaded from a manifest when `--manifest` is used).
4. `engine::execute(pipeline, inputs)` runs:
   - semantic validation (inputs and operations must be non-empty)
   - glob filtering (optional)
   - policy pre-checks (`enforce_pre_execution`)
   - per-item processing:
     - read file bytes (or accept stdin text)
     - detect symlinks/binary according to pipeline modes
     - apply each `Operation` sequentially via `Replacer`
     - generate diffs on `--dry-run`
     - write immediately (`transaction=file`) or stage (`transaction=all`)
   - policy post-checks (`require_match`, `expect`, `fail_on_change`, etc.)
   - commit staged writes if allowed and error-free
5. `src/reporter.rs` prints output (diff/summary/json/agent) and sets an exit code.

### Manifest Apply Mode (`sd2 apply --manifest …`)

Apply mode is the same engine, but the `Pipeline` is deserialized from JSON and CLI flags act as overrides (CLI > manifest > defaults).

### `rg --json` Mode (`--rg-json`)

Instead of re-searching files, `sd2` trusts `rg --json` match spans:

- `src/rgjson.rs` parses match events and records byte/line spans.
- `src/engine.rs` passes spans into the `Replacer`, restricting replacements to those explicit ranges.

This provides deterministic, “no surprise” edits for agent workflows.

## Decision Log

Track architectural decisions here. Keep entries short and include tradeoffs.

- **Atomic writes via temp+rename**: Prevents partial modifications on crash; supports `transaction=all` staging/commit.
- **No implicit traversal**: `sd2` intentionally avoids walking directories; external tools (`rg`, `fd`, `find`) determine the file set.
- **Literal by default, regex opt-in**: Optimizes for safety and predictability; `--regex` enables regex matching explicitly.
- **NDJSON event output**: Streamable, pipeline-friendly JSON events enable agent integration and incremental reporting.
- **`rg --json` span mode**: Allows deterministic “replace exactly these matches” workflows without re-search heuristics.
