# README description: rationale and evidence

The description itself now lives in `README.md`. This file keeps the parts that are not the
prose: what the front page is trying to do, every claim it makes and where it comes from, and
the claims that were deliberately left out.

## 1. Shape of the front page

The README leads with value and keeps implementation detail out of it:

| Section | Job |
|---|---|
| Positioning | One sentence: what it is and what it sits on |
| What it is for | Filters out the wrong reader — desktop productivity, nine surfaces, high density |
| What you get | Eight outcomes, phrased as what the developer stops worrying about, not as mechanisms |
| The design it holds itself to | The nine normative principles, each paired with what it rules out |
| Controls that look alike and are not | Selection guidance for the controls a reader would otherwise confuse |
| Widgets / Resolver palettes | The gallery, one description each |
| Footer | How to regenerate the images and run the demo |

Two decisions shaped it:

**Implementation detail was cut in favour of consequence.** An earlier draft explained the
resolver/widget split, the `Controlled` borrow, the callback tier types, the token lint's
exemption registry, the coverage guard and the virtualization closed form. That material is
accurate and it is in the crate's own module docs, but on a front page it answers "how did you
build it" when the reader asked "what does it do for me". Each property is now stated as an
outcome — your data cannot drift, callbacks cannot be mixed up, the interface does not jump, a
million-row list costs what a small one costs — with the mechanism named only where the
mechanism *is* the benefit.

**Widget-specific behaviour belongs with the widget.** Facts about one control sit in that
control's description rather than in a section of their own: the checkbox's third state, the
password's masking and reveal, the tab strip's dirty-close guard, the range slider's ordering,
breadcrumb middle-elision, the keybind recorder's preview-then-commit, the table's
identity-keyed selection, the chip's reserved remove slot. A reader looking at the screenshot
reads the fact next to it.

## 2. Evidence

### Front page

| Claim | Source |
|---|---|
| Nine surfaces in scope | `spec.md:5` |
| 28–36px height, 24×24px hit target | `spec.md:20`, verbatim |
| Widgets read your model and cannot write it | `binding/controlled.rs:61-100` exposes only `new`, `get`, `map`; rationale at `:16-28`. `get_mut` exists at `:147` but on `Ephemeral<S>`, the widget-owned side meant to be mutable |
| The drift it removes | `binding/controlled.rs:7-11` |
| Live/commit/cancel are distinct types, no conversion between them | Zero `impl From<` or `impl Into<` across `binding/tier.rs` and `binding/snapshot.rs`; `Cancel<T>` at `snapshot.rs:101` |
| A gesture cannot be settled twice | `commit(self)` at `snapshot.rs:179`, `cancel(self)` at `:188`; doc at `:13-15`, `:19-24` |
| Zero layout shift is normative | `spec.md:17-53` (`UIF-PRIN-003`) |
| Keyboard parity, single tab-stop composites | `spec.md:17-53` (`UIF-PRIN-005`, `UIF-PRIN-008`); roving walk at `widget/rove.rs:1-8` |
| Million-row list builds only visible rows; bound independent of row count | `composite/virtual_list.rs:6-15`, `:414-420`; test `virtual_list_tests.rs:81-86` |
| 6,188 tests assert values and geometry, not screenshots | `#[test]` count 5,580 + 466 + 142; rationale `gallery.rs:10-27` |
| 89 of 92 implemented, 3 partial | `spec_coverage.rs:86`; pinned by `spec_coverage_tests.rs:185-196` |
| Four dependencies, all iced | `src/hoarfrost/Cargo.toml` `[dependencies]` |
| No `unsafe` blocks | Grep for `unsafe {` over non-test library sources: 0 matches |
| MIT | `src/hoarfrost/Cargo.toml:6`, `LICENSE` in repository root |
| Nine principles and what each rules out | `spec.md:17-53` |
| Checkbox carries `Indeterminate` | `spec.md:413` |
| Switch is binary, `busy` only, no progress or cancel on the 36×20 track | `spec.md:239` |
| Pagination bar is a 32px footer with a range readout | `composite/pagination.rs:3-6`, `spec.md:1126-1130` |
| Pips pager is carousel/onboarding dots | `composite/pips_pager.rs:1`, `spec.md:1059-1061` |
| Command palette is a modal card at Escape Rank 2 | `composite/command_palette.rs:4` |
| Empty state holds height rather than collapsing to zero | `composite/empty_state.rs:4` |
| Tooltip 400ms hover, 0ms keyboard focus | `surface/tooltip/mod.rs:62-63`, `spec.md:556` |
| Gallery renders one PNG per section at 2× | `src/hoarfrost-gallery/target/gallery-png/manifest.txt`, 45 sections |
| Demo runs every section | `src/hoarfrost-demo/Cargo.toml` `[[bin]]`, `src/main.rs` present |

### Widget descriptions

