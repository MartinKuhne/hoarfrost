# Plan: `hoarfrost-gallery` — per-section PNG renderer for documentation

Status: approved for execution. Decisions locked by the requester are marked **DECISION**.

---

## 1. Objective

A command-line program, `hoarfrost-gallery`, that renders each hoarfrost widget demo
section to its **own PNG file**, for use in documentation and a widget gallery.

**DECISION — render unit:** one PNG per demo *section*, where a section already shows
one widget across its states (live / disabled / busy / selected / …), exactly as the
demo window presents it. Not one PNG per `(variant × state)` cell.

---

## 2. Verified ground truth

Everything below was read from the repo, not assumed.

| Fact | Evidence |
|------|----------|
| Workspace members are `src/hoarfrost`, `src/hoarfrost-demo` | `Cargo.toml:3-6` |
| `hoarfrost` uses a flat layout: `[lib] path = "mod.rs"` | `src/hoarfrost/Cargo.toml:11-12` |
| A `gallery` **binary already exists** and writes **text**, not PNGs | `src/hoarfrost/bin/gallery.rs:151-203` |
| It writes `manifest.txt` + `cells/<id>.txt` via `cell.snapshot()` | `bin/gallery.rs:158-162` |
| It has a reusable output-path guard refusing `%APPDATA%`/`%LOCALAPPDATA%`/`%USERPROFILE`/`.hoarfrost` | `bin/gallery.rs:36-120` |
| `hoarfrost::gallery` cells hold **resolved values + recorded quads**, *not* `iced::Element` | `gallery.rs:117-135`, `255-284` |
| `gallery::matrix()` is the enumeration entry point | `gallery.rs:1075-1114` |
| `test_helpers` is shipped to downstream crates via `feature = "test-helpers"` | `src/hoarfrost/Cargo.toml:22-38` |
| `headless_renderer()` is `pub` and builds iced's real renderer (~620 ms debug) | `test_helpers.rs:944-951` |
| `renderer.screenshot(physical, scale, background) -> Vec<u8>` RGBA is the raster primitive | `test_helpers.rs:985-990`, `1363-1375` |
| `RecordCandidates` is a `pub` operation recording `(kind, id, bounds)` for every widget | `test_helpers.rs:605-692` |
| `PIXEL_SCALE = 2.0` | `test_helpers.rs:931` |
| `png 0.18.1` is already in the lock graph | `Cargo.lock:2437` |
| `target` is git-ignored | `.gitignore:4` |

### 2.1 Why the demo cannot simply be imported

`hoarfrost-demo` is a **bin-only** crate (`publish = false`, no `[lib]`), and ten of its
modules are `#[cfg(test)]`-gated (`main.rs:9-38`). A sibling crate therefore cannot
`use hoarfrost_demo::pane_atoms`. This is what forces **DECISION — catalog source**:
give `hoarfrost-demo` a `lib` target so the catalog is shared rather than duplicated.

### 2.2 The design tension, stated openly

`gallery.rs:24-27` and `previews.rs:17-20` both argue *against* rasterised PNG goldens:

> "a frozen PNG pins every glyph, antialiasing kernel and rasteriser version in the
> crate, so it fails on unrelated upgrades and gets regenerated until it stops
> meaning anything."

That argument is correct **and does not apply here**, because the purpose differs:

- `hoarfrost::gallery` PNGs would be **assertions** — a test comparing a bitmap.
- `hoarfrost-gallery` PNGs are **documentation artifacts** — regenerated on demand,
  never compared, never committed (**DECISION — output handling**: CLI argument,
  git-ignored output).

The plan preserves the existing philosophy by keeping the text matrix as the assertion
target and adding PNGs as a separate, non-authoritative artifact. No existing test may
be changed to compare pixels.

---

## 3. Architecture

### 3.1 New crate `src/hoarfrost-gallery/`

Conventional layout, `lib` + `bin` in one crate so the logic is unit-testable
(`RUST-001`/`RUST-003`) and `main.rs` stays a thin composer.

```
src/hoarfrost-gallery/
  Cargo.toml
  src/
    lib.rs           module graph + re-exports
    main.rs          parse args -> run -> exit code (composes, does not implement)
    strings.rs       all user-facing CLI text as pub const (HF-001 convention)
    cli.rs           argument parsing: pure, table-driven, no new dependency
    catalog.rs       render targets sourced from hoarfrost-demo's lib
    capture.rs       measure + rasterise one section -> RGBA + dimensions
    png.rs           RGBA -> PNG file (via the `png` crate, RUST-022)
    output.rs        output-dir guard + filename sanitisation
    manifest.rs      index of every emitted PNG
    <module>_tests.rs  sidecar per module
```

