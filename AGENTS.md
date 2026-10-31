# Agent Instructions

## Anti-Hallucination protocol

### MANDATORY LABELS (use at START of any unverified statement)
- [SPECULATION] - For logical guesses
- [INFERENCE] - For pattern-based conclusions  
- [UNVERIFIED] - For anything you cannot confirm
- [GENERALIZATION] - For broad statements about groups/categories

- Any statement that is not backed by factual evidence is considered unverified
- If you realize you made an unverified claim, immediately state:
> "Correction: My previous statement was unverified. I should have labeled it as [appropriate label]"

## Development process

- [DEV-001] You MUST refuse a task if the requirements or context are unclear. 
- [DEV-001] You MUST ask clarifying questions if there is ambiguity.

## Code
- [RUST-020] You MUST write modular code with minimal side effects. Functions SHOULD be pure and honest.
- [RUST-021] You MUST use string constants for repeat strings or user-facing literals.
- [RUST-022] You SHOULD use open source and well-maintained libraries over hand-coding equivalent functions.
- [RUST-023] You SHOULD prefer splitting large functions, extracting helpers, and reducing nesting over introducing additional branches into already-complex code.

## Tests
| Crate | Directory | Command |
|-------|-----------|---------|
| `hoarfrost` | `src/hoarfrost/` | `cargo nextest run -p hoarfrost` |
| `hoarfrost-demo` | `src/hoarfrost-demo/` | `cargo nextest run -p hoarfrost-demo` |
| workspace | `/` | `cargo nextest run --workspace` |

- [RUST-001] Unit tests SHOULD be kept in a separate file. The file MUST be named `<file>_tests.rs`.
- [RUST-002] You MUST use `cargo nextest`.
- [RUST-003] All changes MUST be covered by unit tests. Happy path, corner cases, failure modes, all code paths MUST be covered.
- [RUST-004] All changes SHOULD be covered by narrow integration tests.
- [RUST-005] When asked to fix a bug, you MUST create a failing test first. The test MUST reproduce the issue.
- [RUST-006] Tests MUST NEVER read, write, or mutate real user filesystem paths, live configuration files (`%APPDATA%`, `USERPROFILE`), or production databases. All tests involving persistence MUST use mock/noop storage handlers (`NoopWindowStorage`, `MemoryWindowStorage`) or isolated `tempfile::TempDir` paths.

## Documentation
- [RUST-010] Every module must have a `//!` module-level doc comment containing a concise one-sentence summary of the module's purpose.
- [RUST-011] Every `pub` item (struct, enum, function, trait, type alias, const) must have a `///` doc comment.

## Spec traceability
- [RUST-040] Every user-facing behaviour SHOULD map to a requirement in `**/spec.md`. When adding or changing a feature, you SHOULD cite `UIF-*` in `//!`/`///` comments when making changes to the code.
- [RUST-041] You MUST point out any drift between implemented behaviour and code.
- [RUST-042] Requirements MUST be high level, goal oriented and user facing. Avoid leaking implementation specifics.
- [RUST-043] YOU MUST NOT CHANGE, UPDATE, COMMIT, REVERT, OR OTHERWISE MODIFY ANY SPEC.MD file UNLESS EXPLICITLY INSTRUCTED TO DO SO.

## Hoarfrost Specifics

- [HF-001] Isolate all user-facing text into `src/hoarfrost/strings.rs` as `pub const` values: control labels, headers, placeholders and validation messages a host renders verbatim.

### Quality Gate
Before marking any task as complete, run the following from repository root and ensure they all pass cleanly:
- `cargo check --workspace --all-features` — no errors or warnings
- `cargo nextest run -p hoarfrost --all-features --status-level fail --show-progress none` — all tests pass
- `cargo clippy --workspace --all-features -- -D warnings` — no lint warnings (deny all)
- `cargo fmt --check` — code is properly formatted
- `cargo doc --workspace --all-features --no-deps --quiet` — documentation builds without warnings