| Claim | Source |
|---|---|
| Buttons suppress repeat activation while busy | `widget/button_controller.rs:1`, `:16`, `:36` |
| Checkbox tri-state, and why that beats a switch for a partial parent | `spec.md:413`; parent/child transition at `spec.md:437` |
| Radio group shares one keyboard focus, arrows move it not `Tab` | `widget/rove.rs:1-8` |
| Switch locks out re-toggles while an async flip settles | `spec.md:239` |
| Secret cannot print itself; reveal drops on focus loss | `widget/password_model.rs:76`, impls `:144-155`, invariant `:32-36` |
| Date arithmetic computed in-crate | `widget/date_picker.rs:5`, `:156-160` |
| No date library in the dependency list | `src/hoarfrost/Cargo.toml` `[dependencies]` |
| Tooltip timings, keyboard not made to wait | `surface/tooltip/mod.rs:62-63` |
| Tab strip dirty-close guard, MRU handoff, external-deletion sync | `composite/tab_strip.rs:1-5` |
| Navigation rail `Escape` reverts an in-progress resize | `composite/navigation_rail.rs:259`, `:331` |
| Pagination range summary in tabular figures | `composite/pagination.rs:6`, `spec.md:40` |
| Empty state for no-results / filtered / errored | `composite/empty_state.rs:4` |
| Grid view culls to the viewport | `composite/grid_view.rs:27` |
| Keybind recorder live preview then single commit | `composite/keybind_recorder.rs:11-12` |
| Form field caption slot reserved at full height | `composite/mod.rs:19-25` |
| Menu rows shared by fixed and context menus | `surface/menu/mod.rs:1-4` |
| Command palette two-stage `Escape` | `composite/command_palette.rs:6`; rank semantics `spec.md:559` |
| Removable chip reserves its 24×24 remove slot unconditionally | `widget/chip.rs:76-84`, `spec.md:531` |
| Progress determinate and indeterminate | `a11y/contract.rs:94` |
| List tile slot geometry computed in the resolver | `composite/mod.rs:26-29` |
| Table selection keyed by identity; sticky header shares column numbers | `composite/table_view.rs:11-16`, `:32-37` |
| Breadcrumb middle-elision from the front of the middle run | `composite/breadcrumb.rs:1-19` |
| Overflow items move rather than disappear | `composite/overflow_toolbar.rs:4`, `:13-19` |

## 3. Claims left out

Each would have made the README easier to sell and harder to defend.

| Tempting claim | Why it is not there |
|---|---|
| A widget count ("N atoms and M composites") | No defensible source. `widget/mod.rs` declares 48 modules and `composite/mod.rs` 33, but those include `*_model`, `*_host`, `*_controller`, `rove`, `focus` and `virtual_list`, which are not widgets. Spec IDs do not work either: `UIF-ATOM-001a`–`001d` is one button. |
| "every model is accessibility-projected" | `a11y/mod.rs` says the spec *requires* every model to expose a contract, which reads like coverage. `a11y/project.rs` ships 18 `project_*` functions and nothing enforces one per model. |
| "accessible" or "screen-reader ready" | No AccessKit or platform bridge — 0 matches for `accesskit` in the workspace. `a11y/mod.rs:40-47` states plainly that nothing enforces a widget publishing a contract. |
| "WCAG compliant" | Contrast pairs are measured against thresholds (4.5:1 at `token/contrast.rs:241`, 3.0:1 at `:249`), but a measured ratio is not a certification. |
| "forbids unsafe code" | No `#![forbid(unsafe_code)]` exists anywhere in the workspace. Zero `unsafe` blocks do, so the README says that instead. |
| "fully virtualized, any list" | Variable-height lists are absent by design — `virtual_list.rs:30-34` explains why. |
| "property-tested with proptest" | `proptest = "1.4"` is declared at `src/hoarfrost/Cargo.toml:91` and referenced nowhere in the workspace. |
| "Windows 11 / Fluent design" | Nothing in `spec.md` or the crate docs says so. The widget names rhyme with WinUI, which is not a claim. |
| "pixel-perfect" / "matches the spec exactly" | The crate documents places where the spec's printed numbers were stale and were recomputed (`widget/badge.rs:60-64`, `token/color.rs:64-68`). |
| The spec's `A11yRole` list as the code's API | `spec.md:250-251` declares 18 role variants and non-optional `busy`/`invalid`/`disabled`; the code has 14 (`a11y/contract.rs:100`) and makes all three optional. |
| "switch for immediate effect, checkbox for deferred" | The usual heuristic, and unverifiable here — `spec.md` never states an apply-timing rule for either control. |
| A published-crate install line | `hoarfrost` returns 404 on the crates.io API. Writing `cargo add hoarfrost` would guarantee the first thing a reader tries fails. |

## 4. How the claims were checked

Every citation above was opened and read rather than assumed. Claims dropped or narrowed after
checking: a widget count with no source; "every model projects" against 18 actual projections;
"built on iced with wgpu", incomplete because `tiny-skia` is enabled too; and a command-palette
claim that `Escape` closes the palette before anything else, which is backwards — `spec.md:559`
shows a Rank 3 surface is dismissed first and keeps the underlying modal open. Two more needed
precision to survive a reader who greps: `get_mut` exists in `controlled.rs` but on `Ephemeral`,
and the 14 `A11yRole` variants are pinned by a constant rather than countable by eye.