Dependencies: `hoarfrost` (with `test-helpers`), `hoarfrost-demo` (path), `iced`
(**feature set must match `hoarfrost`'s exactly** — `wgpu`, `tiny-skia`, `fira-sans`,
`advanced-shaping`, `advanced`; see `src/hoarfrost/Cargo.toml:54-60` and the C3 warning
at `hoarfrost-demo/Cargo.toml:11-13`), `iced_runtime` (for `UserInterface::build`),
`png`. Dev-dependency: `tempfile` (`RUST-006`).

### 3.2 The capture algorithm (the crux)

A `Section`'s card is `Length::Fill` width (`section.rs:295`), so it has no intrinsic
size. Two passes per section, mirroring the "why two builds" note in
`render_section.rs:14-19` (`Element` is not `Clone`, so geometry and pixels come from
two calls to the same pure builder):

1. **Measure** — build the section at `Size::new(width, PROBE_HEIGHT)`, run
   `RecordCandidates`, take the record where `id == section.id` and
   `kind == KIND_CONTAINER`. That is the card's own bounds.
2. **Raster** — rebuild the section, build at `Size::new(card.width, card.height)`,
   `draw`, then `renderer.screenshot(physical, scale, background)`.

Because the card is Fill-width and natural-height, rebuilding at its own measured size
reproduces it exactly, so the PNG is tight without crop arithmetic.

One `HeadlessRenderer` is built **once** for the whole run via
`hoarfrost::test_helpers::headless_renderer()` — at ~620 ms, a per-section renderer
would make a ~50-section sweep take half a minute in the renderer alone.

**Font parity:** `hoarfrost_demo::font_fallback::register()` must be called before the
first frame, exactly as `DemoApp::boot` does (`app.rs:101-102`), or glyphs in the PNGs
resolve differently from the demo window.

**Failure mode is reported, not papered over:** if a section's card is not found in the
measure pass, that section is recorded as failed and the run exits non-zero. It must
not silently emit a wrong-sized image.

### 3.3 CLI

```
hoarfrost-gallery [OUTPUT_DIR]
    --width <px>     card width                (default 700, matching PANE_VIEWPORT)
    --scale <f>      physical pixels per logical pixel (default 2.0 = PIXEL_SCALE)
    --filter <frag>  only sections whose id contains <frag>
    --help
```

Default `OUTPUT_DIR` is `target/gallery-png` — git-ignored by construction
(`.gitignore:4`), satisfying **DECISION — output handling** with no `.gitignore` edit.

Exit codes mirror `bin/gallery.rs`: `0` success, `2` refused path / bad arguments,
`1` a render or write failed.

`output.rs` reuses the `refusal`/`guard` logic from `bin/gallery.rs:36-120`
(verbatim copy with attribution, rather than extracting it into `hoarfrost` and
widening the published crate's surface for a downstream tool's convenience).

### 3.4 What is deliberately out of scope

- **Hover / pressed / focused states.** A static section cannot show them without
  driving events. They remain covered by the existing text matrix
  (`gallery::matrix()`), which is the assertion target. This gap is reported in the
  manifest rather than hidden.
- **Light theme.** The demo and `section()` hardcode `Palette::dark()`
  (`section.rs:263`). Adding a theme axis is a demo change, not a gallery change.
- **Committed goldens.** No PNG is committed; no test compares PNG bytes.

---

## 4. Subagent work split

Two subagents, **disjoint file ownership** so they cannot conflict. I supervise: review
each diff, run the quality gate myself, and do not accept a subagent's "done" without
evidence.

| | Subagent A — demo lib extraction | Subagent B — gallery crate |
|---|---|---|
| **Owns** | `src/hoarfrost-demo/**` only | `src/hoarfrost-gallery/**`, root `Cargo.toml` |
| **Must not touch** | root `Cargo.toml`, `src/hoarfrost/**`, any `spec.md` | `src/hoarfrost/**`, any `spec.md` |
| **Depends on** | nothing | A's lib target, for `catalog.rs` only |

### Subagent A — give `hoarfrost-demo` a lib target

1. Add `src/hoarfrost-demo/src/lib.rs` declaring `pub mod` for: `app`, `chrome`,
   `data`, `font_fallback`, `menu_surface`, `pane_atoms`, `pane_composites`,
   `previews`, `section`, `theme`.
2. Keep the ten `#[cfg(test)]` modules (`alignment_audit*`, `glyph_resolution`,
   `id_namespace`, `layout_gate`, `layout_probe`, `render_*`) test-only **in `lib.rs`**,
   so they still compile and run under `cargo nextest -p hoarfrost-demo`.
3. Reduce `main.rs` to the iced builder chain, importing from the lib.
4. `Cargo.toml`: add `[lib] name = "hoarfrost_demo" path = "src/lib.rs"` and
   `[[bin]] name = "hoarfrost-demo" path = "src/main.rs"`.
