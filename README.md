# `sd2`
<div style="background-color: #1e1e1e; padding: 1em; display: inline-block; border-radius: 8px;">
  <img src=".github/sd2.png" alt="sd2 logo" width="300">
</div>

`sd2` is a next-generation **stream-oriented text processor** designed for two audiences:

* **Humans** who want a safer, clearer CLI than `sed` or `awk`
* **AI agents** that require structured inputs, deterministic behavior, and strict JSON validation

It follows the Unix philosophy strictly
`sd2` does **not** walk directories, infer context, or guess intent. It consumes streams, applies explicit operations, and performs **atomic, transactional edits**

---

## ⚡ Quick Start (Humans)

### Basic Replacement

Simple positional arguments
No regex soup, no flag archaeology

```bash
# Replace 'lazy_static' with 'once_cell' in main.rs
sd2 "lazy_static" "once_cell" src/main.rs
```

---

### The “Search & Destroy” Workflow

`sd2` is designed to work *with* existing Unix tools
Let `ripgrep` or `fd` decide **what** to touch, and let `sd2` decide **how** to edit

```bash
# 1. Find files containing 'unwrap()'
# 2. Replace it safely everywhere
rg -l "unwrap\(\)" | sd2 "unwrap()" "expect(\"checked by safe-mode\")"
```

No directory traversal
No implicit recursion
No surprises

---

### Safety First

All writes are **atomic by default**
Files are never left partially modified, even on crash or SIGINT

```bash
# Preview a unified diff without modifying files
sd2 "foo" "bar" src/main.rs --dry-run
```

---

## 🤖 Agent Mode (LLM-Native)

`sd2` exposes a **formal, machine-readable API** so language models can perform large refactors without shell injection, syntax hallucination, or partial failure

---

### 1. Schema Discovery

Agents can query the exact operation schema supported by the tool

```bash
sd2 schema > tools_schema.json
```

This allows planners, validators, and tool routers to reason about edits *before* execution

---

### 2. Pipeline Manifests

Instead of ad-hoc shell commands, agents submit a **Pipeline Manifest**
All operations are validated, ordered, and applied **atomically**

**manifest.json**

```json
{
  "files": ["src/lib.rs", "src/config.rs"],
  "operations": [
    {
      "replace": {
        "find": "fn process(data: String)",
        "with": "fn process(data: &str)",
        "limit": 1
      }
    }
  ],
  "dry_run": false
}
```

**Execute**

```bash
sd2 apply --manifest manifest.json
```

Either *everything* succeeds, or *nothing* changes

---

## 🛠 Advanced Usage

### Input Modes

`sd2` is explicit about how input is interpreted
Stdin is auto-detected, but behavior can be forced when needed

* **`--stdin-text`**
  Treat stdin as raw text content to modify (classic filter mode)

* **`--files0`**
  Read null-terminated file paths from stdin
  Compatible with `find . -print0`

* **`--rg-json`**
  Structured mode that consumes `ripgrep --json` output and applies edits **only** to matched lines

```bash
# Targeted patching using ripgrep's match locations
rg --json "foo" | sd2 --rg-json "foo" "bar"
```

No re-scanning
No fuzzy matching
Only the exact spans reported by `rg`

---

## 📐 Design Principles

* **Pipeline-First**
  `sd2` never re-implements file walking
  Traversal is delegated to `ignore`, `fd`, or `ripgrep`

* **Atomic Writes**
  Files are written to temporary paths and renamed on success
  Permissions and ownership are preserved

* **Structured Errors**
  When stdout is piped, errors are emitted as clean JSON
  Tooling can recover or retry programmatically

* **UTF-8 by Default**
  No locale branching
  No encoding ambiguity

---

## 📦 Installation

*(Pending crates.io release)*

```bash
cargo install --path .
```

---

## License

MIT
