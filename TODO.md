# TODO

## Planned Features

- [ ] Add replacement validation modes (`strict|warn|none`) (`src/replacer/*`, `src/cli.rs`, `src/model.rs`) (needs design decision)
  - [ ] Define semantics for ambiguous `$1foo` when `--expand` is on/off (invariant: no silent mis-expansion; verify: unit tests)

## Bugs

- [ ] Make JSON skip reasons non-lossy (`src/reporter.rs`, `src/events.rs`) (needs design decision)
  - [ ] Remove the `SkipReason::NotModified` fallback for unknown reasons (invariant: unknown reasons are not silently re-labeled; verify: `cargo test`)

## Refactoring

- [ ] Remove unused args from `Replacer::new` (`src/replacer/mod.rs`, `src/engine.rs`) (invariant: API matches behavior; verify: `cargo test`)
  - [ ] Drop unused `_case_sensitive` and `_crlf` parameters and update call sites (verify: `cargo test`)

## Documentation

- [ ] Align `docs/JSON_EVENTS.md` with actual emitted JSON (`docs/JSON_EVENTS.md`, `src/events.rs`, `src/reporter.rs`) (invariant: docs match code; verify: run `sd2 --format=json` and compare)