5. Module-internal `crate::…` paths stay valid — the modules move together and remain
   siblings. No call-site churn expected.

**Gate:** `cargo nextest run -p hoarfrost-demo` green with the **same test count as
before** (the `render_*` ink tests must still run, not silently vanish), clippy clean,
`cargo run` still opens the window.

### Subagent B — build `hoarfrost-gallery`

Round 1 (parallel with A, no dependency on it): `Cargo.toml`, `strings.rs`, `cli.rs`,
`output.rs`, `png.rs`, `manifest.rs`, `capture.rs` + sidecar tests for each, using a
synthetic local section builder so these modules are verifiable standalone.

Round 2 (after A lands): `catalog.rs` wired to the demo lib, `main.rs`, and the
end-to-end integration test.

**Interface contract A must provide** (B codes against this, so they can run in parallel):

```rust
hoarfrost_demo::app::Message
hoarfrost_demo::font_fallback::register()
hoarfrost_demo::section::Section<'a> { element: Element<'a, Message>, widgets: usize, id: String }
hoarfrost_demo::pane_atoms::{AtomsState, live_sections(&AtomsState) -> Vec<Section<'_>>}
hoarfrost_demo::pane_composites::{CompositesState, group1_sections(..), group2_sections(..)}
hoarfrost_demo::previews::{atom_preview_sections() -> Vec<Section<'static>>, composite_preview_sections() -> Vec<Section<'static>>}
```

All of these are already `pub` today (`pane_atoms.rs:512`, `pane_composites.rs:552,742`,
`previews.rs:211,402`, `section.rs:209`) — A only has to make them reachable.

**Note on enumeration:** `PaneBody` folds sections into a `Column` and cannot be
enumerated (`section.rs:334-342`), so `catalog.rs` calls the `*_sections` functions
directly and never goes through `*_pane`.

### Test obligations (`RUST-003`, `RUST-004`, `RUST-006`)

- `output_tests.rs` — refuses the three env roots and any `.hoarfrost` component;
  allows a temp subdir and `target/…`; relative paths resolved then re-checked;
  filename sanitisation and collision handling.
- `cli_tests.rs` — defaults, every flag, unknown flag, `--help`, filter semantics.
- `png_tests.rs` — a known 2×2 RGBA round-trips through a `TempDir`; magic bytes and
  `IHDR` dimensions correct.
- `capture_tests.rs` — measure finds a synthetic card; raster returns exactly
  `w*h*4` bytes; a missing card is reported as a failure.
- `catalog_tests.rs` — non-empty, duplicate-free ids, every id filename-safe.
- Integration — a full sweep into a `TempDir` writes one PNG per section plus a
  manifest, and two runs produce identical file sets and identical byte sizes.

Every test uses `tempfile::TempDir` or the memory/noop paths. **No test may write to
`%APPDATA%`, `%LOCALAPPDATA%` or `%USERPROFILE%`.**

---

## 5. Acceptance criteria

From repo root, all clean (`AGENTS.md` Quality Gate):

```
cargo check --workspace --all-features
cargo nextest run --workspace
cargo clippy --workspace --all-features -- -D warnings
cargo fmt --check
cargo doc --workspace --all-features --no-deps --quiet
```

Plus, demonstrated by running the binary:

- `cargo run -p hoarfrost-gallery -- --help` prints usage and exits 0.
- `cargo run -p hoarfrost-gallery -- target/gallery-png` writes one PNG per section
  plus a manifest, exits 0, and prints a summary in the style of `bin/gallery.rs`.
- A refused path (`%APPDATA%\x`) exits 2 with a message naming the reason.
- Spot-check at least three PNGs (a button section, a composite, a resolver-only
  preview) and confirm they are non-blank and correctly sized.

Documentation (`RUST-010`, `RUST-011`): `//!` on every module, `///` on every `pub`
item. Spec traceability (`RUST-040`): cite `UIF-VER-004` where the PNG sweep is described
as the raster companion to the text matrix. **No `spec.md` file is modified**
(`RUST-043`).

---

## 6. Risks

| # | Risk | Mitigation |
|---|------|-----------|
| R-1 | A's lib extraction silently drops the `#[cfg(test)]` `render_*` ink tests | Gate on identical test count before/after; supervisor verifies the count personally |
| R-2 | `iced` feature sets drift between the three crates, so re-exported types stop lining up | Copy the feature block verbatim; `cargo tree -e normal` sanity check |
| R-3 | **Determinism.** If the headless renderer resolves to a GPU-backed wgpu adapter rather than tiny-skia, PNG bytes may vary across machines | Integration test asserts stable *dimensions and file set*, not byte equality; byte-equality asserted only within one run on one machine, and the limitation is documented |
| R-4 | A section whose content overflows `PROBE_HEIGHT` measures short | Measure pass asserts the card fits the probe and reports the section as failed otherwise |
| R-5 | Full sweep is slow (~50 sections × layout + raster) | One shared renderer; `--filter` for single-section runs; the sweep lives in one integration test, not fifty |
| R-6 | Section titles carry spec refs (`"Tab strip (UIF-TAB-003)"`), so filenames get long | `section_slug` already normalises to `[a-z0-9-]`; filenames derive from `section.id` with the `demo.section.` prefix stripped |

---

## 7. Execution outcome (executed 2026-10-31 / 11-01)

Delivered and verified. Two subagents were supervised across three rounds; every claim
below was re-run by the supervisor rather than taken from the agents' reports.

### 7.1 Measured result

| | |
|---|---|
| PNGs per full run | **45**, ~2.06 MB, all non-flat (min 23 distinct sampled colours) |
| Sweep wall time | ~10.8 s, plus ~0.75 s to build the headless renderer once |
| `hoarfrost-gallery` tests | **142**, all passing |
| `hoarfrost-demo` tests | **464 — unchanged**, the R-1 gate held |
| Workspace | 6185 tests, 6182 pass, 3 fail (all pre-existing, §7.4) |
| `cargo run -p hoarfrost-gallery` | exit **0**, 45 rendered / 0 failed |

### 7.2 Plan drift (`RUST-041`)

**The plan's central assumption was wrong for one section.** §3.2 assumed every section
has a natural height. `demo.section.empty-states-uif-comp-014` does not:
`hoarfrost::composite::empty_state` lays its content out at `Length::Fill` height
(`src/hoarfrost/composite/empty_state.rs:266`), so the card measures whatever viewport it
is handed, at any probe height. Raising `PROBE_HEIGHT` does not help — it re-measures to
exactly the new ceiling.

Resolved with an explicit **declared render height** rather than a silent clip: a closed
table in `catalog.rs` maps section id → height, `capture::Renderer::shot_at` rasterises
at a declared box with no measure pass, and the manifest records
`declared height 668` in the detail column so a reader can always tell a measured PNG
from a declared one. The table entry documents the root cause and how to delete it if the
framework ever gives the widget a natural height.

### 7.3 A real bug the plan's own risk analysis produced

R-4 ("a section that overflows `PROBE_HEIGHT` measures short") was implemented as
`card.height > PROBE_HEIGHT`. That is wrong: iced derives layout limits from the
viewport, so an over-tall card is clamped to **exactly** `PROBE_HEIGHT` and passes a `>`
test. The tool would have emitted a silently-clipped PNG while reporting success — the
precise failure the check existed to prevent. Fixed to `>=`, with a test that builds a
card past the ceiling and asserts `CardTooTall`. This is what turned the empty-states
section from a silent bad image into a visible finding.

### 7.4 Pre-existing defects found, deliberately NOT repaired

All four were independently confirmed to exist at `HEAD` before this work, and all are
outside the task's scope. They are reported, not fixed.

1. **`cargo fmt --check` is red at HEAD.** Five committed files in `src/hoarfrost/`
   contain CRLF while `rustfmt.toml` sets `newline_style = "Unix"`:
   `checkbox_ink_probe_tests.rs`, `composite/side_sheet.rs`,
   `composite/side_sheet_tests.rs`, `strings.rs`, `strings_tests.rs`. Verified by
   reading CR bytes out of `git show HEAD:src/hoarfrost/strings.rs`.
2. **Three `layout_gate` goldens are stale.** They list 105 named widgets while the probe
   finds 111. Verified by stashing every change from this task and re-running: the same
   three fail on the untouched bin-only layout.
3. **Six rustdoc warnings in `src/hoarfrost`** — five `private_intra_doc_links` on
   `DEFAULT_*_ID` constants (`color_picker`, `color_swatch_button`, `date_picker`,
   `rating`, `time_picker`) and one unresolved link in `test_helpers.rs`. The five
   constants are the same five widgets the stale goldens are missing, so both defects
   trace to one unfinished change. `src/hoarfrost` is untouched by this work.
4. **`core.autocrlf = true`** in this clone silently rewrites LF files to CRLF on
   checkout. It converted three `hoarfrost-demo` files during a `git stash pop`; they
   were restored to LF. Anyone editing this repo should expect this.

### 7.5 Deviations from the plan's design

- `Catalog::build` returns `Option<Element>`, not `Element` — a panic would abort a run
  whose contract is "one bad section is recorded, not fatal".
- The render loop lives in `sweep.rs`, not `main.rs`, so it is reachable from a test.
- The draw uses `hoarfrost_demo::theme::demo_theme()` rather than the planned hardcoded
  `Theme::Dark`, so PNGs match what the demo window actually shows.
- Output is accepted as both a positional argument and `--out`.
