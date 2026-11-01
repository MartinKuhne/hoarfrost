# hoarfrost SDK Reference

Machine-oriented reference for the `hoarfrost` crate. Written for AI coding agents. Every
signature, constant name and constant value in this file is copied from the source tree.

## Contents

- [Overview](#overview)
- [Architecture & Core Concepts](#architecture--core-concepts)
- [API Reference](#api-reference)
  - [How to read an entry](#how-to-read-an-entry)
  - [Cross-cutting API conventions](#cross-cutting-api-conventions)
  - [Core Module / Class](#core-module--class)
  - [Composite layer](#composite-layer-composite)
  - [Surface layer](#surface-layer-surface)
  - [Geometry, layout and a11y](#geometry-layout-and-a11y-geometry-layout-a11y)
  - [Utility Functions](#utility-functions)
- [Configuration Reference](#configuration-reference)
- [Error Handling](#error-handling)
- [Appendix A - Spec / code drift](#appendix-a--spec--code-drift)

---

## Overview

### Identity

| Field | Value |
|---|---|
| Crate name | `hoarfrost` |
| Version | `0.14.0` |
| Edition | Rust 2024 |
| License | MIT |
| Repository | https://github.com/MartinKuhne/hoarfrost |
| Categories / keywords | `gui` / `iced`, `ui`, `widgets`, `design-tokens` |
| Library path | `src/hoarfrost/mod.rs` (not `src/lib.rs`) |
| Normative specification | `doc/spec.md` |
| Documentation generated | 2026-11-01 |

### Supported languages and frameworks

| Item | Value |
|---|---|
| Language | Rust, edition 2024 |
| UI framework | [iced](https://github.com/iced-rs/iced) `0.14.0` |
| Required iced features | `wgpu`, `tiny-skia`, `fira-sans`, `advanced-shaping`, `advanced` |
| Direct dependencies | `iced`, `iced_futures`, `iced_renderer`, `iced_runtime` (all `0.14.0`) |
| Dev dependencies | `iced_test 0.14.0`, `proptest 1.4` |
| Rendering targets | `wgpu` and `tiny-skia`; a headless software renderer is exposed for tests |
| `unsafe` code | None. The crate contains no `unsafe` blocks. |

The `advanced` iced feature is **REQUIRED**. Without it, `iced::advanced` is `cfg`-ed out and a
custom widget cannot be written at all. Do **NOT** remove it from your own `iced` dependency: the
re-exported iced types on both sides of the crate boundary **MUST** line up.

### Installation

`hoarfrost` is not published on crates.io. Add it from the repository:

```toml
[dependencies]
hoarfrost = { git = "https://github.com/MartinKuhne/hoarfrost" }
# Your iced feature set MUST match hoarfrost's exactly.
iced = { version = "0.14.0", features = [
    "wgpu", "tiny-skia", "fira-sans", "advanced-shaping", "advanced",
] }
```

The default feature set is empty. A plain dependency compiles the core only: tokens, state,
geometry, identity, strings, focus policy, bindings, validation and layout rules. It compiles
**no** widget. Select one feature per widget you instantiate. See
[Configuration Reference](#configuration-reference).

### Scope

The SDK targets desktop productivity applications. `doc/spec.md` names nine core surfaces as its
scope: main window, navbar, multiple panes, tabbed windows, input dialogs, modal dialogs, movable
overlays, fixed menus and context menus.

### Terms used in this document

| Term | Meaning. Do not substitute a synonym. |
|---|---|
| atom | One control with a complete interaction state machine (`widget::*`). |
| composite | A relationship between controls plus the text that explains them (`composite::*`). |
| surface | A layered, multi-widget surface (`surface::*`). |
| resolver | A pure function `(variant, state, palette) -> resolved style`. |
| model | The host-owned, declarative value a widget reads each frame. |
| host | The application that consumes this crate. |
| rung | One named step of a token scale (for example `space::MD`). |
| tier | One of the three callback classes: `Live`, `Commit`, `Cancel`. |

Normative keywords follow RFC 2119. **MUST** and **MUST NOT** are absolute requirements of the API
contract. **SHOULD** and **SHOULD NOT** are strong recommendations that you may override only with
a documented reason. **MAY** is optional. Words in ordinary case carry no normative weight.

### Ownership boundary — read this before you design a host

This rule is absolute and every API in the crate obeys it (`doc/spec.md` §5, decisions D-2 / D-8):

- This crate **MUST** supply sizing, frame and semantics only.
- The host **MUST** own surface lifecycle: open, close, dismiss.
- The host **MUST** keep the single writer to `SurfaceRegistry`.
- The host **MUST** inject all persistence.
- No host type appears in this API. The dependency points one way.
- A widget **MUST NOT** write the model it reads. There is no second copy of your data inside the
  framework.
- A widget **MUST NOT** self-register, self-unregister, or close the topmost surface on its own.

### Requirement traceability

Every user-facing behaviour maps to a `UIF-*` requirement in `doc/spec.md`. Modules cite the IDs
they implement in their `//!` comments. This document repeats those IDs in the heading of the
entry that implements them, for example
`#### \`widget::button\` - activation atom (\`UIF-ATOM-003\`)`. Where an entry covers a module with
many requirements, the IDs are listed in its `Overview`.

| Spec series | Subject |
|---|---|
| `UIF-PRIN-001..009` | The nine normative design principles |
| `UIF-TOK-001..007` | Design tokens |
| `UIF-ATOM-*` | Atomic widgets, §3 |
| `UIF-COMP-*` | Composite widgets, §3B |
| `UIF-WIN/NAV/PANE/TAB/INP/MOD/MOV/FMENU/CMENU/ZORD-*` | Core surfaces, §4–§5 |
| `UIF-VER-001..005` | Verification gates, §6 |

---

## Architecture & Core Concepts

### Layer model

```text
token/      closed token set: colour, space, size, shape, typography, time, contrast
  |
widget/     atoms        = resolver (pure) + iced widget (paints + drives state)
composite/  composites   = pure geometry/style solver + iced widget or Element builder
surface/    surfaces     = host-owned controller (pure state machine) + renderer
  |
cross-cutting: state, binding, focus, bridge, geometry, a11y, microcopy, layout,
               validation, identity, strings, invariant_gate, token_lint
```

Call sites **MUST** stay qualified. `token`, `widget` and `composite` deliberately do **NOT**
glob-re-export their children, because short rung names (`sm`, `md`, `lg`) collide across scales.
Write `token::space::MD` and `token::size::CONTROL_MD`, not `MD`.

### C1 — Tokenization (`UIF-PRIN-001`)

Every visual and temporal number comes from `token/` and nowhere else. Colour, spacing, sizing,
corner radius, stroke weight, type and duration are all declared inside `token/`. A raw numeric
literal in a widget file is a spec violation, not a style choice.

The only literals an atom file **MAY** carry are the absence of a property (a zero-width ring, a
fully transparent fill) and the opacity modifiers the spec prints. In both cases the value **MUST**
be a named constant. `token_lint` enforces this; see [Utility Functions](#utility-functions).

### C2 — Resolver / widget split

Each atom and composite has two halves:

- The **resolver** is a pure function from `(variant, state, palette)` to a resolved style value.
  It needs no window, no renderer and no GPU. The `UIF-VER-001` and `UIF-VER-003` gates assert on it.
- The **widget** is the iced `Element` that paints that value and drives state from real pointer
  and keyboard events.

Keep the split when you add a widget. A widget whose colour logic lives inside its `draw` cannot be
audited for contrast. A resolver with no widget cannot be seen.

Some modules are resolver-only or model-only. The tables in this document mark them explicitly. A
resolver-only module is complete and audited, but nothing paints it into a control a user can touch.

### C3 — The interaction state model (`UIF-ATOM-001`, `UIF-ATOM-002`, `UIF-VER-003`)

The resolved state is two parts, not one six-value enum:

- `BaseState` — the mutually exclusive visual: `Rest`, `Hovered`, `Pressed`, `Disabled`, `Invalid`.
- `VisualState { base, focused }` — that base plus an orthogonal focus ring drawn on top.

Precedence is `Disabled > Invalid > Pressed > Hovered > Rest`. `focused` resolves independently of
all five, except that a disabled control resolves `focused: false` as well: a focus ring on a dead
control advertises an affordance that cannot happen.

Two rules to remember:

- `pressed` without `hovered` is real and resolves to `Pressed`. It arrives when a pointer drags
  into a control and releases there, and when a harness or an accessibility layer synthesises the
  press.
- Validation is a property of the data, not of the pointer. `Invalid` outranks the pointer states so
  the error stays visible while the control is hovered, pressed or focused.

### C4 — Ownership: `Controlled` and `Ephemeral` (`UIF-ATOM-000b` §3.0.4.1)

Two newtypes carry the ownership boundary. There is no `trait WidgetModel`, on purpose.

- `Controlled<'a, M>` — a borrow of the host's model that the widget cannot write through. The host
  owns the value; the widget reads it.
- `Ephemeral<S>` — transient state the widget owns, such as hover or a drag in flight.

Pass a model to a widget in exactly one of three ways, and the way differs per widget:

| Style | Used by | Signature shape |
|---|---|---|
| `Controlled<'_, M>` | `Button::controlled`, `TextInput::controlled`, `Checkbox`, `RadioGroup`, `ToggleSwitch`, `SegmentedButton`, `SplitButton`, `Hyperlink` | `Widget::controlled(model.controlled(), focus_id, emit)` |
| plain `&'a M` | `ComboBox`, `TabStrip`, `NavBar` | `widget(&model)` |
| by value (a clone) | pickers, `RatingControl`, `Avatar`, `RangeSlider`, `GridView`, `SideSheet`, `NavigationRail`, `CommandPalette`, `Stepper`, `SplitDivider`, `KeybindRecorder`, `Skeleton`, `EmptyState` | `widget(model)` |

### C5 — The three callback tiers (`UIF-ATOM-000b` §3.0.4.2)

A live tick, a final commit and a cancellation are three distinct types. Passing one where another
is required is a compile error. The three tiers deliberately share **no** trait, so generic code
cannot erase the tier.

| Tier | Type | Fires | Host **MUST** |
|---|---|---|---|
| Live | `Live<T>` | every tick of a continuous gesture | paint immediately; **MUST NOT** write durable state |
| Commit | `Commit<T>` | exactly once, on settle or submit | write durable state |
| Cancel | `Cancel<T>` | on `Escape` or lost capture | restore `cancel.restored()`; the widget already reverted |

Two consumption shapes exist:

1. **Per-callback closures** — pickers and range widgets: `.on_change(f)`, `.on_commit(f)`,
   `.on_cancel(f)`.
2. **One `emit` closure over an `*Emission` sum** — atoms and hosts:
   `emit: impl Fn(ButtonEmission) -> Option<Message> + 'a`. Return `None` to emit nothing.

`LiveCoalescer<T>` keeps only the newest pending value, so a slow host never replays a stale tick.

### C6 — Busy actions and activation gating (`UIF-ATOM-000b` §3.0.4.3)

`ActionPending` is a value, not a clock read. `ActivationGate` is a latch. Together they make a
gesture impossible to settle twice.

- `ActionPending::{idle, indeterminate, determinate(progress), with_cancellable(bool)}`
- `ActivationGate::try_activate(&pending) -> Activation` returns `Fired` or
  `Suppressed(SuppressedReason::{Busy, AwaitingProjection})`.
- `ActionPending::try_cancel() -> Result<CancelRequest, CancelRefusal::{NotBusy, NotCancellable}>`
  hands out an opaque `CancelRequest` token. A cancel request **MUST** carry that token.

A busy button suppresses repeat activation instead of firing twice. It reserves its spinner slot
before the spinner exists, so nothing moves (`UIF-PRIN-003`).

### C7 — Focus: model, bridge, traps, roving (`UIF-PRIN-005`, decision D-7)

`focus::FocusState` is a pure policy model with no iced in it. `bridge` is the one-way projection
from that model into iced's focus router.

Rules the bridge enforces for the whole framework:

- The bridge runs strictly **model → router**. It never reads focus back. Read-back operations exist
  only in the `test_helpers` harness, so production code cannot reach them by accident.
- `Tab` is never handed to iced. `operation::focusable::focus_next` does not wrap — it *loses*
  focus at both ends. `bridge::tab_task` computes the target from `FocusState::focus_next` /
  `focus_previous`, which wrap, skip disabled members and honour the active trap.
- "Nothing is focusable" is a state, not a no-op. The bridge emits an explicit `unfocus`. It never
  emits `focus(id)` for an id the model does not name.

`FocusState` holds ordered groups of members, a current focus, and a stack of traps. A trap confines
traversal to one group and records the focus to restore on `pop_trap`.

### C8 — Z-bands and Escape ranks (`UIF-ZORD-001`, `UIF-WIN-003`)

Visual elevation and keyboard unwind priority are **separate** numbers. Do not confuse them.

| `ZBand` | index | Escape rank | Typical surfaces |
|---|---|---|---|
| `Workspace` | 0 | 0 | window, navbar, panes, tabs, rail, docked sheet |
| `UtilityOverlay` | 1 | 1 | movable overlays, modeless sheet, toast stack |
| `Menu` | 2 | **3** (unwinds first) | menus, flyouts, pickers, popovers |
| `Modal` | 3 | 2 | modals, input dialogs, command palette, modal sheet |

The unwind walk visits surfaces in Escape-rank order and stops at the first surface that answers.
`Declined(CannotUnwind)` consumes the key and terminates the walk: a later-rank surface never
closes from another surface's refusal.

A pointer-captured drag is **not** a registered surface, so `SurfaceRegistry::escape` cannot see
one. A host with drag-capable controls **MUST** call `escape_with_pre_rank`, which cancels the drag
first (Pre-Rank) and then walks the ranks.

### C9 — Geometry is pure

Every boundary solver is a pure function: anchor placement, position/size clamping, pane widths,
column widths, breadcrumb elision, toolbar overflow, virtual-list metrics, grid column solving.
Non-finite inputs are handled, not propagated. Because the geometry is pure, a test can prove a
requirement instead of observing it.

Two structural consequences of purity:

- **A band that is reserved is a band that is measured.** The form-field caption geometry function
  cannot see the caption's content. The missing parameter is the proof that validation text cannot
  move the form (`UIF-PRIN-003`).
- **A slot the spec fixes is a slot the host does not get to choose.** `list_tile::slots` computes
  the three-slot row here rather than leaving it to whatever lays the row out (decision D-8).

### C10 — Single tab-stop composites (`UIF-PRIN-008`)

A large composite is one tab stop, not one press per row. Roving arrow keys move the active member
inside the composite. `widget::rove::rove_index` implements the shared walk: skip disabled members,
wrap at both ends, never lose the keyboard on a member disabled underneath the focus.
`a11y::tab_stop::check_composite` asserts the rule mechanically.

### C11 — Reward early, punish late (`UIF-PRIN-009`)

`validation::EditStance` is `Pristine` or `Touched`. `validation::punish(stance, invalid,
disallowed)` returns a `Punishment` that separates *invalid* from *painted*. A pristine field that
is invalid is invalid but **not** painted: the error caption and error border stay withheld until
the user commits, blurs or submits. A disallowed character paints immediately.

### C12 — Time arrives as a value

Nothing in this crate reads a clock except `tooltip::hover::TimeMs::now()`. Every timing decision
takes a `TimeMs` argument that the host reads once per frame. Durations come from `token::time`,
and `token::time::resolve_ms` applies the mandatory reduced-motion fallback.

### C13 — Verification gates (`UIF-VER-001..005`)

| Gate | What it proves | Crate support |
|---|---|---|
| `UIF-VER-001` | token discipline and WCAG contrast | `token::contrast`, `token_lint` |
| `UIF-VER-002` | geometry, hit targets, responsive containers | pure solvers in `geometry`, `composite` |
| `UIF-VER-003` | state machine and callback lifecycle | `state::resolve`, `binding`, tier types |
| `UIF-VER-004` | resolved tokens and ordered draw layers | `Layer` structs, `test_helpers` |
| `UIF-VER-005` | microcopy and ergonomic invariants | `microcopy`, `layout`, `invariant_gate` |

Tests live in sibling `<file>_tests.rs` sidecars and assert resolved colours and measured geometry,
not screenshots. Run them with `cargo nextest run -p hoarfrost --all-features`.

### Minimal host skeleton

```rust
use iced::{Element, Length};
use hoarfrost::binding::Controlled;
use hoarfrost::token::{color::Palette, space};
use hoarfrost::widget::button_model::ButtonModel;
use hoarfrost::widget::button_widget::Button;

#[derive(Clone)]
enum Message { Saved }

struct View { save: ButtonModel }

fn view(view: &View) -> Element<'_, Message> {
    let button = Button::controlled(
        view.save.controlled(),          // read-only borrow of the host's model
        "Save",                          // content
        "demo.save",                     // FocusId; also the iced widget Id
        |_emission| Some(Message::Saved) // emit: Option<Message>
    )
    .palette(Palette::dark());

    iced::widget::row![button].spacing(space::SM).width(Length::Fill).into()
}
```

Every focusable widget takes a `FocusId`. Use the same id when you register the control in
`FocusState`, so a model transition and a click address the same control.

## API Reference

### How to read an entry

Each entry below is one API surface. It uses these sub-headings in this order:

| Sub-heading | Contents |
|---|---|
| Overview | What the surface is for, and the requirement it serves. |
| Prerequisites & Requirements | Cargo features, imports, and preconditions. |
| Syntax / Method Signature | Exact signatures, copied from source. |
| Examples | Minimal compilable usage. |
| Type references | Types a caller MUST understand to use the surface. |
| Return values | What comes back, including every failure arm. |
| Side effects | What changes outside the return value. |
| References | Links to the source files. |

Headings sit one level below the entry name so the entry name stays the anchor.

### Cross-cutting API conventions

Learn these seven rules once. They apply to almost every widget.

1. **Three ways to pass a model.** A widget takes its model as `Controlled<'_, M>`
   (shared borrow, host keeps ownership), as a plain `&'a M`, or by value.
   The signature says which. Check it before you assume ownership.
2. **Two callback styles.** Atoms built on the button engine take one closure,
   `emit: impl Fn(<X>Emission) -> Option<Message>`. Returning `None` means
   "this host has no handler for that tier". Pickers, sliders and pagers take
   named callbacks instead: `.on_change(...)`, `.on_commit(...)`, `.on_cancel(...)`.
3. **Tier wrappers carry meaning.** `Live<T>` is an in-progress tick,
   `Commit<T>` is a settled value, `Cancel<T>` is a revert instruction that
   carries the value to restore. Never unwrap one into the other by accident.
4. **`From<Widget> for Element` needs `Message: 'a + Clone`.** This is true for
   every widget except `Avatar`, `Hyperlink` and `Focusable`, which need only
   `Message: 'a`. Your message enum MUST therefore be `Clone`.
5. **Geometry helpers are pure.** Functions named `*_bounds`, `*_rect`,
   `resolve_style`, `*_layer` take values and return values. They mutate nothing.
   You SHOULD call them directly in tests; no widget tree is needed.
6. **Style resolution is separate from painting.** Every widget exposes a
   `resolve_style(...)` free function. You SHOULD use it to assert a colour instead
   of rendering a frame.
7. **Feature gates are per widget.** Importing a gated module fails to compile.
   Add the feature first. See [Configuration Reference](#configuration-reference).

### Core Module / Class

#### `token` — design tokens

##### Overview

The `token` module holds every visual constant in the framework: colour, type,
spacing, size, shape, motion, and the contrast rules that judge them.
It adapts nothing and resolves no style. It is values plus pure derivations.
Requirement: `UIF-PRIN-001` (no visual value outside the token layer).

##### Prerequisites & Requirements

- No cargo feature. `token` is always compiled.
- Import paths: `hoarfrost::token::color`, `::typography`, `::space`, `::size`,
  `::shape`, `::time`, `::contrast`.
- The module re-exports nothing. Import the submodule you need.

##### Syntax / Method Signature

```rust
// token/color.rs
pub struct Palette { /* 23 pub iced::Color fields */ }
impl Palette { pub const fn dark() -> Self; }

pub struct EditorPalette { pub background: Color, pub text: Color, pub error: Color }
impl EditorPalette { pub const fn inverted() -> Self; }

// token/typography.rs
pub enum Weight { Regular, Medium, SemiBold, Bold }
pub enum Family { Sans, Mono }
pub type FontFamily = Family;
pub const FAMILY_SANS: Family = Family::Sans;
pub const FAMILY_MONO: Family = Family::Mono;

pub struct TypeRung {
    pub size: f32,
    pub line_height: f32,
    pub weight: Weight,
    pub emphasis_weight: Weight,
    pub allowed_weights: &'static [Weight],
    pub tracking: f32,
}
pub const CAPTION: TypeRung;      // 12 / 16
pub const UI_COMPACT: TypeRung;   // 13 / 18
pub const BODY: TypeRung;         // 14 / 20
pub const TITLE_SM: TypeRung;     // 16 / 24
pub const TITLE_MD: TypeRung;     // 20 / 28
pub const TITLE_LG: TypeRung;     // 24 / 32
pub const MIN_SIZE: f32 = 12.0;

// token/space.rs
pub const XXS: f32 = 2.0;   pub const XS: f32 = 4.0;   pub const SM: f32 = 8.0;
pub const MD: f32 = 12.0;   pub const LG: f32 = 16.0;  pub const XL: f32 = 24.0;
pub const XXL: f32 = 32.0;

// token/size.rs
pub const HIT_TARGET_MIN: f32 = 24.0;   pub const BAR_STATUS: f32 = 24.0;
pub const CONTROL_SM: f32 = 28.0;       pub const CONTROL_MD: f32 = 32.0;
pub const CONTROL_LG: f32 = 36.0;       pub const INDICATOR_DOT: f32 = 6.0;
pub const SPLITTER_GRAB: f32 = 10.0;    pub const ICON_SM: f32 = 14.0;
pub const ICON_MD: f32 = 16.0;          pub const ICON_LG: f32 = 20.0;
pub const CHECKBOX_BOX: f32 = 16.0;     pub const SWITCH_TRACK_WIDTH: f32 = 36.0;
pub const SWITCH_TRACK_HEIGHT: f32 = 20.0; pub const SWITCH_THUMB: f32 = 14.0;
pub const SWITCH_THUMB_INSET: f32 = (SWITCH_TRACK_HEIGHT - SWITCH_THUMB) / 2.0;
pub const SLIDER_TRACK_HEIGHT: f32 = 4.0;  pub const SLIDER_THUMB: f32 = 16.0;
pub const PROGRESS_BAR_HEIGHT: f32 = 4.0;  pub const SPINNER_SM: f32 = 14.0;
pub const SPINNER_MD: f32 = 20.0;       pub const BADGE_COUNT_HEIGHT: f32 = 18.0;
pub const BADGE_CHIP_HEIGHT: f32 = 24.0;
pub const AVATAR_SM: f32 = 24.0; pub const AVATAR_MD: f32 = 32.0; pub const AVATAR_LG: f32 = 36.0;

// token/shape.rs
pub const RADIUS_WINDOW: f32 = 8.0;  pub const RADIUS_CARD: f32 = 6.0;
pub const RADIUS_WIDGET: f32 = 4.0;  pub const RADIUS_PILL: f32 = 999.0;
pub const BORDER_HAIRLINE: f32 = 1.0; pub const BORDER_FOCUS: f32 = 2.0;

// token/time.rs
pub enum Easing { Linear, EaseOutCubic, EaseInOutCubic /* ... */ }
pub struct DurationToken { pub name: &'static str, pub ms: u64, pub easing: Option<Easing>, pub alias: Option<&'static str> }
pub const fn resolve_ms(intended: DurationToken, reduced_motion: bool) -> u64;
pub fn eased_progress(intended: DurationToken, elapsed_ms: u64, reduced_motion: bool) -> f32;
pub const INSTANT: DurationToken;              // 0
pub const FAST: DurationToken;                 // 100
pub const NORMAL: DurationToken;               // 180
pub const SLOW: DurationToken;                 // 240
pub const DEBOUNCE_LIVE: DurationToken;        // 200
pub const DELAY_SUBMENU: DurationToken;        // 150
pub const DELAY_TOOLTIP_OPEN: DurationToken;   // 400
pub const DELAY_TOOLTIP_WARM: DurationToken;   // 100
pub const HOLD_COPIED: DurationToken;          // 1500
pub const TOAST_AUTO_DISMISS: DurationToken;   // 5000
pub fn switch_thumb_progress(elapsed_ms: u64, reduced_motion: bool) -> f32;
pub fn flyout_arrival_progress(elapsed_ms: u64, reduced_motion: bool) -> f32;
pub fn slow_transition_progress(elapsed_ms: u64, reduced_motion: bool) -> f32;
pub fn rail_width_interpolate(from_width: f32, to_width: f32, elapsed_ms: u64, reduced_motion: bool) -> f32;
```

##### Examples

```rust
use hoarfrost::token::color::Palette;
use hoarfrost::token::{space, size, shape};

let palette = Palette::dark();
let height = size::CONTROL_MD;            // 32.0
let gap = space::SM;                      // 8.0
let radius = shape::RADIUS_WIDGET;        // 4.0
```

##### Type references

`Palette` fields, in declaration order. Each maps to one `color.*` spec token.

| Field | Spec token |
|---|---|
| `surface_base` | `color.surface.base` |
| `surface_sunken` | `color.surface.sunken` |
| `surface_raised` | `color.surface.raised` |
| `surface_code` | `color.surface.code` |
| `surface_overlay` | `color.surface.overlay` |
| `surface_hover` | `color.surface.hover` |
| `surface_active` | `color.surface.active` |
| `surface_scrim` | `color.surface.scrim` |
| `border_subtle` | `color.border.subtle` |
| `border_strong` | `color.border.strong` |
| `accent_fill` | `color.accent.fill` |
| `accent_fill_active` | `color.accent.fill_active` |
| `accent_border` | `color.accent.border` |
| `accent_text` | `color.accent.text` |
| `text_primary` | `color.text.primary` |
| `text_strong` | `color.text.strong` |
| `text_muted` | `color.text.muted` |
| `text_disabled` | `color.text.disabled` |
| `status_success` | `color.status.success` |
| `status_warning` | `color.status.warning` |
| `status_danger_text` | `color.status.danger.text` |
| `status_danger_fill` | `color.status.danger.fill` |
| `status_danger_fill_active` | `color.status.danger.fill_active` |

`Palette` has **one** constructor, `dark()`. There is no `light()` constructor
and no per-role accessor. Build an alternative palette by constructing the
struct with your own `iced::Color` values.

##### Return values

`resolve_ms` returns whole milliseconds, or `0` when `reduced_motion` is `true`
and the rung is a transition rather than a delay. `eased_progress` returns a
progress fraction in `0.0..=1.0`.

##### Side effects

None. Every item is a constant or a pure function.

##### References

- [`token/color.rs`](../src/hoarfrost/token/color.rs)
- [`token/typography.rs`](../src/hoarfrost/token/typography.rs)
- [`token/space.rs`](../src/hoarfrost/token/space.rs),
  [`token/size.rs`](../src/hoarfrost/token/size.rs),
  [`token/shape.rs`](../src/hoarfrost/token/shape.rs)
- [`token/time.rs`](../src/hoarfrost/token/time.rs)
- [`token/contrast.rs`](../src/hoarfrost/token/contrast.rs)

---

#### `token::contrast` — contrast audit rules

##### Overview

`token::contrast` judges whether a colour pair is legal. It implements WCAG
relative luminance and ratio maths, the framework's contrast floors, the
chrome-class rule, and the fill/foreground lock. You SHOULD call it from tests and from the
verification gate. You SHOULD NOT call it from a per-frame paint path.

##### Prerequisites & Requirements

- No cargo feature.
- Import `hoarfrost::token::contrast::*`.

##### Syntax / Method Signature

```rust
pub const MIN_TEXT_CONTRAST: f32 = 4.5;
pub const MIN_CHROME_CONTRAST: f32 = 3.0;
pub const MIN_COINCIDENT_FILL_CONTRAST: f32 = 3.0;
pub const DISABLED_TEXT_MIN_CONTRAST: f32 = 4.0;
pub const DISABLED_TEXT_MAX_CONTRAST: f32 = 6.05;

pub enum ChromeClass { Judged /* ... */ }

pub fn relative_luminance(color: Color) -> f32;
pub fn contrast_ratio(a: Color, b: Color) -> f32;
pub fn composite_over(fg: Color, bg: Color) -> Color;
pub fn meets_text_contrast(fg: Color, bg: Color) -> bool;
pub fn meets_chrome_contrast(a: Color, b: Color) -> bool;
pub fn meets_coincident_fill_contrast(fill: Color, surface: Color) -> bool;
pub fn meets_chrome_contrast_as(class: ChromeClass, a: Color, b: Color) -> bool;
pub fn meets_disabled_text_band(fg: Color, bg: Color) -> bool;

pub enum FillLockRole { LockedFill, ForegroundOnly }
pub enum FillLock { NotSolidFill, Locked /* ... */ }
pub fn fill_lock_role(color: Color, palette: &Palette) -> FillLockRole;
pub fn fill_lock(background: Color, foreground: Color, palette: &Palette) -> FillLock;
pub fn meets_fill_foreground_lock(background: Color, foreground: Color, palette: &Palette) -> bool;

// Debug-only panics. They abort a debug build, never a release build.
pub fn assert_fill_foreground_lock(background: Color, foreground: Color, palette: &Palette);
pub fn assert_contrast(fg: Color, bg: Color, min_ratio: f32, label: &str);
```

##### Examples

```rust
use hoarfrost::token::color::Palette;
use hoarfrost::token::contrast::meets_text_contrast;

let p = Palette::dark();
assert!(meets_text_contrast(p.text_primary, p.surface_base));
```

##### Type references

- `Color` is `iced::Color` (`{ r, g, b, a }`, each `f32` in `0.0 ..= 1.0`).
- `Palette` is `token::color::Palette`; see [`token`](#token--design-tokens).
- `ChromeClass` classifies a colour pair so `meets_chrome_contrast_as` can pick the
  right floor. `FillLockRole` says whether a colour is a locked fill or a foreground.
- `FillLock` is the verdict enum: `NotSolidFill` or `Locked { ... }`.

##### Return values

Every `meets_*` function returns `bool`. `contrast_ratio` returns the raw ratio,
where `1.0` means identical colours. `fill_lock` returns which arm of the
fill/foreground rule applies; `NotSolidFill` means the rule makes no demand.

##### Side effects

`assert_*` panics in a debug build when the pair fails. It is a no-op in a
release build. Do not call them from a host's production render path.

##### References

- [`token/contrast.rs`](../src/hoarfrost/token/contrast.rs)

---

#### `state` — interaction state resolution

##### Overview

`state` turns raw observations (enabled, pointer, focus, validation) into the one
visual a control paints. It is the single owner of state precedence.
Requirement: `UIF-ATOM-001`, `UIF-ATOM-002`, `UIF-VER-003`.

##### Prerequisites & Requirements

- No cargo feature.
- Import `hoarfrost::state::{BaseState, InteractionInput, VisualState, resolve}`.

##### Syntax / Method Signature

```rust
pub enum BaseState { Rest, Hovered, Pressed, Disabled, Invalid }

pub struct InteractionInput {
    pub enabled: bool,
    pub hovered: bool,
    pub pressed: bool,
    pub focused: bool,
    pub invalid: bool,
}

pub struct VisualState {
    pub base: BaseState,
    pub focused: bool,
}

impl VisualState {
    pub const fn ring_visible(&self) -> bool;
    pub const fn snapshot_label(&self) -> &'static str;
}

pub const fn resolve(input: InteractionInput) -> VisualState;

pub const SNAPSHOT_REST: &str = "Rest";
pub const SNAPSHOT_HOVERED: &str = "Hovered";
pub const SNAPSHOT_PRESSED: &str = "Pressed";
pub const SNAPSHOT_FOCUSED: &str = "Focused";
pub const SNAPSHOT_DISABLED: &str = "Disabled";
pub const SNAPSHOT_INVALID: &str = "Invalid";
pub const SNAPSHOT_STATE_NAMES: [&str; 6] = [ /* in spec order */ ];
```

##### Examples

```rust
use hoarfrost::state::{resolve, BaseState, InteractionInput};

let visual = resolve(InteractionInput {
    enabled: true,
    hovered: true,
    pressed: false,
    focused: true,
    invalid: false,
});
assert_eq!(visual.base, BaseState::Hovered);
assert!(visual.ring_visible());
assert_eq!(visual.snapshot_label(), "Focused");
```

##### Type references

Precedence for `base` is `Disabled` > `Invalid` > `Pressed` > `Hovered` > `Rest`.
`focused` is resolved independently, except that a disabled control never gets a
ring. `InteractionInput::default()` is all-false, which resolves to `Disabled`.
That is deliberate: a control not told it is enabled shows the disabled skin.

`snapshot_label` collapses the pair to the six spec snapshot names.
`Hovered + focused` and `Rest + focused` both read `Focused`.
`Pressed + focused` reads `Pressed`.

##### Return values

`resolve` always returns a `VisualState`. It never fails and never panics.

##### Side effects

None. `resolve` is `const` and holds no memory of a previous call.

##### References

- [`state.rs`](../src/hoarfrost/state.rs)

---

#### `focus` — focus model

##### Overview

`focus` is a plain data model of tab order, groups, and focus traps. It owns no
widget and reads no event. A host mutates it; `bridge` projects it into iced.
Requirement: `UIF-PRIN-005`, single-tab-stop composites.

##### Prerequisites & Requirements

- No cargo feature.
- Import `hoarfrost::focus::{FocusId, GroupId, FocusMember, FocusGroup, FocusState, TrapPop}`.

##### Syntax / Method Signature

```rust
pub struct FocusId(String);
impl FocusId {
    pub fn new(id: impl Into<String>) -> Self;
    pub fn as_str(&self) -> &str;
}
impl From<&str> for FocusId;      // "a.b".into()
impl From<String> for FocusId;

pub struct GroupId(String);
impl GroupId {
    pub fn new(id: impl Into<String>) -> Self;
    pub fn as_str(&self) -> &str;
}
impl From<&str> for GroupId;
impl From<String> for GroupId;

pub struct FocusMember { pub id: FocusId, pub enabled: bool }
impl FocusMember {
    pub fn new(id: impl Into<FocusId>) -> Self;
    pub fn disabled(id: impl Into<FocusId>) -> Self;
}

pub struct FocusGroup { /* private */ }
impl FocusGroup {
    pub fn id(&self) -> &GroupId;
    pub fn members(&self) -> &[FocusMember];
    pub fn enabled_members(&self) -> impl Iterator<Item = &FocusMember>;
}

pub struct TrapPop {
    pub group: GroupId,
    pub recorded: Option<FocusId>,
    pub restored: Option<FocusId>,
    pub lost: Option<FocusId>,
}

pub struct FocusState { /* private: groups, current, traps */ }
impl FocusState {
    pub fn new() -> Self;
    pub fn register_group(&mut self, id: GroupId, members: Vec<FocusMember>) -> Option<FocusId>;
    pub fn unregister_group(&mut self, id: &GroupId) -> Option<FocusId>;
    pub fn set_member_enabled(&mut self, group: &GroupId, member: &FocusId, enabled: bool) -> Option<FocusId>;
    pub fn focus(&mut self, id: &FocusId) -> bool;
    pub fn clear(&mut self) -> Option<FocusId>;
    pub fn focus_next(&mut self) -> Option<FocusId>;
    pub fn focus_previous(&mut self) -> Option<FocusId>;
    pub fn focus_first_in_group(&mut self, group: &GroupId) -> Option<FocusId>;
    pub fn focus_last_in_group(&mut self, group: &GroupId) -> Option<FocusId>;
    pub fn move_within(&mut self, group: &GroupId, delta: i32) -> Option<FocusId>;
    pub fn push_trap(&mut self, group: GroupId);
    pub fn pop_trap(&mut self) -> Option<TrapPop>;
    pub fn active_trap(&self) -> Option<&GroupId>;
    pub fn trap_depth(&self) -> usize;
    pub fn is_trapped(&self) -> bool;
    pub fn current(&self) -> Option<&FocusId>;
    pub fn is_focused(&self, id: &FocusId) -> bool;
    pub fn is_focusable(&self, id: &FocusId) -> bool;
    pub fn in_active_scope(&self, id: &FocusId) -> bool;
    pub fn groups(&self) -> &[FocusGroup];
    pub fn group(&self, id: &GroupId) -> Option<&FocusGroup>;
    pub fn group_of(&self, id: &FocusId) -> Option<&FocusGroup>;
    pub fn focusable_members(&self, group: &GroupId) -> Vec<&FocusId>;
}
```

##### Examples

```rust
use hoarfrost::focus::{FocusId, FocusMember, FocusState, GroupId};

let mut focus = FocusState::new();
let group = GroupId::new("demo.toolbar");
focus.register_group(
    group.clone(),
    vec![
        FocusMember::new("demo.toolbar.save"),
        FocusMember::disabled("demo.toolbar.undo"),
        FocusMember::new("demo.toolbar.redo"),
    ],
);

// The disabled member keeps its position but is stepped over.
assert_eq!(focus.focus_next().map(|id| id.as_str().to_owned()), Some("demo.toolbar.save".to_owned()));
assert_eq!(focus.focus_next().map(|id| id.as_str().to_owned()), Some("demo.toolbar.redo".to_owned()));
```

##### Type references

- `FocusId` and `GroupId` are string newtypes. They are **not** integers.
  Use stable, dotted, host-assigned names. Never generate one from a counter.
- A `FocusMember` that is disabled keeps its slot in traversal order but is skipped.
- `push_trap` records the currently focused id. Only the innermost trap governs.
- `pop_trap` restores the recorded id when it is still focusable, otherwise it
  falls back and reports what was lost.

##### Return values

- Mutation methods that move focus return `Option<FocusId>`: the id that now
  holds focus, or `None` when nothing moved.
- `focus` returns `bool`: whether the id was focusable at all.
- `pop_trap` returns `Option<TrapPop>`, `None` when no trap was active.
- `register_group` returns the id that lost focus, if any.

##### Side effects

Every `&mut self` method mutates the model only. Nothing reaches iced until you
run a `bridge` operation.

##### References

- [`focus.rs`](../src/hoarfrost/focus.rs)

---

#### `bridge` — focus model to iced operations

##### Overview

`bridge` converts a `FocusState` into iced `Operation`s and `Task`s so the real
keyboard router agrees with the model. It is one-way: model to iced.
Requirement: `UIF-PRIN-005`.

##### Prerequisites & Requirements

- No cargo feature.
- Import `hoarfrost::bridge::{Direction, focus_operation, focus_task, tab_operation, tab_task, widget_id}`.
- `Message` MUST satisfy `Send + 'static`.

##### Syntax / Method Signature

```rust
pub fn widget_id(focus_id: &FocusId) -> iced::advanced::widget::Id;

pub enum Direction { Forward, Backward }
impl Direction { pub const fn from_shift(shifted: bool) -> Self; }

pub fn focus_operation<Message: Send + 'static>(model: &FocusState) -> Box<dyn Operation<Message>>;
pub fn focus_task<Message: Send + 'static>(model: &FocusState) -> Task<Message>;
pub fn tab_operation<Message: Send + 'static>(model: &mut FocusState, direction: Direction) -> Box<dyn Operation<Message>>;
pub fn tab_task<Message: Send + 'static>(model: &mut FocusState, direction: Direction) -> Task<Message>;
```

##### Examples

```rust
use hoarfrost::bridge::{tab_task, Direction};

fn on_tab(&mut self, shifted: bool) -> iced::Task<Message> {
    tab_task(&mut self.focus, Direction::from_shift(shifted))
}
```

##### Type references

`widget_id` derives the iced widget `Id` from a `FocusId`. Every hoarfrost widget
uses this mapping, so a `FocusId` and the iced focus router address the same
control. Use the same string in both places.

##### Return values

`*_operation` returns a boxed operation for manual composition. `*_task` returns
an iced `Task` you can return straight from an `update` handler.

##### Side effects

`tab_operation` and `tab_task` mutate the `FocusState`: the model advances before
the router does, so the two stay in step. `focus_*` variants only read.

##### References

- [`bridge.rs`](../src/hoarfrost/bridge.rs)

---

#### `binding` — ownership, tiers, gestures, gating

##### Overview

`binding` holds the shared plumbing every interactive widget uses: model
ownership wrappers, the three callback tiers, gesture snapshots, the activation
gate, drag pre-emption, and responsive breakpoints. You use these types directly
when you write a host or a custom widget.

##### Prerequisites & Requirements

- No cargo feature.
- Import from `hoarfrost::binding::*`.

##### Syntax / Method Signature

```rust
// Ownership
pub struct Controlled<'a, M> { /* &M */ }
impl<'a, M> Controlled<'a, M> {
    pub fn new(model: &'a M) -> Self;
    pub fn get(&self) -> &'a M;
    pub fn map<N>(&self, project: impl FnOnce(&'a M) -> &'a N) -> Controlled<'a, N>;
}
impl<'a, M> Clone for Controlled<'a, M>;   // Copy-like: it wraps a shared borrow

pub struct Ephemeral<S> { /* S */ }
impl<S> Ephemeral<S> {
    pub fn new(state: S) -> Self;
    pub fn get(&self) -> &S;
    pub fn get_mut(&mut self) -> &mut S;
    pub fn set(&mut self, state: S) -> S;
    pub fn reset(&mut self) -> S;
}

// Tiers
pub struct Live<T>;     // an in-progress tick
pub struct Commit<T>;   // a settled value
impl<T> Live<T>    { pub fn new(value: T) -> Self; pub fn value(&self) -> &T; pub fn into_value(self) -> T; }
impl<T> Commit<T>  { pub fn new(value: T) -> Self; pub fn value(&self) -> &T; pub fn into_value(self) -> T; }

pub struct LiveCoalescer<T>;
impl<T> LiveCoalescer<T> {
    pub fn new() -> Self;
    pub fn push(&mut self, value: T) -> bool;   // false when it displaced a pending value
    pub fn is_pending(&self) -> bool;
    pub fn coalesced(&self) -> usize;
    pub fn take(&mut self) -> Option<T>;
}

// Snapshots and gestures
pub struct Snapshot<T>;
impl<T> Snapshot<T> {
    pub fn capture(value: &T) -> Self;
    pub fn value(&self) -> &T;
    pub fn into_value(self) -> T;
    pub fn matches(&self, current: &T) -> bool;
}

pub struct Cancel<T>;
impl<T> Cancel<T> {
    pub fn new(value: T) -> Self;
    pub fn restored(&self) -> &T;
    pub fn into_restored(self) -> T;
}

pub enum GestureStart { Started, AlreadyOpen }

pub struct Gesture<T>;
impl<T> Gesture<T> {
    pub fn new(snapshot: Snapshot<T>) -> Self;
    pub fn live(&self, value: T) -> Live<T>;
    pub fn commit(self, value: T) -> Commit<T>;
    pub fn cancel(self) -> Cancel<T>;
}

pub struct GestureSlot<T>;
impl<T> GestureSlot<T> {
    pub fn new() -> Self;
    pub fn is_open(&self) -> bool;
    pub fn begin(&mut self, current: &T) -> GestureStart;
    pub fn live(&self, value: T) -> Option<Live<T>>;
    pub fn commit(&mut self, value: T) -> Option<Commit<T>>;
    pub fn cancel(&mut self) -> Option<Cancel<T>>;
    pub fn abort(&mut self);
}

// Busy actions and activation gating
pub struct ActionPending;
impl ActionPending {
    pub const fn idle() -> Self;
    pub const fn indeterminate() -> Self;
    pub fn determinate(progress: f32) -> Self;
    pub const fn with_cancellable(mut self, cancellable: bool) -> Self;
    pub const fn is_busy(&self) -> bool;
    pub const fn progress(&self) -> Option<f32>;
    pub const fn is_cancellable(&self) -> bool;
    pub const fn blocks_activation(&self) -> bool;
    pub const fn locks_bounds(&self) -> bool;
    pub const fn reserves_spinner(&self) -> bool;
    pub fn try_cancel(&self) -> Result<CancelRequest, CancelRefusal>;
}

pub struct CancelRequest;
pub enum CancelRefusal { NotBusy, NotCancellable }
pub enum SuppressedReason { Busy, AwaitingProjection }
pub enum Activation { Fired, Suppressed(SuppressedReason) }
impl Activation {
    pub const fn is_fired(&self) -> bool;
    pub const fn is_suppressed(&self) -> bool;
    pub const fn reason(&self) -> Option<SuppressedReason>;
}

pub struct ActivationGate;
impl ActivationGate {
    pub const fn new() -> Self;
    pub const fn is_pending(&self) -> bool;
    pub const fn busy_observed(&self) -> bool;
    pub const fn fired(&self) -> usize;
    pub const fn suppressed(&self) -> usize;
    pub fn observe(&mut self, pending: &ActionPending);
    pub fn try_activate(&mut self, pending: &ActionPending) -> Activation;
    pub fn settle(&mut self) -> bool;
}

// Drag pre-emption (Escape ranks below an in-flight drag)
pub enum DragKind { Slider, RangeSlider, SplitDivider, ColumnResize /* ... */ }
pub struct CaptureId(String);
impl CaptureId { pub fn new(id: impl Into<String>) -> Self; pub fn as_str(&self) -> &str; }
pub struct DragCapture;
impl DragCapture { pub fn new(id: impl Into<CaptureId>, kind: DragKind) -> Self;
                   pub fn id(&self) -> &CaptureId; pub fn kind(&self) -> DragKind; }
pub enum CaptureStart { Started, AlreadyHeld { held: DragCapture } }
impl CaptureStart { pub fn is_started(&self) -> bool; pub fn held_by(&self) -> Option<&DragCapture>; }
pub enum PreRankOutcome { Cancelled { capture: DragCapture } /* ... */ }
impl PreRankOutcome { pub fn is_cancelled(&self) -> bool;
                      pub fn proceed_with_rank_walk(&self) -> bool;
                      pub fn capture(&self) -> Option<&DragCapture>; }
pub struct PreRankCoordinator;
impl PreRankCoordinator {
    pub fn new() -> Self;
    pub fn is_captured(&self) -> bool;
    pub fn capture(&self) -> Option<&DragCapture>;
    pub fn begin(&mut self, capture: DragCapture) -> CaptureStart;
    pub fn release(&mut self, id: &CaptureId) -> bool;
    pub fn clear(&mut self) -> Option<DragCapture>;
    pub fn pre_rank_cancel<F>(&mut self, cancel: F) -> PreRankOutcome;
}

// Responsive rungs
pub enum Breakpoint { Expanded, Compact }
impl Breakpoint { pub const fn is_compact(self) -> bool; pub const fn is_expanded(self) -> bool; }
pub struct Measurement;
impl Measurement {
    pub fn new(expanded_width: f32, compact_width: f32) -> Self;
    pub const fn expanded_width(&self) -> f32;
    pub const fn compact_width(&self) -> f32;
    pub const fn fits_expanded(&self, allocated: f32) -> bool;
    pub const fn fits_compact(&self, allocated: f32) -> bool;
}
pub struct ResponsiveLayout;
impl ResponsiveLayout {
    pub fn solve(allocated: f32, measurement: &Measurement) -> Self;
    pub const fn breakpoint(self) -> Breakpoint;
    pub const fn is_compact(self) -> bool;
    pub const fn horizontal_padding(self) -> f32;
    pub const fn label_mode(self) -> LabelMode;
    pub const fn vertical_shift(self) -> f32;
}
pub enum LabelMode { Full, IconOnly }
impl LabelMode { pub const fn is_visible(self) -> bool; pub const fn requires_tooltip(self) -> bool; }
```

##### Examples

```rust
use hoarfrost::binding::{ActivationGate, ActionPending, GestureSlot};

// A gate suppresses a second click while the first is still in flight.
let mut gate = ActivationGate::new();
let busy = ActionPending::indeterminate().with_cancellable(true);
let first = gate.try_activate(&ActionPending::idle());
let second = gate.try_activate(&busy);
assert!(first.is_fired());
assert!(second.is_suppressed());

// A gesture slot snapshots the value at press and reverts it on Escape.
let mut slot = GestureSlot::new();
let mut width = 240.0f32;
if let hoarfrost::binding::GestureStart::Started = slot.begin(&width) {
    width = 300.0;
    if let Some(cancel) = slot.cancel() {
        width = *cancel.restored();   // back to 240.0
    }
}
```

##### Type references

- `Controlled<'a, M>` is a shared borrow with a name. The host owns the model and
  mutates it; the widget reads it every frame.
- `Ephemeral<S>` marks per-control state that lives in the widget tree and is
  dropped on rebuild. Store controllers here, never in a global.
- `GestureSlot<T>` refuses a nested `begin` and keeps the **outer** snapshot.
  This is what stops a nested edit from shortening a revert.
- `ActivationGate` closes the double-click window: it fires once, then suppresses
  with `SuppressedReason::Busy` or `AwaitingProjection` until `settle`.
- `PreRankCoordinator` holds exactly one drag capture. Escape cancels that drag
  and does **not** dismiss a surface in the same keypress.
- `ResponsiveLayout::solve` is one comparison: `Expanded` only when the full
  presentation genuinely fits. `NaN`, negative widths, and impossible
  measurements all resolve to `Compact`.

##### Return values

- `GestureSlot::live/commit/cancel` return `None` when no gesture is open.
- `ActionPending::try_cancel` returns `Err(CancelRefusal::NotBusy)` when nothing
  runs and `Err(CancelRefusal::NotCancellable)` when no stop path exists.
- `ActivationGate::settle` returns whether a dispatch was actually in flight.
- `PreRankCoordinator::release` returns whether the given id held the capture.

##### Side effects

None of these types touch I/O, a clock, or a global. `Ephemeral` state is dropped
when the widget tree rebuilds; that is a feature, not a leak.

##### References

- [`binding/controlled.rs`](../src/hoarfrost/binding/controlled.rs),
  [`binding/tier.rs`](../src/hoarfrost/binding/tier.rs),
  [`binding/snapshot.rs`](../src/hoarfrost/binding/snapshot.rs)
- [`binding/pending.rs`](../src/hoarfrost/binding/pending.rs),
  [`binding/pre_rank.rs`](../src/hoarfrost/binding/pre_rank.rs),
  [`binding/responsive.rs`](../src/hoarfrost/binding/responsive.rs)

---

#### `identity` — addressable sub-part text ids

##### Overview

`iced`'s `Text` widget carries no `Id`. `identity` lets a custom widget publish
the text it drew under a stable, derived id, so a host can address "the title of
this row" from the outside. Ids are derived, never invented: a widget with no id
of its own publishes nothing.

##### Prerequisites & Requirements

- No cargo feature.
- Import `hoarfrost::identity::*`.
- Call `publish_text` from inside your widget's `Widget::operate` implementation.

##### Syntax / Method Signature

```rust
pub const ROLE_SEPARATOR: char = '.';
pub const INDEX_SEPARATOR: char = '-';

pub mod role {
    pub const TITLE: &str = "title";
    pub const SUBTITLE: &str = "subtitle";
    pub const HEADER: &str = "header";
    pub const HINT: &str = "hint";
    pub const LABEL: &str = "label";
}
pub const ROLES: &[&str];

pub fn custom_name(id: &Id) -> Option<String>;
pub fn sub_part_id(base: &Id, role_name: &str) -> Option<Id>;
pub fn indexed_role(role_name: &str, index: usize) -> String;
pub fn is_role(segment: &str) -> bool;
pub fn is_sub_part_name(name: &str) -> bool;
pub fn split_sub_part_name(name: &str) -> Option<(String, String)>;
pub fn split_sub_part_id(id: &Id) -> Option<(String, String)>;
pub fn publish_text(operation: &mut dyn Operation, base: &Id, role_name: &str, bounds: Rectangle, content: &str);
```

##### Examples

```rust
use hoarfrost::identity::{publish_text, role};

// Inside Widget::operate, after you computed where the title was drawn:
publish_text(operation, &self.id, role::TITLE, title_bounds, "Workspace");
// Published name: "<widget-id>.title"
```

##### Type references

The naming convention is `<widget-id>.<role>`, or `<widget-id>.<role>-<n>` when
one widget renders several parts with the same role. `INDEX_SEPARATOR` is `-` so
a dotted widget id such as `a.b.c` survives `split_sub_part_id` intact.

##### Return values

`custom_name` returns `None` for an id with no stable name, which includes
`Id::unique()`. `sub_part_id` returns `None` when the base has no name or either
part is empty. `publish_text` is silent in those cases and for empty content.

##### Side effects

`publish_text` adds one `Candidate::Text` to the operation stream. It touches
neither layout nor draw, so it cannot move a bound.

##### References

- [`identity.rs`](../src/hoarfrost/identity.rs)

---

#### `strings` — user-facing text catalogue

##### Overview

`strings` isolates every string a hoarfrost widget renders verbatim. Rule `HF-001`
requires this. A host that localises MUST replace these at the call site, not
patch them in place.

##### Prerequisites & Requirements

- No cargo feature. Import `hoarfrost::strings::*`.

##### Syntax / Method Signature

All items are `pub const &str`. Current catalogue:

| Constant | Value |
|---|---|
| `LABEL_APPLY` | `✓ Apply` |
| `LABEL_CANCEL` | `Cancel` |
| `LABEL_HEX` | `Hex` |
| `LABEL_PRESETS` | `Presets` |
| `ERR_INVALID_HEX` | `Invalid hex color` |
| `LABEL_ACCEPT_HUNK` | `Accept hunk` |
| `LABEL_REJECT_HUNK` | `Reject hunk` |
| `LABEL_UNIFIED` | `Unified` |
| `LABEL_SIDE_BY_SIDE` | `Side-by-Side` |
| `LABEL_ESC` | `Esc` |
| `PLACEHOLDER_FILES` | `Search files by name...` |
| `PLACEHOLDER_COMMANDS` | `Type a command...` |
| `PLACEHOLDER_SYMBOLS` | `Search symbols...` |
| `PLACEHOLDER_TAGS` | `Search by tag...` |
| `LABEL_RECORDING` | `Press shortcut keys...` |
| `LABEL_EMPTY` | `Record shortcut` |
| `LABEL_TOGGLE` | `Toggle navigation rail` |
| `LABEL_PIN` | `Pin` |
| `LABEL_UNPIN` | `Unpin` |
| `LABEL_CLOSE` | `Close` |
| `LABEL_PREVIOUS` | `← Previous` |
| `LABEL_NEXT` | `Next →` |
| `LABEL_FINISH` | `Finish ✓` |
| `PLACEHOLDER_DATE` | `YYYY-MM-DD` |
| `LABEL_HR` / `LABEL_MIN` / `LABEL_SEC` | `Hr` / `Min` / `Sec` |
| `LABEL_AM_PM` / `LABEL_AM` / `LABEL_PM` | `AM/PM` / `AM` / `PM` |
| `PLACEHOLDER_24H` / `PLACEHOLDER_24H_SEC` | `--:--` / `--:--:--` |
| `PLACEHOLDER_12H` / `PLACEHOLDER_12H_SEC` | `--:-- --` / `--:--:-- --` |

Several composites re-export the constants they use so historical import paths
keep working, for example `composite::stepper::LABEL_NEXT`.

##### Examples

```rust
use hoarfrost::strings::{LABEL_FINISH, PLACEHOLDER_COMMANDS};

// Render the constant verbatim. Do not rebuild it from a literal.
let label = LABEL_FINISH;               // "Finish ✓"
let hint = PLACEHOLDER_COMMANDS;        // "Type a command..."
```

To localise, pass your own string to the widget's builder instead of editing this
module. Builders that accept a label take `impl Into<String>`.

##### Type references

Every item is `pub const &'static str`. There are no structs, enums or functions in
this module.

##### Return values

Not applicable. The module exposes constants only.

##### Side effects

None. The module is compile-time data.

##### References

- [`strings.rs`](../src/hoarfrost/strings.rs)

#### `widget::button` — activation atom (`UIF-ATOM-003`)

##### Overview

`button` is the activation primitive. Every other interactive atom composes it or copies its
gesture rules. It ships as three parts: `ButtonModel` (data), `ButtonController` (gesture and
busy-action state machine), and `Button` (the iced `Widget` that paints).

##### Prerequisites & Requirements

- Enable the cargo feature `button`. This feature also enables `tooltip`.
- The host owns a `ButtonController` per live button and calls `observe(&model)` once per update
  before the widget is built. This keeps the busy gate in sync with `ActionPending`.
- A busy button MUST be `cancellable` or MUST NOT be able to enter the busy state.

##### Syntax / Method Signature

```rust
// Model (button_model.rs)
pub struct ButtonModel { /* private */ }
impl ButtonModel {
    pub fn new(label: impl Into<String>) -> Self;
    pub fn icon_only(icon: &'static str) -> Self;
    pub fn with_variant(mut self, variant: ButtonVariant) -> Self;
    pub fn leading(mut self, icon: ButtonIcon) -> Self;
    pub fn trailing(mut self, icon: ButtonIcon) -> Self;
    pub fn with_density(mut self, density: ButtonDensity) -> Self;
    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self;
    pub fn busy(mut self) -> Self;
    pub fn cancellable(mut self) -> Self;
    pub fn disabled(mut self) -> Self;
    pub fn controlled(&self) -> Controlled<'_, ButtonModel>;
    pub fn tracks_action(&self) -> bool;
    pub fn pending(&self) -> ActionPending;
    pub fn is_icon_only(&self, mode: LabelMode) -> bool;
    pub fn tooltip_for(&self, mode: LabelMode) -> Option<&str>;
    pub fn collapsed_min_width(&self) -> f32;
    pub fn expanded_min_width(&self) -> f32;
    pub fn measurement(&self) -> Measurement;
    pub fn measurement_with_label(&self, label_width: f32) -> Measurement;
    pub fn responsive(&self) -> ResponsiveLayout;
    pub fn responsive_to(&self, allocated: f32) -> ResponsiveLayout;
}
pub enum ButtonVariant { Primary, Secondary, Ghost, Danger }
pub enum ButtonDensity { Standard, Compact }

// Controller (button_controller.rs)
pub struct ButtonController { /* private */ }
impl ButtonController {
    pub fn new() -> Self;
    pub fn observe(&mut self, model: &ButtonModel);
    pub fn begin(&mut self, model: &ButtonModel, source: ActivationSource) -> Option<ButtonEmission>;
    pub fn commit(&mut self, model: &ButtonModel, source: ActivationSource) -> Option<ButtonEmission>;
    pub fn cancel(&mut self, model: &ButtonModel) -> Option<ButtonEmission>;
    pub fn undispatched(source: ActivationSource, pending: ActionPending) -> Self;
    pub fn settlement(&self) -> Option<&ButtonSettlement>;
    pub fn dispatched(&self) -> bool;
    pub fn gesture_open(&self) -> bool;
    pub fn last_begin(&self) -> Option<GestureStart>;
    pub fn is_pending(&self) -> bool;
    pub fn busy_observed(&self) -> bool;
    pub fn fired(&self) -> usize;
    pub fn suppressed(&self) -> usize;
    pub fn cancel_requested(&self) -> bool;
    pub fn reset(&mut self);
}
pub enum ButtonEmission { Activated, Cancelled, CancelRequested }
pub struct ButtonSettlement { /* private: source, tier, pending */ }

// Widget (button_widget.rs)
pub struct Button<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> { /* private */ }
impl<'a, Message, Theme, Renderer> Button<'a, Message, Theme, Renderer> {
    pub fn new(
        variant: ButtonVariant,
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
        on_press: Message,
        focus_id: impl Into<FocusId>,
    ) -> Self;
    pub fn controlled(
        model: Controlled<'a, ButtonModel>,
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
        focus_id: impl Into<FocusId>,
        emit: impl Fn(ButtonEmission) -> Option<Message> + 'a,
    ) -> Self;
}
```

##### Examples

```rust
use hoarfrost::widget::{Button, ButtonController, ButtonEmission, ButtonModel, ButtonVariant};

let mut model = ButtonModel::new("Save").with_variant(ButtonVariant::Primary).cancellable();
if save_in_flight {
    model = model.busy();
}
controller.observe(&model);

let button = Button::controlled(
    model.controlled(),
    iced::widget::text("Save"),
    "save-button",
    |emission| match emission {
        ButtonEmission::Activated => Some(Msg::Save),
        ButtonEmission::CancelRequested => Some(Msg::CancelSave),
        ButtonEmission::Cancelled => None,
    },
);
```

##### Type references

| Type | Meaning |
| --- | --- |
| `ButtonVariant` | `Primary` / `Secondary` / `Ghost` / `Danger`. |
| `ButtonDensity` | `Standard` / `Compact`. Compact is the navbar and filter-bar density. |
| `ButtonEmission` | `Activated` (committed activation), `Cancelled` (gesture abandoned), `CancelRequested` (user asked a busy action to stop). |
| `ButtonSettlement` | Record of the last settled activation: source and tier. |
| `ActivationSource` | `Pointer` / `Keyboard`. |
| `LabelMode` | `Full` or `IconOnly`. `IconOnly` requires a tooltip. Lives in `binding::responsive`. |
| `Measurement` | Label and icon widths a button needs; lives in `binding::responsive`. |
| `ResponsiveLayout` | Declared collapse behaviour; see `binding`. |

##### Return values

- `begin` / `commit` / `cancel` return `Some(emission)` only when the emission must reach the host.
  A suppressed activation returns `None` and increments `suppressed()`.
- `Button::controlled` returns the widget; convert with `.into()` to `Element`.

##### Side effects

- None. The controller mutates only its own counters. Painting happens in `Widget::draw`.

##### References

- [`button_model.rs`](../src/hoarfrost/widget/button_model.rs),
  [`button_controller.rs`](../src/hoarfrost/widget/button_controller.rs),
  [`button_widget.rs`](../src/hoarfrost/widget/button_widget.rs)

#### `widget::text_input` — single-line editors (`UIF-ATOM-004`)

##### Overview

One widget covers five editor kinds through `InputKind`: text entry, search, dropdown trigger,
password, and text area. `TextInputModel` holds the value, validation stance, and selection.
`TextInputController` owns the edit gesture and the live-tier coalescing.

##### Prerequisites & Requirements

- Enable the cargo feature `text-input`. It enables `button`.
- Set the stance with `with_stance`. A `Pristine` model MUST NOT show an error even when invalid.
- Attach a `DisallowedClass` so illegal characters are rejected immediately.
- Call `controller.observe(&model)` each update, then `note_edit` for every incoming keystroke.

##### Syntax / Method Signature

```rust
pub enum InputKind { TextEntry, Search, DropdownTrigger, Password, TextArea }

pub struct TextInputModel { /* private */ }
impl TextInputModel {
    pub fn new(value: impl Into<String>) -> Self;
    pub fn empty() -> Self;
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self;
    pub fn with_validation(mut self, message: impl Into<String>) -> Self;
    pub fn with_selection(mut self, selection: TextRange) -> Self;
    pub fn read_only(mut self) -> Self;
    pub fn disabled(mut self) -> Self;
    pub fn with_clearable(mut self, clearable: bool) -> Self;
    pub fn with_stance(mut self, stance: EditStance) -> Self;
    pub fn with_disallowed(mut self, disallowed: DisallowedClass) -> Self;
    pub fn controlled(&self) -> Controlled<'_, TextInputModel>;
    pub fn has_value(&self) -> bool;
    pub fn is_invalid(&self) -> bool;
    pub fn note_commit(&mut self);
    pub fn note_blur(&mut self);
    pub fn note_submit(&mut self);
    pub fn reset_stance(&mut self);
    pub fn punishment(&self) -> Punishment;
    pub fn punishment_under(&self, class: DisallowedClass) -> Punishment;
    pub fn accepts_input(&self) -> bool;
    pub fn slot_for(&self, kind: InputKind) -> SlotContent;
    pub fn clear_affordance(&self, kind: InputKind) -> bool;
    pub fn selection_in_value(&self) -> Option<TextRange>;
}

pub struct TextInputController { /* private */ }
impl TextInputController {
    pub fn new() -> Self;
    pub fn with_value(value: impl Into<String>) -> Self;
    pub fn observe(&mut self, model: &TextInputModel);
    pub fn sync_value(&mut self, value: &str);
    pub fn note_edit(&mut self, value: String) -> bool;
    pub fn take_live(&mut self) -> Option<String>;
    pub fn commit(&mut self) -> Option<TextInputEmission>;
    pub fn revert(&mut self) -> Option<TextInputEmission>;
    pub fn clear(&mut self) -> Option<TextInputEmission>;
    pub fn note_focus(&mut self, focused: bool) -> Vec<TextInputEmission>;
    pub fn value(&self) -> Option<&str>;
    pub fn session_open(&self) -> bool;
    pub fn snapshot(&self) -> Option<&str>;
    pub fn is_focused(&self) -> bool;
    pub fn live_pending(&self) -> bool;
    pub fn coalesced(&self) -> usize;
    pub fn superseded(&self) -> usize;
    pub fn clears(&self) -> usize;
    pub fn last_begin(&self) -> Option<GestureStart>;
    pub fn reset(&mut self);
}

pub enum TextInputEmission {
    Changed, Committed, FocusCommitted, Cleared, Cancelled, FocusChanged,
}

pub struct TextInput<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> { /* private */ }
impl<'a, Message, Theme, Renderer> TextInput<'a, Message, Theme, Renderer> {
    pub fn new(
        kind: InputKind,
        placeholder: impl Into<String>,
        value: impl Into<String>,
        on_input: impl Fn(String) -> Message + 'a,
    ) -> Self;
    pub fn controlled(
        kind: InputKind,
        model: Controlled<'a, TextInputModel>,
        focus_id: impl Into<FocusId>,
        emit: impl Fn(TextInputEmission) -> Option<Message> + 'a,
    ) -> Self;
    pub fn compact(mut self) -> Self;
    pub fn standard(mut self) -> Self;
    pub fn width(mut self, width: impl Into<Length>) -> Self;
    pub fn disabled(mut self) -> Self;
    pub fn enabled(mut self) -> Self;
    pub fn invalid(mut self) -> Self;
    pub fn valid(mut self) -> Self;
    pub fn read_only(mut self) -> Self;
    pub fn on_clear(mut self, message: Message) -> Self;
    pub fn on_reveal(mut self, message: Message) -> Self;
    pub fn masked(mut self, masked: bool) -> Self;
    pub fn palette(mut self, palette: Palette) -> Self;
}
```

Pure helpers (no state, safe to call from tests and resolvers):

```rust
pub fn resolve_input_style(kind: InputKind, state: VisualState, palette: &Palette) -> InputStyle;
pub fn text_area_bounds(bounds: Rectangle, kind: InputKind) -> Rectangle;
pub fn trailing_slot_bounds(bounds: Rectangle, kind: InputKind) -> Rectangle;
pub fn background_layer(bounds: Rectangle, style: &InputStyle) -> Option<Layer>;
pub fn focus_ring_layer(bounds: Rectangle, style: &InputStyle) -> Option<Layer>;
pub fn visual_from_status(status: EditorStatus, enabled: bool, invalid: bool) -> VisualState;
```

##### Examples

```rust
let mut model = TextInputModel::new("report.pdf")
    .with_placeholder("File name")
    .with_disallowed(DisallowedClass::FilenameBasename)
    .with_stance(EditStance::Pristine);

let input = TextInput::controlled(
    InputKind::TextEntry,
    model.controlled(),
    "rename-entry",
    |e| match e {
        TextInputEmission::Changed => Some(Msg::Edited),
        TextInputEmission::Committed => Some(Msg::Committed),
        TextInputEmission::Cancelled => Some(Msg::Cancelled),
        _ => None,
    },
);
```

##### Type references

| Type | Meaning |
| --- | --- |
| `InputKind` | Selects chrome and the trailing slot. `DropdownTrigger` never shows a caret. |
| `TextRange` | Inclusive-start, exclusive-end range inside the value. |
| `EditorStatus` | Alias for iced's `text_input::Status`, imported in `text_input.rs`. |
| `SlotContent` | What the trailing slot renders: clear button, reveal button, or nothing. |
| `InputStyle` | Resolved colours, border, and ring for one frame. |

##### Return values

- `note_edit` returns `true` when a live emission is pending; read it with `take_live()`.
- `note_focus(false)` can return two emissions: `FocusCommitted` then `FocusChanged`.

##### Side effects

- None outside the model and controller. The widget never writes files or opens URIs.

##### References

- [`text_input.rs`](../src/hoarfrost/widget/text_input.rs),
  [`text_input_model.rs`](../src/hoarfrost/widget/text_input_model.rs),
  [`text_input_controller.rs`](../src/hoarfrost/widget/text_input_controller.rs)

#### `widget::text_area` — multi-line editor (`UIF-ATOM-005`)

##### Overview

`TextAreaModel` is a full text buffer with line/column cursor, selection, undo history, growth
policy, and parse diagnostics. `TextAreaController` turns editor events into tiered emissions and
applies undo/redo to the model.

##### Prerequisites & Requirements

- Enable the cargo feature `text-area`. It enables `text-input`, which enables `button`.
- Pass `&mut TextAreaModel` to every mutating controller call. The controller never mutates a model
  it was not given.
- Feed `note_cursor` from the editor so `scroll_to_show` keeps the caret visible.

##### Syntax / Method Signature

```rust
pub struct LineCol { pub line: u32, pub col: u32 }
impl LineCol {
    pub fn offset_in(self, text: &str) -> usize;
    pub fn from_offset(text: &str, offset: usize) -> Self;
    pub fn clamped_to(self, text: &str) -> Self;
}
pub struct ParseDiagnostic { /* line, col, message */ }
impl ParseDiagnostic { pub fn new(at: LineCol, message: impl Into<String>) -> Self; }
pub struct Growth { /* min_height, max_height */ }
impl Growth {
    pub fn min_height(&self) -> f32;
    pub fn max_height(&self) -> f32;
    pub fn height_for_lines(&self, lines: u32) -> f32;
    pub fn can_grow(&self, lines: u32) -> bool;
    pub fn rows_fitting(height: f32) -> u32;
    pub fn clamp_height(&self, height: f32) -> f32;
}
pub struct EditHistory { /* bounded undo stack */ }
impl EditHistory {
    pub fn new() -> Self;
    pub fn with_limit(limit: usize) -> Self;
    pub fn record(&mut self, before: &str);
    pub fn can_undo(&self) -> bool;
    pub fn can_redo(&self) -> bool;
    pub fn undo(&mut self, current: &str) -> Option<String>;
    pub fn redo(&mut self, current: &str) -> Option<String>;
    pub fn undo_depth(&self) -> usize;
    pub fn redo_depth(&self) -> usize;
    pub fn clear(&mut self);
}

pub struct TextAreaModel { /* private */ }
impl TextAreaModel {
    pub fn new() -> Self;
    pub fn with_text(mut self, text: impl Into<String>) -> Self;
    pub fn with_growth(mut self, growth: Growth) -> Self;
    pub fn with_disabled(mut self, disabled: bool) -> Self;
    pub fn with_read_only(mut self, read_only: bool) -> Self;
    pub fn with_stance(mut self, stance: EditStance) -> Self;
    pub fn with_font_family(mut self, font_family: typography::FontFamily) -> Self;
    pub fn mono(mut self) -> Self;
    pub fn text(&self) -> &str;
    pub fn cursor(&self) -> LineCol;
    pub fn selection(&self) -> Option<TextRange>;
    pub fn is_dirty(&self) -> bool;
    pub fn validation(&self) -> Option<&ParseDiagnostic>;
    pub fn is_invalid(&self) -> bool;
    pub fn note_commit(&mut self);
    pub fn note_blur(&mut self);
    pub fn note_submit(&mut self);
    pub fn reset_stance(&mut self);
    pub fn punishment(&self) -> Punishment;
    pub fn line_count(&self) -> u32;
    pub fn content_height(&self) -> f32;
    pub fn height(&self) -> f32;
    pub fn max_scroll(&self) -> f32;
    pub fn is_scrollable(&self) -> bool;
    pub fn scroll_offset(&self) -> f32;
    pub fn set_scroll(&mut self, offset: f32) -> f32;
    pub fn scroll_to_show(&mut self, at: LineCol) -> f32;
    pub fn set_text(&mut self, text: impl Into<String>);
    pub fn set_cursor(&mut self, at: LineCol);
    pub fn set_selection(&mut self, selection: Option<TextRange>);
    pub fn set_validation(&mut self, validation: Option<ParseDiagnostic>);
    pub fn mark_clean(&mut self);
}

pub struct TextAreaController { /* private */ }
impl TextAreaController {
    pub fn new() -> Self;
    pub fn with_value(text: impl Into<String>) -> Self;
    pub fn sync(&mut self, model: &TextAreaModel);
    pub fn note_edit(&mut self, model: &mut TextAreaModel, text: String) -> TextAreaOutcome;
    pub fn submit(&mut self, model: &TextAreaModel) -> TextAreaOutcome;
    pub fn commit(&mut self) -> TextAreaOutcome;
    pub fn cancel(&mut self, model: &mut TextAreaModel) -> TextAreaOutcome;
    pub fn undo(&mut self, model: &mut TextAreaModel) -> TextAreaOutcome;
    pub fn redo(&mut self, model: &mut TextAreaModel) -> TextAreaOutcome;
    pub fn note_cursor(&mut self, model: &mut TextAreaModel, at: LineCol) -> TextAreaOutcome;
    pub fn note_focus(&mut self, model: &mut TextAreaModel, focused: bool) -> TextAreaOutcome;
    pub fn value(&self) -> &str;
    pub fn session_open(&self) -> bool;
    pub fn snapshot(&self) -> Option<&str>;
    pub fn is_focused(&self) -> bool;
    pub fn history(&self) -> &EditHistory;
    pub fn reset(&mut self);
}

pub enum TextAreaEmission {
    Changed, Committed, Submitted, Cancelled, CursorChanged, Undone, Redone, FocusChanged,
}
pub struct TextAreaOutcome { /* emissions */ }
impl TextAreaOutcome {
    pub fn noop() -> Self;
    pub fn emissions(emissions: Vec<TextAreaEmission>) -> Self;
    pub fn is_noop(&self) -> bool;
}
```

##### Examples

```rust
let mut model = TextAreaModel::new().with_text(source).mono();
let mut controller = TextAreaController::with_value(source);

let outcome = controller.undo(&mut model);
for emission in outcome.emissions() {
    match emission {
        TextAreaEmission::Undone => state.dirty = model.is_dirty(),
        _ => {}
    }
}
```

##### Type references

| Type | Meaning |
| --- | --- |
| `LineCol` | Zero-based line and zero-based column caret position. |
| `ParseDiagnostic` | Error text plus position; drives the error caption and `Punishment`. |
| `Growth` | Min/max height policy for auto-growing areas. |
| `EditHistory` | Bounded undo/redo stack of buffer snapshots. |

##### Return values

- Every mutating controller call returns `TextAreaOutcome`. Iterate `emissions()`; an empty
  outcome means nothing changed.

##### Side effects

- `undo`, `redo`, `cancel`, and `note_cursor` mutate the model you pass in. Nothing else is touched.

##### References

- [`text_area_model.rs`](../src/hoarfrost/widget/text_area_model.rs),
  [`text_area_host.rs`](../src/hoarfrost/widget/text_area_host.rs)

#### `widget::password` — secret input (`UIF-ATOM-006`)

##### Overview

`SecretInput` wraps a secret string so `Debug` and `Display` never leak the plaintext.
`PasswordInputModel` adds reveal state. `PasswordController` translates text-input emissions into
password emissions.

##### Prerequisites & Requirements

- Enable the cargo feature `password`. It enables `text-input`.
- NEVER implement `Debug` for a host message that carries `SecretInput` plaintext. Use
  `invariant_gate::check_secret_redaction` in a test to prove the redaction holds.
- Reveal MUST be transient: call `note_focus(false)` on blur so the plaintext is hidden again.

##### Syntax / Method Signature

```rust
pub struct SecretInput(String);
impl SecretInput {
    pub fn new(plaintext: impl Into<String>) -> Self;
    pub fn expose(&self) -> &str;          // explicit, auditable access
    pub fn into_exposed(self) -> String;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn masked(&self) -> String;        // bullet string of the same length
    pub fn masked_len(&self) -> usize;
}

pub struct PasswordInputModel { /* private */ }
impl PasswordInputModel {
    pub fn new() -> Self;
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self;
    pub fn with_value(mut self, value: SecretInput) -> Self;
    pub fn with_disabled(mut self, disabled: bool) -> Self;
    pub fn with_read_only(mut self, read_only: bool) -> Self;
    pub fn with_stance(mut self, stance: EditStance) -> Self;
    pub fn value(&self) -> &SecretInput;
    pub fn masked_len(&self) -> usize;
    pub fn revealed_value(&self) -> Option<&str>;
    pub fn validation(&self) -> Option<&str>;
    pub fn is_invalid(&self) -> bool;
    pub fn is_revealed(&self) -> bool;
    pub fn is_focused(&self) -> bool;
    pub fn can_reveal(&self) -> bool;
    pub fn toggle_revealed(&mut self) -> bool;
    pub fn note_focus(&mut self, focused: bool) -> bool;
    pub fn set_value(&mut self, value: SecretInput);
    pub fn set_validation(&mut self, validation: Option<String>);
    pub fn note_commit(&mut self);
    pub fn note_blur(&mut self);
    pub fn note_submit(&mut self);
    pub fn reset_stance(&mut self);
    pub fn punishment(&self) -> Punishment;
    pub fn slot_for(&self) -> SlotContent;
}

pub struct PasswordController { /* private */ }
impl PasswordController {
    pub fn new() -> Self;
    pub fn observe(&mut self, model: &PasswordInputModel) -> bool;
    pub fn toggle_reveal(&mut self, model: &mut PasswordInputModel) -> PasswordOutcome;
    pub fn translate(emission: TextInputEmission) -> Option<PasswordEmission>;
    pub fn reset(&mut self);
}

pub enum PasswordEmission { Changed, Committed, Cancelled, Revealed, FocusChanged }
pub struct PasswordOutcome { /* emissions */ }
impl PasswordOutcome {
    pub fn noop() -> Self;
    pub fn emissions(emissions: Vec<PasswordEmission>) -> Self;
    pub fn is_noop(&self) -> bool;
}
```

##### Examples

```rust
let mut model = PasswordInputModel::new().with_value(SecretInput::new(secret));
let mut controller = PasswordController::new();
controller.observe(&model);

let outcome = controller.toggle_reveal(&mut model);
// outcome.emissions() contains PasswordEmission::Revealed
```

##### Type references

| Type | Meaning |
| --- | --- |
| `SecretInput` | Redacting wrapper for a secret. `Debug` prints a fixed placeholder. |
| `PasswordEmission::Revealed` | Reveal state flipped; the host re-renders the field. |

##### Return values

- `observe` returns `true` when the observed reveal state changed.
- `toggle_reveal` returns the emissions the host MUST deliver.

##### Side effects

- `toggle_reveal` mutates the model's reveal flag. No logging, no storage.

##### References

- [`password_model.rs`](../src/hoarfrost/widget/password_model.rs),
  [`password_host.rs`](../src/hoarfrost/widget/password_host.rs)

#### `widget::number_box` — numeric entry (`UIF-ATOM-007`)

##### Overview

`NumberBoxHost` is a pure state machine over `RangeModel`. It resolves stepper presses, repeat
holds, range keys, and free-text commits into tiered emissions. The visual resolvers and geometry
live in `widget::number_box`.

##### Prerequisites & Requirements

- Enable the cargo feature `number-box`. It enables `button`, `range-model`, and `text-input`.
- Commit typed text through `commit_text`. Do not write the model directly; parsing and clamping
  belong to the host.
- Call `blur` when the entry loses focus so an out-of-range draft is clamped and committed.

##### Syntax / Method Signature

```rust
pub struct NumberBoxHost { /* private */ }
impl NumberBoxHost {
    pub fn new() -> Self;
    pub fn is_open(&self) -> bool;
    pub fn snapshot(&self) -> Option<f64>;
    pub fn last_tier(&self) -> &'static str;
    pub fn hover(&mut self, over: bool);
    pub fn press_stepper(&mut self, model: &RangeModel, direction: StepDirection) -> NumberBoxOutcome;
    pub fn hold_stepper(
        &mut self,
        model: &RangeModel,
        direction: StepDirection,
        elapsed_ms: u64,
        repeat_interval_ms: u64,
    ) -> NumberBoxOutcome;
    pub fn release_stepper(&mut self, model: &RangeModel) -> NumberBoxOutcome;
    pub fn key(&mut self, model: &RangeModel, key: RangeKey) -> NumberBoxOutcome;
    pub fn commit_text(&mut self, model: &RangeModel, text: &str) -> NumberBoxOutcome;
    pub fn blur(&mut self, model: &RangeModel) -> NumberBoxOutcome;
    pub fn cancel(&mut self) -> NumberBoxOutcome;
    pub fn reset(&mut self);
}
pub enum StepDirection { Decrement, Increment }
pub enum RangeKey { Decrease, Increase, PageDown, PageUp, Home, End }
pub enum NumberBoxEmission { Changed, Committed, Cancelled }
pub struct NumberBoxOutcome { /* emissions */ }
impl NumberBoxOutcome {
    pub fn noop() -> Self;
    pub fn emissions(emissions: Vec<NumberBoxEmission>) -> Self;
    pub fn is_noop(&self) -> bool;
    pub fn is_value_change(&self) -> bool;
}

// pure resolvers and geometry (number_box.rs)
pub fn resolve_style(state: VisualState, palette: &Palette) -> NumberBoxStyle;
pub fn control_rect(origin: Point, width: f32) -> Rectangle;
pub fn stepper_rect(bounds: Rectangle) -> Rectangle;
pub fn stepper_rect_for(shape: StepperShape, bounds: Rectangle) -> Rectangle;
pub fn entry_rect(bounds: Rectangle) -> Rectangle;
pub fn entry_rect_for(shape: StepperShape, bounds: Rectangle) -> Rectangle;
pub fn step_button_rects(bounds: Rectangle, shape: StepperShape) -> [Rectangle; 2];
pub fn step_at(bounds: Rectangle, shape: StepperShape, point: Point) -> Option<StepDirection>;
pub fn entry_layer(bounds: Rectangle, style: &NumberBoxStyle) -> Option<Layer>;
pub fn entry_focus_ring_layer(bounds: Rectangle, style: &NumberBoxStyle) -> Option<Layer>;
pub fn stepper_layer(bounds: Rectangle, style: &NumberBoxStyle) -> Layer;
pub fn stepper_layer_for(
    shape: StepperShape,
    bounds: Rectangle,
    style: &NumberBoxStyle,
) -> Layer;
pub fn step_button_layers(
    bounds: Rectangle,
    style: &NumberBoxStyle,
    shape: StepperShape,
) -> [Layer; 2];
pub enum StepperShape { Stacked, SideBySide }
```

##### Examples

```rust
let mut host = NumberBoxHost::new();
let outcome = host.press_stepper(&range, StepDirection::Increment);
for e in outcome.emissions() {
    if let NumberBoxEmission::Changed(live) = e { send(Msg::VolumeLive(live)); }
}
```

##### Type references

| Type | Meaning |
| --- | --- |
| `RangeModel` | Shared interval model: value, min, max, step, precision, suffix. See `widget::ranges`. |
| `StepperShape` | `Stacked` (up/down arrows) or `SideBySide` (minus/plus). |
| `NumberBoxStyle` | Resolved entry and stepper colours for one frame. |

##### Return values

- Each action returns `NumberBoxOutcome`. `Changed` carries `Live<f64>`, `Committed` carries
  `Commit<f64>`, `Cancelled` carries `Cancel<f64>`.

##### Side effects

- None. `NumberBoxHost` never mutates `RangeModel`; the host applies the emitted value.

##### References

- [`number_box_host.rs`](../src/hoarfrost/widget/number_box_host.rs),
  [`number_box.rs`](../src/hoarfrost/widget/number_box.rs)

#### `widget::toggle` — checkbox, radio group, switch (`UIF-ATOM-008`)

##### Overview

Three single-control atoms share one module. `CheckboxModel` supports three states.
`RadioGroupModel` is a roving-tab-stop composite. `SwitchModel` models a pending, busy toggle.

##### Prerequisites & Requirements

- Enable the cargo feature `toggle`. It enables `button`.
- A radio group MUST expose exactly one tab stop. Use `RadioGroupModel::rove_and_select` for arrow
  keys and `rove` for the boundary policy.
- A busy switch MUST be rendered disabled; `SwitchModel::pending` reports the busy state.
- A tri-state checkbox MUST be given a `ChildSummary` when it drives children, so the parent state
  is derived, not hand-set.

##### Syntax / Method Signature

```rust
pub enum CheckState { Unchecked, Indeterminate, Checked }

pub struct CheckboxModel { /* private */ }
impl CheckboxModel {
    pub fn new(label: impl Into<String>) -> Self;
    pub fn with_state(mut self, state: CheckState) -> Self;
    pub fn with_description(mut self, description: impl Into<String>) -> Self;
    pub fn disabled(mut self) -> Self;
    pub fn toggle(&self, children: ChildSummary) -> Option<Commit<CheckState>>;
}
pub struct ChildSummary { /* total, checked */ }

pub struct RadioOption<Id> { /* id, label, enabled */ }
pub struct Roving<Id> { /* index, id, commit */ }
pub struct RadioGroupModel<Id> { /* private */ }
impl<Id> RadioGroupModel<Id> {
    pub fn new(options: Vec<RadioOption<Id>>) -> Self;
    pub fn with_selected(mut self, id: Id) -> Self;
    pub fn with_orientation(mut self, orientation: RadioOrientation) -> Self;
    pub fn disabled(mut self) -> Self;
    pub fn option_enabled(&self, index: usize) -> bool;
    pub fn enabled_indices(&self) -> Vec<usize>;
    pub fn selected_index(&self) -> Option<usize>;
    pub fn rove(&self, from: Option<usize>, direction: RoveDirection) -> Option<usize>;
    pub fn rove_and_select(&self, from: Option<usize>, key: ToggleKey) -> Option<Roving<Id>>;
    pub fn select(&self, id: &Id) -> Option<Commit<Id>>;
}

pub struct SwitchModel { /* private */ }
impl SwitchModel {
    pub fn new(label: impl Into<String>) -> Self;
    pub fn with_on(mut self, on: bool) -> Self;
    pub fn with_texts(mut self, on_text: impl Into<String>, off_text: impl Into<String>) -> Self;
    pub fn busy(mut self) -> Self;
    pub fn disabled(mut self) -> Self;
    pub fn toggle(&self) -> Option<Commit<bool>>;
    pub fn pending(&self) -> ActionPending;
}

// Widgets (toggle_widget.rs)
pub struct Checkbox<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> { /* private */ }
impl<'a, Message, Theme, Renderer> Checkbox<'a, Message, Theme, Renderer> {
    pub fn controlled(
        model: Controlled<'a, CheckboxModel>,
        focus_id: impl Into<FocusId>,
        emit: impl Fn(Commit<CheckState>) -> Option<Message> + 'a,
    ) -> Self;
    pub fn standalone(
        label: impl Into<String>,
        state: CheckState,
        focus_id: impl Into<FocusId>,
        on_toggle: impl Fn(CheckState) -> Option<Message> + 'a,
    ) -> Self;
    pub fn with_children(mut self, children: ChildSummary) -> Self;
    pub fn with_palette(mut self, palette: Palette) -> Self;
    pub fn disabled(mut self, disabled: bool) -> Self;
}
pub struct RadioGroup<'a, Message, Id = String, Theme = iced::Theme, Renderer = iced::Renderer> { /* private */ }
impl<'a, Message, Id, Theme, Renderer> RadioGroup<'a, Message, Id, Theme, Renderer> {
    pub fn controlled(
        model: Controlled<'a, RadioGroupModel<Id>>,
        focus_id: impl Into<FocusId>,
        emit: impl Fn(Commit<Id>) -> Option<Message> + 'a,
    ) -> Self;
    pub fn with_palette(mut self, palette: Palette) -> Self;
}
pub struct ToggleSwitch<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> { /* private */ }
impl<'a, Message, Theme, Renderer> ToggleSwitch<'a, Message, Theme, Renderer> {
    pub fn controlled(
        model: Controlled<'a, SwitchModel>,
        focus_id: impl Into<FocusId>,
        emit: impl Fn(Commit<bool>) -> Option<Message> + 'a,
    ) -> Self;
    pub fn standalone(
        label: impl Into<String>,
        on: bool,
        focus_id: impl Into<FocusId>,
        on_toggle: impl Fn(bool) -> Option<Message> + 'a,
    ) -> Self;
    pub fn with_palette(mut self, palette: Palette) -> Self;
    pub fn disabled(mut self, disabled: bool) -> Self;
    pub fn busy(mut self, busy: bool) -> Self;
    pub fn with_texts(mut self, on_text: impl Into<String>, off_text: impl Into<String>) -> Self;
}
```

##### Examples

```rust
let radios = RadioGroupModel::new(vec![
    RadioOption::new("low", "Low"),
    RadioOption::new("high", "High"),
]).with_selected("low");

let group = RadioGroup::controlled(radios.controlled(), "priority", |Commit { value }| {
    Some(Msg::Priority(value))
});
```

##### Type references

| Type | Meaning |
| --- | --- |
| `CheckState` | Tri-state value of a checkbox. |
| `ChildSummary` | `{ enabled: usize, checked: usize }`; used to derive a parent's indeterminate state. |
| `Roving<Id>` | Result of an arrow-key move inside a radio group: new index plus the commit. |
| `RoveDirection` | `Next` / `Previous`; roving wraps. |
| `ToggleKey` | Key abstraction for roving and activation inside toggles. |
| `RadioOrientation` | `Horizontal` / `Vertical`; sets the arrow-key axis. |

##### Return values

- `toggle`, `select`, and `rove_and_select` return `None` when the control is disabled or the target
  is not selectable.
- The `emit` closure returns `Option<Message>`; return `None` to swallow the emission.

##### Side effects

- None. Models are pure; the host applies the committed value.

##### References

- [`toggle_model.rs`](../src/hoarfrost/widget/toggle_model.rs),
  [`toggle_widget.rs`](../src/hoarfrost/widget/toggle_widget.rs)

#### `widget::combo_box` — select and combo box (`UIF-ATOM-009`)

##### Overview

`ComboBoxModel` holds items, selection, filter text, and validation, and can project itself as a
`surface::menu::model::Menu`. `ComboHost` drives open/close, filtering, keyboard navigation, and
the three tiers. `Select` is the plain iced-style select for hosts that need no filter.

##### Prerequisites & Requirements

- Enable the cargo feature `combo-box`. It enables `menu`, `text-input`.
- The host MUST call `sync(&model, &mut focus)` after any external change to selection or filter.
- The flyout is a menu card. Register it in the `SurfaceRegistry` so Escape closes it first.
- `Id` MUST implement `Clone + fmt::Display + PartialEq`.

##### Syntax / Method Signature

```rust
pub struct SelectOption<Id> { /* id, label, enabled */ }
impl<Id> SelectOption<Id> {
    pub fn new(id: Id, label: impl Into<String>) -> Self;
    pub fn matches(&self, query: &str, label: &str) -> bool;
}
pub struct ComboSnapshot<Id> { /* selected, filter */ }
impl<Id> ComboSnapshot<Id> { pub fn new(selected: Option<Id>, filter: Option<String>) -> Self; }

pub struct ComboBoxModel<Id> { /* private */ }
impl<Id> ComboBoxModel<Id> {
    pub fn new(items: Vec<SelectOption<Id>>, placeholder: impl Into<String>) -> Self;
    pub fn with_selected(mut self, id: impl Into<Option<Id>>) -> Self;
    pub fn with_filter(mut self, query: impl Into<Option<String>>) -> Self;
    pub fn with_validation(mut self, message: impl Into<String>) -> Self;
    pub fn with_menu_id(mut self, id: impl Into<String>) -> Self;
    pub fn items(&self) -> &[SelectOption<Id>];
    pub fn placeholder(&self) -> &str;
    pub fn filter_query(&self) -> Option<&str>;
    pub fn is_invalid(&self) -> bool;
    pub fn display_label(&self) -> &str;
    pub fn shows_placeholder(&self) -> bool;
    pub fn selected_option(&self) -> Option<&SelectOption<Id>>;
    pub fn selected_index(&self) -> Option<usize>;
    pub fn filtered(&self) -> Vec<&SelectOption<Id>>;
    pub fn filtered_with(&self, query: Option<&str>) -> Vec<&SelectOption<Id>>;
    pub fn has_pickable(&self) -> bool;
    pub fn filtered_option_with(&self, query: Option<&str>, index: usize) -> Option<&SelectOption<Id>>;
    pub fn id_by_key(&self, key: &str) -> Option<&Id>;
    pub fn snapshot(&self) -> ComboSnapshot<Id>;
    pub fn menu(&self) -> Menu;
    pub fn menu_with(&self, query: Option<&str>) -> Menu;
}

pub struct ComboHost<Id> { /* private */ }
impl<Id> ComboHost<Id> {
    pub fn new(control: FocusId) -> Self;
    pub fn is_open(&self) -> bool;
    pub fn filter(&self) -> Option<&str>;
    pub fn gesture_open(&self) -> bool;
    pub fn open(&mut self, model: &ComboBoxModel<Id>, focus: &mut FocusState) -> ComboOutcome<Id>;
    pub fn close(&mut self, focus: &mut FocusState) -> ComboOutcome<Id>;
    pub fn toggle(&mut self, model: &ComboBoxModel<Id>, focus: &mut FocusState) -> ComboOutcome<Id>;
    pub fn on_filter_typed(&mut self, model: &ComboBoxModel<Id>, query: String, focus: &mut FocusState) -> ComboOutcome<Id>;
    pub fn key(&mut self, key: ComboKey, model: &ComboBoxModel<Id>, focus: &mut FocusState) -> ComboOutcome<Id>;
    pub fn escape(&mut self, focus: &mut FocusState) -> ComboOutcome<Id>;
    pub fn hover(&mut self, over: bool);
    pub fn press(&mut self, model: &ComboBoxModel<Id>);
    pub fn release(&mut self, inside: bool, model: &ComboBoxModel<Id>, focus: &mut FocusState) -> ComboOutcome<Id>;
    pub fn sync(&mut self, model: &ComboBoxModel<Id>, focus: &mut FocusState) -> ComboOutcome<Id>;
    pub fn reset(&mut self);
}
pub enum ComboKey { Down, Up, Enter, Space, Home, End, Escape }
pub enum ComboEmission<Id> { OpenChanged, FilterChanged, Committed, Cancelled }
pub struct ComboOutcome<Id> { /* emissions */ }
impl<Id> ComboOutcome<Id> {
    pub fn noop() -> Self;
    pub fn emissions(emissions: Vec<ComboEmission<Id>>) -> Self;
    pub fn is_noop(&self) -> bool;
    pub fn is_value_change(&self) -> bool;
}

pub fn combo_box<'a, Message, Id: Clone + fmt::Display + PartialEq>(
    model: &'a ComboBoxModel<Id>,
) -> ComboBox<'a, Message, Id>;
pub struct ComboBox<'a, Message, Id> { /* private */ }
impl<'a, Message, Id> ComboBox<'a, Message, Id> {
    pub fn with_width(mut self, width: impl Into<Length>) -> Self;
    pub fn with_compact_width(mut self, width: impl Into<Length>) -> Self;
    pub fn on_toggle_open(mut self, msg: Message) -> Self;
    pub fn on_press(self, msg: Message) -> Self;
    pub fn on_select(mut self, f: impl Fn(Id) -> Message + 'a) -> Self;
}
pub const MIN_WIDTH_COMPACT: f32;
pub enum ComboRung { /* responsive width rungs */ }

pub fn select<'a, T, Message>(
    options: &'a [T],
    selected: Option<&'a T>,
    on_select: impl Fn(T) -> Message + 'a,
) -> Select<'a, T, Message>;
```

##### Examples

```rust
let model = ComboBoxModel::new(options, "Pick a font").with_selected("mono");
let mut host = ComboHost::new("font-combo");

let outcome = host.toggle(&model, &mut focus);
let flyout = combo_box(&model).on_select(|id| Msg::FontPicked(id));
```

##### Type references

| Type | Meaning |
| --- | --- |
| `SelectOption<Id>` | One item: id, label, enabled flag. |
| `ComboSnapshot<Id>` | Pre-gesture selection and filter, used to build `Cancel`. |
| `Menu` | The menu model the flyout renders; see `surface::menu`. |
| `ComboRung` | `Standard` or `Compact`; picks the floor width `MIN_WIDTH` (120) or `MIN_WIDTH_COMPACT` (64). |

##### Return values

- `ComboEmission::Committed` carries `Commit<Id>`; `Cancelled` carries `Cancel<ComboSnapshot<Id>>`.
- `is_value_change()` is `true` when the outcome contains `Committed`.

##### Side effects

- `open`, `close`, and `key` mutate `FocusState` through the trap and group roster. They never touch
  the model.

##### References

- [`combo_model.rs`](../src/hoarfrost/widget/combo_model.rs),
  [`combo_host.rs`](../src/hoarfrost/widget/combo_host.rs),
  [`combo_box.rs`](../src/hoarfrost/widget/combo_box.rs)

#### `widget::auto_suggest` — type-ahead (`UIF-ATOM-010`)

##### Overview

`AutoSuggestModel` holds the query and the current suggestion list. `AutoSuggestHost` adds debounce
timing, in-flight generation guarding, highlight movement, and dismissal that survives until the
query changes.

##### Prerequisites & Requirements

- Enable the cargo feature `auto-suggest`. It enables `menu`, `text-input`, `tooltip`.
- Pass the current clock as `TimeMs`. The host never reads the system clock.
- Ignore `apply_suggestions` results whose `generation` does not match `in_flight_query()`. Stale
  replies MUST NOT overwrite a newer query.
- Debounce defaults to the token duration; use `with_debounce` only to override it deliberately.

##### Syntax / Method Signature

```rust
pub struct SuggestionItem<Id> { /* id, label, detail */ }
impl<Id> SuggestionItem<Id> { pub fn new(id: Id, label: impl Into<String>) -> Self; }
pub struct AutoSuggestModel<Id> { /* private */ }
impl<Id> AutoSuggestModel<Id> {
    pub fn new(id: Id, source: CommitSource, label: impl Into<String>) -> Self;
    pub fn with_query(mut self, query: impl Into<String>) -> Self;
    pub fn with_suggestions(mut self, suggestions: Vec<SuggestionItem<Id>>) -> Self;
    pub fn disabled(mut self) -> Self;
    pub fn has_suggestions(&self) -> bool;
    pub fn query_is_blank(&self) -> bool;
    pub fn suggestion_at(&self, index: usize) -> Option<&SuggestionItem<Id>>;
    pub fn highlighted(&self) -> Option<&SuggestionItem<Id>>;
    pub fn index_of(&self, id: &Id) -> Option<usize>;
    pub fn menu_for(&self, control: &FocusId) -> Menu;
    pub fn menu_with(&self, menu_id: MenuId) -> Menu;
    pub fn has_pickable(&self, control: &FocusId) -> bool;
}
pub fn menu_id_for(control: &FocusId) -> MenuId;

pub struct AutoSuggestHost<Id> { /* private */ }
impl<Id> AutoSuggestHost<Id> {
    pub fn new(control: FocusId) -> Self;
    pub fn with_debounce(control: FocusId, debounce_ms: u64) -> Self;
    pub fn is_open(&self) -> bool;
    pub fn is_busy(&self) -> bool;
    pub fn is_debouncing(&self) -> bool;
    pub fn highlighted(&self) -> Option<usize>;
    pub fn in_flight_query(&self) -> Option<&str>;
    pub fn dismissed_query(&self) -> Option<&str>;
    pub fn is_dismissed(&self, query: &str) -> bool;
    pub fn note_edit(&mut self, model: &mut AutoSuggestModel<Id>, value: String, now: TimeMs) -> Vec<AutoSuggestEmission<Id>>;
    pub fn tick(&mut self, model: &mut AutoSuggestModel<Id>, now: TimeMs) -> Vec<AutoSuggestEmission<Id>>;
    pub fn apply_suggestions(&mut self, model: &mut AutoSuggestModel<Id>, generation: u64, suggestions: Vec<SuggestionItem<Id>>, focus: &mut FocusState) -> Vec<AutoSuggestEmission<Id>>;
    pub fn key(&mut self, model: &mut AutoSuggestModel<Id>, key: AutoSuggestKey, focus: &mut FocusState) -> Vec<AutoSuggestEmission<Id>>;
    pub fn escape(&mut self, model: &mut AutoSuggestModel<Id>, focus: &mut FocusState) -> Vec<AutoSuggestEmission<Id>>;
    pub fn click(&mut self, model: &mut AutoSuggestModel<Id>, index: usize, focus: &mut FocusState) -> Vec<AutoSuggestEmission<Id>>;
    pub fn note_focus(&mut self, model: &mut AutoSuggestModel<Id>, focused: bool, focus: &mut FocusState) -> Vec<AutoSuggestEmission<Id>>;
    pub fn sync(&mut self, model: &mut AutoSuggestModel<Id>, focus: &mut FocusState) -> Vec<AutoSuggestEmission<Id>>;
    pub fn reset(&mut self);
}
pub enum AutoSuggestKey { Up, Down, Enter, Tab, Escape }
pub enum AutoSuggestEmission<Id> {
    QueryChanged, QueryRequested, HighlightChanged, Committed,
    QueryCommitted, QueryCleared, Dismissed, OpenChanged, FocusChanged,
}
```

##### Examples

```rust
let mut host = AutoSuggestHost::new("city-box");
let emissions = host.note_edit(&mut model, "Ber".into(), TimeMs::now());
for e in emissions {
    if let AutoSuggestEmission::QueryRequested { query, generation } = e {
        commands.push(Command::Perform(fetch(query, generation)));
    }
}
```

##### Type references

| Type | Meaning |
| --- | --- |
| `SuggestionItem<Id>` | One suggestion. |
| `CommitSource` | Where a commit came from: typed, picked, or completed. |
| `TimeMs` | Millisecond clock value supplied by the host. |
| `MenuId` | String newtype naming the suggestion flyout's menu. |

##### Return values

- Every action returns `Vec<AutoSuggestEmission<Id>>`. An empty vector means nothing changed.
- `QueryRequested` carries the query and a generation counter for the async lookup.

##### Side effects

- All actions mutate the model you pass (query, suggestions, highlight). They also move focus.

##### References

- [`auto_suggest_model.rs`](../src/hoarfrost/widget/auto_suggest_model.rs),
  [`auto_suggest_host.rs`](../src/hoarfrost/widget/auto_suggest_host.rs)

#### `widget::date_picker`, `widget::time_picker`, `widget::color_picker` — value pickers (`UIF-ATOM-011`)

##### Overview

The three pickers own their value types (`CivilDate`, `CivilTime`, `Rgba8`/`Hsva`), their flyout
state, and their tiered callbacks. They use `.on_change` / `.on_commit` / `.on_cancel` instead of a
single `emit` closure.

##### Prerequisites & Requirements

- Enable `date-picker`, `time-picker`, or `color-picker`. None of them pulls in another feature.
- Register each open flyout as a `SurfaceKind::MenuCard` so Escape closes the flyout before the
  parent surface.
- Flyout edits are drafts. `cancel()` restores the pre-gesture value; `commit()` accepts it.
- `CivilDate::new` and `CivilTime::new` return `Option`; validate calendar values, never panic.

##### Syntax / Method Signature

```rust
// date_picker.rs
pub struct CivilDate { /* year, month, day */ }
impl CivilDate {
    pub fn new(year: i32, month: u8, day: u8) -> Option<Self>;
    pub fn day_of_week(&self) -> Weekday;
    pub fn next_day(&self) -> CivilDate;
    pub fn prev_day(&self) -> CivilDate;
    pub fn add_days(&self, days: i32) -> CivilDate;
    pub fn format_iso(&self) -> String;
    pub fn parse_iso(s: &str) -> Option<CivilDate>;
}
pub struct YearMonth { /* year, month */ }
impl YearMonth {
    pub fn new(year: i32, month: u8) -> Option<Self>;
    pub fn next_month(&self) -> YearMonth;
    pub fn prev_month(&self) -> YearMonth;
    pub fn first_date(&self) -> CivilDate;
    pub fn days_in_month(&self) -> u8;
    pub fn format_month_year(&self) -> String;
}
pub struct DatePickerModel { /* private */ }
impl DatePickerModel {
    pub fn new(initial_date: CivilDate) -> Self;
    pub fn with_selection(mut self, selection: DateSelection) -> Self;
    pub fn with_hover_preview_date(mut self, preview: Option<CivilDate>) -> Self;
    pub fn with_displayed_month(mut self, ym: YearMonth) -> Self;
    pub fn with_view_mode(mut self, mode: CalendarViewMode) -> Self;
    pub fn with_min_date(mut self, min: impl Into<Option<CivilDate>>) -> Self;
    pub fn with_max_date(mut self, max: impl Into<Option<CivilDate>>) -> Self;
    pub fn with_blackout_dates(mut self, dates: impl IntoIterator<Item = CivilDate>) -> Self;
    pub fn with_first_day_of_week(mut self, first_day: Weekday) -> Self;
    pub fn with_open(mut self, open: bool) -> Self;
    pub fn with_validation(mut self, validation: impl Into<Option<String>>) -> Self;
    pub fn with_disabled(mut self, disabled: bool) -> Self;
    pub fn is_date_disabled(&self, date: CivilDate) -> bool;
    pub fn select_date(&mut self, date: CivilDate);
}
pub fn date_picker<'a, Message>(model: DatePickerModel) -> DatePicker<'a, Message>;
impl<'a, Message> DatePicker<'a, Message> {
    pub fn with_id(mut self, id: impl Into<widget::Id>) -> Self;
    pub fn with_today(mut self, today: impl Into<Option<CivilDate>>) -> Self;
    pub fn with_palette(mut self, palette: Palette) -> Self;
    pub fn on_open_change<F>(mut self, callback: F) -> Self;   // Fn(Live<bool>) -> Message
    pub fn on_view_mode_change<F>(mut self, callback: F) -> Self;
    pub fn on_month_navigate<F>(mut self, callback: F) -> Self;
    pub fn on_change<F>(mut self, callback: F) -> Self;        // Fn(Live<CivilDate>) -> Message
    pub fn on_commit<F>(mut self, callback: F) -> Self;        // Fn(Commit<CivilDate>) -> Message
    pub fn on_cancel<F>(mut self, callback: F) -> Self;        // Fn(Cancel<CivilDate>) -> Message
}

// time_picker.rs
pub struct CivilTime { /* hour, minute, second */ }
impl CivilTime {
    pub fn new(hour: u8, minute: u8, second: u8) -> Option<Self>;
    pub fn from_12h(hour_12: u8, minute: u8, second: u8, am_pm: AmPm) -> Option<Self>;
    pub fn step_segment(&self, segment: TimeSegment, delta: i32, increment: u8) -> Self;
    pub fn toggle_am_pm(&self) -> Self;
    pub fn format_display(&self, use_24_hour: bool, show_seconds: bool) -> String;
    pub fn parse(s: &str) -> Option<Self>;
}
pub enum TimeSegment { Hour, Minute, Second, AmPm }
pub enum AmPm { Am, Pm }
pub struct TimePickerModel { /* private */ }
impl TimePickerModel {
    pub fn new(value: Option<CivilTime>) -> Self;
    pub fn with_24_hour(mut self, use_24_hour: bool) -> Self;
    pub fn with_show_seconds(mut self, show_seconds: bool) -> Self;
    pub fn with_minute_increment(mut self, minute_increment: u8) -> Self;
    pub fn with_open(mut self, open: bool) -> Self;
    pub fn with_validation(mut self, validation: impl Into<String>) -> Self;
    pub fn with_disabled(mut self, disabled: bool) -> Self;
    pub fn open_flyout(&mut self);
    pub fn close_flyout(&mut self);
    pub fn cancel(&mut self) -> Option<CivilTime>;
    pub fn commit(&mut self);
    pub fn step_segment(&mut self, segment: TimeSegment, delta: i32);
    pub fn select_time(&mut self, time: CivilTime);
    pub fn format_display(&self) -> String;
}
pub fn time_picker<'a, Message>(model: TimePickerModel) -> TimePicker<'a, Message>;
impl<'a, Message> TimePicker<'a, Message> {
    pub fn with_id(mut self, id: impl Into<widget::Id>) -> Self;
    pub fn palette(mut self, palette: Palette) -> Self;
    pub fn on_open_change(mut self, f: impl Fn(Live<bool>) -> Message + 'a) -> Self;
    pub fn on_change(mut self, f: impl Fn(Live<CivilTime>) -> Message + 'a) -> Self;
    pub fn on_commit(mut self, f: impl Fn(Commit<CivilTime>) -> Message + 'a) -> Self;
    pub fn on_cancel(mut self, f: impl Fn(Cancel<Option<CivilTime>>) -> Message + 'a) -> Self;
}

// color_picker.rs
pub struct Rgba8 { /* r, g, b, a */ }
impl Rgba8 {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self;
    pub fn from_hex(s: &str) -> Option<Self>;
    pub fn to_hex_rgb(&self) -> String;
    pub fn to_hex_rgba(&self) -> String;
}
pub struct Hsva { /* h, s, v, a */ }
impl Hsva {
    pub fn new(h: f32, s: f32, v: f32, a: f32) -> Self;
    pub fn to_rgba8(&self) -> Rgba8;
    pub fn from_rgba8(rgba: Rgba8) -> Self;
}
pub struct ColorPickerModel { /* private */ }
impl ColorPickerModel {
    pub fn new(value: Rgba8) -> Self;
    pub fn with_alpha(mut self, enabled: bool) -> Self;
    pub fn with_preset_swatches(mut self, swatches: impl IntoIterator<Item = Rgba8>) -> Self;
    pub fn with_contrast_reference(mut self, reference: Rgba8) -> Self;
    pub fn with_open(mut self, open: bool) -> Self;
    pub fn with_disabled(mut self, disabled: bool) -> Self;
    pub fn set_rgba(&mut self, rgba: Rgba8);
    pub fn set_hsva(&mut self, hsva: Hsva);
    pub fn set_hex_input(&mut self, hex: &str) -> bool;
    pub fn open_flyout(&mut self);
    pub fn close_flyout(&mut self);
    pub fn cancel(&mut self) -> Rgba8;
    pub fn commit(&mut self) -> Rgba8;
    pub fn format_display(&self) -> String;
}
pub fn color_picker<'a, Message>(model: ColorPickerModel) -> ColorPicker<'a, Message>;
impl<'a, Message> ColorPicker<'a, Message> {
    pub fn with_id(mut self, id: impl Into<iced::advanced::widget::Id>) -> Self;
    pub fn on_change(mut self, on_change: impl Fn(Live<Rgba8>) -> Message + 'a) -> Self;
    pub fn on_commit(mut self, on_commit: impl Fn(Commit<Rgba8>) -> Message + 'a) -> Self;
    pub fn on_cancel(mut self, on_cancel: impl Fn(Cancel<Rgba8>) -> Message + 'a) -> Self;
    pub fn on_open_change(mut self, on_open_change: impl Fn(Live<bool>) -> Message + 'a) -> Self;
}
pub struct ColorFlyoutLayout { /* ... */ }
impl ColorFlyoutLayout { pub fn compute(top_left: Point, model: &ColorPickerModel) -> Self; }
```

##### Examples

```rust
let model = DatePickerModel::new(CivilDate::new(2026, 11, 1).unwrap())
    .with_min_date(CivilDate::new(2026, 1, 1));

date_picker(model)
    .on_commit(|Commit { value }| Msg::Due(value))
    .on_cancel(|Cancel { previous }| Msg::DueRestored(previous));
```

##### Type references

| Type | Meaning |
| --- | --- |
| `DateSelection` | Single date or range selection inside the calendar. |
| `Weekday` | Hoarfrost's own weekday enum, defined in `date_picker.rs`; `week_order` and `days_until` drive the header row. |
| `CalendarViewMode` | Day grid or month/year navigation. |
| `GridCell` | One rendered calendar cell plus its role flags. |
| `ColorSwatchButton` | Preset swatch button used inside the colour flyout. |
| `ColorFlyoutLayout` | Resolved flyout geometry for the current model. |

##### Return values

- `on_change` fires the live tier while a draft is open. `on_commit` fires `Commit<T>`.
  `on_cancel` fires `Cancel<T>` with the pre-gesture value.
- `ColorPickerModel::set_hex_input` returns `false` and leaves the value unchanged on bad input.

##### Side effects

- None beyond the model. Flyout placement uses `geometry::anchor::solve`; nothing is persisted.

##### References

- [`date_picker.rs`](../src/hoarfrost/widget/date_picker.rs),
  [`time_picker.rs`](../src/hoarfrost/widget/time_picker.rs),
  [`color_picker.rs`](../src/hoarfrost/widget/color_picker.rs)

#### `widget::slider`, `widget::range_slider`, `widget::rating` — continuous controls (`UIF-ATOM-012`)

##### Overview

`RangeModel` is the shared interval model behind the slider, the range slider, and the number box.
`SliderHost` is the pure gesture machine for the single slider. `RangeSlider` and `RatingControl`
are self-contained widgets with tiered callbacks.

##### Prerequisites & Requirements

- Enable `slider`, `range-slider`, or `rating`. `slider` enables `button` and `range-model`;
  `range-slider` enables `range-model`.
- Emit the live tier on every drag tick and the commit tier once, on release. Do not commit on tick.
- A read-only rating MUST NOT be focusable. Set `RatingModel::with_read_only(true)`.

##### Syntax / Method Signature

```rust
// range_model.rs (feature "range-model")
pub struct RangeModel { /* private */ }
impl RangeModel {
    pub fn new(value: f64, min: f64, max: f64) -> Self;
    pub fn with_step(mut self, step: Option<f64>) -> Self;
    pub fn with_precision(mut self, precision: u8) -> Self;
    pub fn with_unit_suffix(mut self, unit: impl Into<String>) -> Self;
    pub fn with_validation(mut self, message: impl Into<String>) -> Self;
    pub fn with_disabled(mut self, disabled: bool) -> Self;
    pub fn unit_suffix(&self) -> Option<&str>;
    pub fn validation(&self) -> Option<&str>;
    pub fn ordered_bounds(&self) -> (f64, f64);
    pub fn low(&self) -> f64;
    pub fn high(&self) -> f64;
    pub fn is_inverted(&self) -> bool;
    pub fn is_degenerate(&self) -> bool;
    pub fn clamp(&self, value: f64) -> f64;
    pub fn clamped_value(&self) -> f64;
    pub fn effective_step(&self) -> Option<f64>;
    pub fn snap(&self, value: f64) -> f64;
    pub fn step_by(&self, value: f64, delta: i64) -> Option<f64>;
    pub fn apply_key(&self, key: RangeKey) -> Option<f64>;
    pub fn fraction(&self) -> f32;
    pub fn value_at_fraction(&self, fraction: f32) -> f64;
    pub fn display(&self) -> String;
    pub fn parse(&self, text: &str) -> Option<f64>;
}
```

`RangeModel` bounds may be inverted: `low()` and `high()` return the ordered pair and
`ordered_bounds()` returns `(low, high)` regardless of declaration order. `parse` returns `None`
for text that is not a number or falls outside the interval.

```rust

// slider_host.rs
pub struct SliderHost { /* private */ }
impl SliderHost {
    pub fn new() -> Self;
    pub fn is_open(&self) -> bool;
    pub fn snapshot(&self) -> Option<f64>;
    pub fn last_tier(&self) -> &'static str;
    pub fn hover(&mut self, over: bool);
    pub fn press(&mut self, model: &RangeModel, fraction: f32) -> SliderOutcome;
    pub fn drag(&mut self, model: &RangeModel, fraction: f32) -> SliderOutcome;
    pub fn release(&mut self, model: &RangeModel) -> SliderOutcome;
    pub fn cancel(&mut self) -> SliderOutcome;
    pub fn blur(&mut self, model: &RangeModel) -> SliderOutcome;
    pub fn key(&mut self, model: &RangeModel, key: RangeKey) -> SliderOutcome;
    pub fn reset(&mut self);
}
pub enum SliderEmission { Changed, Committed, Cancelled }
pub struct SliderOutcome { /* emissions */ }
impl SliderOutcome {
    pub fn noop() -> Self;
    pub fn emissions(emissions: Vec<SliderEmission>) -> Self;
    pub fn is_noop(&self) -> bool;
    pub fn is_value_change(&self) -> bool;
}

// slider.rs (pure resolvers)
pub fn resolve_style(state: VisualState, mount: Color, palette: &Palette) -> SliderStyle;
pub fn track_rect(bounds: Rectangle) -> Rectangle;
pub fn thumb_travel(track: Rectangle) -> f32;
pub fn thumb_center_x(track: Rectangle, fraction: f32) -> f32;
pub fn fraction_at_x(track: Rectangle, x: f32) -> f32;
pub fn thumb_rect(track: Rectangle, fraction: f32) -> Rectangle;
pub fn hit_rect(track: Rectangle, fraction: f32) -> Rectangle;
pub fn tick_fractions(count: usize) -> Vec<f32>;
pub fn rail_layer(track: Rectangle, style: &SliderStyle) -> Layer;
pub fn filled_layer(track: Rectangle, style: &SliderStyle, fraction: f32) -> Option<Layer>;
pub fn thumb_layer(track: Rectangle, style: &SliderStyle, fraction: f32) -> Layer;
pub fn tick_layers(track: Rectangle, style: &SliderStyle, count: usize) -> Vec<Layer>;
pub fn focus_ring_layer(track: Rectangle, style: &SliderStyle, fraction: f32) -> Option<Layer>;
pub fn filled_on_unfilled(palette: &Palette) -> f32;
pub fn thumb_is_distinguishable(style: &SliderStyle, behind: Color) -> bool;
pub fn thumb_carrier(style: &SliderStyle, behind: Color) -> ThumbCarrier;

// range_slider_model.rs / range_slider.rs
pub enum RangeThumb { Lower, Upper }
pub struct RangeSliderModel { /* private */ }
impl RangeSliderModel {
    pub fn new(min: f64, max: f64, lower: f64, upper: f64) -> Self;
    pub fn with_min_span(mut self, min_span: f64) -> Self;
    pub fn with_step(mut self, step: Option<f64>) -> Self;
    pub fn with_precision(mut self, precision: u8) -> Self;
    pub fn with_unit_suffix(mut self, unit: impl Into<String>) -> Self;
    pub fn with_validation(mut self, message: impl Into<String>) -> Self;
    pub fn with_disabled(mut self, disabled: bool) -> Self;
    pub fn low(&self) -> f64;
    pub fn high(&self) -> f64;
    pub fn ordered_bounds(&self) -> (f64, f64);
    pub fn effective_min_span(&self) -> f64;
    pub fn effective_step(&self) -> Option<f64>;
    pub fn clamp_lower(&self, val: f64) -> f64;
    pub fn clamp_upper(&self, val: f64) -> f64;
    pub fn clamped_range(&self) -> (f64, f64);
    pub fn snap(&self, value: f64) -> f64;
    pub fn lower_fraction(&self) -> f32;
    pub fn upper_fraction(&self) -> f32;
    pub fn value_at_fraction(&self, fraction: f32) -> f64;
    pub fn disambiguate_thumb(&self, click_fraction: f32) -> RangeThumb;
    pub fn step_thumb(&self, thumb: RangeThumb, key: RangeKey) -> (f64, f64);
    pub fn format_value(&self, val: f64) -> String;
    pub fn display(&self) -> (String, String);
}
pub fn range_slider<'a, Message>(model: RangeSliderModel) -> RangeSlider<'a, Message>;

// rating.rs
pub struct RatingModel { /* private */ }
impl RatingModel {
    pub fn new(max: u8) -> Self;
    pub fn with_value(mut self, value: impl Into<Option<f32>>) -> Self;
    pub fn with_hover_preview(mut self, hover_preview: impl Into<Option<f32>>) -> Self;
    pub fn with_step(mut self, step: RatingStep) -> Self;
    pub fn with_allow_clear(mut self, allow_clear: bool) -> Self;
    pub fn with_read_only(mut self, read_only: bool) -> Self;
    pub fn with_disabled(mut self, disabled: bool) -> Self;
    pub fn effective_value(&self) -> Option<f32>;
    pub fn total_width(&self) -> f32;
}
pub enum RatingStep { Half, Whole }
pub fn rating_control<'a, Message>(model: RatingModel) -> RatingControl<'a, Message>;
impl<'a, Message> RatingControl<'a, Message> {
    pub fn on_hover_preview(
        mut self,
        callback: impl Fn(Live<Option<f32>>) -> Message + 'a,
    ) -> Self;
    pub fn on_commit(mut self, callback: impl Fn(Commit<Option<f32>>) -> Message + 'a) -> Self;
    pub fn on_cancel(mut self, callback: impl Fn(Cancel<Option<f32>>) -> Message + 'a) -> Self;
    pub fn palette(mut self, palette: Palette) -> Self;
    pub fn id(mut self, id: widget::Id) -> Self;
}
pub fn total_rating_width(max: u8) -> f32;
pub fn star_slot_bounds(bounds: Rectangle, slot_index: usize) -> Rectangle;
pub fn value_at_position(
    bounds: Rectangle,
    pos: Point,
    max: u8,
    step: RatingStep,
) -> Option<(usize, f32)>;
```

##### Examples

```rust
let range = RangeModel::new(0.4, 0.0, 1.0).with_step(Some(0.05)).with_precision(2);
let mut host = SliderHost::new();

let outcome = host.drag(&range, 0.75);
for e in outcome.emissions() {
    if let SliderEmission::Changed(Live { value }) = e { volume = value; }
}
```

##### Type references

| Type | Meaning |
| --- | --- |
| `RangeModel` | Interval plus step, precision, suffix, validation. Shared by three controls. |
| `RangeKey` | `Decrease` / `Increase` / `PageDown` / `PageUp` / `Home` / `End`. |
| `RangeThumb` | Which thumb of a range slider a gesture targets. |
| `ThumbCarrier` | How the thumb stays visible over any fill: border, shadow, or both. |
| `RatingStep` | Half-star or whole-star granularity. |

##### Return values

- Hosts return `Outcome` structs; iterate `emissions()`. `Changed` carries `Live<f64>`,
  `Committed` carries `Commit<f64>`, `Cancelled` carries `Cancel<f64>` with the pre-gesture value.
- Pure resolvers return `Layer`, `Rectangle`, `Color`, `f32`, or `Option<Layer>`.

##### Side effects

- Hosts never mutate `RangeModel`. The host writes the committed value back.

##### References

- [`range_model.rs`](../src/hoarfrost/widget/range_model.rs),
  [`slider_host.rs`](../src/hoarfrost/widget/slider_host.rs),
  [`slider.rs`](../src/hoarfrost/widget/slider.rs),
  [`range_slider_model.rs`](../src/hoarfrost/widget/range_slider_model.rs),
  [`rating.rs`](../src/hoarfrost/widget/rating.rs)

#### `widget::segmented` and `widget::split_button` (`UIF-ATOM-013`)

##### Overview

`SegmentedButton` is a roving-tab-stop strip of segments, single- or multi-select.
`SplitButton` pairs a primary activation with a menu half; `SplitButtonHost` keeps the two halves
independent and gates the primary half through `ActionPending`.

##### Prerequisites & Requirements

- Enable `segmented` or `split-button`. Both enable `button`; `split-button` also enables `menu`.
- A segmented strip MUST expose one tab stop. Use `SegmentedModel::rove` for arrow movement.
- A split button's menu MUST be registered as a `SurfaceKind::MenuCard` while open.
- Escape on a split button MUST close the menu before it cancels anything else.

##### Syntax / Method Signature

```rust
// segmented_model.rs
pub struct SegmentItem<Id> { /* id, label, enabled, is_on */ }
impl<Id> SegmentItem<Id> {
    pub fn new(id: Id, label: impl Into<String>) -> Self;
    pub fn toggle_button(on: bool) -> Self;
    pub fn disabled(mut self) -> Self;
    pub fn is_on(&self) -> bool;
}
pub struct SegmentedModel<Id> { /* private */ }
impl<Id> SegmentedModel<Id> {
    pub fn new(segments: Vec<SegmentItem<Id>>) -> Self;
    pub fn multi(segments: Vec<SegmentItem<Id>>) -> Self;
    pub fn with_selected(mut self, id: Id) -> Self;
    pub fn with_selection(mut self, selected: BTreeSet<Id>) -> Self;
    pub fn with_mode(mut self, mode: SelectionMode) -> Self;
    pub fn with_density(mut self, density: ButtonDensity) -> Self;
    pub fn disabled(mut self) -> Self;
    pub fn segment_enabled(&self, index: usize) -> bool;
    pub fn enabled_indices(&self) -> Vec<usize>;
    pub fn index_of(&self, id: &Id) -> Option<usize>;
    pub fn is_selected(&self, id: &Id) -> bool;
    pub fn selected_count(&self) -> usize;
    pub fn rove(&self, from: Option<usize>, direction: RoveDirection) -> Option<usize>;
    pub fn toggle(&self, id: &Id) -> Option<Commit<BTreeSet<Id>>>;
    pub fn toggle_index(&self, index: usize) -> Option<Commit<BTreeSet<Id>>>;
}

// segmented.rs
pub enum SegmentPolarity { Unselected, Selected }
pub struct SegmentedState { /* roving index, hover, press */ }
pub struct SegmentedButton<'a, Message, Id = String> { /* private */ }
impl<'a, Message, Id> SegmentedButton<'a, Message, Id> {
    pub fn controlled(
        model: Controlled<'a, SegmentedModel<Id>>,
        focus_id: impl Into<FocusId>,
        emit: impl Fn(BTreeSet<Id>) -> Option<Message> + 'a,
    ) -> Self;
    pub fn palette(mut self, palette: Palette) -> Self;
}
pub fn resolve_style(
    polarity: SegmentPolarity,
    state: VisualState,
    palette: &Palette,
) -> SegmentStyle;
pub fn resolve_strip_ring(state: VisualState, mount: Color, palette: &Palette) -> Option<(Color, f32)>;
pub fn segment_width(label_width: f32) -> f32;
pub fn strip_width(label_widths: &[f32]) -> f32;
pub fn segment_bounds(strip: Rectangle, index: usize, label_widths: &[f32]) -> Rectangle;
pub fn check_slot(segment: Rectangle) -> Rectangle;
pub fn label_bounds(segment: Rectangle) -> Rectangle;
pub fn segment_radius(index: usize, count: usize) -> iced::border::Radius;
pub fn perimeter_class(style: &SegmentStyle, palette: &Palette) -> ChromeClass;

// split_button_model.rs
pub struct SplitMenuRow<ActionId> { /* id, label, shortcut */ }
impl<ActionId> SplitMenuRow<ActionId> {
    pub fn new(id: ActionId, label: impl Into<String>) -> Self;
    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self;
    pub fn to_row(&self) -> Row;
}
pub struct SplitButtonModel<ActionId> { /* private */ }
impl<ActionId> SplitButtonModel<ActionId> {
    pub fn new(label: impl Into<String>, menu_rows: Vec<SplitMenuRow<ActionId>>) -> Self;
    pub fn primary(label: impl Into<String>, menu_rows: Vec<SplitMenuRow<ActionId>>) -> Self;
    pub fn with_menu_id(mut self, id: impl Into<String>) -> Self;
    pub fn with_menu_rows(mut self, menu_rows: Vec<SplitMenuRow<ActionId>>) -> Self;
    pub fn with_leading_icon(mut self, icon: ButtonIcon) -> Self;
    pub fn with_density(mut self, density: ButtonDensity) -> Self;
    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self;
    pub fn busy(mut self) -> Self;
    pub fn disabled(mut self) -> Self;
    pub fn cancellable(mut self) -> Self;
    pub fn label(&self) -> &str;
    pub fn variant(&self) -> ButtonVariant;
    pub fn density(&self) -> ButtonDensity;
    pub fn pending(&self) -> ActionPending;
    pub fn tracks_action(&self) -> bool;
    pub fn has_navigable_menu(&self) -> bool;
    pub fn menu(&self) -> Menu;
    pub fn action_at(&self, index: usize) -> Option<&ActionId>;
}

// split_button_host.rs
pub struct SplitButtonHost { /* private */ }
impl SplitButtonHost {
    pub fn new(control: FocusId) -> Self;
    pub fn observe<ActionId: Clone + fmt::Display>(&mut self, model: &SplitButtonModel<ActionId>);
    pub fn is_menu_open(&self) -> bool;
    pub fn half_state(&self, half: SplitZone, focused: bool) -> VisualState;
    pub fn hover(&mut self, zone: SplitZone);
    pub fn pointer_leave(&mut self);
    pub fn press<ActionId: Clone + fmt::Display>(&mut self, zone: SplitZone, model: &SplitButtonModel<ActionId>) -> SplitButtonOutcome<ActionId>;
    pub fn release<ActionId: Clone + fmt::Display>(&mut self, zone: SplitZone, model: &SplitButtonModel<ActionId>, focus: &mut FocusState) -> SplitButtonOutcome<ActionId>;
    pub fn activate_primary<ActionId: Clone + fmt::Display>(&mut self, model: &SplitButtonModel<ActionId>) -> SplitButtonOutcome<ActionId>;
    pub fn open_menu<ActionId: Clone + fmt::Display>(&mut self, model: &SplitButtonModel<ActionId>, focus: &mut FocusState) -> SplitButtonOutcome<ActionId>;
    pub fn close_menu<ActionId>(&mut self, focus: &mut FocusState) -> SplitButtonOutcome<ActionId>;
    pub fn menu_key<ActionId: Clone + fmt::Display>(&mut self, key: MenuKey, model: &SplitButtonModel<ActionId>, focus: &mut FocusState) -> SplitButtonOutcome<ActionId>;
    pub fn escape<ActionId: Clone + fmt::Display>(&mut self, model: &SplitButtonModel<ActionId>, focus: &mut FocusState) -> SplitButtonOutcome<ActionId>;
    pub fn settlement(&self) -> Option<&ButtonSettlement>;
    pub fn is_primary(&self) -> bool;
    pub fn reset(&mut self);
}
pub enum SplitZone { Primary, Menu, Outside }
pub enum SplitButtonEmission<ActionId> {
    PrimaryActivated, PrimaryCancelled, StopRequested, MenuToggled, MenuSelected,
}

// split_button.rs (pure geometry)
pub fn divider_x(width: f32) -> f32;
pub fn zone_at(x: f32, width: f32) -> SplitZone;
pub fn primary_zone(box_: Rectangle) -> Rectangle;
pub fn menu_zone(box_: Rectangle) -> Rectangle;
pub fn divider_bounds(box_: Rectangle) -> Rectangle;
pub fn control_width(label_width: f32) -> f32;
pub fn label_bounds(primary: Rectangle) -> Rectangle;
pub fn chevron_bounds(menu: Rectangle) -> Rectangle;
pub fn height_for_density(density: ButtonDensity) -> f32;
pub fn resolve_style(
    variant: ButtonVariant,
    primary_state: VisualState,
    menu_state: VisualState,
    palette: &Palette,
) -> SplitStyle;
pub fn chevron_color(style: &SplitStyle) -> Color;
```

##### Examples

```rust
let model = SplitButtonModel::new("Send", rows).with_density(ButtonDensity::Compact);
let mut host = SplitButtonHost::new("send-split");
host.observe(&model);

let outcome = host.release(SplitZone::Menu, &model, &mut focus);
```

##### Type references

| Type | Meaning |
| --- | --- |
| `SegmentItem<Id>` | One segment; `toggle_button` marks it as an on/off segment. |
| `SegmentPolarity` | `Unselected` / `Selected`; the style input for one segment. |
| `SelectionMode` | `SingleSelect` / `MultiSelect`, defined in `segmented_model`. It is a different type from `tree_view::SelectionMode`. |
| `SplitZone` | Which half a gesture landed on. `Outside` cancels. |
| `SplitStyle` | Resolved colours for both halves and the divider. |

##### Return values

- `SegmentedButton` emits the full selected `BTreeSet<Id>` on each change, not a delta.
- `SplitButtonHost` returns `SplitButtonOutcome<ActionId>`; `MenuSelected` carries the chosen
  `ActionId`.

##### Side effects

- `open_menu`, `close_menu`, and `menu_key` move focus through `FocusState`. Nothing else changes.

##### References

- [`segmented_model.rs`](../src/hoarfrost/widget/segmented_model.rs),
  [`segmented.rs`](../src/hoarfrost/widget/segmented.rs),
  [`split_button_model.rs`](../src/hoarfrost/widget/split_button_model.rs),
  [`split_button_host.rs`](../src/hoarfrost/widget/split_button_host.rs),
  [`split_button.rs`](../src/hoarfrost/widget/split_button.rs)

#### Display atoms: `badge`, `chip`, `avatar`, `hyperlink`, `search_box`, `progress` (`UIF-ATOM-014`)

##### Overview

These atoms carry status and identity rather than commands. `Badge`, `Chip`, `Avatar`, and
`Progress` are model-driven. `Hyperlink` and `SearchBox` add activation.

##### Prerequisites & Requirements

- Enable `badge`, `chip`, `avatar`, `hyperlink`, `search-box`, or `progress`. `avatar` enables
  `badge`; `hyperlink` and `progress` enable `button`.
- A numeric badge MUST use tabular figures. Use `BadgeModel::count`.
- An icon-only button or a truncated label MUST carry a tooltip. `ButtonModel::tooltip_for` returns
  the required text; `invariant_gate::check_icon_only_button_tooltip` proves the rule.
- A progress bar that can be cancelled MUST set `with_cancellable(true)`.

##### Syntax / Method Signature

```rust
// badge_model.rs
pub enum BadgeTone { Neutral, Accent, Success, Warning, Danger }
pub struct BadgeModel { /* private */ }
impl BadgeModel {
    pub fn dot() -> Self;
    pub fn count(n: u32) -> Self;
    pub fn status(icon: StatusIcon, label: &'static str) -> Self;
    pub fn with_tone(mut self, tone: BadgeTone) -> Self;
    pub fn kind(&self) -> &BadgeKind;
    pub fn tone(&self) -> BadgeTone;
    pub fn is_empty(&self) -> bool;
    pub fn text(&self) -> Option<String>;
}
// badge.rs (resolvers)
pub fn resolve_style(model: &BadgeModel, palette: &Palette) -> BadgeStyle;
pub fn resolve_tone(tone: BadgeTone, palette: &Palette) -> ToneColors;
pub fn pill_width(style: &BadgeStyle, text_width: f32) -> f32;
pub fn chip_width(style: &BadgeStyle, glyph_width: f32, text_width: f32) -> f32;
pub fn badge_chrome_class(style: &BadgeStyle, palette: &Palette) -> ChromeClass;

// chip_model.rs
pub enum ChipVariant { Assist, Filter, Input }
pub struct ChipModel<Id> { /* private */ }
impl<Id> ChipModel<Id> {
    pub fn assist(id: Id, label: impl Into<String>) -> Self;
    pub fn filter(id: Id, label: impl Into<String>, selected: bool) -> Self;
    pub fn input(id: Id, label: impl Into<String>, removable: bool) -> Self;
    pub fn new(id: Id, label: impl Into<String>, variant: ChipVariant) -> Self;
    pub fn with_height(mut self, height: ChipHeight) -> Self;
    pub fn with_leading(mut self, icon: ChipIcon) -> Self;
    pub fn disabled(mut self) -> Self;
    pub fn id(&self) -> &Id;
    pub fn label(&self) -> &str;
    pub fn is_selected(&self) -> bool;
    pub fn is_removable(&self) -> bool;
    pub fn controlled(&self) -> Controlled<'_, Self>;
    pub fn respond(&self, gesture: ChipGesture) -> Option<ChipEvent<Id>>;
}
pub enum ChipPolarity { Unselected, Selected }
pub fn resolve_style(
    polarity: ChipPolarity,
    height: ChipHeight,
    state: VisualState,
    palette: &Palette,
) -> ChipStyle;
pub fn resolve_model<Id>(
    model: &ChipModel<Id>,
    input: InteractionInput,
    palette: &Palette,
) -> ChipStyle;
pub fn with_variant(style: ChipStyle, is_assist: bool, is_removable: bool) -> ChipStyle;
pub fn chip_width(style: &ChipStyle, leading_width: f32, text_width: f32) -> f32;
pub fn remove_slot(style: &ChipStyle, bounds: Rectangle) -> Option<Rectangle>;
pub fn chip_chrome_class(style: &ChipStyle, palette: &Palette) -> ChromeClass;

// avatar.rs
pub enum AvatarSize { Sm, Md, Lg }
pub enum AvatarShape { Circle, RoundedSquare }
pub struct AvatarModel { /* private */ }
impl AvatarModel {
    pub fn new(display_name: impl Into<String>) -> Self;
    pub fn with_initials(mut self, initials: impl Into<String>) -> Self;
    pub fn with_image_uri(mut self, uri: impl Into<String>) -> Self;
    pub fn with_presence(mut self, presence: impl Into<Option<BadgeTone>>) -> Self;
    pub fn with_size(mut self, size: AvatarSize) -> Self;
    pub fn with_shape(mut self, shape: AvatarShape) -> Self;
    pub fn fallback_stage(&self) -> FallbackStage;
}
pub fn avatar(model: AvatarModel) -> Avatar;

// hyperlink_model.rs
pub struct HyperlinkTarget { /* uri, external */ }
impl HyperlinkTarget { pub fn new(uri: impl Into<String>, external: bool) -> Self; }
pub struct HyperlinkModel { /* private */ }
impl HyperlinkModel {
    pub fn new(label: impl Into<String>, uri: impl Into<String>) -> Self;
    pub fn external(label: impl Into<String>, uri: impl Into<String>) -> Self;
    pub fn target(&self) -> HyperlinkTarget;
    pub fn activate(&self) -> Option<Commit<HyperlinkTarget>>;
    pub fn hover_uri(&self) -> Option<String>;
    pub fn hover_entered(&self) -> Option<HyperlinkEmission>;
    pub fn hover_cleared() -> HyperlinkEmission;
}
pub enum HyperlinkEmission { Opened, HoverUri }
impl Hyperlink<'a, Message> {
    pub fn controlled(
        model: Controlled<'a, HyperlinkModel>,
        focus_id: impl Into<FocusId>,
        emit: impl Fn(HyperlinkEmission) -> Option<Message> + 'a,
    ) -> Self;
    pub fn standalone(
        label: impl Into<String>,
        uri: impl Into<String>,
        focus_id: impl Into<FocusId>,
        on_press: impl Fn(HyperlinkTarget) -> Option<Message> + 'a,
    ) -> Self;
    pub fn mount(mut self, mount: Color) -> Self;
    pub fn palette(mut self, palette: Palette) -> Self;
    pub fn external(mut self, external: bool) -> Self;
    pub fn enabled(mut self, enabled: bool) -> Self;
}
pub fn resolve_style(state: VisualState, mount: Color, palette: &Palette) -> HyperlinkStyle;
pub fn control_size(label_width: f32) -> Size;
pub fn underline_bounds(control: Rectangle) -> Rectangle;

// search_box.rs
pub struct SearchBoxModel { /* private */ }
impl SearchBoxModel {
    pub fn new(query: impl Into<String>) -> Self;
    pub fn with_active(mut self, active: bool) -> Self;
    pub fn is_active(&self) -> bool;
    pub fn trailing_glyph(&self) -> char;
}
impl<'a, Message> SearchBox<'a, Message> {
    pub fn new(query: impl Into<String>) -> Self;
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self;
    pub fn with_active(mut self, active: bool) -> Self;
    pub fn with_palette(mut self, palette: Palette) -> Self;
    pub fn on_input(mut self, f: impl Fn(String) -> Message + 'a) -> Self;
    pub fn on_submit(mut self, msg: Message) -> Self;
    pub fn on_clear(mut self, msg: Message) -> Self;
}

// progress_model.rs
pub struct ProgressModel { /* private */ }
impl ProgressModel {
    pub fn determinate(completed: u64, total: u64) -> Self;
    pub fn indeterminate() -> Self;
    pub fn with_status(mut self, status: ProgressStatus) -> Self;
    pub fn with_label(mut self, label: &'static str) -> Self;
    pub fn with_cancellable(mut self, cancellable: bool) -> Self;
    pub fn with_mode(mut self, mode: ProgressMode) -> Self;
    pub fn with_completed(mut self, completed: u64) -> Self;
    pub fn mode(&self) -> ProgressMode;
    pub fn status(&self) -> ProgressStatus;
    pub fn label(&self) -> Option<&'static str>;
    pub fn cancellable(&self) -> bool;
    pub fn is_indeterminate(&self) -> bool;
    pub fn fraction(&self) -> Option<f32>;
}
// progress.rs (resolvers)
pub fn resolve_style(model: &ProgressModel, palette: &Palette) -> ProgressStyle;
pub fn indeterminate_segment(/* ... */) -> SegmentStyle;
pub fn spinner_style(large: bool, inside_solid: bool, palette: &Palette) -> SpinnerStyle;
pub fn track_layer(bounds: Rectangle, style: &ProgressStyle) -> Layer;
pub fn fill_layer(bounds: Rectangle, style: &ProgressStyle, fraction: f32) -> Option<Layer>;
pub fn segment_layer(bounds: Rectangle, segment: &SegmentStyle) -> Layer;
```

##### Examples

```rust
let badge = BadgeModel::count(12).with_tone(BadgeTone::Accent);
let link = HyperlinkModel::external("Documentation", "https://example.com/docs");
let bar = ProgressModel::determinate(3, 8).with_cancellable(true);
```

##### Type references

| Type | Meaning |
| --- | --- |
| `BadgeKind` | `Dot`, `Count`, or `Status` rendering. |
| `ChipGesture` / `ChipEvent<Id>` | Chip press kinds and the resulting event. |
| `FallbackStage` | Avatar fallback chain: image, initials, or glyph. |
| `ProgressMode` | `Determinate` or `Indeterminate`. |
| `ProgressStatus` | Status colour role for the bar. |
| `ChromeClass` | Which contrast rule a border or fill is judged under. |

##### Return values

- `HyperlinkModel::activate` returns `None` when the link is disabled.
- `AvatarModel::fallback_stage` never returns an error; it degrades instead.

##### Side effects

- None. The host opens URIs; the crate never does.

##### References

- [`badge_model.rs`](../src/hoarfrost/widget/badge_model.rs),
  [`chip_model.rs`](../src/hoarfrost/widget/chip_model.rs),
  [`avatar.rs`](../src/hoarfrost/widget/avatar.rs),
  [`hyperlink_model.rs`](../src/hoarfrost/widget/hyperlink_model.rs),
  [`search_box.rs`](../src/hoarfrost/widget/search_box.rs),
  [`progress_model.rs`](../src/hoarfrost/widget/progress_model.rs)

#### `widget::focus` and `widget::focusable` — focus rendering (`UIF-PRIN-005`)

##### Overview

`FocusIndicator` draws the ring for a focused control. `focusable()` wraps any element so the iced
focus system can reach it and so hoarfrost's focus model can track it.

##### Prerequisites & Requirements

- No cargo feature is required; both are always compiled.
- Every custom control MUST be wrapped in `focusable` or MUST implement the `Focusable` operation,
  otherwise Tab cannot reach it.
- The ring colour MUST be resolved against the control's background, not the window background.
  Use `resolve_ring_color_for_background`.

##### Syntax / Method Signature

```rust
pub enum FocusPlacement { Outer, Inset }
impl FocusPlacement {
    pub const fn stroke(self) -> f32;
    pub fn bounds(self, control: Rectangle, width: f32) -> Rectangle;
}
pub struct FocusIndicator { /* placement, colour, radius */ }
impl FocusIndicator {
    pub fn bounds(self, control: Rectangle) -> Rectangle;
    pub fn radius(self, radius: f32) -> f32;
}
pub fn resolve_ring_color(fill: Color, palette: &Palette) -> Color;
pub fn resolve_ring_color_for_background(background: Color, palette: &Palette) -> Color;
pub fn resolve_focus_indicator_for_background(
    background: Color,
    palette: &Palette,
    placement: FocusPlacement,
) -> FocusIndicator;
pub fn resolve_inset_outline_for_background(
    background: Color,
    palette: &Palette,
) -> FocusIndicator;

pub fn focusable<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
    id: FocusId,
    enabled: bool,
) -> Element<'a, Message, Theme, Renderer>;
pub struct Focusable<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> { /* private */ }
impl<'a, Message, Theme, Renderer> Focusable<'a, Message, Theme, Renderer> {
    pub fn new(
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
        id: FocusId,
        enabled: bool,
    ) -> Self;
}
```

##### Examples

```rust
let ring = resolve_focus_indicator_for_background(palette.surface_base, &palette, FocusPlacement::Outer);
let quad = ring.bounds(control_bounds);
```

##### Type references

| Type | Meaning |
| --- | --- |
| `FocusPlacement` | `Outer` grows the ring 2px outside the control. `Inset` draws 1px inside a row perimeter. |
| `FocusIndicator` | Ring placement, colour, and radius for one control. |
| `Focusable` | Wrapper element that registers a focus target with iced. |
| `FocusId` | String newtype naming a focus target. |

##### Return values

- `focusable` returns an `Element` directly; no conversion is needed.

##### Side effects

- None.

##### References

- [`focus.rs`](../src/hoarfrost/widget/focus.rs),
  [`focusable.rs`](../src/hoarfrost/widget/focusable.rs)

### Composite layer (`composite::*`)

Composites sit above the atoms. They either own a widget (a `Widget` implementation) or they are
**resolver-only**: pure geometry, style, and state machines that the host composes with iced
primitives.

| Module | Kind | Feature |
| --- | --- | --- |
| `card`, `icon`, `skeleton` | `skeleton` is a widget; `card` and `icon` are resolvers | `card`, `skeleton` |
| `expander`, `form_field` | resolver-only | `expander`, `form-field` |
| `list_tile`, `settings_row`, `tree_row` | resolver-only | `list-tile`, `settings-row`, `tree-row` |
| `scroll_view`, `virtual_list` | resolver-only | `scroll-view`, `virtual-list` |
| `table_cell`, `table_column`, `table_header`, `table_view` | resolver-only | `table` |
| `tree_view` | model plus footer renderer | `tree-view` |
| `tab_strip`, `navbar` | models and element helpers | `tab-strip`, `navbar` |
| `navigation_rail` | widget | `navigation-rail` |
| `breadcrumb`, `overflow_toolbar` | resolvers plus hosts | `breadcrumb`, `overflow-toolbar` |
| `notification`, `info_bar`, `toast` | models and resolvers | `notification`, `info-bar`, `toast` |
| `pagination`, `pips_pager` | widgets | `pagination`, `pips-pager` |
| `stepper` | widget plus footer element | `stepper` |
| `grid_view` | widget | `grid-view` |
| `side_sheet`, `split_divider` | widgets with resize gestures | `side-sheet`, `split-divider` |
| `code_block`, `diff_viewer`, `empty_state`, `command_palette`, `keybind_recorder` | widgets | `code-block`, `diff-viewer`, `empty-state`, `command-palette`, `keybind-recorder` |

> [!IMPORTANT]
> A resolver-only module has no `Widget` implementation. The README lists several of these as
> widgets. Treat the source as authoritative. See Appendix A.

#### `composite::card`, `composite::icon`, `composite::skeleton`

##### Overview

`card` resolves elevation, content insets, and the paint layer. `icon` is the icon wrapper type
other composites accept. `skeleton` paints a shimmer placeholder of a given size.

##### Prerequisites & Requirements

- Enable `card` or `skeleton`. `card` enables `button`.
- Resolve card style from a `CardElevation`; never hard-code a shadow.
- A skeleton MUST match the size of the content it replaces.

##### Syntax / Method Signature

```rust
// card.rs
pub enum CardElevation { /* resting, raised, overlay */ }
pub struct CardStyle { /* fill, border, shadow, inset */ }
pub fn resolve_style(elevation: CardElevation, palette: &Palette) -> CardStyle;
pub fn content_bounds(bounds: Rectangle, style: &CardStyle) -> Rectangle;
pub fn card_layer(bounds: Rectangle, style: &CardStyle) -> Layer;

// icon.rs
pub struct Icon { /* glyph or image source */ }

// skeleton.rs
pub struct Skeleton { /* width, height, phase */ }
pub struct SkeletonStyle { /* from, to colours */ }
pub fn skeleton(width: f32, height: f32) -> Skeleton;
pub fn resolve_style(skeleton: &Skeleton, palette: &Palette) -> SkeletonStyle;
pub fn interpolate_color(from: Color, to: Color, t: f32) -> Color;
```

##### Examples

```rust
let style = card::resolve_style(CardElevation::Raised, &palette);
let inner = card::content_bounds(card_rect, &style);
```

##### Type references

| Type | Meaning |
| --- | --- |
| `CardElevation` | The elevation rung a card sits at. |
| `CardStyle` | Resolved fill, border, shadow, and content inset. |
| `Layer` | A single paint primitive: bounds, fill, border, radius. |
| `SkeletonStyle` | The two endpoints of the shimmer animation. |

##### Return values

- Resolvers return owned styles, `Rectangle`, or `Layer`. They never mutate.

##### Side effects

- None.

##### References

- [`card.rs`](../src/hoarfrost/composite/card.rs),
  [`icon.rs`](../src/hoarfrost/composite/icon.rs),
  [`skeleton.rs`](../src/hoarfrost/composite/skeleton.rs)

#### `composite::expander` and `composite::form_field`

##### Overview

`expander` models a disclosure header plus body and enforces the accordion policy.
`form_field` frames a control with a label, an optional hint, and a helper-or-error caption.

##### Prerequisites & Requirements

- Enable `expander` or `form-field`. Both enable `button`.
- An accordion MUST enforce `AccordionPolicy`: opening one toggle closes the others.
- A form field MUST show the error caption instead of the helper caption when the value is invalid.
- The disclosure header is the only tab stop when the body is closed. Use `focus_members`.

##### Syntax / Method Signature

```rust
// expander.rs
pub enum AccordionPolicy { /* independent, exclusive */ }
pub enum HeaderRung { /* density rungs */ }
pub enum DisclosureKey { /* Enter, Space, ... */ }
pub struct DisclosureToggle { /* open, pressed */ }
impl DisclosureToggle {
    pub fn new(open: bool) -> Self;
    pub fn is_open(&self) -> bool;
    pub fn is_pressed(&self) -> bool;
    pub fn press(&mut self) -> GestureStart;
    pub fn release(&mut self) -> Option<Commit<bool>>;
    pub fn cancel(&mut self) -> Option<Cancel<bool>>;
    pub fn set_open(&mut self, open: bool) -> Commit<bool>;
}
pub struct HeaderStyle;
pub struct BodyStyle;
pub fn header_bounds(bounds: Rectangle) -> Rectangle;
pub fn header_bounds_at(bounds: Rectangle, rung: HeaderRung) -> Rectangle;
pub fn chevron_bounds(header: Rectangle) -> Rectangle;
pub fn title_bounds(header: Rectangle, trailing_width: f32) -> Rectangle;
pub fn body_bounds(bounds: Rectangle, open: bool, content_height: f32) -> Option<Rectangle>;
pub fn body_bounds_at(bounds: Rectangle, rung: HeaderRung, open: bool, content_height: f32) -> Option<Rectangle>;
pub fn body_content_bounds(body: Rectangle) -> Rectangle;
pub fn focus_members(header_id: impl Into<FocusId>, body_ids: &[FocusId], open: bool, disabled: bool) -> Vec<FocusMember>;
pub fn resolve_header_style(base: BaseState, palette: &Palette) -> HeaderStyle;
pub fn body_style(palette: &Palette) -> BodyStyle;
pub fn header_layer(bounds: Rectangle, style: &HeaderStyle) -> Layer;
pub fn body_layer(body: Rectangle, style: &BodyStyle) -> Layer;
pub fn apply_accordion_policy(policy: AccordionPolicy, toggles: &mut [DisclosureToggle], index: usize) -> Vec<Commit<bool>>;
pub fn apply_key(toggle: &mut DisclosureToggle, key: DisclosureKey) -> Option<Commit<bool>>;

// form_field.rs
pub struct FieldBands { /* header, control, caption bands */ }
pub struct HeaderParts<'a> { /* label, required marker, hint */ }
pub enum CaptionKind { Empty, Helper, Error }
pub struct CaptionParts<'a> { /* kind, text, colour */ }
pub enum OverflowPolicy { /* label overflow behaviour */ }
pub fn bands(bounds: Rectangle) -> FieldBands;
pub fn resolve_header<'a>(label: &'a str, required: bool, hint: Option<&'a str>, palette: &Palette) -> HeaderParts<'a>;
pub fn resolve_caption<'a>(valid: bool, helper: Option<&'a str>, error: Option<&'a str>, palette: &Palette) -> CaptionParts<'a>;
pub fn slot_marker(bounds: Rectangle, colour: Color) -> Layer;
pub fn stacked<'a, Message>(header: HeaderParts<'a>, control: iced::Element<'a, Message>, caption: CaptionParts<'a>) -> iced::Element<'a, Message>;
```

##### Examples

```rust
let header = form_field::resolve_header("Name", true, None, &palette);
let caption = form_field::resolve_caption(model.is_invalid(), Some("Required"), Some(error), &palette);
let field = form_field::stacked(header, input.into(), caption);
```

##### Type references

| Type | Meaning |
| --- | --- |
| `DisclosureToggle` | Press/release state machine for one disclosure. |
| `AccordionPolicy` | Whether opening one toggle closes the rest. |
| `FieldBands` | Vertical bands for header, control, and caption. |
| `CaptionKind` | `Empty`, `Helper`, or `Error`; error wins over helper. |

##### Return values

- `release` and `apply_key` return `None` when the gesture produced no change.
- `apply_accordion_policy` returns the commits for the toggles it closed.

##### Side effects

- `press`, `release`, `cancel`, `set_open`, and `apply_accordion_policy` mutate the toggles you pass.

##### References

- [`expander.rs`](../src/hoarfrost/composite/expander.rs),
  [`form_field.rs`](../src/hoarfrost/composite/form_field.rs)

#### `composite::list_tile`, `composite::settings_row`, `composite::tree_row`

##### Overview

Row resolvers for lists, settings pages, and tree rows. Each returns slot rectangles, a resolved
style, and paint layers. None of them is a widget.

##### Prerequisites & Requirements

- Enable `list-tile`, `settings-row`, or `tree-row`. All three enable `button`.
- A row's hit target MUST cover the whole row when the row is interactive. Use `hit_target`.
- A tree row's indent guides MUST be drawn from `guide_positions`; never hand-place them.
- A settings row's height is derived from its content: `row_height(has_description, control_height)`.

##### Syntax / Method Signature

```rust
// list_tile.rs
pub enum Leading { /* icon, avatar, none */ }
pub struct TileLines;
pub struct TileSlots { /* leading, text, trailing slots */ }
pub struct RowPadding;
pub struct TileStyle;
pub fn slots(bounds: Rectangle, leading: Leading, interactive: bool, trailing_width: f32) -> TileSlots;
pub fn slots_in(bounds: Rectangle, padding: &RowPadding, leading: Leading, interactive: bool, trailing_width: f32) -> TileSlots;
pub fn trailing_targets(bounds: Rectangle, count: usize) -> Vec<Rectangle>;
pub fn hit_target(bounds: Rectangle, interactive: bool) -> Option<Rectangle>;
pub fn resolve_style(selected: bool, disabled: bool, base: BaseState, palette: &Palette) -> TileStyle;
pub fn subtitle_on(fill: Color, palette: &Palette) -> Color;
pub fn focus_outline_layer(bounds: Rectangle, palette: &Palette) -> Layer;
pub fn row_layer(bounds: Rectangle, style: &TileStyle) -> Option<Layer>;

// settings_row.rs
pub struct SettingsBands;
pub struct SettingsRowStyle;
pub fn description_present(description: Option<&str>) -> bool;
pub fn text_column_height(has_description: bool) -> f32;
pub fn row_height(has_description: bool, control_height: f32) -> f32;
pub fn interior_floor() -> f32;
pub fn floor_slack(has_description: bool) -> f32;
pub fn row_padding() -> Padding;
pub fn bands(bounds: Rectangle, control_width: f32) -> SettingsBands;
pub fn resolve_style(disabled: bool, palette: &Palette) -> SettingsRowStyle;
pub fn card_layer(bounds: Rectangle, style: &SettingsRowStyle) -> Layer;
pub fn text_column<'a, Message>(title: &'a str, description: Option<&'a str>, style: &SettingsRowStyle) -> Column<'a, Message>;
pub fn horizontal<'a, Message>(title: &'a str, description: Option<&'a str>, control: iced::Element<'a, Message>, style: &SettingsRowStyle) -> iced::Element<'a, Message>;

// tree_row.rs
pub struct IndentGeometry;
pub struct TreeRowSlots;
pub struct TreeRowStyle;
pub fn indent_allowance(depth: usize) -> f32;
pub fn indent_geometry(depth: usize, row_width: f32) -> IndentGeometry;
pub fn guide_positions(levels: usize, step: f32) -> Vec<f32>;
pub fn row_padding(depth: usize) -> Padding;
pub fn row_slots(row: Rectangle, depth: usize, leading: Leading, interactive: bool, trailing_width: f32) -> TreeRowSlots;
pub fn content_width(row: Rectangle, depth: usize) -> f32;
pub fn resolve_style(selected: bool, disabled: bool, warning: bool, base: BaseState, surface: Color, palette: &Palette) -> TreeRowStyle;
pub fn row_layer(bounds: Rectangle, style: &TreeRowStyle) -> Option<Layer>;
pub fn selection_bar_layer(bounds: Rectangle, style: &TreeRowStyle) -> Option<Layer>;
pub fn guide_layers(row: Rectangle, geometry: &IndentGeometry) -> Vec<Layer>;
```

##### Examples

```rust
let style = tree_row::resolve_style(selected, disabled, warning, base, surface, &palette);
let slots = tree_row::row_slots(row_rect, depth, Leading::Icon, true, trailing);
```

##### Type references

| Type | Meaning |
| --- | --- |
| `Leading` | What occupies the leading slot: icon, avatar, thumbnail, or nothing. |
| `TileSlots` / `TreeRowSlots` | Resolved slot rectangles for one row. |
| `IndentGeometry` | Indent width and guide x positions for one depth. |
| `CellBand` | Row background role: rest, hover, selected, zebra. |

##### Return values

- `hit_target` returns `None` for a non-interactive row.
- `row_layer` returns `None` when the row paints nothing behind its content.

##### Side effects

- None. All three modules are pure.

##### References

- [`list_tile.rs`](../src/hoarfrost/composite/list_tile.rs),
  [`settings_row.rs`](../src/hoarfrost/composite/settings_row.rs),
  [`tree_row.rs`](../src/hoarfrost/composite/tree_row.rs)

#### `composite::scroll_view` and `composite::virtual_list`

##### Overview

`scroll_view` resolves track phases, thumb geometry, and offset maths. `virtual_list` computes the
realised row range for a fixed-height list. Both are pure.

##### Prerequisites & Requirements

- Enable `scroll-view` or `virtual-list`. `virtual-list` enables `scroll-view`.
- A virtual list MUST use a uniform row height. `RowRung` is a closed set on purpose; there is no
  per-row height API.
- The track MUST be hidden when the content fits. Use `resolve_phase`.
- Overscan defaults to `DEFAULT_OVERSCAN_ROWS` (3). Reduce it only with a measured reason.

##### Syntax / Method Signature

```rust
// scroll_view.rs
pub enum Axis { /* horizontal, vertical */ }
pub enum LayoutMode { /* overlay, inline */ }
pub enum TrackPhase { Hidden, Rest, Hovered, Dragged }
pub enum ScrollKey { Up, Down, PageUp, PageDown, Home, End }
pub enum TickKind { SearchHit, Heading, Error }
pub struct ScrollTick;
pub struct ThumbDrag;
pub fn is_scrollable(content_len: f32, viewport_len: f32) -> bool;
pub fn resolve_phase(scrollable: bool, pointer_over: bool, dragging: bool) -> TrackPhase;
pub fn track_width(phase: TrackPhase, mode: LayoutMode) -> f32;
pub fn content_bounds(viewport: Rectangle, mode: LayoutMode, axis: Axis) -> Rectangle;
pub fn track_bounds(viewport: Rectangle, phase: TrackPhase, mode: LayoutMode, axis: Axis) -> Rectangle;
pub fn max_offset(content_len: f32, viewport_len: f32) -> f32;
pub fn clamp_offset(offset: f32, content_len: f32, viewport_len: f32) -> f32;
pub fn thumb_length(content_len: f32, viewport_len: f32) -> f32;
pub fn thumb_offset(offset: f32, content_len: f32, viewport_len: f32, thumb_len: f32) -> f32;
pub fn thumb_rect(track: Rectangle, offset: f32, content_len: f32, viewport_len: f32, axis: Axis) -> Rectangle;
pub fn thumb_color(/* ... */) -> Color;
pub fn chrome_floor() -> f32;
pub fn at_bottom_within_2px(offset: f32, content_len: f32, viewport_len: f32) -> bool;
pub fn in_reach_end(offset: f32, content_len: f32, viewport_len: f32) -> bool;
pub fn page_step(viewport_len: f32) -> f32;
pub fn scroll_to_key(offset: f32, key: ScrollKey, content_len: f32, viewport_len: f32) -> Option<f32>;
pub fn tick_rect(track: Rectangle, tick: &ScrollTick, content_len: f32, axis: Axis) -> Rectangle;

// virtual_list.rs
pub const COMPACT_ROW_HEIGHT: f32 = 24.0;
pub const STANDARD_ROW_HEIGHT: f32 = 28.0;
pub const DEFAULT_OVERSCAN_ROWS: usize = 3;
pub enum RowRung { Compact, Standard }
impl RowRung { pub const fn height(self) -> f32; }
pub struct VirtualMetrics { /* total_rows, row_height, viewport_len, overscan */ }
impl VirtualMetrics {
    pub fn new(total_rows: usize, rung: RowRung, viewport_len: f32) -> Self;
    pub fn with_row_height(total_rows: usize, row_height: f32, viewport_len: f32, overscan: usize) -> Self;
    pub fn content_len(&self) -> f32;
    pub fn max_offset(&self) -> f32;
    pub fn is_scrollable(&self) -> bool;
    pub fn first_visible(&self, offset: f32) -> usize;
    pub fn visible_range(&self, offset: f32) -> Range<usize>;
    pub fn realized_range(&self, offset: f32) -> Range<usize>;
    pub fn realized_count(&self, offset: f32) -> usize;
    pub fn is_realized(&self, index: usize, offset: f32) -> bool;
    pub fn realized_count_bound(&self) -> usize;
    pub fn row_top(&self, index: usize) -> f32;
    pub fn row_bottom(&self, index: usize) -> f32;
    pub fn row_rect(&self, index: usize, viewport: Rectangle, offset: f32) -> Rectangle;
    pub fn row_at(&self, position: f32) -> Option<usize>;
    pub fn offset_to_reveal(&self, index: usize, current: f32) -> Option<f32>;
    pub fn thumb_length(&self) -> f32;
    pub fn thumb_offset(&self, offset: f32) -> f32;
    pub fn thumb_rect(&self, track: Rectangle, offset: f32, axis: scroll_view::Axis) -> Rectangle;
}
```

##### Examples

```rust
let metrics = VirtualMetrics::new(rows.len(), RowRung::Standard, viewport.height);
for index in metrics.realized_range(offset) {
    let rect = metrics.row_rect(index, viewport, offset);
}
```

##### Type references

| Type | Meaning |
| --- | --- |
| `TrackPhase` | `Hidden` / `Rest` / `Hovered` / `Dragged`; drives track width. |
| `LayoutMode` | Overlay track (floats over content) or inline track (takes space). |
| `RowRung` | The only two permitted row heights: 24 and 28. |
| `VirtualMetrics` | Row count plus viewport; the single source of realised-range truth. |
| `ScrollTick` | A named marker drawn on the track: search hit, heading, or error. |

##### Return values

- `scroll_to_key` returns `None` when the offset already equals the target.
- `offset_to_reveal` returns `None` when the row is already fully visible.

##### Side effects

- None.

##### References

- [`scroll_view.rs`](../src/hoarfrost/composite/scroll_view.rs),
  [`virtual_list.rs`](../src/hoarfrost/composite/virtual_list.rs)

#### `composite::table_*` — the table family (`UIF-COMP-007`)

##### Overview

Four resolver modules make a table: `table_column` resolves widths, `table_header` resolves header
hit targets and sort cycling, `table_cell` resolves band and grid-line colours, and `table_view`
owns selection, focus, and the resize gesture. There is no table widget; the host paints rows.

##### Prerequisites & Requirements

- Enable the cargo feature `table`. It enables `tree-view` and `virtual-list`.
- Declare columns with `ColumnSpec::fixed`, `::content`, or `::proportional`. Then call
  `resolve_widths(specs, available)` once per layout pass.
- Sort cycling MUST go ascending, then descending, then none. Use `cycle_sort`.
- Column resize MUST use the three tiers: `drag_resize` live, `release_resize` commit,
  `cancel_resize` cancel.
- Row geometry MUST come from `VirtualMetrics`; `TableView::metrics` wires this for you.

##### Syntax / Method Signature

```rust
// table_column.rs
pub type ColId = String;
pub enum CellAlign { Start, Center, End }
pub enum WidthPolicy { /* Fixed, Content, Proportional */ }
pub struct ColumnSpec { /* id, title, policy, min, max, align, sortable, resizable */ }
impl ColumnSpec {
    pub fn fixed(id: ColId, title: &'static str, width: f32) -> Self;
    pub fn content(id: ColId, title: &'static str, preferred: f32) -> Self;
    pub fn proportional(id: ColId, title: &'static str, weight: f32) -> Self;
    pub fn with_min(mut self, width: f32) -> Self;
    pub fn with_max(mut self, width: f32) -> Self;
    pub fn with_align(mut self, align: CellAlign) -> Self;
    pub fn with_sortable(mut self, sortable: bool) -> Self;
    pub fn with_resizable(mut self, resizable: bool) -> Self;
    pub fn bounds(&self) -> ColumnBounds;
}
pub struct ColumnBounds;
pub struct ColumnLayout { /* ids, widths, offsets */ }
impl ColumnLayout {
    pub fn empty() -> Self;
    pub fn ids(&self) -> &[ColId];
    pub fn widths(&self) -> &[f32];
    pub fn offsets(&self) -> &[f32];
    pub fn total_width(&self) -> f32;
    pub fn sum_minimums(&self) -> f32;
    pub fn available(&self) -> f32;
    pub fn slack(&self) -> f32;
    pub fn oversubscribed(&self) -> bool;
    pub fn needs_horizontal_scroll(&self) -> bool;
    pub fn width_at(&self, index: usize) -> f32;
    pub fn x_at(&self, index: usize) -> f32;
    pub fn index_of(&self, id: ColId) -> Option<usize>;
    pub fn width_of(&self, id: ColId) -> f32;
    pub fn divider_x(&self, index: usize) -> f32;
}
pub fn resolve_widths(specs: &[ColumnSpec], available: f32) -> ColumnLayout;

// table_header.rs
pub enum SortDirection { Ascending, Descending }
pub enum HeaderPress { /* Sort, Resize, None */ }
pub fn column_is_resizable(specs: &[ColumnSpec], index: usize) -> bool;
pub fn column_is_sortable(specs: &[ColumnSpec], index: usize) -> bool;
pub fn header_cell_rect(layout: &ColumnLayout, index: usize, header: Rectangle) -> Rectangle;
pub fn resize_handle_rect(layout: &ColumnLayout, index: usize, header: Rectangle) -> Rectangle;
pub fn sort_target_rect(specs: &[ColumnSpec], layout: &ColumnLayout, index: usize, header: Rectangle) -> Rectangle;
pub fn press_at(specs: &[ColumnSpec], layout: &ColumnLayout, header: Rectangle, cursor: Point) -> HeaderPress;
pub fn cycle_sort(current: Option<(ColId, SortDirection)>, column: ColId) -> Option<(ColId, SortDirection)>;

// table_cell.rs
pub enum CellBand { /* Rest, Hovered, Selected, Zebra, Active */ }
pub struct CellStyle;
impl CellBand { pub fn colour(self, palette: &Palette) -> Color; }
pub fn resolve(band: CellBand, palette: &Palette) -> CellStyle;

// table_view.rs
pub enum CellAction { Activate, Toggle }
pub enum TableKey { Up, Down, Left, Right, Home, End, PageUp, PageDown, Activate, ToggleSelect, SelectAll }
pub enum TableEvent<Id> {
    SelectionChanged, CellAction, SortChanged, ColumnResizeLive, ColumnResizeCommit, ColumnResizeCancel,
}
pub struct TableFocus<Id>;
pub struct TableContext<'a, Id>;
pub struct TableView<Id> { /* selection, focus, sort, resize */ }
impl<Id> TableView<Id> {
    pub fn new(mode: SelectionMode) -> Self;
    pub fn selected(&self) -> &BTreeSet<Id>;
    pub fn is_selected(&self, id: &Id) -> bool;
    pub fn anchor(&self) -> Option<&Id>;
    pub fn focus(&self) -> Option<&TableFocus<Id>>;
    pub fn is_resizing(&self) -> bool;
    pub fn select_single(&mut self, id: &Id) -> bool;
    pub fn toggle_selected(&mut self, id: &Id) -> bool;
    pub fn clear_selection(&mut self) -> bool;
    pub fn select_range(&mut self, rows: &[Id], from: usize, to: usize) -> bool;
    pub fn select_all(&mut self, rows: &[Id]) -> bool;
    pub fn resolve_anchor(&self, rows: &[Id]) -> Option<usize>;
    pub fn focus_cell(&mut self, rows: &[Id], column: ColId, row_index: usize) -> Option<TableFocus<Id>>;
    pub fn focus_header(&mut self, column: ColId) -> TableFocus<Id>;
    pub fn move_row(&mut self, rows: &[Id], delta: i32) -> Option<TableFocus<Id>>;
    pub fn move_column(&mut self, specs: &[ColumnSpec], delta: i32) -> Option<TableFocus<Id>>;
    pub fn focus_edge_column(&mut self, specs: &[ColumnSpec], first: bool) -> Option<TableFocus<Id>>;
    pub fn handle_key(&mut self, key: TableKey, mods: Modifiers, ctx: &TableContext<'_, Id>) -> Vec<TableEvent<Id>>;
    pub fn press_header(&mut self, specs: &[ColumnSpec], layout: &ColumnLayout, header: Rectangle, cursor: Point) -> Vec<TableEvent<Id>>;
    pub fn press_row(&mut self, rows: &[Id], column: ColId, row_index: usize, mods: Modifiers) -> Vec<TableEvent<Id>>;
    pub fn drag_resize(&mut self, specs: &[ColumnSpec], pointer_x: f32) -> Option<Live<f32>>;
    pub fn release_resize(&mut self) -> Option<Commit<f32>>;
    pub fn cancel_resize(&mut self) -> Option<Cancel<f32>>;
    pub fn header_rect(viewport: Rectangle) -> Rectangle;
    pub fn body_viewport(viewport: Rectangle) -> Rectangle;
    pub fn metrics(row_count: usize, body: Rectangle) -> VirtualMetrics;
    pub fn row_band(&self, id: &Id, hovered: bool, zebra: bool) -> CellBand;
}
pub struct ResizeGesture;
impl ResizeGesture {
    pub fn begin(&mut self, start_width: f32, start_x: f32, column: ColId) -> binding::GestureStart;
    pub fn live(&mut self, width: f32) -> Option<Live<f32>>;
    pub fn commit(&mut self) -> Option<Commit<f32>>;
    pub fn cancel(&mut self) -> Option<Cancel<f32>>;
    pub fn current(&self) -> f32;
    pub fn column(&self) -> Option<ColId>;
    pub fn is_active(&self) -> bool;
}
pub fn sorted_rows<Id: Clone>(
    rows: &[Id],
    sort: Option<(ColId, SortDirection)>,
    mut compare: impl FnMut(&Id, &Id, ColId) -> Ordering,
) -> Vec<Id>;
pub fn scrolled_x(base: Rectangle, offset_x: f32) -> Rectangle;
pub fn needs_horizontal_scroll(layout: &ColumnLayout, viewport: Rectangle) -> bool;
pub fn header_band() -> CellBand;
```

##### Examples

```rust
let layout = table_column::resolve_widths(&specs, viewport.width);
let events = table.press_header(&specs, &layout, header_rect, cursor);
```

##### Type references

| Type | Meaning |
| --- | --- |
| `ColumnSpec` | One column declaration: policy, bounds, alignment, sort and resize flags. |
| `ColumnLayout` | Resolved widths and offsets for one width budget. |
| `CellBand` | Which background role a row paints with. |
| `TableFocus<Id>` | Focus on a header cell or a body cell. |
| `TableContext<'_, Id>` | The rows, specs, and layout a key press is resolved against. |
| `Modifiers` | Keyboard modifier set shared by the table and tree. |

##### Return values

- `handle_key`, `press_header`, and `press_row` return `Vec<TableEvent<Id>>`; empty means no change.
- Selection methods return `true` only when the selection actually changed.

##### Side effects

- `TableView` mutates its own selection, focus, sort, and resize state. It never mutates `specs`.

##### References

- [`table_column.rs`](../src/hoarfrost/composite/table_column.rs),
  [`table_header.rs`](../src/hoarfrost/composite/table_header.rs),
  [`table_cell.rs`](../src/hoarfrost/composite/table_cell.rs),
  [`table_view.rs`](../src/hoarfrost/composite/table_view.rs)

#### `composite::tree_view` (`UIF-COMP-006`)

##### Overview

`TreeStructure` is a flat parent/child description. `TreeView` owns expansion, selection, focus, and
key handling over the visible rows. `visible_rows` flattens the structure for rendering.

##### Prerequisites & Requirements

- Enable the cargo feature `tree-view`. It enables `virtual-list`.
- Derive rows only from `visible_rows(&structure, &expanded)`. Never walk the tree yourself.
- Call `sync_to` after a structural change so stale selection and focus are dropped; inspect the
  returned `SyncReport`.
- A context menu on an unselected node MUST select it first. Use `context_menu(target, at, preserve)`.
- `Id` MUST implement `Clone + Ord + core::fmt::Debug`.

##### Syntax / Method Signature

```rust
pub enum SelectionMode { Single, ExtendedMulti }
pub enum TreeKey { Up, Down, Left, Right, Home, End, Activate, ToggleSelect, SelectAll }
pub struct Modifiers { /* ctrl, shift, alt */ }
pub enum TreeEvent<Id> { SelectionChanged, Activate, ExpandToggle, ContextMenu }
pub struct VisibleRow<Id> { /* id, depth, expanded, expandable */ }
pub struct SyncReport<Id>;

pub struct TreeStructure<Id> { /* nodes and parent map */ }
impl<Id> TreeStructure<Id> {
    pub fn new() -> Self;
    pub fn from_pairs(pairs: &[(Id, Option<Id>)]) -> Self;
    pub fn nodes(&self) -> &[Id];
    pub fn contains(&self, id: &Id) -> bool;
    pub fn parent_of(&self, id: &Id) -> Option<&Id>;
    pub fn children_of(&self, id: &Id) -> &[Id];
    pub fn is_expandable(&self, id: &Id) -> bool;
    pub fn roots(&self) -> Vec<&Id>;
    pub fn ancestors(&self, id: &Id) -> Vec<Id>;
    pub fn is_ancestor_of(&self, ancestor: &Id, id: &Id) -> bool;
}

pub struct TreeView<Id> { /* expanded, selected, anchor, focus */ }
impl<Id> TreeView<Id> {
    pub fn new(mode: SelectionMode) -> Self;
    pub fn expanded(&self) -> &BTreeSet<Id>;
    pub fn is_expanded(&self, id: &Id) -> bool;
    pub fn selected(&self) -> &BTreeSet<Id>;
    pub fn is_selected(&self, id: &Id) -> bool;
    pub fn anchor(&self) -> Option<&Id>;
    pub fn focused(&self) -> Option<&Id>;
    pub fn set_expanded(&mut self, id: &Id, open: bool) -> Option<Commit<bool>>;
    pub fn toggle_expanded(&mut self, id: &Id) -> Commit<bool>;
    pub fn select_single(&mut self, id: &Id) -> bool;
    pub fn toggle_selected(&mut self, id: &Id) -> bool;
    pub fn clear_selection(&mut self) -> bool;
    pub fn select_range(&mut self, rows: &[VisibleRow<Id>], from: usize, to: usize) -> bool;
    pub fn select_all(&mut self, rows: &[VisibleRow<Id>]) -> bool;
    pub fn focus_index(&mut self, rows: &[VisibleRow<Id>], index: usize) -> Option<Id>;
    pub fn move_focus(&mut self, rows: &[VisibleRow<Id>], delta: i32) -> Option<Id>;
    pub fn enter_focus(&mut self, rows: &[VisibleRow<Id>], delta: i32) -> Option<Id>;
    pub fn resolve_anchor(&self, structure: &TreeStructure<Id>, rows: &[VisibleRow<Id>]) -> Option<usize>;
    pub fn handle_key(&mut self, key: TreeKey, mods: Modifiers, structure: &TreeStructure<Id>, rows: &[VisibleRow<Id>]) -> Vec<TreeEvent<Id>>;
    pub fn context_menu(&mut self, target: &Id, at: Point, preserve: bool) -> Vec<TreeEvent<Id>>;
    pub fn scroll_into_view(&self, rows: &[VisibleRow<Id>], metrics: &VirtualMetrics, current: f32, drag_active: bool) -> Option<f32>;
    pub fn sync_to(&mut self, structure: &TreeStructure<Id>, rows: &[VisibleRow<Id>]) -> SyncReport<Id>;
    pub fn focus_members(&self, rows: &[VisibleRow<Id>], disabled: &BTreeSet<Id>) -> Vec<FocusMember>;
}

pub struct SearchFooterConfig;
impl SearchFooterConfig {
    pub fn new() -> Self;
    pub fn with_query(mut self, query: impl Into<String>) -> Self;
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self;
    pub fn with_active(mut self, active: bool) -> Self;
}

pub fn visible_rows<Id: Clone + Ord + core::fmt::Debug>(
    structure: &TreeStructure<Id>,
    expanded: &BTreeSet<Id>,
) -> Vec<VisibleRow<Id>>;
pub fn render_tree_with_footer<'a, Message: 'a + Clone>(
    tree_content: Element<'a, Message>,
    footer_input: Element<'a, Message>,
    palette: &Palette,
) -> Element<'a, Message>;
```

##### Examples

```rust
let rows = tree_view::visible_rows(&structure, &tree.expanded());
let events = tree.handle_key(TreeKey::Down, Modifiers::default(), &structure, &rows);
let scroll = tree.scroll_into_view(&rows, &metrics, offset, false);
```

##### Type references

| Type | Meaning |
| --- | --- |
| `TreeStructure<Id>` | Flat node list plus parent map. |
| `VisibleRow<Id>` | One flattened, visible row with its depth and disclosure flags. |
| `SyncReport<Id>` | What `sync_to` dropped: stale selection, stale focus. |
| `SearchFooterConfig` | Configuration for the optional in-tree search footer. |

##### Return values

- `set_expanded` returns `None` for a leaf. `toggle_expanded` always returns a `Commit<bool>`.
- `scroll_into_view` returns `None` when no scroll is needed or a drag is active.

##### Side effects

- `TreeView` mutates only itself. `render_tree_with_footer` builds elements only.

##### References

- [`tree_view.rs`](../src/hoarfrost/composite/tree_view.rs)

#### `composite::tab_strip`, `composite::navbar`, `composite::navigation_rail`

##### Overview

`tab_strip` models closable, dirty-tracked tabs and their context menu. `navbar` models a menu-bar
trigger. `navigation_rail` is a widget: a rail of destinations that expands and resizes.

##### Prerequisites & Requirements

- Enable `tab-strip`, `navbar`, or `navigation-rail`. `tab-strip` enables `menu` and `tooltip`;
  `navbar` enables `tooltip`; `navigation-rail` enables `badge`, `range-model`, `tooltip`.
- A navbar trigger that opens a dialog MUST end with a spaced ASCII ellipsis.
  `NavBarMenuTrigger::validate_ellipsis` returns the violation.
- Closing a dirty tab MUST go through `request_close`, which returns a verdict. Only `confirm_close`
  removes the tab.
- A rail destination label that is truncated MUST carry a tooltip.

##### Syntax / Method Signature

```rust
// tab_strip.rs
pub struct TabItem { /* id, title, dirty, tooltip, pinned */ }
impl TabItem {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self;
    pub fn with_dirty(mut self, dirty: bool) -> Self;
    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self;
    pub fn with_pinned(mut self, pinned: bool) -> Self;
}
pub enum TabCloseVerdict { /* Closed, Confirm */ }
pub struct TabStripModel;
impl TabStripModel {
    pub fn new() -> Self;
    pub fn open_tab(&mut self, tab: TabItem);
    pub fn select_tab(&mut self, id: &str);
    pub fn request_close(&mut self, id: &str) -> TabCloseVerdict;
    pub fn confirm_close(&mut self, id: &str) -> Option<String>;
    pub fn external_deletion_sync(&mut self, vanished_path: &str) -> Option<String>;
    pub fn request_close_all(&self) -> Vec<String>;
}
pub struct TabContextMenuModel;
impl TabContextMenuModel { pub fn new(tab_id: impl Into<String>) -> Self; pub fn build_menu(&self) -> Menu; }
pub struct TabItemStyle;
pub struct TabStrip<'a, Message>;
impl<'a, Message> TabStrip<'a, Message> {
    pub fn new(model: &'a TabStripModel) -> Self;
    pub fn with_palette(mut self, palette: Palette) -> Self;
    pub fn on_select_tab(mut self, f: impl Fn(String) -> Message + 'a) -> Self;
    pub fn on_close_tab(mut self, f: impl Fn(String) -> Message + 'a) -> Self;
    pub fn on_context_menu(mut self, f: impl Fn(String, iced::Point) -> Message + 'a) -> Self;
}
pub fn clamp_tab_width(width: f32) -> f32;
pub fn tab_context_menu(tab_id: &str) -> Menu;
pub fn resolve_tab_style(palette: &Palette, is_active: bool, is_hovered: bool, is_dirty: bool) -> TabItemStyle;

// navbar.rs
pub struct NavBarMenuTrigger;
impl NavBarMenuTrigger {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self;
    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self;
    pub fn with_open(mut self, is_open: bool) -> Self;
    pub fn with_disabled(mut self, disabled: bool) -> Self;
    pub fn with_prompt_behaviour(mut self, behaviour: PromptBehaviour) -> Self;
    pub fn with_dialog_prompt(mut self, launches: bool) -> Self;
    pub fn validate_ellipsis(&self) -> Result<(), InvariantViolation>;
}
pub struct NavBarModel;
impl NavBarModel {
    pub fn new() -> Self;
    pub fn with_brand(mut self, brand: impl Into<String>) -> Self;
    pub fn with_trigger(mut self, trigger: NavBarMenuTrigger) -> Self;
    pub fn toggle_menu(&mut self, id: &str);
    pub fn close_menu(&mut self);
}
pub struct TriggerStyle;
pub struct NavBar<'a, Message>;
impl<'a, Message: 'a + Clone> NavBar<'a, Message> {
    pub fn new(model: &'a NavBarModel) -> Self;
    pub fn with_palette(mut self, palette: Palette) -> Self;
    pub fn compact(mut self) -> Self;
    pub fn on_toggle_menu(mut self, f: impl Fn(String) -> Message + 'a) -> Self;
}
pub fn resolve_trigger_style(palette: &Palette, is_open: bool, is_hovered: bool, is_focused: bool) -> TriggerStyle;
pub fn navbar_trigger_element<'a, Message: 'a + Clone>(
    trigger: &NavBarMenuTrigger,
    palette: &Palette,
    on_toggle: Option<Message>,
) -> Element<'a, Message>;

// navigation_rail.rs
pub struct NavDestination<DestId>;
impl<DestId> NavDestination<DestId> {
    pub fn new(id: DestId, label: impl Into<String>, icon: impl Into<Icon>) -> Self;
    pub fn with_badge(mut self, badge: impl Into<Option<BadgeModel>>) -> Self;
    pub fn with_disabled(mut self, disabled: bool) -> Self;
}
pub struct NavigationRailModel<DestId>;
impl<DestId> NavigationRailModel<DestId> {
    pub fn new(top_items: Vec<NavDestination<DestId>>, bottom_items: Vec<NavDestination<DestId>>, selected: DestId) -> Self;
    pub fn with_expanded(mut self, expanded: bool) -> Self;
    pub fn with_expanded_width(mut self, width: f32) -> Self;
    pub fn clamp_expanded_width(w: f32) -> f32;
    pub fn current_width(&self) -> f32;
    pub fn all_items(&self) -> impl Iterator<Item = &NavDestination<DestId>>;
    pub fn enabled_destinations(&self) -> Vec<DestId>;
    pub fn rove(&self, key: RangeKey) -> Option<DestId>;
}
pub struct NavRailResize;
impl NavRailResize {
    pub fn new() -> Self;
    pub fn is_active(&self) -> bool;
    pub fn begin(&mut self, start_width: f32, start_x: f32) -> GestureStart;
    pub fn update(&mut self, current_x: f32) -> Option<Live<f32>>;
    pub fn commit(&mut self) -> Option<Commit<f32>>;
    pub fn cancel(&mut self) -> Option<Cancel<f32>>;
}
pub fn navigation_rail<'a, Message, DestId: 'a + Clone + PartialEq>(
    model: NavigationRailModel<DestId>,
) -> NavigationRail<'a, Message, DestId>;
```

##### Examples

```rust
let mut tabs = TabStripModel::new();
tabs.open_tab(TabItem::new("a.rs", "a.rs").with_dirty(true));
if let TabCloseVerdict::Confirm = tabs.request_close("a.rs") { ask_user(); }
```

##### Type references

| Type | Meaning |
| --- | --- |
| `TabCloseVerdict` | Whether a close may proceed or needs confirmation. |
| `PromptBehaviour` | `Immediate` or `LaunchesPrompt`; drives the ellipsis rule. |
| `NavRailResize` | Three-tier width gesture for the rail. |
| `NavigationRailState` | Internal hover, press, and focus state of the rail. |

##### Return values

- `confirm_close` and `external_deletion_sync` return the removed tab's title, or `None`.
- `rove` returns `None` when no destination is enabled.

##### Side effects

- Models mutate only themselves. No persistence.

##### References

- [`tab_strip.rs`](../src/hoarfrost/composite/tab_strip.rs),
  [`navbar.rs`](../src/hoarfrost/composite/navbar.rs),
  [`navigation_rail.rs`](../src/hoarfrost/composite/navigation_rail.rs)

#### `composite::breadcrumb` and `composite::overflow_toolbar` (`UIF-COMP-008`)

##### Overview

Both modules solve the same problem: items that do not fit collapse into a menu. `breadcrumb`
collapses the middle of a path. `overflow_toolbar` evicts the lowest-ranked commands. Each ships a
host that owns the overflow menu's open state.

##### Prerequisites & Requirements

- Enable `breadcrumb` or `overflow-toolbar`. Both enable `button` and `menu`.
- Measure each item before you call `collapse` or `fit`. The solvers take declared widths, not
  layout results.
- The overflow menu MUST be registered as a `SurfaceKind::MenuCard` while open.
- Escape MUST close the overflow menu before it unwinds anything below it.
- A collapsed breadcrumb MUST keep the root and the terminal crumb visible.

##### Syntax / Method Signature

```rust
// breadcrumb.rs
pub enum CrumbRole { Root, Middle, Terminal }
pub enum CrumbPlacement { Inline, Hidden }
pub struct CrumbPlan;
pub struct BreadcrumbLayout;
pub struct BreadcrumbItem<Id>;
pub struct BreadcrumbHost;
pub enum BreadcrumbOutcome<Id>;
pub enum BreadcrumbEmission<Id>;
pub fn collapse<Id>(items: &[BreadcrumbItem<Id>], available_width: f32) -> BreadcrumbLayout;
pub fn segment_rects(layout: &BreadcrumbLayout, x: f32, y: f32) -> Vec<(usize, Rectangle)>;
pub fn separator_rects(layout: &BreadcrumbLayout, x: f32, y: f32) -> Vec<Rectangle>;
pub fn trigger_rect(layout: &BreadcrumbLayout, x: f32, y: f32) -> Option<Rectangle>;
pub fn crumb_focus_id(index: usize) -> FocusId;
pub fn ellipsis_focus_id() -> FocusId;
pub fn crumb_member(index: usize) -> FocusMember;
pub fn ellipsis_member() -> FocusMember;
pub fn navigable_members(layout: &BreadcrumbLayout) -> Vec<FocusMember>;
pub fn overflow_menu<Id: fmt::Display>(items: &[BreadcrumbItem<Id>], layout: &BreadcrumbLayout) -> Menu;
impl BreadcrumbHost {
    pub fn new(trigger: FocusId) -> Self;
    pub fn is_menu_open(&self) -> bool;
    pub fn item_for_row(&self, row: usize) -> Option<usize>;
    pub fn activate_crumb<Id: Clone>(items: &[BreadcrumbItem<Id>], index: usize) -> BreadcrumbOutcome<Id>;
    pub fn open_overflow<Id: fmt::Display>(&mut self, items: &[BreadcrumbItem<Id>], layout: &BreadcrumbLayout, focus: &mut FocusState) -> BreadcrumbOutcome<Id>;
    pub fn close_overflow<Id>(&mut self, focus: &mut FocusState) -> BreadcrumbOutcome<Id>;
    pub fn menu_key<Id: Clone + fmt::Display>(&mut self, key: MenuKey, items: &[BreadcrumbItem<Id>], focus: &mut FocusState) -> BreadcrumbOutcome<Id>;
    pub fn escape<Id>(&mut self, focus: &mut FocusState) -> BreadcrumbOutcome<Id>;
}

// overflow_toolbar.rs
pub enum OverflowRank { /* eviction priority */ }
pub enum ItemPlacement { Inline, Overflow }
pub struct ItemPlan;
pub struct OverflowItem<Id>;
pub struct OverflowLayout;
pub struct OverflowToolbarHost;
pub enum ToolbarEmission<Id>;
pub struct ToolbarOutcome<Id>;
impl<Id> OverflowItem<Id> {
    pub fn new(id: Id, label: impl Into<String>, measure: f32, rank: OverflowRank) -> Self;
    pub fn id(&self) -> &Id;
    pub fn label(&self) -> &str;
    pub fn width(&self) -> f32;
}
impl OverflowLayout {
    pub fn plans(&self) -> &[ItemPlan];
    pub fn plan(&self, index: usize) -> Option<&ItemPlan>;
    pub fn inline_count(&self) -> usize;
    pub fn overflow_count(&self) -> usize;
    pub fn overflow_indices(&self) -> Vec<usize>;
    pub fn is_overflowing(&self) -> bool;
}
pub fn eviction_order<Id>(items: &[OverflowItem<Id>]) -> Vec<usize>;
pub fn fit<Id>(items: &[OverflowItem<Id>], available_width: f32) -> OverflowLayout;
pub fn item_rects(layout: &OverflowLayout, x: f32, strip_y: f32) -> Vec<(usize, Rectangle)>;
pub fn trigger_rect(layout: &OverflowLayout, x: f32, strip_y: f32) -> Option<Rectangle>;
pub fn overflow_menu<Id: fmt::Display>(items: &[OverflowItem<Id>], layout: &OverflowLayout) -> Menu;
impl OverflowToolbarHost {
    pub fn new(trigger: FocusId) -> Self;
    pub fn is_menu_open(&self) -> bool;
    pub fn trigger_state(&self, focused: bool) -> VisualState;
    pub fn item_for_row(&self, row: usize) -> Option<usize>;
    pub fn open_menu<Id: fmt::Display>(&mut self, items: &[OverflowItem<Id>], layout: &OverflowLayout, focus: &mut FocusState) -> ToolbarOutcome<Id>;
    pub fn close_menu<Id>(&mut self, focus: &mut FocusState) -> ToolbarOutcome<Id>;
    pub fn toggle_menu<Id: fmt::Display>(&mut self, items: &[OverflowItem<Id>], layout: &OverflowLayout, focus: &mut FocusState) -> ToolbarOutcome<Id>;
    pub fn menu_key<Id: Clone + fmt::Display>(&mut self, key: MenuKey, items: &[OverflowItem<Id>], focus: &mut FocusState) -> ToolbarOutcome<Id>;
    pub fn escape<Id: fmt::Display>(&mut self, focus: &mut FocusState) -> ToolbarOutcome<Id>;
    pub fn press_trigger(&mut self);
    pub fn release_trigger<Id: fmt::Display>(&mut self, items: &[OverflowItem<Id>], layout: &OverflowLayout, focus: &mut FocusState) -> ToolbarOutcome<Id>;
    pub fn reset(&mut self);
}
```

##### Examples

```rust
let layout = overflow_toolbar::fit(&items, strip_width);
let outcome = host.toggle_menu(&items, &layout, &mut focus);
```

##### Type references

| Type | Meaning |
| --- | --- |
| `BreadcrumbLayout` | Which crumbs stay inline, which are hidden, and whether the trigger shows. |
| `OverflowLayout` | Which items stay inline plus the eviction order actually applied. |
| `OverflowRank` | The order items are evicted in. Lower ranks leave first. |
| `MenuKey` | `Up` / `Down` / `Left` / `Right` / `Enter` / `Space` / `Escape`. |

##### Return values

- Host outcomes carry emissions plus focus transitions. `is_menu_open()` tells you whether the
  surface MUST be registered.

##### Side effects

- Hosts mutate their own open state and move focus through `FocusState`.

##### References

- [`breadcrumb.rs`](../src/hoarfrost/composite/breadcrumb.rs),
  [`overflow_toolbar.rs`](../src/hoarfrost/composite/overflow_toolbar.rs)

#### `composite::notification`, `composite::info_bar`, `composite::toast` (`UIF-COMP-002`)

##### Overview

`notification` is the severity-plus-message model. `info_bar` renders one notification inline, with
an optional action and dismiss button. `toast` keeps a timed stack of notifications anchored to a
screen corner.

##### Prerequisites & Requirements

- Enable `notification`, `info-bar`, or `toast`. `info-bar` enables `button` and `notification`;
  `toast` enables `info-bar`, `notification`, `tooltip`, `zorder`.
- Auto-dismiss MUST be paused while the toast is hovered or focused.
- A `Danger` notification MUST NOT auto-dismiss unless the host asks for it explicitly.
  `effective_auto_dismiss_ms` encodes the default.
- The toast stack MUST be registered as a `SurfaceKind::ToastStack` so Escape and pointer routing
  see it.
- A toast that would overlap a modal MUST be pushed below it. Use `anchor_bounds` with the modal's
  barrier top.

##### Syntax / Method Signature

```rust
// notification.rs
pub enum Severity { Info, Success, Warning, Danger }
pub struct NotificationId(String);
impl NotificationId { pub fn new(id: impl Into<String>) -> Self; pub fn as_str(&self) -> &str; }
pub struct NotificationAction<ActionId>;
impl<ActionId> NotificationAction<ActionId> { pub fn new(label: impl Into<String>, id: ActionId) -> Self; }
pub enum DismissReason { /* user, action, timeout, overflow, escape, host */ }
pub struct NotificationModel<ActionId>;
impl<ActionId> NotificationModel<ActionId> {
    pub fn new(id: impl Into<NotificationId>, severity: Severity, title: impl Into<String>, message: impl Into<String>) -> Self;
    pub fn with_action(mut self, label: impl Into<String>, id: ActionId) -> Self;
    pub fn with_dismissible(mut self, dismissible: bool) -> Self;
    pub fn with_auto_dismiss(mut self, ms: u32) -> Self;
    pub fn effective_auto_dismiss_ms(&self) -> Option<u32>;
}
pub fn effective_auto_dismiss_ms(severity: Severity, requested_ms: Option<u32>) -> Option<u32>;

// info_bar.rs
pub enum BarSurface { Raised, Overlay }
pub struct BarSlots;
pub struct InfoBarStyle;
pub enum BarPart { Action, Dismiss }
pub struct BarRelease;
pub struct BarDismissal;
impl BarDismissal {
    pub fn new(dismissed: bool) -> Self;
    pub fn is_dismissed(&self) -> bool;
    pub fn pressed_part(&self) -> Option<BarPart>;
    pub fn press(&mut self, part: BarPart) -> GestureStart;
    pub fn release(&mut self) -> Option<BarRelease>;
    pub fn cancel(&mut self) -> Option<Cancel<bool>>;
    pub fn set_dismissed(&mut self, dismissed: bool) -> Commit<bool>;
}
pub fn bar_bounds(container: Rectangle) -> Rectangle;
pub fn stripe_bounds(bar: Rectangle) -> Rectangle;
pub fn content_bounds(bar: Rectangle) -> Rectangle;
pub fn icon_bounds(bar: Rectangle) -> Rectangle;
pub fn text_bounds(bar: Rectangle, trailing_width: f32) -> Rectangle;
pub fn dismiss_bounds(bar: Rectangle) -> Rectangle;
pub fn action_bounds(bar: Rectangle, action_width: f32, dismiss_present: bool) -> Rectangle;
pub fn trailing_width(has_action: bool, action_width: f32, has_dismiss: bool) -> f32;
pub fn slots(container: Rectangle, action: Option<&NotificationAction<impl Sized>>, action_width: f32, has_dismiss: bool) -> BarSlots;
pub fn resolve_style(severity: Severity, surface: BarSurface, palette: &Palette) -> InfoBarStyle;
pub fn background_layer(bar: Rectangle, style: &InfoBarStyle) -> Layer;
pub fn stripe_layer(stripe: Rectangle, style: &InfoBarStyle) -> Layer;
pub fn focus_members(id: &NotificationId, has_action: bool, has_dismiss: bool) -> Vec<FocusMember>;
pub fn action_focus_id(id: &NotificationId) -> FocusId;
pub fn dismiss_focus_id(id: &NotificationId) -> FocusId;

// toast.rs
pub enum TimerState { /* armed, paused, expired */ }
pub struct ToastTimer;
impl ToastTimer {
    pub fn armed(total_ms: u64, now: TimeMs) -> Self;
    pub fn for_severity(severity: Severity, requested_ms: Option<u32>, now: TimeMs) -> Self;
    pub fn remaining_at(&self, now: TimeMs) -> Option<u64>;
    pub fn tick(&self, now: TimeMs) -> TimerState;
    pub fn pause(&mut self, now: TimeMs) -> bool;
    pub fn resume(&mut self, now: TimeMs) -> bool;
    pub fn advance(&mut self, now: TimeMs) -> bool;
}
pub struct ToastEntry<ActionId>;
pub struct PushOutcome<ActionId>;
pub struct ToastStack<ActionId>;
impl<ActionId> ToastStack<ActionId> {
    pub fn new() -> Self;
    pub fn visible(&self) -> &[ToastEntry<ActionId>];
    pub fn pending_count(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn push(&mut self, model: NotificationModel<ActionId>, now: TimeMs) -> PushOutcome<ActionId>;
    pub fn advance(&mut self, now: TimeMs) -> Vec<ToastEntry<ActionId>>;
    pub fn set_hovered(&mut self, id: &NotificationId, hovered: bool, now: TimeMs) -> bool;
    pub fn set_focused(&mut self, id: &NotificationId, focused: bool, now: TimeMs) -> bool;
    pub fn remove(&mut self, id: &NotificationId, now: TimeMs) -> Option<ToastEntry<ActionId>>;
    pub fn group_for(id: &NotificationId) -> GroupId;
    pub fn focus_members(model: &NotificationModel<ActionId>) -> Vec<FocusMember>;
    pub fn owned_focus_ids(&self) -> Vec<FocusId>;
    pub fn holds_focus(&self, focus: &FocusState) -> bool;
    pub fn focused_entry(&self, focus: &FocusState) -> Option<&ToastEntry<ActionId>>;
    pub fn unwind_answer(&mut self, focus: &FocusState, now: TimeMs) -> UnwindAnswer;
    pub fn may_take_focus(focus: &FocusState) -> bool;
}
pub fn toast_width(viewport_width: f32) -> f32;
pub fn anchor_bounds(viewport: Rectangle, barrier_top: f32, width: f32, height: f32) -> Rectangle;
pub fn stack_bounds(viewport: Rectangle, barrier_top: f32, height: f32, count: usize) -> Vec<Rectangle>;
```

##### Examples

```rust
let model = NotificationModel::new("saved", Severity::Success, "Saved", "3 files written")
    .with_action("Undo", Action::Undo);
let _outcome = stack.push(model, TimeMs::now());
let expired = stack.advance(TimeMs::now());
```

##### Type references

| Type | Meaning |
| --- | --- |
| `Severity` | `Info` / `Success` / `Warning` / `Danger`. |
| `ToastEntry<ActionId>` | One visible toast plus its timer. |
| `PushOutcome<ActionId>` | What a push did: added, replaced, or queued behind the cap. |
| `BarPart` | Which part of an info bar a gesture pressed. |
| `TimeMs` | Host-supplied millisecond clock. |

##### Return values

- `pause` and `resume` return `true` only when the timer state changed.
- `advance` returns the entries whose timer expired.

##### Side effects

- `ToastStack` mutates its own entries and timers. It never touches the registry; the host does.

##### References

- [`notification.rs`](../src/hoarfrost/composite/notification.rs),
  [`info_bar.rs`](../src/hoarfrost/composite/info_bar.rs),
  [`toast.rs`](../src/hoarfrost/composite/toast.rs)

#### `composite::pagination`, `composite::pips_pager`, `composite::stepper`

##### Overview

`pagination` is a numeric pager with a page-size picker. `pips_pager` is a dot pager. `stepper`
shows wizard progress and can render a wizard footer.

##### Prerequisites & Requirements

- Enable `pagination`, `pips-pager`, or `stepper`.
- Page numbers are 1-based in the UI and 0-based in the model. `PaginationModel` does the
  translation; do not offset by hand.
- A linear stepper MUST refuse forward jumps. `can_select` encodes the rule.
- Large counts MUST be formatted with `format_thousands` so digits stay grouped.

##### Syntax / Method Signature

```rust
// pagination.rs
pub struct PaginationModel;
impl PaginationModel {
    pub fn new(page: usize, page_size: usize, total_items: usize, page_size_options: Vec<usize>) -> Self;
    pub fn total_pages(&self) -> usize;
    pub fn start_item(&self) -> usize;
    pub fn end_item(&self) -> usize;
    pub fn can_first(&self) -> bool;
    pub fn can_prev(&self) -> bool;
    pub fn can_next(&self) -> bool;
    pub fn can_last(&self) -> bool;
    pub fn range_summary(&self) -> String;
    pub fn page_indicator(&self) -> String;
}
pub fn pagination_bar<'a, Message: 'a + Clone>(
    model: PaginationModel,
    on_page_change: impl Fn(Commit<usize>) -> Message + 'a,
) -> PaginationBar<'a, Message>;
pub fn format_thousands(n: usize) -> String;

// pips_pager.rs
pub struct PipsPagerModel;
pub fn pips_pager<'a, Message: 'a + Clone>(
    model: PipsPagerModel,
    on_page_change: impl Fn(Commit<usize>) -> Message + 'a,
) -> PipsPager<'a, Message>;

// stepper.rs
pub enum StepStatus { Pending, Current, Completed, Error }
pub enum StepperOrientation { Horizontal, Vertical }
pub struct StepItem<StepId>;
impl<StepId> StepItem<StepId> {
    pub fn new(id: StepId, label: impl Into<String>, status: StepStatus) -> Self;
    pub fn disabled(mut self, disabled: bool) -> Self;
}
pub struct StepperModel<StepId>;
impl<StepId> StepperModel<StepId> {
    pub fn new(steps: Vec<StepItem<StepId>>, current_step: StepId, linear: bool) -> Self;
    pub fn current_index(&self) -> Option<usize>;
    pub fn can_select(&self, target_id: &StepId) -> bool;
    pub fn can_previous(&self) -> bool;
    pub fn can_next(&self) -> bool;
    pub fn is_last_step(&self) -> bool;
    pub fn previous_step_id(&self) -> Option<StepId>;
    pub fn next_step_id(&self) -> Option<StepId>;
}
pub struct WizardFooterConfig<'a, Message, StepId>;
pub fn stepper<'a, Message: 'a + Clone, StepId: 'a + Clone + PartialEq>(
    model: StepperModel<StepId>,
) -> Stepper<'a, Message, StepId>;
pub fn wizard_footer<'a, Message: 'a + Clone, StepId: 'a + Clone + PartialEq>(
    model: &StepperModel<StepId>,
    config: WizardFooterConfig<'a, Message, StepId>,
) -> Element<'a, Message>;
```

##### Examples

```rust
let pager = PaginationModel::new(0, 25, 1_024, vec![25, 50, 100]);
pagination_bar(pager, |Commit { value }| Msg::Page(value));
```

##### Type references

| Type | Meaning |
| --- | --- |
| `PaginationModel` | Page, page size, total, and the allowed page sizes. |
| `StepStatus` | `Pending` / `Current` / `Completed` / `Error`. |
| `WizardFooterConfig` | Callback bundle for the generated Previous/Next/Finish/Cancel footer. |

##### Return values

- `total_pages` returns at least one page for a zero-item list.
- `range_summary` and `page_indicator` return display-ready strings.

##### Side effects

- None.

##### References

- [`pagination.rs`](../src/hoarfrost/composite/pagination.rs),
  [`pips_pager.rs`](../src/hoarfrost/composite/pips_pager.rs),
  [`stepper.rs`](../src/hoarfrost/composite/stepper.rs)

#### `composite::grid_view`, `composite::side_sheet`, `composite::split_divider`

##### Overview

`grid_view` lays out and selects tiles in a responsive column grid. `side_sheet` is a panel that
slides in from an edge and can be resized or made full-screen. `split_divider` is a draggable
divider between two panes.

##### Prerequisites & Requirements

- Enable `grid-view`, `side-sheet`, or `split-divider`. `grid-view` enables `tree-view`;
  `split-divider` enables `button`; `side-sheet` enables `zorder`.
- A side sheet MUST register its surface and MUST honour `z_band(viewport_width)`: a full-screen
  sheet ranks as a modal.
- A sheet with unsaved edits MUST refuse a light-dismiss close. `can_close()` encodes the rule.
- A split divider MUST clamp its ratio: `clamp_ratio(raw, min, max)`. Never let a pane reach zero.
- Resize gestures MUST emit `Live` while dragging and exactly one `Commit` or `Cancel` at the end.

##### Syntax / Method Signature

```rust
// grid_view.rs
pub enum GridNavKey { Left, Right, Up, Down, Home, End }
pub struct GridTile<Id>;
impl<Id> GridTile<Id> {
    pub fn new(id: Id, title: impl Into<String>) -> Self;
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self;
    pub fn preview_icon(mut self, icon: impl Into<Icon>) -> Self;
}
pub struct GridViewModel<Id>;
impl<Id> GridViewModel<Id> {
    pub fn new(items: Vec<GridTile<Id>>) -> Self;
    pub fn index_of(&self, id: &Id) -> Option<usize>;
    pub fn id_at(&self, index: usize) -> Option<Id>;
    pub fn step_nav(&self, current_index: usize, cols: usize, key: GridNavKey) -> usize;
    pub fn select_single(&mut self, id: Id) -> bool;
    pub fn select_range_to(&mut self, id: Id) -> bool;
    pub fn toggle_selection(&mut self, id: Id) -> bool;
    pub fn select_all(&mut self) -> bool;
    pub fn clear_selection(&mut self) -> bool;
}
pub fn solve_columns(available_width: f32, min_tile_width: f32, gap: f32, gutter: f32) -> (usize, f32);
pub fn tile_rect(origin: Point, index: usize, cols: usize, tile_width: f32, tile_height: f32, gap: f32) -> Rectangle;
pub fn grid_view<'a, Message, Id: Clone + Ord + 'a>(
    model: GridViewModel<Id>,
    available_width: f32,
) -> GridView<'a, Message, Id>;

// side_sheet.rs
pub enum SheetMode { /* inline, overlay, full screen */ }
pub enum SheetPlacement { /* leading, trailing */ }
pub struct SideSheetModel;
impl SideSheetModel {
    pub fn new(title: impl Into<String>) -> Self;
    pub fn with_mode(mut self, mode: SheetMode) -> Self;
    pub fn with_placement(mut self, placement: SheetPlacement) -> Self;
    pub fn with_width(mut self, width: f32) -> Self;
    pub fn with_open(mut self, open: bool) -> Self;
    pub fn with_dirty(mut self, dirty: bool) -> Self;
    pub fn with_busy(mut self, busy: bool) -> Self;
    pub fn clamp_width(w: f32) -> f32;
    pub fn effective_mode(&self, viewport_width: f32) -> SheetMode;
    pub fn can_close(&self) -> bool;
    pub fn z_band(&self, viewport_width: f32) -> ZBand;
    pub fn escape_rank(&self, viewport_width: f32) -> u8;
}
pub struct SideSheetResize;
impl SideSheetResize {
    pub fn new(placement: SheetPlacement) -> Self;
    pub fn is_active(&self) -> bool;
    pub fn begin(&mut self, start_width: f32, start_x: f32) -> GestureStart;
    pub fn update(&mut self, current_x: f32) -> Option<Live<f32>>;
    pub fn commit(&mut self) -> Option<Commit<f32>>;
    pub fn cancel(&mut self) -> Option<Cancel<f32>>;
}
pub fn side_sheet<
    'a,
    Message: 'a + Clone,
    Theme: 'a,
    Renderer: 'a + renderer::Renderer + text_core::Renderer<Font = iced::Font>,
>(
    model: SideSheetModel,
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> SideSheet<'a, Message, Theme, Renderer>;

// split_divider.rs
pub enum SplitAxis { /* horizontal, vertical */ }
pub struct SplitSnapshot { /* ratio at gesture start */ }
pub struct SplitDividerModel;
impl SplitDividerModel {
    pub fn new(axis: SplitAxis, ratio: f32) -> Self;
    pub fn with_min_ratio(mut self, min: f32) -> Self;
    pub fn with_max_ratio(mut self, max: f32) -> Self;
    pub fn with_disabled(mut self, disabled: bool) -> Self;
    pub fn clamp_ratio(raw: f32, min_ratio: f32, max_ratio: f32) -> f32;
    pub fn clamped_ratio(&self) -> f32;
}
pub struct SplitDividerState;
impl SplitDividerState {
    pub fn new(axis: SplitAxis, ratio: f32, min_ratio: f32, max_ratio: f32) -> Self;
    pub fn is_active(&self) -> bool;
    pub fn axis(&self) -> SplitAxis;
    pub fn start_ratio(&self) -> f32;
    pub fn current_ratio(&self) -> f32;
    pub fn gate(&self) -> &ActivationGate;
    pub fn capture(&self, id: impl Into<CaptureId>) -> DragCapture;
    pub fn begin(&mut self, ratio: f32, cursor_pos: f32, total_length: f32, min_ratio: f32, max_ratio: f32) -> GestureStart;
    pub fn update(&mut self, cursor_pos: f32) -> Option<Live<f32>>;
    pub fn commit(&mut self) -> Option<Commit<f32>>;
    pub fn cancel(&mut self) -> Option<Cancel<SplitSnapshot>>;
    pub fn abort(&mut self);
}
pub fn split_divider_geometry(bounds: Rectangle, axis: SplitAxis) -> (Rectangle, Rectangle);
pub fn split_divider_layers(bounds: Rectangle, axis: SplitAxis, visual: VisualState, palette: &Palette) -> Vec<(&'static str, Layer)>;
pub fn split_divider<'a, Message: 'a>(model: SplitDividerModel) -> SplitDivider<'a, Message>;
```

##### Examples

```rust
let (cols, tile_w) = grid_view::solve_columns(width, 200.0, 8.0, 16.0);
let sheet = side_sheet(SideSheetModel::new("Details").with_open(true), body)
    .on_resize_cancel(|Cancel { previous }| Msg::WidthRestored(previous));
```

##### Type references

| Type | Meaning |
| --- | --- |
| `SheetMode` | Inline, overlay, or full-screen sheet behaviour. |
| `SheetPlacement` | Which viewport edge the sheet attaches to. |
| `SplitSnapshot` | The ratio captured at gesture start; carried by `Cancel`. |
| `DragCapture` | Token that lets Escape cancel an in-flight drag before the surface unwinds. |
| `ActivationGate` | Shared gate that suppresses activation while a drag is open. |

##### Return values

- `solve_columns` returns `(columns, tile_width)`.
- `z_band` and `escape_rank` depend on the viewport width, so call them per layout pass.

##### Side effects

- Resize state machines mutate only themselves. The host writes the committed width or ratio back.

##### References

- [`grid_view.rs`](../src/hoarfrost/composite/grid_view.rs),
  [`side_sheet.rs`](../src/hoarfrost/composite/side_sheet.rs),
  [`split_divider.rs`](../src/hoarfrost/composite/split_divider.rs)

#### `composite::code_block`, `composite::diff_viewer`, `composite::empty_state`, `composite::command_palette`, `composite::keybind_recorder`

##### Overview

Five self-contained content widgets: a code viewer with a copy action, a diff viewer with per-hunk
accept and reject, an empty-state panel, a mode-driven command palette, and a keybind recorder.

##### Prerequisites & Requirements

- Enable `code-block`, `diff-viewer`, `empty-state`, `command-palette`, or `keybind-recorder`.
  `empty-state` enables `toast`; `command-palette` enables `text-input`.
- A command palette MUST set a mode. `set_query_from_input` infers it from the prefix; do not guess.
- A palette query is asynchronous. Ignore replies whose generation is not the one you emitted.
- A keybind recorder MUST report conflicts. Set `with_conflict` when the chord is already taken.
- A diff viewer MUST emit at most one accept or reject per hunk.

##### Syntax / Method Signature

```rust
// code_block.rs
pub struct CodeBlockModel;
impl CodeBlockModel {
    pub fn new(code: impl Into<String>) -> Self;
    pub fn with_language(mut self, language: impl Into<String>) -> Self;
    pub fn with_line_numbers(mut self, show: bool) -> Self;
    pub fn mark_copied(&mut self);
    pub fn tick_copied(&mut self, delta_ms: u32);
    pub fn is_copied(&self) -> bool;
    pub fn line_count(&self) -> usize;
}
pub fn code_block<'a, Message: 'a + Clone>(model: CodeBlockModel) -> CodeBlock<'a, Message>;

// diff_viewer.rs
pub enum DiffLineKind { Context, Added, Deleted }
pub enum DiffDisplayMode { Unified, SideBySide }
pub struct DiffLine;
impl DiffLine {
    pub fn context(old_line: u32, new_line: u32, text: impl Into<String>) -> Self;
    pub fn added(new_line: u32, text: impl Into<String>) -> Self;
    pub fn deleted(old_line: u32, text: impl Into<String>) -> Self;
}
pub struct DiffHunk<HunkId>;
impl<HunkId> DiffHunk<HunkId> {
    pub fn new(id: HunkId, header: impl Into<String>, lines: Vec<DiffLine>) -> Self;
}
pub struct DiffViewerModel<HunkId>;
impl<HunkId> DiffViewerModel<HunkId> {
    pub fn new(hunks: Vec<DiffHunk<HunkId>>) -> Self;
    pub fn with_mode(mut self, mode: DiffDisplayMode) -> Self;
}
pub fn diff_viewer<'a, Message: 'a + Clone, HunkId: 'a + Clone + PartialEq>(
    model: DiffViewerModel<HunkId>,
) -> DiffViewer<'a, Message, HunkId>;

// empty_state.rs
pub struct EmptyStateKind;
impl EmptyStateKind { pub fn is_error(&self) -> bool; }
pub struct EmptyStateModel<ActionId>;
impl<ActionId> EmptyStateModel<ActionId> {
    pub fn new(kind: EmptyStateKind, icon: impl Into<Icon>, title: impl Into<String>, description: impl Into<String>) -> Self;
    pub fn with_action(mut self, label: impl Into<String>, action: ActionId) -> Self;
}
pub fn empty_state<'a, Message: 'a + Clone, ActionId: Clone + 'a>(
    model: EmptyStateModel<ActionId>,
    on_action: impl Fn(Commit<ActionId>) -> Message + 'a,
) -> EmptyState<'a, Message, ActionId>;

// command_palette.rs
pub enum PaletteMode { Files, Commands, Symbols, Tags }
pub enum EscapeDisposition { ClearedQuery, Dismiss }
pub struct PaletteItem<ItemId>;
impl<ItemId> PaletteItem<ItemId> {
    pub fn new(id: ItemId, title: impl Into<String>) -> Self;
    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self;
    pub fn with_match_spans(mut self, match_spans: Vec<TextRange>) -> Self;
    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self;
}
pub struct PaletteGroup<ItemId>;
impl<ItemId> PaletteGroup<ItemId> {
    pub fn new(heading: impl Into<String>, items: Vec<PaletteItem<ItemId>>) -> Self;
}
pub struct CommandPaletteModel<ItemId>;
impl<ItemId> CommandPaletteModel<ItemId> {
    pub fn new(groups: Vec<PaletteGroup<ItemId>>) -> Self;
    pub fn with_query(mut self, q: impl Into<String>) -> Self;
    pub fn with_mode(mut self, m: PaletteMode) -> Self;
    pub fn with_open(mut self, open: bool) -> Self;
    pub fn with_busy(mut self, busy: bool) -> Self;
    pub fn with_highlighted_id(mut self, id: Option<ItemId>) -> Self;
    pub fn all_items(&self) -> Vec<&PaletteItem<ItemId>>;
    pub fn item_count(&self) -> usize;
    pub fn highlight_first(&mut self);
    pub fn highlight_last(&mut self);
    pub fn highlight_next(&mut self);
    pub fn highlight_prev(&mut self);
    pub fn selected_item(&self) -> Option<&PaletteItem<ItemId>>;
    pub fn handle_escape(&mut self) -> EscapeDisposition;
    pub fn set_query_from_input(&mut self, input: &str) -> Option<PaletteMode>;
    pub fn backspace_prefix(&mut self) -> Option<PaletteMode>;
    pub fn close(&mut self);
    pub fn open_palette(&mut self);
}
pub fn command_palette<'a, Message, ItemId: 'static + Clone + PartialEq, Theme, Renderer>(
    model: CommandPaletteModel<ItemId>,
) -> CommandPalette<'a, Message, ItemId, Theme, Renderer>;

// keybind_recorder.rs
pub struct KeyChord;
impl KeyChord {
    pub fn new(ctrl: bool, alt: bool, shift: bool, super_key: bool, key: impl Into<String>) -> Self;
    pub fn parts(&self) -> Vec<String>;
    pub fn format(&self) -> String;
}
pub struct KeybindRecorderModel;
impl KeybindRecorderModel {
    pub fn new() -> Self;
    pub fn with_chord(mut self, chord: Option<KeyChord>) -> Self;
    pub fn with_listening(mut self, listening: bool) -> Self;
    pub fn with_conflict(mut self, conflict: Option<String>) -> Self;
    pub fn with_disabled(mut self, disabled: bool) -> Self;
}
pub fn keybind_recorder<'a, Message: 'a + Clone>(model: KeybindRecorderModel) -> KeybindRecorder<'a, Message>;
```

##### Examples

```rust
let mut palette = CommandPaletteModel::new(groups).with_mode(PaletteMode::Commands);
palette.set_query_from_input(">save");
command_palette(palette)
    .on_execute(|Commit { value }| Msg::Run(value))
    .on_dismiss(|_| Msg::ClosePalette);
```

##### Type references

| Type | Meaning |
| --- | --- |
| `PaletteMode` | `Files` / `Commands` / `Symbols` / `Tags`; chosen from the query prefix. |
| `EscapeDisposition` | Whether Escape cleared the query or should dismiss the palette. |
| `DiffDisplayMode` | Unified or side-by-side rendering. |
| `KeyChord` | Modifiers plus one key; `format` renders the display string. |
| `EmptyStateKind` | The kind of empty state, including the error variant. |

##### Return values

- `set_query_from_input` and `backspace_prefix` return the mode when it changed.
- `handle_escape` tells the host whether to keep the palette open.

##### Side effects

- None beyond the model. The host copies to the clipboard and executes commands.

##### References

- [`code_block.rs`](../src/hoarfrost/composite/code_block.rs),
  [`diff_viewer.rs`](../src/hoarfrost/composite/diff_viewer.rs),
  [`empty_state.rs`](../src/hoarfrost/composite/empty_state.rs),
  [`command_palette.rs`](../src/hoarfrost/composite/command_palette.rs),
  [`keybind_recorder.rs`](../src/hoarfrost/composite/keybind_recorder.rs)

### Surface layer (`surface::*`)

A surface is a whole layered card, not a control. Each surface module splits a pure
model and lifecycle host from anything that paints. Only two surface types implement
`iced::Widget`: `surface::tooltip::Tooltip` and `surface::menu::view::MenuRow`. Every
other surface type is a host, a model, or a pure resolver. Your application draws the
card frame.

Enable one feature per surface family:

| Module | Cargo feature | Feature implies |
|---|---|---|
| `surface::zorder` | `zorder` | `menu` → `button`, `tooltip` |
| `surface::menu` | `menu` | `button`, `tooltip` |
| `surface::modal`, `surface::modal_tier` | `modal` | `zorder` |
| `surface::input_dialog`, `surface::dialog_button` | `dialog` | `button`, `zorder` |
| `surface::movable`, `surface::drag_tier` | `movable` | `zorder` |
| `surface::popover` | `popover` | `zorder` |
| `surface::tooltip` | `tooltip` | — |

**The host contract.** Every surface host follows the same five steps:

1. Build a `Spec` value that describes the card.
2. Call `open(...)`. Pass `&mut FocusState` and `&mut SurfaceRegistry`. The host
   registers the surface and takes the focus trap.
3. Route input through `on_key(KeyClass, ...)` and `on_pointer(...)`. Pass a
   `KeyClass`, never a raw key code.
4. Answer the escape walk through `unwind_answer(...)` or `escape_via(...)`.
5. Call `close(...)`. The host unregisters the surface and releases the trap.

Do not call `SurfaceRegistry::register` yourself for a surface that has a host. The
host does it, and a second registration duplicates the identity.

---

#### `surface::zorder` — z-bands, pointer routing, Escape unwinding

##### Overview

`surface::zorder` is the single authority for stacking order in a window
(`UIF-ZORD-001`, `UIF-WIN-002`, `UIF-WIN-003`). It answers three questions: which band
owns the keyboard, who receives a pointer press at a point, and which surface consumes
`Escape`. It holds no widget and paints nothing.

Four bands stack in a fixed order. A `SurfaceKind` maps each surface onto one band.
`Escape` unwinds by **escape rank**, which is not band order: the menu band unwinds
before the modal band.

##### Prerequisites & Requirements

- Feature `zorder`, which implies `menu`.
- Exactly one `SurfaceRegistry` per window, owned by your application.
- `SurfaceId` values are host-chosen strings. They MUST be unique inside one registry.
- A surface with no regions is never hit by the pointer. Register the card's real
  rectangles, in root coordinates.
- Hit testing is iced's own: the right and bottom edges of a region are exclusive.

##### Syntax / Method Signature

```rust
// Bands and band policy
pub enum ZBand { Workspace = 0, UtilityOverlay = 1, Menu = 2, Modal = 3 }
impl ZBand {
    pub const fn index(self) -> u8;
    pub const fn escape_rank(self) -> u8;
    pub const fn backdrop(self) -> Backdrop;
    pub const fn pointer_rule(self) -> PointerRule;
    pub const fn key_disposition(self, class: KeyClass) -> KeyDisposition;
    pub fn swallowed_keys(self) -> Vec<KeyClass>;
    pub const fn blocks_lower_bands(self) -> bool;
    pub const fn promotes_on_click(self) -> bool;
}
pub enum Backdrop { None, TransparentCatcher, DimmedScrim }
pub enum PointerRule { Direct, CaptureInsidePassOutside, CaptureInsideDismissOutside,
                       CaptureInsideAbsorbOutside }
pub enum KeyClass { Tab, ShiftTab, ArrowNavigation, Enter, Space, Escape }
pub enum KeyDisposition { Swallow, SwallowWhenFocused, PassThrough }

// Identity and registration
pub struct SurfaceId(String);
impl SurfaceId {
    pub fn new(id: impl Into<String>) -> Self;
    pub fn as_str(&self) -> &str;
}
impl From<&str> for SurfaceId;
impl From<String> for SurfaceId;

pub enum SurfaceKind { Workspace, SearchFilter, MovableCard, ToastStack, MenuCard,
                       TeachingPopover, Modal, InputDialog }
impl SurfaceKind { pub const fn band(self) -> ZBand; }

pub struct Surface;
impl Surface {
    pub fn new(id: SurfaceId, kind: SurfaceKind, regions: Vec<Rectangle>) -> Self;
    pub fn id(&self) -> &SurfaceId;
    pub fn band(&self) -> ZBand;
    pub fn regions(&self) -> &[Rectangle];
    pub fn has_regions(&self) -> bool;
    pub fn contains(&self, point: Point) -> bool;
}

pub struct SurfaceRegistry;
impl SurfaceRegistry {
    pub fn new() -> Self;
    pub fn register(&mut self, id: SurfaceId, kind: SurfaceKind,
                    regions: Vec<Rectangle>) -> &Surface;
    pub fn unregister(&mut self, id: &SurfaceId) -> Option<Surface>;
    pub fn clear(&mut self);
    pub fn is_empty(&self) -> bool;
    pub fn len(&self) -> usize;
    pub fn surfaces(&self) -> &[Surface];
    pub fn get(&self, id: &SurfaceId) -> Option<&Surface>;
    pub fn contains(&self, id: &SurfaceId) -> bool;
    pub fn surfaces_in(&self, band: ZBand) -> impl DoubleEndedIterator<Item = &Surface>;
    pub fn count_in(&self, band: ZBand) -> usize;
    pub fn band_is_empty(&self, band: ZBand) -> bool;
    pub fn topmost_in(&self, band: ZBand) -> Option<&Surface>;
    pub fn topmost(&self) -> Option<&Surface>;
    pub fn highest_open_band(&self) -> Option<ZBand>;
    pub fn keyboard_owner(&self) -> Option<ZBand>;
    pub fn promote(&mut self, id: &SurfaceId) -> bool;
    pub fn route_pointer(&self, point: Point) -> PointerRoute;
    pub fn pointer_down(&mut self, point: Point) -> PointerRoute;
    pub fn escape<F>(&mut self, unwind: F) -> EscapeOutcome
    where F: FnMut(&Surface) -> UnwindAnswer;
}

// Pointer routing answers
pub enum PointerTarget { Surface(SurfaceId), DismissBand(ZBand), Scrim(SurfaceId), Workspace }
impl PointerTarget {
    pub fn surface_id(&self) -> Option<&SurfaceId>;
    pub fn is_surface_hit(&self) -> bool;
}
pub struct PointerRoute;
impl PointerRoute {
    pub fn target(&self) -> &PointerTarget;
    pub fn passed_through(&self) -> &[ZBand];
    pub fn reaches_workspace(&self) -> bool;
    pub fn is_band_rule(&self) -> bool;
}

// Escape unwinding
pub enum UnwindAnswer { Unwound, Dismissed, Declined(DeclineReason) }
impl UnwindAnswer { pub fn from_cascade_close(outcome: &CascadeOutcome) -> Self; }
pub enum DeclineReason { NotOpen, NotFocused, CannotUnwind }
pub struct Declination { pub surface: SurfaceId, pub reason: DeclineReason }
pub enum EscapeRefusal { EmptyStack, AllDeclined { declined: Vec<Declination> } }
impl EscapeRefusal { pub fn is_empty_stack(&self) -> bool; }
pub enum EscapeOutcome {
    Consumed {
        surface: SurfaceId,
        kind: SurfaceKind,
        band: ZBand,
        dismissed: bool,
        declined_before: Vec<Declination>,
    },
    Ignored(EscapeRefusal),
}
impl EscapeOutcome {
    pub fn is_consumed(&self) -> bool;
    pub fn consumed_by(&self) -> Option<&SurfaceId>;
    pub fn dismissed(&self) -> bool;
    pub fn refusal(&self) -> Option<&EscapeRefusal>;
    pub fn declined_before(&self) -> &[Declination];
}

// Pre-Rank composition (UIF-WIN-003 step 1)
pub enum PreRankedEscape {
    DragCancelled { capture: DragCapture },
    RankWalked { outcome: EscapeOutcome, pre_rank: PreRankOutcome },
}
impl PreRankedEscape {
    pub fn drag_cancelled(&self) -> bool;
    pub fn outcome(&self) -> Option<&EscapeOutcome>;
    pub fn pre_rank(&self) -> PreRankOutcome;
}
pub fn escape_with_pre_rank<C, U>(
    captures: &mut PreRankCoordinator,
    registry: &mut SurfaceRegistry,
    cancel_capture: C,
    unwind: U,
) -> PreRankedEscape
where
    C: FnMut(&DragCapture) -> bool,
    U: FnMut(&Surface) -> UnwindAnswer;

// Scrim composition
pub const SCRIM_ALPHA: f32 = 0.55;
pub const fn scrim(base: Color) -> Color;
pub const fn scrim_for(palette: &Palette) -> Color;
```

Band policy, read from the code, not restated:

| Band | `index` | `escape_rank` | `backdrop()` | `pointer_rule()` | `swallowed_keys()` | blocks lower | promotes on click |
|---|---|---|---|---|---|---|---|
| `Workspace` | 0 | 0 | `None` | `Direct` | `Tab`, `ShiftTab` | no | no |
| `UtilityOverlay` | 1 | 1 | `None` | `CaptureInsidePassOutside` | *(empty)* | no | yes |
| `Menu` | 2 | **3** | `TransparentCatcher` | `CaptureInsideDismissOutside` | all six classes | no | no |
| `Modal` | 3 | **2** | `DimmedScrim` | `CaptureInsideAbsorbOutside` | `Tab`, `ShiftTab`, `Enter`, `Escape` | yes | no |

`SurfaceKind::band()` maps: `Workspace`/`SearchFilter` → Band 0; `MovableCard`/
`ToastStack` → Band 1; `MenuCard`/`TeachingPopover` → Band 2; `Modal`/`InputDialog` →
Band 3.

##### Examples

```rust
use hoarfrost::surface::zorder::{EscapeOutcome, SurfaceKind, SurfaceRegistry, UnwindAnswer};

let mut registry = SurfaceRegistry::new();
registry.register("filter".into(), SurfaceKind::SearchFilter, vec![card_rect]);
registry.register("menu".into(), SurfaceKind::MenuCard, vec![menu_rect]);

// Drive UIF-WIN-003 from your key handler.
match registry.escape(|surface| {
    if surface.kind == SurfaceKind::MenuCard {
        UnwindAnswer::from_cascade_close(&cascade.close_deepest(&mut focus))
    } else {
        UnwindAnswer::Declined(DeclineReason::CannotUnwind)
    }
}) {
    EscapeOutcome::Consumed { dismissed, .. } => { /* repaint */ }
    EscapeOutcome::Ignored(refusal) => { /* the key was a defined no-op */ }
}
```

A host with any drag-capable control MUST call `escape_with_pre_rank` instead. It runs
`UIF-WIN-003` step 1 first, then the rank walk.

```rust
use hoarfrost::surface::zorder::escape_with_pre_rank;

let answer = escape_with_pre_rank(
    &mut captures,
    &mut registry,
    |capture| overlay.cancel_drag(capture),   // true = a Cancel tier was produced
    |surface| overlay.unwind_answer(&mut focus, &mut storage),
);
if answer.drag_cancelled() {
    // The key is spent. No surface was asked.
}
```

##### Type references

- `ZBand` — one of the four stacking bands. Policy is a method per question, so no call
  site re-implements the table.
- `SurfaceKind` — what a surface is. Carries its band and lets an Escape report read as
  "the submenu" instead of an opaque id.
- `Surface` — one identity plus the rectangles it occupies. Regions are a list because
  one surface can own several rectangles.
- `SurfaceRegistry` — the ordered stack. Insertion order is z-order inside a band.
- `PointerRoute` — who would receive a press, plus which bands the event passed
  through.
- `UnwindAnswer` — a surface's reply to "can you step back one level?".
- `Declination` — a refusal with a reason, so a report can say who declined.
- `PreRankedEscape` — whether `Escape` went to a captured drag or to the rank walk.

##### Return values

- `register` returns `&Surface`. `unregister` returns the removed `Surface`, or `None`.
- `promote` returns whether the order actually changed. It returns `false` for an
  unknown id and for a surface already on top.
- `route_pointer` is pure. `pointer_down` routes and then promotes when the hit surface
  lives in a band whose `promotes_on_click` is set.
- `escape` returns `EscapeOutcome::Ignored(EscapeRefusal::EmptyStack)` on an empty
  registry and never calls `unwind`.
- `escape_with_pre_rank` returns `PreRankedEscape::outcome()` as `None` when the
  Pre-Rank consumed the key, because no surface was asked.

##### Side effects

- `register`, `unregister`, `clear`, `promote`, and `pointer_down` mutate the registry.
- `escape` calls your `unwind` closure once per surface until one accepts. A
  `Dismissed` answer means your closure MUST remove the surface from the registry.
- Nothing in this module reads a clock, a filesystem, or a window.

##### References

[`surface/zorder.rs`](../src/hoarfrost/surface/zorder.rs)

---

#### `surface::modal` and `surface::modal_tier` — modal cards

##### Overview

`surface::modal` is the Band 3 modal card: sizing policy, focus trap, error slot, and
dismiss rules (`UIF-MOD-*`). `surface::modal_tier` is the pending-action half of the
same card, so a busy primary action cannot be double-fired. The module computes geometry
and behaviour; it does not paint the card.

##### Prerequisites & Requirements

- Feature `modal`, which implies `zorder`.
- A `ModalSizing` policy per card, and a measured content `Size` from your renderer.
- The card's control roster as `Vec<FocusMember>`. The trap is confined to the derived
  group, `modal::group_for(&id)`.
- Observe the action's pending state every frame with `set_pending`.

##### Syntax / Method Signature

```rust
// Sizing policy (pure)
pub enum Desired { Fixed(f32), Viewport(f32), Content }
pub enum Cap { Pixels(f32), Viewport(f32), None }
impl Cap { pub fn resolve(self, available: f32) -> f32; }
pub struct AxisPolicy { pub desired: Desired, pub min: f32, pub max: Cap }
impl AxisPolicy { pub fn resolve(&self, available: f32, content: f32) -> f32; }
pub struct ModalSizing { pub width: AxisPolicy, pub height: AxisPolicy }
impl ModalSizing { pub fn solve(&self, viewport: Rectangle, content: Size) -> Rectangle; }

pub struct ResizeFloors { pub width: f32, pub height: f32 }
impl ResizeFloors { pub fn resolve(self, wanted: Size, viewport: Size) -> Size; }
pub struct SubRegionBands { pub header: f32, pub footer: f32, pub body_padding: f32 }
pub fn subregion_max_height(card_height: f32, bands: SubRegionBands) -> f32;
pub fn subregion_height(card_height: f32, bands: SubRegionBands, cap: f32) -> f32;

// Errors
pub enum ErrorLocation { Position { line: u32, column: u32 }, Region(String) }
pub enum Recovery { Retry, FocusControl(FocusId), Acknowledge }
pub struct ModalError;
impl ModalError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self;
    pub fn at(mut self, location: ErrorLocation) -> Self;
    pub fn with_recovery(mut self, recovery: Recovery) -> Self;
    pub fn code(&self) -> &str;
    pub fn message(&self) -> &str;
    pub fn location(&self) -> Option<&ErrorLocation>;
}

// Spec and host
pub struct ModalSpec;
impl ModalSpec {
    pub fn new(id: impl Into<SurfaceId>, title: impl Into<String>, sizing: ModalSizing) -> Self;
    pub fn with_dirty(mut self, dirty: bool) -> Self;
    pub fn with_error(mut self, error: ModalError) -> Self;
    pub fn with_controls(mut self, members: Vec<FocusMember>) -> Self;
    pub fn with_primary(mut self, primary: impl Into<FocusId>) -> Self;
    pub fn id(&self) -> &SurfaceId;
    pub fn title(&self) -> &str;
    pub fn error(&self) -> Option<&ModalError>;
    pub fn controls(&self) -> &[FocusMember];
    pub fn primary(&self) -> Option<&FocusId>;
}
pub fn group_for(id: &SurfaceId) -> GroupId;

pub struct ModalHost;
impl ModalHost {
    pub fn open(spec: ModalSpec, viewport: Rectangle, content: Size,
                focus: &mut FocusState, registry: &mut SurfaceRegistry) -> Self;
    pub fn id(&self) -> &SurfaceId;
    pub fn group_id(&self) -> &GroupId;
    pub fn title(&self) -> &str;
    pub fn is_inside_card(&self, at: Point) -> bool;
    pub fn hit_test(&self, at: Point) -> ModalHit;
    pub fn resize(&mut self, viewport: Rectangle, content: Size,
                  registry: &mut SurfaceRegistry);
    pub fn set_dirty(&mut self, dirty: bool) -> bool;
    pub fn error(&self) -> Option<&ModalError>;
    pub fn has_error(&self) -> bool;
    pub fn set_error(&mut self, error: ModalError) -> Option<ModalError>;
    pub fn clear_error(&mut self) -> Option<ModalError>;
    pub fn take_retry(&mut self) -> Option<ModalError>;
    pub fn set_pending(&mut self, pending: ActionPending);
    pub fn pending(&self) -> ActionPending;
    pub fn is_busy(&self) -> bool;
    pub fn try_activate_primary(&mut self) -> Activation;
    pub fn request_cancel(&mut self) -> Option<CancelRequest>;
    pub fn controlled(&self) -> Controlled<'_, Self>;
    pub fn ephemeral(&self) -> Ephemeral<&ModalAction>;
    pub fn sync_members(&mut self, members: Vec<FocusMember>,
                        focus: &mut FocusState) -> Option<FocusId>;
    pub fn on_key(&mut self, class: KeyClass, focus: &mut FocusState,
                  registry: &mut SurfaceRegistry) -> ModalOutcome;
    pub fn on_pointer(&mut self, at: Point, target: CardTarget, focus: &mut FocusState,
                       registry: &mut SurfaceRegistry) -> ModalOutcome;
    pub fn close(&mut self, focus: &mut FocusState,
                 registry: &mut SurfaceRegistry) -> ModalOutcome;
    pub fn unwind_answer(&mut self, focus: &mut FocusState) -> UnwindAnswer;
    pub fn escape_via(&mut self, registry: &mut SurfaceRegistry,
                      focus: &mut FocusState) -> EscapeOutcome;
}

// Action tier
pub struct ModalAction;
impl ModalAction {
    pub fn new() -> Self;
    pub fn observe(&mut self, pending: ActionPending);
    pub fn pending(&self) -> ActionPending;
    pub fn is_busy(&self) -> bool;
    pub fn tracks(&self) -> bool;
    pub fn try_activate(&mut self) -> Activation;
    pub fn request_cancel(&mut self) -> Option<CancelRequest>;
    pub fn blocks_dismiss(&self) -> bool;
    pub fn settle(&mut self) -> bool;
    pub fn is_pending(&self) -> bool;
    pub fn busy_observed(&self) -> bool;
    pub fn fired(&self) -> usize;
    pub fn suppressed(&self) -> usize;
    pub fn cancel_requested(&self) -> bool;
    pub fn reset(&mut self);
}
pub fn modal_action_state(action: &ModalAction) -> &'static str;

pub enum ModalHit { Card, Scrim }
pub enum CardTarget { Padding, Disabled, CloseButton, ErrorRecovery, Control(FocusId) }
pub enum DismissCause { Escape, Scrim, CloseButton, Programmatic }
pub enum ModalNoOp { NotOpen, KeyNotTrapped, KeyOwnedByControl, NothingFocusable,
                     KeyConsumedElsewhere, NotRegistered }
pub enum ModalOutcome {
    Dismissed { /* cause */ },
    DismissRefused { /* cause, dirty */ },
    Absorbed { /* reason */ },
    FocusMoved { /* .. */ },
    NoOp { /* reason */ },
}

// Paint resolution (a plain struct of token reads)
pub struct ModalFrame {
    pub surface: Color, pub border: Color, pub border_width: f32, pub radius: f32,
    pub scrim: Color, pub divider: Color, pub content_padding: f32,
    pub footer_padding_horizontal: f32, pub footer_padding_vertical: f32,
    pub close_target: f32,
}

// Geometry constants (A4.6.2)
pub const VIEWPORT_GUTTER: f32 = space::XXL;
pub const EDITOR_VIEWPORT_FRACTION: f32 = 0.8;
pub const EDITOR_WIDTH_MIN: f32 = 520.0;
pub const EDITOR_WIDTH_MAX: f32 = 960.0;
pub const EDITOR_HEIGHT_MIN: f32 = 380.0;
pub const EDITOR_HEIGHT_MAX: f32 = 760.0;
pub const TOOLS_WIDTH: f32 = 720.0;
pub const TOOLS_HEIGHT_MIN: f32 = 320.0;
pub const TOOLS_HEIGHT_MAX_FRACTION: f32 = 0.8;
pub const BATCH_WIDTH: f32 = 560.0;
pub const CONTENT_HEIGHT_MAX: f32 = 480.0;
pub const ABOUT_WIDTH: f32 = 440.0;
pub const ATTRIBUTIONS_WIDTH: f32 = 640.0;
pub const ATTRIBUTIONS_HEIGHT: f32 = 480.0;
pub const ATTRIBUTIONS_LIST_MAX_HEIGHT: f32 = 200.0;
```

##### Examples

```rust
use hoarfrost::surface::modal::{AxisPolicy, Cap, Desired, ModalHost, ModalSizing, ModalSpec};

let sizing = ModalSizing {
    width: AxisPolicy { desired: Desired::Fixed(720.0), min: 480.0, max: Cap::Viewport(0.8) },
    height: AxisPolicy { desired: Desired::Content, min: 240.0, max: Cap::Pixels(480.0) },
};
let spec = ModalSpec::new("about", "About", sizing)
    .with_controls(vec![member_close])
    .with_primary("close");
let mut host = ModalHost::open(spec, viewport, content, &mut focus, &mut registry);

// A dirty card refuses dismissal until the host confirms.
match host.on_key(KeyClass::Escape, &mut focus, &mut registry) {
    ModalOutcome::DismissRefused { .. } => ask_user_to_confirm(),
    ModalOutcome::Dismissed { .. } => { /* gone */ }
    _ => {}
}
```

##### Type references

- `ModalSizing` / `AxisPolicy` / `Desired` / `Cap` — the sizing policy per axis. Solve
  against the viewport and the measured content size.
- `ModalError` — code, message, optional location, optional recovery affordance.
- `Recovery` — what the error slot offers: `Retry`, focus one control, or acknowledge.
- `ModalAction` — the pending-action half. `blocks_dismiss` is what keeps a busy card
  from closing under the user.
- `CardTarget` — what your hit test resolved a press onto. Pass it back to `on_pointer`.
- `ModalFrame` — the resolved paint and box for the card, read from the palette.

##### Return values

- `open` registers the surface and takes the trap. It returns the host by value.
- `set_error` and `clear_error` return the previous error, if any.
- `take_retry` returns the error once and clears the retry request.
- `try_activate_primary` returns `Activation`, which tells you whether the activation
  was suppressed because the action was busy.
- `on_key` / `on_pointer` / `close` return `ModalOutcome`. Match all five variants.
- `unwind_answer` returns `UnwindAnswer::Declined(DeclineReason::NotFocused)` when the
  card does not hold focus, which is what lets a menu above it unwind first.

##### Side effects

- `open`, `resize`, `close`, `on_key`, `on_pointer`, and `sync_members` mutate the
  registry and `FocusState`.
- `set_pending` records the clock-free busy signal for this frame.
- No I/O, no clock reads, no thread use.

##### References

[`surface/modal.rs`](../src/hoarfrost/surface/modal.rs),
[`surface/modal_tier.rs`](../src/hoarfrost/surface/modal_tier.rs)

---

#### `surface::input_dialog` and `surface::dialog_button` — input dialogs

##### Overview

`surface::input_dialog` is the small Band 3 card that asks for one value: rename,
create, move, or confirm (`UIF-INP-*`). It owns the layout box model, the validation
rule, the selection rule, and the focus trap. `surface::dialog_button` is the footer
buttons' activation engine, so a busy `Ok` cannot fire twice.

##### Prerequisites & Requirements

- Feature `dialog`, which implies `button` and `zorder`.
- One `ValidationRule` per dialog. The rule is pure data; `check` returns a `Verdict`.
- The host measures the footer button widths and passes them to `InputLayout::solve`.
  Use `BUTTON_MIN_WIDTH` before the first frame.
- Pass `keyboard_invoked` to `open`. It selects the initial focus target.
- `UIF-INP-004`: a rename dialog selects the basename stem, not the extension.

##### Syntax / Method Signature

```rust
// Selection and validation (pure)
pub struct Selection { pub start: usize, pub end: usize }   // indices are chars(), not bytes
impl Selection {
    pub const EMPTY: Self;
    pub const fn len(&self) -> usize;
    pub fn slice<'a>(&self, value: &'a str) -> &'a str;
    pub fn all(value: &str) -> Self;
    pub fn rename_stem(value: &str) -> Self;
}
pub struct ValidationMessage;
impl ValidationMessage {
    pub fn new(code: impl Into<String>, text: impl Into<String>) -> Self;
}
pub enum Verdict { Accept, Reject(ValidationMessage) }
impl Verdict { pub fn message(&self) -> Option<&ValidationMessage>; }
pub enum ValidationRule {
    Name,
    Rename { /* current */ },
    Destination,
    Confirm { /* .. */ },
    Always,
}
impl ValidationRule { pub fn check(&self, value: &str) -> Verdict; }

// Box model (pure)
pub enum InputControl { TextEntry { /* .. */ }, FolderList { /* .. */ }, Confirmation { /* .. */ } }
impl InputControl {
    pub fn row_height(&self) -> f32;
    pub fn value_at(&self, selected: usize) -> String;
    pub fn folder_list_height(rows: usize) -> f32;
}
pub struct FooterButtons { pub cancel: f32, pub ok: f32 }
pub struct InputLayout {
    pub card: Rectangle, pub title: Rectangle, pub context: Rectangle,
    pub control: Rectangle, pub validation: Rectangle, pub footer: Rectangle,
    pub cancel: Rectangle, pub ok: Rectangle,
}
impl InputLayout {
    pub fn card_height(control: &InputControl) -> f32;
    pub fn card_size(control: &InputControl) -> Size;
    pub fn solve(card: Rectangle, control: &InputControl, buttons: FooterButtons) -> Self;
}
pub enum ButtonIntent { Standard, Danger }
pub enum DeleteTargets { Single(String), Many(usize) }
impl DeleteTargets { pub fn count(&self) -> usize; pub fn summary(&self) -> String; }

// Spec and host
pub struct InputDialogSpec;
impl InputDialogSpec {
    pub fn new(id: impl Into<SurfaceId>, title: impl Into<String>, context: impl Into<String>,
               control: InputControl, rule: ValidationRule) -> Self;
    pub fn rename(id: impl Into<SurfaceId>, container: impl Into<String>,
                  current: impl Into<String>) -> Self;
    pub fn create(id: impl Into<SurfaceId>, title: &str, container: impl Into<String>) -> Self;
    pub fn move_item(id: impl Into<SurfaceId>, name: impl Into<String>,
                     folders: Vec<String>) -> Self;
    pub fn delete_confirmation(id: impl Into<SurfaceId>, targets: DeleteTargets) -> Self;
    pub fn card_size(&self) -> Size;
    pub fn roster(&self) -> Vec<FocusMember>;
    pub fn primary_focus(&self) -> FocusId;
    pub fn control_id(&self, role: &str) -> FocusId;
    pub fn selected(&self) -> usize;
}
pub const ENTRY_ROLE: &str = "entry";
pub const LIST_ROLE: &str = "list";
pub const CANCEL_ROLE: &str = "cancel";
pub const OK_ROLE: &str = "ok";
pub fn group_for(id: &SurfaceId) -> GroupId;

pub struct InputDialogHost;
impl InputDialogHost {
    pub fn open(spec: InputDialogSpec, point: Point, keyboard_invoked: bool,
                viewport: Rectangle, focus: &mut FocusState,
                registry: &mut SurfaceRegistry) -> Self;
    pub fn id(&self) -> &SurfaceId;
    pub fn group_id(&self) -> &GroupId;
    pub fn control_id(&self, role: &str) -> FocusId;
    pub fn ok_id(&self) -> &FocusId;
    pub fn cancel_id(&self) -> &FocusId;
    pub fn primary_id(&self) -> &FocusId;
    pub fn is_inside_card(&self, at: Point) -> bool;
    pub fn value(&self) -> &str;
    pub fn verdict(&self) -> &Verdict;
    pub fn ok_is_enabled(&self) -> bool;
    pub fn revealed(&self) -> Option<&ValidationMessage>;
    pub fn selected(&self) -> usize;
    pub fn set_value(&mut self, value: String, focus: &mut FocusState) -> Verdict;
    pub fn set_selected(&mut self, index: usize, focus: &mut FocusState) -> usize;
    pub fn commit_answer(&mut self, focus: &mut FocusState,
                         registry: &mut SurfaceRegistry) -> Option<Commit<InputAnswer>>;
    pub fn cancel_answer(&mut self, focus: &mut FocusState,
                         registry: &mut SurfaceRegistry) -> Option<Cancel<InputAnswer>>;
    pub fn submit(&mut self, focus: &mut FocusState,
                  registry: &mut SurfaceRegistry) -> Option<InputAnswer>;
    pub fn cancel(&mut self, focus: &mut FocusState,
                  registry: &mut SurfaceRegistry) -> Option<InputAnswer>;
    pub fn set_pending(&mut self, pending: ActionPending);
    pub fn pending(&self) -> ActionPending;
    pub fn is_busy(&self) -> bool;
    pub fn request_cancel(&mut self) -> Option<CancelRequest>;
    pub fn ephemeral(&self) -> Ephemeral<&DialogButtonController>;
    pub fn on_key(&mut self, class: KeyClass, focus: &mut FocusState,
                  registry: &mut SurfaceRegistry) -> InputOutcome;
    pub fn on_pointer(&mut self, at: Point, target: InputTarget, focus: &mut FocusState,
                       registry: &mut SurfaceRegistry) -> InputOutcome;
    pub fn close(&mut self, focus: &mut FocusState,
                 registry: &mut SurfaceRegistry) -> InputOutcome;
    pub fn unwind_answer(&mut self, focus: &mut FocusState) -> UnwindAnswer;
    pub fn escape_via(&mut self, registry: &mut SurfaceRegistry,
                      focus: &mut FocusState) -> EscapeOutcome;
}

pub enum InputTarget { Padding, Disabled, Entry, FolderRow(usize), Cancel, Ok }
pub enum InputNoOp { NotOpen, KeyNotTrapped, KeyOwnedByControl, NothingFocusable,
                     KeyConsumedElsewhere, NotRegistered }
pub enum DismissCause { Escape, CancelButton, Scrim, Programmatic }
pub enum InputAnswer { Accepted { /* value */ }, Cancelled }
pub enum InputOutcome {
    Accepted { /* answer */ },
    Cancelled { /* cause */ },
    ErrorRevealed { /* message */ },
    Absorbed { /* reason */ },
    FocusMoved { /* .. */ },
    NoOp { /* reason */ },
}
pub const fn card_border() -> f32;

// Footer button engine
pub enum DialogButtonRole { Primary, Secondary }
pub struct DialogButtonSettlement {
    pub role: DialogButtonRole,
    pub source: ActivationSource,
    pub pending: ActionPending,
    pub dispatched: bool,
}
impl DialogButtonSettlement {
    pub fn undispatched(role: DialogButtonRole, source: ActivationSource,
                        pending: ActionPending) -> Self;
}
pub enum DialogButtonEmission {
    Committed(Commit<DialogButtonSettlement>),
    Cancelled(Cancel<DialogButtonSettlement>),
    CancelRequested(CancelRequest),
}
impl DialogButtonEmission { pub fn settlement(&self) -> Option<&DialogButtonSettlement>; }
pub struct DialogButtonController;
impl DialogButtonController {
    pub fn new(role: DialogButtonRole) -> Self;
    pub fn observe(&mut self, pending: ActionPending);
    pub fn pending(&self) -> ActionPending;
    pub fn tracks(&self) -> bool;
    pub fn begin(&mut self, source: ActivationSource) -> Option<DialogButtonEmission>;
    pub fn try_dispatch(&mut self) -> bool;
    pub fn commit(&mut self, source: ActivationSource) -> Option<DialogButtonEmission>;
    pub fn cancel(&mut self) -> Option<DialogButtonEmission>;
    pub fn take_cancel_request(&mut self) -> Option<CancelRequest>;
    pub fn gesture_open(&self) -> bool;
    pub fn last_begin(&self) -> Option<GestureStart>;
    pub fn is_pending(&self) -> bool;
    pub fn busy_observed(&self) -> bool;
    pub fn fired(&self) -> usize;
    pub fn suppressed(&self) -> usize;
    pub fn cancel_requested(&self) -> bool;
    pub fn reset(&mut self);
}

// Geometry constants (A4.5.1)
pub const CARD_WIDTH: f32 = 320.0;
pub const CARD_PADDING: f32 = space::LG;
pub const ROW_GAP: f32 = space::SM;
pub const TITLE_ROW_HEIGHT: f32 = typography::TITLE_SM.line_height;
pub const CONTEXT_ROW_HEIGHT: f32 = typography::BODY.line_height;
pub const TEXT_ENTRY_HEIGHT: f32 = size::CONTROL_MD;
pub const FOLDER_ROW_HEIGHT: f32 = size::CONTROL_SM;
pub const FOLDER_LIST_MAX_HEIGHT: f32 = 200.0;
pub const VALIDATION_SLOT_HEIGHT: f32 = typography::BODY.line_height;
pub const FOOTER_HEIGHT: f32 = size::CONTROL_MD;
pub const BUTTON_MIN_WIDTH: f32 = 64.0;
pub const CONFIRMATION_CONTROL_HEIGHT: f32 = 0.0;
```

##### Examples

```rust
use hoarfrost::surface::input_dialog::{InputDialogHost, InputDialogSpec, InputOutcome};

let spec = InputDialogSpec::rename("rename-1", "/notes", "draft.md");
let mut host = InputDialogHost::open(spec, point, false, viewport, &mut focus, &mut registry);
// UIF-INP-004: the stem is selected, so typing replaces the name and keeps ".md".

match host.on_key(KeyClass::Enter, &mut focus, &mut registry) {
    InputOutcome::Accepted { .. } => apply_rename(host.value()),
    InputOutcome::ErrorRevealed { .. } => {}   // the reserved slot now shows the message
    _ => {}
}
```

##### Type references

- `ValidationRule` — the closed set of dialog validators. `check` is pure.
- `Verdict` — `Accept` or `Reject(ValidationMessage)` with a stable code.
- `Selection` — a char-index range. `rename_stem` implements `UIF-INP-004`.
- `InputLayout` — eight rectangles, one per row and one per footer button. The
  validation slot is permanently reserved, so the card never shifts.
- `InputAnswer` — what the user finally gave you: an accepted value, or nothing.
- `DialogButtonController` — one footer button's gesture, pending state, and suppression
  counters.

##### Return values

- `set_value` returns the new `Verdict`. It does not close the dialog.
- `ok_is_enabled` is `false` while the current value is rejected. Do not disable `Ok`
  yourself; read this.
- `submit` and `commit_answer` return `None` when the value is rejected.
- `open` selects the initial focus target from `keyboard_invoked`.

##### Side effects

- `open`, `close`, `on_key`, `on_pointer`, `set_value`, and `set_selected` mutate the
  registry and `FocusState`.
- Validation codes are stable strings: `INP-1001` empty, `INP-1002` edge whitespace,
  `INP-1003` control characters, `INP-1004` illegal basename characters, `INP-1005`
  unchanged name, `INP-1006` nothing to confirm, `INP-1007` no destination.
- The illegal-character set is aliased to
  `validation::disallowed::FILENAME_ILLEGAL_CHARS`, so the dialog and the instant-punish
  class cannot disagree.

##### References

[`surface/input_dialog.rs`](../src/hoarfrost/surface/input_dialog.rs),
[`surface/dialog_button.rs`](../src/hoarfrost/surface/dialog_button.rs)

---

#### `surface::movable` and `surface::drag_tier` — movable overlay cards

##### Overview

`surface::movable` is the Band 1 detachable card: drag, resize, clamp, geometry
persistence, focus trap, and follow-on-append scrolling (`UIF-MOV-001..004`).
`surface::drag_tier` records one drag as a tier transcript, so a live drag emits `Live`
frames and settles into exactly one `Commit` or `Cancel`. The module computes geometry
and behaviour; it does not paint the card.

##### Prerequisites & Requirements

- Feature `movable`, which implies `zorder`.
- A `GeometryStorage` implementation for persistence. Use `NoopGeometryStorage` in tests
  and `MemoryGeometryStorage` in-memory. Tests MUST NOT touch real user paths.
- A viewport `Rectangle` in root coordinates on every call that can clamp.
- Register the card's regions so the pointer router can see it. The host does this in
  `open`.
- Band 1 passes an outside press through. Read `MovableResponse::passed_through`.
- The title bar's trailing action controls are a drag exclusion zone. Use
  `is_draggable_at`, never "any point on the title bar".

##### Syntax / Method Signature

```rust
// The serialisable geometry value
pub struct MovableGeometry { pub position: Point, pub size: Size }
impl MovableGeometry {
    pub const DEFAULT: Self;                       // 680 x 440 at the origin
    pub const fn new(position: Point, size: Size) -> Self;
    pub fn sanitized(self) -> Self;
    pub fn encode(self) -> String;                 // lossless canonical text
    pub fn decode(text: &str) -> Option<Self>;
}

// Persistence seam
pub trait GeometryStorage {
    fn load(&mut self, key: &str) -> Option<MovableGeometry>;
    fn save(&mut self, key: &str, geometry: MovableGeometry);
    fn clear(&mut self, key: &str);
}
pub struct NoopGeometryStorage;                    // stores nothing
pub struct MemoryGeometryStorage;
impl MemoryGeometryStorage {
    pub fn new() -> Self;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn keys(&self) -> impl Iterator<Item = &str>;
}
pub fn storage_key(id: &SurfaceId) -> String;
pub fn group_for(id: &SurfaceId) -> GroupId;

// Scrolling and auto-scroll (pure)
pub struct ScrollGeometry { pub offset: f32, pub content: f32, pub viewport: f32 }
impl ScrollGeometry {
    pub fn max_offset(self) -> f32;
    pub fn is_empty(self) -> bool;
    pub fn at_bottom(self) -> bool;
    pub fn clamped(self) -> Self;
    pub fn scrolled_by(self, delta: f32) -> Self;
    pub fn at_bottom_now(self) -> Self;
}
pub enum AutoScrollSetting { Enabled, Disabled }
pub enum FollowState { Following, Paused }
pub enum HoldReason { Disabled, Inspecting }
pub enum AutoScrollDecision {
    Follow { entries: usize },
    Hold { entries: usize, reason: HoldReason, behind: usize },
}
pub const AT_BOTTOM_TOLERANCE: f32 = space::XXS;

// Input vocabulary
pub enum PointerPhase { Pressed, Moved, Released }
pub enum SurfaceTarget { Outside, TitleBar, TitleGrip, Title, Summary, Dock, Close,
                         FilterBar, AutoScrollToggle, Control(FocusId), Content,
                         Footer, Resize(ResizeHandle) }
pub enum Zone { Outside, TitleDrag, TitleActions, FilterBar, Content, Footer,
                Grip(ResizeHandle) }
pub enum CardControl { Dock, AutoScrollToggle, Custom(FocusId) }
pub enum MovableTask { ScrollToBottom, SetCursor(mouse::Interaction), Activate(FocusId) }
pub enum MovableNoOp { NotOpen, NotRegistered, KeyNotTrapped, NotFocused, AlreadyCapturing,
                       GestureCancelled, AlreadyOpen, Idle, OwnedByOtherSurface }
pub enum MovableAction {
    NoOp(MovableNoOp),
    DragStarted { geometry: MovableGeometry },
    Dragged { geometry: MovableGeometry, delta: Vector },
    DragEnded { geometry: MovableGeometry },
    ResizeStarted { handle: ResizeHandle },
    Resized { handle: ResizeHandle, geometry: MovableGeometry, delta: Vector },
    ResizeEnded { handle: ResizeHandle, geometry: MovableGeometry },
    PassedThrough { route: PointerRoute },
    Activated { control: CardControl },
    Absorbed { zone: Zone },
    Scrolled { follow: FollowState },
    AutoScrollChanged { setting: AutoScrollSetting, follow: FollowState },
    Closed { geometry: MovableGeometry },
    Reclamped { geometry: MovableGeometry, adjusted: bool },
}

pub struct MovableResponse {
    pub action: MovableAction,
    pub promoted: bool,
    pub passed_through: bool,
    pub tasks: Vec<MovableTask>,
}
impl MovableResponse {
    pub fn new(action: MovableAction) -> Self;
    pub fn no_op(reason: MovableNoOp) -> Self;
    pub fn with_task(mut self, task: MovableTask) -> Self;
    pub fn raised(mut self, promoted: bool) -> Self;
    pub fn through(mut self, passed_through: bool) -> Self;
    pub fn stays_open(&self) -> bool;
    pub fn is_drag(&self) -> bool;
    pub fn is_resize(&self) -> bool;
}

// Spec and host
pub struct MovableSpec;
impl MovableSpec {
    pub fn new(id: impl Into<SurfaceId>, title: impl Into<String>) -> Self;
    pub fn with_geometry(mut self, geometry: MovableGeometry) -> Self;
    pub fn with_auto_scroll(mut self, setting: AutoScrollSetting) -> Self;
    pub fn with_controls(mut self, controls: Vec<FocusMember>) -> Self;
    pub fn id(&self) -> &SurfaceId;
    pub fn title(&self) -> &str;
    pub fn controls(&self) -> &[FocusMember];
}

pub struct MovableHost;
impl MovableHost {
    pub fn open(spec: MovableSpec, viewport: Rectangle, focus: &mut FocusState,
                registry: &mut SurfaceRegistry,
                storage: &mut dyn GeometryStorage) -> Self;
    pub fn id(&self) -> &SurfaceId;
    pub fn title(&self) -> &str;
    pub fn storage_key(&self) -> String;
    pub fn group_id(&self) -> &GroupId;
    pub fn card(&self) -> Rectangle;
    pub fn title_bar(&self) -> Rectangle;
    pub fn trailing_actions(&self) -> Rectangle;
    pub fn filter_bar(&self) -> Rectangle;
    pub fn content_viewport(&self) -> Rectangle;
    pub fn footer(&self) -> Rectangle;
    pub fn resize_grip(&self, handle: ResizeHandle) -> Option<Rectangle>;
    pub fn body(&self) -> Rectangle;
    pub fn is_inside_card(&self, at: Point) -> bool;
    pub fn zone_at(&self, at: Point) -> Zone;
    pub fn is_draggable_at(&self, at: Point) -> bool;
    pub fn is_grip_at(&self, at: Point) -> bool;
    pub fn is_reachable(&self) -> bool;
    pub fn cursor(&self) -> mouse::Interaction;
    pub fn cursor_at(&self, at: Point) -> mouse::Interaction;
    pub fn is_dragging(&self) -> bool;
    pub fn resizing_handle(&self) -> Option<ResizeHandle>;
    pub fn is_capturing(&self) -> bool;
    pub fn on_pointer(&mut self, phase: PointerPhase, at: Point, target: SurfaceTarget,
                      focus: &mut FocusState, registry: &mut SurfaceRegistry,
                      storage: &mut dyn GeometryStorage) -> MovableResponse;
    pub fn drag_by(&mut self, delta: Vector) -> MovableAction;
    pub fn resize_by(&mut self, handle: ResizeHandle, delta: Vector) -> MovableAction;
    pub fn end_drag(&mut self, registry: &mut SurfaceRegistry,
                    storage: &mut dyn GeometryStorage) -> MovableAction;
    pub fn end_resize(&mut self, registry: &mut SurfaceRegistry,
                      storage: &mut dyn GeometryStorage) -> MovableAction;
    pub fn set_geometry(&mut self, geometry: MovableGeometry, registry: &mut SurfaceRegistry,
                        storage: &mut dyn GeometryStorage) -> MovableAction;
    pub fn set_viewport(&mut self, viewport: Rectangle, registry: &mut SurfaceRegistry,
                        storage: &mut dyn GeometryStorage) -> MovableAction;
    pub fn at_bottom(&self) -> bool;
    pub fn set_auto_scroll(&mut self, setting: AutoScrollSetting) -> MovableAction;
    pub fn sync_scroll(&mut self, scroll: ScrollGeometry) -> MovableAction;
    pub fn scroll_by(&mut self, delta: f32) -> MovableAction;
    pub fn scroll_to_bottom(&mut self) -> MovableAction;
    pub fn on_append(&mut self, entries: usize) -> AutoScrollDecision;
    pub fn on_scroll(&mut self, at: Point, delta: f32,
                     registry: &mut SurfaceRegistry) -> MovableResponse;
    pub fn holds_focus(&self, focus: &FocusState) -> bool;
    pub fn on_key(&mut self, class: KeyClass, focus: &mut FocusState,
                  registry: &mut SurfaceRegistry,
                  storage: &mut dyn GeometryStorage) -> MovableResponse;
    pub fn unwind_answer(&mut self, focus: &mut FocusState,
                         storage: &mut dyn GeometryStorage) -> UnwindAnswer;
    pub fn escape_via(&mut self, registry: &mut SurfaceRegistry, focus: &mut FocusState,
                      storage: &mut dyn GeometryStorage) -> EscapeOutcome;
    pub fn close(&mut self, focus: &mut FocusState, registry: &mut SurfaceRegistry,
                 storage: &mut dyn GeometryStorage) -> MovableResponse;
    pub fn reopen(&mut self, viewport: Rectangle, focus: &mut FocusState,
                  registry: &mut SurfaceRegistry,
                  storage: &mut dyn GeometryStorage) -> MovableResponse;
    pub fn persist(&mut self, storage: &mut dyn GeometryStorage);
    pub fn sync_members(&mut self, members: Vec<FocusMember>,
                        focus: &mut FocusState) -> Option<FocusId>;
}

// One drag as a tier transcript
pub struct DragGesture;
impl DragGesture {
    pub fn new() -> Self;
    pub fn is_open(&self) -> bool;
    pub fn snapshot(&self) -> Option<MovableGeometry>;
    pub fn begin(&mut self, geometry: MovableGeometry) -> GestureStart;
    pub fn live(&mut self, geometry: MovableGeometry) -> Option<Live<MovableGeometry>>;
    pub fn commit(&mut self, geometry: MovableGeometry) -> Option<Commit<MovableGeometry>>;
    pub fn cancel(&mut self) -> Option<Cancel<MovableGeometry>>;
    pub fn abort(&mut self);
    pub fn live_ticks(&self) -> usize;
    pub fn committed(&self) -> bool;
    pub fn cancelled(&self) -> bool;
    pub fn settled(&self) -> bool;
    pub fn last_tier(&self) -> &'static str;
}

// Paint resolution and geometry constants (A4.7.1)
pub struct MovableAnatomy {
    pub card_surface: Color, pub card_border: Color, pub card_border_width: f32,
    pub card_radius: f32, pub title_bar_surface: Color, pub bar_surface: Color,
    pub content_surface: Color, pub title_text: Color, pub meta_text: Color,
    pub tag_text: Color, pub divider: Color, pub title_bar_height: f32,
    pub title_bar_padding_horizontal: f32, pub filter_bar_height: f32,
    pub filter_bar_padding_vertical: f32, pub filter_bar_gap: f32, pub footer_height: f32,
    pub footer_padding_horizontal: f32, pub resize_handle_size: f32,
    pub action_control_size: f32,
}
pub const CARD_DEFAULT_WIDTH: f32 = 680.0;
pub const CARD_DEFAULT_HEIGHT: f32 = 440.0;
pub const CARD_MIN_WIDTH: f32 = clamp::SizePolicy::MOVABLE.min.width;
pub const CARD_MIN_HEIGHT: f32 = clamp::SizePolicy::MOVABLE.min.height;
pub const TITLE_BAR_HEIGHT: f32 = size::CONTROL_MD;
pub const TITLE_BAR_PADDING_VERTICAL: f32 = 0.0;
pub const TITLE_BAR_PADDING_HORIZONTAL: f32 = space::MD;
pub const TITLE_GRIP_WIDTH: f32 = size::ICON_MD;
pub const TITLE_FONT_SIZE: f32 = typography::UI_COMPACT.size;
pub const TITLE_FONT_WEIGHT: typography::Weight = typography::Weight::Bold;
pub const ACTION_CONTROL_SIZE: f32 = size::HIT_TARGET_MIN;
pub const ACTION_GAP: f32 = space::SM;
pub const FILTER_BAR_HEIGHT: f32 = size::CONTROL_LG;
pub const FILTER_BAR_PADDING_VERTICAL: f32 = space::XS;
pub const FILTER_BAR_PADDING_HORIZONTAL: f32 = space::MD;
pub const FILTER_BAR_GAP: f32 = space::SM;
pub const CONTENT_FONT_SMALL: f32 = typography::CAPTION.size;
pub const CONTENT_FONT_LARGE: f32 = typography::UI_COMPACT.size;
pub const FOOTER_HEIGHT: f32 = size::BAR_STATUS;
pub const FOOTER_PADDING_VERTICAL: f32 = 0.0;
pub const FOOTER_PADDING_HORIZONTAL: f32 = space::MD;
pub const RESIZE_HANDLE_SIZE: f32 = size::ICON_MD;
pub const CHROME_HEIGHT: f32 = TITLE_BAR_HEIGHT + FILTER_BAR_HEIGHT + FOOTER_HEIGHT;
pub const CONTENT_VIEWPORT_MIN: f32 = CARD_MIN_HEIGHT - CHROME_HEIGHT;
```

##### Examples

```rust
use hoarfrost::surface::movable::{MovableHost, MovableSpec, PointerPhase, SurfaceTarget};

let mut host = MovableHost::open(
    MovableSpec::new("log", "Session log"),
    viewport, &mut focus, &mut registry, &mut storage,
);

// Drag: press on the draggable zone, move, release.
host.on_pointer(PointerPhase::Pressed, at, SurfaceTarget::TitleBar,
                &mut focus, &mut registry, &mut storage);
host.on_pointer(PointerPhase::Moved, at, SurfaceTarget::TitleBar,
                &mut focus, &mut registry, &mut storage);
let done = host.on_pointer(PointerPhase::Released, at, SurfaceTarget::TitleBar,
                           &mut focus, &mut registry, &mut storage);
// The geometry is already saved by the Commit tier. Do not save it again.
```

##### Type references

- `MovableGeometry` — position plus size, as one serialisable value. `encode` / `decode`
  give a lossless text form, so the framework never depends on a serialiser.
- `GeometryStorage` — your persistence seam. Keys are strings; `storage_key` derives one
  from a `SurfaceId`.
- `ScrollGeometry` — content extent, viewport extent, offset. All scroll maths is pure.
- `AutoScrollDecision` — follow the stream or hold, with the reason and how far behind.
- `Zone` — what a point lands on. Use it to pick a cursor and to decide drag eligibility.
- `MovableTask` - a side effect the host MUST carry out: scroll, set a cursor, activate.
- `DragGesture` — one drag's tier transcript: `begin`, N `live`, then one `commit` or
  `cancel`.

##### Return values

- `open` loads persisted geometry, clamps it to the viewport, and falls back to
  `MovableGeometry::DEFAULT` when nothing is stored.
- Every mutating method returns `MovableAction` or `MovableResponse`. Match
  `NoOp(MovableNoOp)` before you assume anything moved.
- `resize_grip` returns `None` for a handle the card does not expose.
- `on_append` returns a decision, never a scroll. Emit the scroll yourself.
- `persist` writes the current geometry through `GeometryStorage::save`.

##### Side effects

- `open`, `close`, `reopen`, `on_pointer`, `on_key`, `on_scroll`, `set_geometry`,
  `set_viewport`, and `sync_members` mutate the registry and `FocusState`.
- Drag and resize commits write through `GeometryStorage`. `close` persists before it
  unregisters.
- A drag registers a `PreRankCoordinator` capture. `Escape` MUST go through
  `escape_with_pre_rank` or the drag is never cancelled.
- No wall-clock reads and no filesystem access inside the crate.

##### References

[`surface/movable.rs`](../src/hoarfrost/surface/movable.rs),
[`surface/drag_tier.rs`](../src/hoarfrost/surface/drag_tier.rs)

---

#### `surface::menu` — fixed and context menus

##### Overview

`surface::menu` is one engine for fixed menus and context menus
(`UIF-FMENU-*`, `UIF-CMENU-*`). Five modules split the concerns:

| Module | Owns |
|---|---|
| `menu::model` | the menu tree as pure data: rows, groups, selection, close policy |
| `menu::view` | the row state matrix and the focusable menu row widget |
| `menu::cascade` | which cards are open, mutual exclusion, keyboard navigation |
| `menu::row_action` | one row's action engine: pending, suppression, revertibility |
| `menu::aim` | the diagonal aiming corridor that keeps a branch open while aimed |

Openness lives in the cascade, never in the model. A row that is busy lives in
`row_action`, never in the model, because the cascade clones models and `sync`
overwrites local changes.

##### Prerequisites & Requirements

- Feature `menu`, which implies `button` and `tooltip`.
- Every interactive row MUST have a unique key inside its own menu. Call
  `Menu::validate` in a test and assert `is_valid`.
- Destructive rows MUST be last in their group and MUST NOT share a group with a
  non-destructive row. `validate` reports both as violations.
- A radio group MUST have at most one selected member.
- Drive the cascade with `MenuKey`, not raw keys. Call `focus_task` to move keyboard
  focus.
- The aiming corridor takes `TimeMs`. Never read a wall clock inside a menu rule.

##### Syntax / Method Signature

```rust
// model — pure data
pub struct MenuId(String);
impl MenuId {
    pub fn new(id: impl Into<String>) -> Self;
    pub fn as_str(&self) -> &str;
}
impl From<&str> for MenuId;
impl From<String> for MenuId;
pub struct RadioGroupId(String);
impl RadioGroupId {
    pub fn new(id: impl Into<String>) -> Self;
    pub fn as_str(&self) -> &str;
}
impl From<&str> for RadioGroupId;
impl From<String> for RadioGroupId;
pub fn submenu_id(parent: &MenuId, key: &str) -> MenuId;

pub enum RowKind { Action, Check { checked: bool }, Radio { group: RadioGroupId, selected: bool },
                   Submenu { child: Box<Menu> }, Heading, Separator }
pub enum ClosePolicy { Close, StayOpen }
pub enum LeadingSlot { Check, Radio, Icon, None }
pub enum TrailingSlot { Shortcut, Chevron, None }

pub struct Row;
impl Row {
    pub fn action(key: impl Into<String>, label: impl Into<String>) -> Self;
    pub fn checkable(key: impl Into<String>, label: impl Into<String>, checked: bool) -> Self;
    pub fn radio(key: impl Into<String>, label: impl Into<String>,
                 group: impl Into<RadioGroupId>, selected: bool) -> Self;
    pub fn submenu(key: impl Into<String>, label: impl Into<String>, child: Menu) -> Self;
    pub fn heading(label: impl Into<String>) -> Self;
    pub fn separator() -> Self;
    pub fn with_key(mut self, key: impl Into<String>) -> Self;
    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self;
    pub fn with_icon(mut self, icon: impl Into<String>) -> Self;
    pub fn key(&self) -> &str;
    pub fn label(&self) -> &str;
    pub fn shortcut(&self) -> Option<&str>;
    pub fn icon(&self) -> Option<&str>;
    pub fn radio_group(&self) -> Option<&RadioGroupId>;
    pub fn child(&self) -> Option<&Menu>;
    pub fn child_id(&self) -> Option<&MenuId>;
    pub fn leading_slot(&self) -> LeadingSlot;
    pub fn trailing_slot(&self) -> TrailingSlot;
}

pub struct Menu;
impl Menu {
    pub fn new(id: impl Into<MenuId>, rows: Vec<Row>) -> Self;
    pub fn empty(id: impl Into<MenuId>) -> Self;
    pub fn id(&self) -> &MenuId;
    pub fn rows(&self) -> &[Row];
    pub fn row(&self, index: usize) -> Option<&Row>;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn group_id(&self) -> GroupId;
    pub fn focus_id(&self, index: usize) -> Option<FocusId>;
    pub fn row_focus_id(&self, row: &Row) -> FocusId;
    pub fn index_of_focus_id(&self, id: &FocusId) -> Option<usize>;
    pub fn navigable_indices(&self) -> Vec<usize>;
    pub fn navigable_focus_ids(&self) -> Vec<FocusId>;
    pub fn focus_members(&self) -> Vec<FocusMember>;
    pub fn first_navigable(&self) -> Option<usize>;
    pub fn last_navigable(&self) -> Option<usize>;
    pub fn step_navigable(&self, current: Option<usize>, delta: i32) -> Option<usize>;
    pub fn groups(&self) -> Vec<RowGroup>;
    pub fn radio_group_ids(&self) -> Vec<RadioGroupId>;
    pub fn radio_group(&self, group: &RadioGroupId) -> Option<RadioGroupInfo>;
    pub fn radio_selection(&self, group: &RadioGroupId) -> Option<usize>;
    pub fn radio_members(&self, group: &RadioGroupId) -> Vec<usize>;
    pub fn select_radio(&mut self, index: usize) -> Option<Vec<usize>>;
    pub fn activate(&mut self, index: usize) -> Option<Activation>;
    pub fn set_checked(&mut self, index: usize, checked: bool) -> Option<bool>;
    pub fn is_valid(&self) -> bool;
    pub fn validate(&self) -> Vec<MenuViolation>;
}

pub struct RowGroup;
impl RowGroup {
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn has_interactive(&self) -> bool;
    pub fn has_destructive(&self) -> bool;
    pub fn mixes_destructive(&self) -> bool;
}
pub struct RadioGroupInfo;
impl RadioGroupInfo { pub fn is_selected(&self, row: usize) -> bool; }
pub enum ActivationEffect {
    Command,
    Toggled { checked: bool },
    Selected { group: RadioGroupId, cleared: Vec<usize> },
    OpenSubmenu { child: MenuId },
    Absorbed,
}
pub struct Activation {
    pub row: usize,
    pub effect: ActivationEffect,
    pub closes_menu: bool,
}
impl Activation { pub fn is_absorbed(&self) -> bool; }
pub enum MenuViolation {
    MissingKey { row: usize, label: String },
    DuplicateKey { row: usize, key: String, first: usize },
    DestructiveNotLast { row: usize, group: usize },
    MixedGroup { group: usize, row: usize },
    RadioOverSelected { group: RadioGroupId, rows: Vec<usize> },
}

// cascade — openness
pub enum MenuTreeKind { Context, Fixed }
pub struct SubmenuTrigger { pub parent: MenuId, pub row: usize }
pub struct OpenCard;
impl OpenCard {
    pub fn id(&self) -> &MenuId;
    pub fn group_id(&self) -> &GroupId;
    pub fn menu(&self) -> &Menu;
    pub fn trigger(&self) -> Option<&SubmenuTrigger>;
    pub fn is_root(&self) -> bool;
    pub fn focused_row(&self) -> Option<usize>;
}
pub enum MenuKey { Up, Down, Left, Right, Enter, Space, Escape }
pub enum IgnoredReason { NotOpen, AlreadyOpen { card: MenuId }, UnknownCard { card: MenuId },
                         NotSubmenu { card: MenuId, row: usize },
                         NoFocusableRows { card: MenuId },
                         RowOutOfRange { card: MenuId, row: usize },
                         NothingFocused { card: MenuId } }
pub enum FocusTransition {
    Unchanged,
    Moved { from: Option<FocusId>, to: FocusId },
    Cleared { from: FocusId },
}
pub struct CascadeOutcome { /* opened, closed, activation, focus, ignored */ }
impl CascadeOutcome {
    pub fn noop() -> Self;
    pub fn ignored(reason: IgnoredReason) -> Self;
    pub fn opened(&self) -> &[MenuId];
    pub fn closed(&self) -> &[MenuId];
    pub fn activation(&self) -> Option<&Activation>;
    pub fn focus(&self) -> &FocusTransition;
    pub fn ignored_reason(&self) -> Option<&IgnoredReason>;
    pub fn is_ignored(&self) -> bool;
    pub fn focus_moved(&self) -> bool;
    pub fn focus_task<Message: Send + 'static>(&self, focus: &FocusState) -> Task<Message>;
}
pub struct MenuCascade;
impl MenuCascade {
    pub fn new() -> Self;
    pub fn is_open(&self) -> bool;
    pub fn kind(&self) -> Option<MenuTreeKind>;
    pub fn root_id(&self) -> Option<&MenuId>;
    pub fn open_cards(&self) -> &[OpenCard];
    pub fn depth(&self) -> usize;
    pub fn active_card(&self) -> Option<&OpenCard>;
    pub fn active_menu(&self) -> Option<&Menu>;
    pub fn focused_row(&self) -> Option<usize>;
    pub fn level_of(&self, card: &MenuId) -> Option<usize>;
    pub fn is_card_open(&self, card: &MenuId) -> bool;
    pub fn card_menu(&self, card: &MenuId) -> Option<&Menu>;
    pub fn focused_row_in(&self, card: &MenuId) -> Option<usize>;
    pub fn open(&mut self, source: &Menu, kind: MenuTreeKind,
                focus: &mut FocusState) -> CascadeOutcome;
    pub fn toggle(&mut self, source: &Menu, kind: MenuTreeKind,
                  focus: &mut FocusState) -> CascadeOutcome;
    pub fn open_submenu(&mut self, card: &MenuId, row: usize,
                        focus: &mut FocusState) -> CascadeOutcome;
    pub fn hover_submenu_row(&mut self, aim: &mut AimCorridor, card: &MenuId, row: usize,
                             now: TimeMs, focus: &mut FocusState) -> CascadeOutcome;
    pub fn pointer_left_card(aim: &mut AimCorridor, now: TimeMs) -> bool;
    pub fn activate_row(&mut self, card: &MenuId, row: usize,
                        focus: &mut FocusState) -> CascadeOutcome;
    pub fn on_key(&mut self, key: MenuKey, focus: &mut FocusState) -> CascadeOutcome;
    pub fn move_focus(&mut self, delta: i32, focus: &mut FocusState) -> CascadeOutcome;
    pub fn open_focused_submenu(&mut self, focus: &mut FocusState) -> CascadeOutcome;
    pub fn activate_focused(&mut self, focus: &mut FocusState) -> CascadeOutcome;
    pub fn close_deepest(&mut self, focus: &mut FocusState) -> CascadeOutcome;
    pub fn close_tree(&mut self, focus: &mut FocusState) -> CascadeOutcome;
    pub fn sync(&mut self, source: &Menu, focus: &mut FocusState) -> CascadeOutcome;
}

// aim — the corridor
pub fn grace_ms() -> u64;                       // time.delay.submenu (150 ms)
pub enum AimTarget { ParentRow { row: usize }, ChildBranch, Outside }
pub enum AimDecision { Idle, Preserve, Hold { row: usize, remaining_ms: u64 },
                       Switch { row: usize }, Collapse }
pub struct AimCorridor;
impl AimCorridor {
    pub fn branch_opened(&mut self, trigger: usize, at: TimeMs);
    pub fn branch_closed(&mut self);
    pub fn pointer_enter(&mut self, target: AimTarget, at: TimeMs);
    pub fn pointer_out(&mut self, at: TimeMs);
    pub fn reset(&mut self);
    pub fn is_aiming(&self, now: TimeMs) -> bool;
    pub fn corridor_remaining(&self, now: TimeMs) -> u64;
    pub fn decision(&self, now: TimeMs) -> AimDecision;
}

// row_action — one row's action engine
pub enum RevertKind { Value, FireAndSettle, Navigation, Nothing }
pub fn row_revert(row: &Row, effect: &ActivationEffect) -> RevertKind;
pub fn row_tracks_action(pending: &ActionPending) -> bool;
pub struct RowSettlement {
    pub source: ActivationSource,
    pub row_key: String,
    pub effect: Option<ActivationEffect>,
    pub revert: RevertKind,
    pub pending: ActionPending,
    pub dispatched: bool,
}
impl RowSettlement {
    pub fn undispatched(row: &Row, source: ActivationSource, pending: ActionPending) -> Self;
}
pub enum RowEmission {
    Committed(Commit<RowSettlement>),
    Cancelled(Cancel<RowSettlement>),
    CancelRequested(CancelRequest),
}
impl RowEmission { pub fn settlement(&self) -> Option<&RowSettlement>; }
pub struct RowActionEngine;
impl RowActionEngine {
    pub fn new() -> Self;
    pub fn observe(&mut self, pending: ActionPending);
    pub fn pending(&self) -> ActionPending;
    pub fn begin(&mut self, row: &Row, source: ActivationSource) -> Option<RowEmission>;
    pub fn commit(&mut self, row: &Row, effect: &ActivationEffect,
                  source: ActivationSource) -> Option<RowEmission>;
    pub fn cancel(&mut self) -> Option<RowEmission>;
    pub fn gesture_open(&self) -> bool;
    pub fn last_begin(&self) -> Option<GestureStart>;
    pub fn is_pending(&self) -> bool;
    pub fn busy_observed(&self) -> bool;
    pub fn fired(&self) -> usize;
    pub fn suppressed(&self) -> usize;
    pub fn cancel_requested(&self) -> bool;
    pub fn reset(&mut self);
}

// view — the row matrix and the row widget
pub struct RowState { pub base: RowBase, pub focused: bool }
impl RowState {
    pub const REST: Self;  pub const HOVERED: Self;  pub const PRESSED: Self;
    pub const KEYBOARD_FOCUSED: Self;  pub const ACTIVE_SUBMENU_PARENT: Self;
    pub const DESTRUCTIVE_HOVERED: Self;  pub const DESTRUCTIVE_PRESSED: Self;
    pub const DESTRUCTIVE_FOCUSED: Self;  pub const SUBMENU_PARENT_FOCUSED: Self;
    pub const REST_FOCUSED: Self;  pub const DISABLED: Self;  pub const SECTION_HEADING: Self;
    pub const SPEC_ROWS: [Self; 8];
    pub const DERIVED_ROWS: [Self; 3];
    pub const ALL: [Self; 12];
    pub const fn matrix_row(&self) -> &'static str;
}
pub enum RowBase { Rest, Hovered, Pressed, ActiveSubmenuParent, DestructiveHovered,
                   DestructivePressed, Disabled, SectionHeading }
pub struct RowStyle {
    pub background: Color,
    pub leading: Color,
    pub label: Color,
    pub trailing: Color,
    pub focus_outline: Option<FocusIndicator>,
    pub cursor: mouse::Interaction,
}
pub fn resolve_row_style(row: &Row, state: RowState, palette: &Palette) -> RowStyle;
pub fn derive_row_state(row: &Row, visual: VisualState, submenu_open: bool) -> RowState;
pub fn background_layer(bounds: Rectangle, style: &RowStyle) -> Option<Layer>;
pub fn focus_outline_layer(bounds: Rectangle, style: &RowStyle) -> Option<Layer>;
pub fn separator_layer(bounds: Rectangle, palette: &Palette) -> Layer;
pub fn paint_layers(row: &Row, bounds: Rectangle, style: &RowStyle,
                    palette: &Palette) -> Vec<Layer>;
pub fn row_height(row: &Row) -> f32;
pub fn row_padding_y(row: &Row) -> f32;
pub enum RowEvent { Activated, Absorbed, Hovered, Unhovered }
pub enum SlotKind { Leading, Label, Trailing, Custom }
pub struct RowTreeState;
pub struct MenuRow<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>;
impl<'a, Message, Theme, Renderer> MenuRow<'a, Message, Theme, Renderer> {
    pub fn new(row: Row, focus_id: impl Into<FocusId>,
               on_event: impl Fn(RowEvent) -> Message + 'a) -> Self;
    pub fn with_body(row: Row, body: impl Into<Element<'a, Message, Theme, Renderer>>,
                     focus_id: impl Into<FocusId>,
                     on_event: impl Fn(RowEvent) -> Message + 'a) -> Self;
    pub fn palette(mut self, palette: Palette) -> Self;
    pub fn submenu_open(mut self, open: bool) -> Self;
    pub fn row(&self) -> &Row;
    pub fn focus_id(&self) -> &FocusId;
    pub fn widget_id(&self) -> &Id;
    pub fn is_submenu_open(&self) -> bool;
    pub fn slot_kinds(&self) -> &[SlotKind];
    pub fn current_style(&self, state: &RowTreeState) -> RowStyle;
}
pub const MENU_CARD_WIDTH_MIN: f32 = 224.0;
pub const MENU_CARD_WIDTH_MAX: f32 = 320.0;
pub const ROW_HEIGHT: f32 = size::CONTROL_MD;
pub const ROW_PADDING_X: f32 = space::SM + space::XXS;
pub const ROW_GAP: f32 = space::SM;
pub const ROW_RADIUS: f32 = shape::RADIUS_WIDGET;
pub const LEADING_SLOT: f32 = size::ICON_MD;
pub const TRAILING_TYPE: f32 = typography::CAPTION.size;
pub const HEADING_HEIGHT: f32 = size::BAR_STATUS;
pub const HEADING_PADDING_Y: f32 = space::XS;
pub const SEPARATOR_HEIGHT: f32 = space::SM + shape::BORDER_HAIRLINE;
pub const SEPARATOR_THICKNESS: f32 = shape::BORDER_HAIRLINE;
pub const FOCUS_OUTLINE_WIDTH: f32 = shape::BORDER_HAIRLINE;
pub const GLYPH_CHECK: &str = "\u{2713}";
pub const GLYPH_RADIO: &str = "\u{2022}";
pub const GLYPH_CHEVRON: &str = ">";
```

##### Examples

```rust
use hoarfrost::surface::menu::{cascade::MenuKey, model::{Menu, Row}, MenuCascade, MenuTreeKind};

let menu = Menu::new("file", vec![
    Row::action("new", "New"),
    Row::submenu("open", "Open Recent", Menu::new("open-recent", vec![Row::action("a", "a.md")])),
    Row::separator(),
    Row::action("close", "Close"),
]);
assert!(menu.is_valid());                       // assert this in a test

let mut cascade = MenuCascade::new();
let mut aim = AimCorridor::new();
cascade.open(&menu, MenuTreeKind::Fixed, &mut focus);
let task = cascade.on_key(MenuKey::Down, &mut focus).focus_task(&focus);
```

##### Type references

- `Menu` / `Row` / `RowKind` — the tree as data. Cloning is cheap and safe.
- `MenuViolation` — the structural rules the spec states, as machine-checkable errors.
- `MenuCascade` — the open-card stack. One tree open at a time; a sibling replaces its
  sibling.
- `CascadeOutcome` — what one interaction did: opened, closed, activation, focus move.
- `AimCorridor` — the diagonal-aim memory. It answers Preserve, Hold, Switch, or Collapse.
- `RowActionEngine` — one row's pending state and duplicate-activation suppression.
- `RowState` / `RowBase` — the resolved row visual. Focus is additive, not a base.
- `MenuRow` — the focusable row widget. `with_body` lets you supply your own label body.

##### Return values

- `open` returns a `CascadeOutcome`. `is_ignored()` tells you nothing changed.
- `focus_task` returns an iced `Task` that moves keyboard focus. Return it from your
  `update`.
- `activate` returns `Option<Activation>`. `Activation::is_absorbed()` is `true` for a
  heading, a separator, or a disabled row.
- `select_radio` returns the indices it changed, or `None` when the row is not a radio.
- `validate` returns an empty `Vec` for a well-formed menu.
- `pointer_left_card` returns whether the branch should collapse now.

##### Side effects

- Every `MenuCascade` method that changes openness also moves keyboard focus, and
  returns the transition so you can emit the matching task.
- `sync` overwrites cloned models from your source menu. Any busy state you keep in the
  model is lost; keep it in `RowActionEngine`.
- `AimCorridor` methods mutate the corridor only. They read `TimeMs` arguments and never
  a clock.

##### References

[`surface/menu/mod.rs`](../src/hoarfrost/surface/menu/mod.rs),
[`model.rs`](../src/hoarfrost/surface/menu/model.rs),
[`cascade.rs`](../src/hoarfrost/surface/menu/cascade.rs),
[`aim.rs`](../src/hoarfrost/surface/menu/aim.rs),
[`row_action.rs`](../src/hoarfrost/surface/menu/row_action.rs),
[`view.rs`](../src/hoarfrost/surface/menu/view.rs)

---

#### `surface::popover` — teaching popovers

##### Overview

`surface::popover` is the interactive callout: a card with buttons the user is expected
to reach and dismiss. It splits into `model` (what it says and what acting means),
`place` (where it lands), `host` (behaviour and dismissal precedence), and
`PopoverStyle::resolve` (what it paints).

##### Prerequisites & Requirements

- Feature `popover`, which implies `zorder`.
- The card registers as `SurfaceKind::TeachingPopover`, which is Band 2.
- The model MUST have at least one focusable footer control. `open` refuses otherwise.
- Pass the measured `card_size`. The solver places a card; it does not measure text.
- Dismissal precedence is fixed: footer button, then `Escape`, then click-away, then
  programmatic close. Do not add a pointer-leave rule.

##### Syntax / Method Signature

```rust
// model
pub struct PopoverModel<ActionId> { /* private */ }
impl<ActionId: Clone + PartialEq> PopoverModel<ActionId> {
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self;
    pub fn with_action(mut self, label: impl Into<String>, action: ActionId) -> Self;
    pub fn with_placement(mut self, side: PreferredSide) -> Self;
    pub fn with_open(mut self, wanted: bool) -> Self;
    pub fn title(&self) -> &str;
    pub fn body(&self) -> &str;
    pub fn primary_action(&self) -> Option<(&str, &ActionId)>;
    pub fn has_primary_action(&self) -> bool;
    pub fn footer_members(&self) -> Vec<FocusMember>;
    pub fn initial_focus(&self) -> FocusId;
    pub fn group_for() -> GroupId;
    pub const POPOVER_GROUP: &str = "popover";
    pub fn primary_focus_id() -> FocusId;
    pub fn dismiss_focus_id() -> FocusId;
}
pub const DISMISS_LABEL: &str = "Dismiss";

// place
pub enum PreferredSide { #[default] Below, Above, Leading, Trailing }
impl PreferredSide { pub const fn alignment(self) -> ControlAlignment; }
pub const POPOVER_GAP: f32 = space::XS;
pub type Placement = anchor::Placement;
pub fn resolve(trigger: Rectangle, side: PreferredSide, card_size: Size,
               viewport: Rectangle) -> Placement;

// host
pub enum DismissCause { PrimaryAction, DismissButton, Escape, ClickOutside, Programmatic }
pub enum IgnoredReason { NotOpen, NoPrimaryAction, InsideCard }
pub enum PopoverEvent<ActionId> {
    Action(ActionId),
    Dismissed(DismissCause),
    Ignored(IgnoredReason),
}
pub enum OpenRefusal { NotRequested, NoFocusableControl }
pub struct PopoverHost<ActionId: Clone + PartialEq>;
impl<ActionId: Clone + PartialEq> PopoverHost<ActionId> {
    pub fn open(id: SurfaceId, model: PopoverModel<ActionId>, trigger: Rectangle,
                card_size: Size, viewport: Rectangle, focus: &mut FocusState,
                registry: &mut SurfaceRegistry) -> Result<Self, OpenRefusal>;
    pub fn id(&self) -> &SurfaceId;
    pub fn group_id(&self) -> &crate::focus::GroupId;
    pub fn model(&self) -> &PopoverModel<ActionId>;
    pub fn is_inside_card(&self, at: Point) -> bool;
    pub fn footer_ids(&self) -> Vec<FocusId>;
    pub fn activate_primary(&mut self, focus: &mut FocusState) -> PopoverEvent<ActionId>;
    pub fn dismiss_dismiss_button(&mut self, focus: &mut FocusState) -> PopoverEvent<ActionId>;
    pub fn dismiss(&mut self, cause: DismissCause,
                   focus: &mut FocusState) -> PopoverEvent<ActionId>;
    pub fn on_pointer(&mut self, at: Point, focus: &mut FocusState) -> PopoverEvent<ActionId>;
    pub fn on_escape(&mut self, focus: &mut FocusState) -> PopoverEvent<ActionId>;
    pub fn close(&mut self, focus: &mut FocusState,
                 registry: &mut SurfaceRegistry) -> PopoverEvent<ActionId>;
    pub fn unwind_answer(&mut self, focus: &mut FocusState) -> UnwindAnswer;
    pub fn escape_via(&mut self, registry: &mut SurfaceRegistry,
                      focus: &mut FocusState) -> EscapeOutcome;
}

// style and box limits
pub struct PopoverStyle {
    pub background: Color, pub border_color: Color, pub border_width: f32, pub radius: f32,
    pub padding: f32, pub title_color: Color, pub title_size: f32, pub title_weight: Weight,
    pub body_color: Color, pub body_size: f32, pub body_weight: Weight,
    pub width_min: f32, pub width_max: f32,
}
impl PopoverStyle {
    pub fn resolve(palette: &Palette) -> Self;
    pub fn clamp_width(measured: f32) -> f32;
    pub fn card_chrome_class(style: &PopoverStyle, palette: &Palette) -> ChromeClass;
}
pub const WIDTH_MIN: f32 = 280.0;
pub const WIDTH_MAX: f32 = 360.0;
pub const PADDING: f32 = space::MD;
pub const BORDER_WIDTH: f32 = shape::BORDER_HAIRLINE;
pub const RADIUS: f32 = shape::RADIUS_CARD;
pub const TYPE_RUNG: typography::TypeRung = typography::UI_COMPACT;
pub const TITLE_WEIGHT: Weight = Weight::SemiBold;
pub const BODY_WEIGHT: Weight = Weight::Regular;
```

##### Examples

```rust
use hoarfrost::surface::popover::{PopoverHost, PopoverModel, PopoverEvent};

let model = PopoverModel::new("Export", "Pick a folder first.")
    .with_action("Got it", AppAction::AckExport)
    .with_open(true);
match PopoverHost::open("tip-export".into(), model, trigger, card, viewport,
                        &mut focus, &mut registry) {
    Ok(host) => { /* live */ }
    Err(OpenRefusal::NotRequested) => {}
    Err(OpenRefusal::NoFocusableControl) => {}
}
```

##### Type references

- `PopoverModel<ActionId>` — the callout's text and its one primary action. `ActionId`
  is your own action type; the model only clones it.
- `PreferredSide` — which side the card prefers. The solver may flip it near an edge.
- `OpenRefusal` — why a callout did not open. Match both variants.
- `PopoverEvent` — what an interaction produced: your action, a dismissal with a cause,
  or an ignored event with a reason.
- `PopoverStyle` — one resolved style. There is no state matrix; the card is a frame and
  its buttons carry the states.

##### Return values

- `open` returns `Result<Self, OpenRefusal>`. A refused open registers nothing.
- `activate_primary` returns `Ignored(NoPrimaryAction)` when the model has no action.
- `on_pointer` returns `Ignored(InsideCard)` for a press inside the card that is not a
  footer control, so the click-away rule cannot double-fire.
- `dismiss` returns `Dismissed(cause)` and leaves the registry change to `close`.

##### Side effects

- `open` registers the surface and takes the focus trap. `close` releases both.
- `dismiss` moves focus back through `FocusState`. It does not unregister.
- `PopoverStyle::resolve` is pure and runs a contrast lock check on both text channels.

##### References

[`surface/popover/mod.rs`](../src/hoarfrost/surface/popover/mod.rs),
[`model.rs`](../src/hoarfrost/surface/popover/model.rs),
[`place.rs`](../src/hoarfrost/surface/popover/place.rs),
[`host.rs`](../src/hoarfrost/surface/popover/host.rs)

---

#### `surface::tooltip` — tooltips

##### Overview

`surface::tooltip` is the framework-owned hover and keyboard-focus tooltip
(`UIF-ATOM-007`, `UIF-ATOM-001`, `UIF-NAV-003`, `UIF-TAB-003`). It is the only surface
whose visibility is a function of time. It splits into `hover` (when it paints), `warm`
(when the second one paints), `place` (where it paints), `TooltipStyle::resolve` (what
it paints), and `Tooltip` (the widget that wraps your content).

##### Prerequisites & Requirements

- Feature `tooltip`.
- Pass a `TimeMs` to every timed call. The crate never reads a wall clock.
- Use `HoverController::tick(now)` to decide visibility. Do not derive it from a
  timestamp difference in your own code.
- Feed both arrival channels: `pointer_enter` or `pointer_enter_warm` for the pointer,
  `keyboard_focus_enter` for the keyboard.
- Clamp the measured card width with `clamp_width`. `MAX_WIDTH` is the framework's cap.
- A tooltip is a layer, not a band. It paints above everything it is not related to and
  is pointer-transparent.

##### Syntax / Method Signature

```rust
pub struct TimeMs(pub u64);
impl TimeMs { pub fn now() -> Self; }

pub enum Visibility { Hidden, Pending, Shown }
pub enum ArrivalSource { Pointer, Keyboard }
pub enum DismissReason { PointerExit, MousePress, AnchorGone, FocusMoved, Escape, LeftWindow }
pub struct Dismissal { pub reason: DismissReason, pub was_active: bool,
                       pub consumed_input: bool }

pub struct HoverController;
impl HoverController {
    pub fn new() -> Self;
    pub fn with_delay(delay: Duration) -> Self;
    pub fn pointer_enter(&mut self, now: TimeMs) -> bool;
    pub fn pointer_enter_warm(&mut self, now: TimeMs, warmth: ArrivalWarmth) -> bool;
    pub fn keyboard_focus_enter(&mut self, now: TimeMs) -> bool;
    pub fn pointer_exit(&mut self);
    pub fn keyboard_focus_exit(&mut self);
    pub fn dismiss(&mut self, reason: DismissReason) -> Dismissal;
    pub fn is_active(&self) -> bool;
    pub fn is_suppressed(&self) -> bool;
    pub fn entered_at(&self) -> Option<TimeMs>;
    pub fn arrival_source(&self) -> Option<ArrivalSource>;
    pub fn is_keyboard_armed(&self) -> bool;
    pub fn active_delay(&self) -> Duration;
    pub fn tick(&self, now: TimeMs) -> Visibility;
}

// warm handoff
pub enum ArrivalWarmth { Cold, Warm }
pub struct WarmSession;
impl WarmSession {
    pub fn note_visible(&mut self, at: TimeMs);
    pub fn warmth_for(&self, arrival: TimeMs) -> ArrivalWarmth;
    pub fn is_warm(&self, now: TimeMs) -> bool;
    pub fn warmth_remaining(&self, now: TimeMs) -> u64;
    pub fn reset(&mut self);
}
pub const fn handoff_window_ms() -> u64;   // time.delay.tooltip_warm
pub const fn cold_wait_ms() -> u64;        // the full hover delay
pub const fn warm_wait_ms() -> u64;        // 0

// placement
pub enum TooltipAnchor {
    Control(Rectangle),
    Band { band: Rectangle, control: Rectangle },
    Pointer(Point),
}
impl TooltipAnchor {
    pub const fn band(band: Rectangle, control: Rectangle) -> Self;
    pub fn rect(self) -> Rectangle;
    pub fn to_anchor(self) -> Anchor;
    pub fn resolve(self, card_size: Size, viewport: Rectangle) -> Placement;
}
pub const TOOLTIP_ALIGNMENT: ControlAlignment = ControlAlignment::below_start();
pub const TOOLTIP_GAP: f32 = space::XS;

// content, style, widget
pub struct TooltipContent { pub label: String, pub shortcut: Option<String> }
impl TooltipContent {
    pub fn new(label: impl Into<String>) -> Self;
    pub fn with_shortcut(label: impl Into<String>, shortcut: impl Into<String>) -> Self;
}
pub const SHORTCUT_GAP: f32 = space::SM;
pub const HOVER_DELAY: u64 = time::DELAY_TOOLTIP_OPEN.ms;      // 400 ms
pub const KEYBOARD_FOCUS_DELAY: u64 = time::INSTANT.ms;        // 0 ms
pub const MAX_WIDTH: f32 = 280.0;
pub const PADDING_VERTICAL: f32 = space::XS;
pub const PADDING_HORIZONTAL: f32 = space::SM;
pub fn clamp_width(measured: f32) -> f32;

pub struct TooltipStyle {
    pub background: Color, pub foreground: Color, pub border_color: Color,
    pub border_width: f32, pub radius: f32, pub font: typography::TypeRung,
    pub max_width: f32, pub padding_vertical: f32, pub padding_horizontal: f32,
    pub shortcut_foreground: Color, pub shortcut_font: typography::FontFamily,
}
impl TooltipStyle { pub fn resolve(palette: &Palette) -> Self; }

pub fn card_size(label_width: f32, shortcut_width: Option<f32>,
                 style: &TooltipStyle) -> Size;
pub fn card_quad(bounds: Rectangle, style: &TooltipStyle) -> renderer::Quad;

pub struct TooltipState;
impl TooltipState { pub fn new() -> Self; pub fn is_shown(&self, now: TimeMs) -> bool; }

pub struct Tooltip<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>;
impl<'a, Message, Theme, Renderer> Tooltip<'a, Message, Theme, Renderer> {
    pub fn new(content: impl Into<Element<'a, Message, Theme, Renderer>>,
               tooltip: impl Into<TooltipContent>) -> Self;
    pub fn with_delay(mut self, delay: Duration) -> Self;
    pub fn with_palette(mut self, palette: Palette) -> Self;
    pub fn with_anchor(mut self, anchor: TooltipAnchor) -> Self;
    pub fn content(&self) -> &TooltipContent;
}
pub fn tooltip<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
    tooltip: impl Into<TooltipContent>,
) -> Tooltip<'a, Message, Theme, Renderer>;
```

##### Examples

```rust
use hoarfrost::surface::tooltip::{tooltip, TooltipContent, TimeMs};

let button = my_button().map(|m| Msg::Run(m));
let element = tooltip(button, TooltipContent::new("Run the batch")
    .with_shortcut("Run the batch", "Ctrl+Enter")).into();

// Driving the controller by hand:
let mut hover = hoarfrost::surface::tooltip::hover::HoverController::new();
hover.pointer_enter(TimeMs(0));
let visible = hover.tick(TimeMs(400));            // Visibility::Shown
```

##### Type references

- `TimeMs` — the injected clock. `TimeMs::now()` exists for call sites that genuinely
  need wall time; rules never call it.
- `HoverController` — the hover state machine. `tick` is the only visibility decision.
- `WarmSession` — the cross-control memory that makes the next tooltip open at `0 ms`.
- `TooltipAnchor` — what the card describes. `Band` mixes a bar's vertical extent with an
  entry's horizontal extent, which is the only reading of "below the navbar" that works.
- `TooltipContent` — a label plus an optional shortcut, rendered in a second column.
- `TooltipStyle` — the resolved paint and box, including the monospace shortcut font.

##### Return values

- `pointer_enter` and friends return whether the controller became active, not whether
  the tooltip is visible. Call `tick` for visibility.
- `tick` returns `Pending` while the delay has not elapsed and `Shown` once it has.
- `dismiss` returns a `Dismissal`. `consumed_input` is `true` for a press that closed a
  visible tooltip, so the press MUST NOT also reach the control.
- `clamp_width` returns `MAX_WIDTH` for a non-finite measurement and `0` for a negative
  one.
- `resolve` returns `Placement` with flip and clamp flags preserved.

##### Side effects

- `HoverController` and `WarmSession` mutate only their own state.
- `Tooltip` owns a `TooltipState` internally and repaints itself on the delay timer.
- `TooltipStyle::resolve` is pure and runs a contrast lock check on the fill and text.

##### References

[`surface/tooltip/mod.rs`](../src/hoarfrost/surface/tooltip/mod.rs),
[`hover.rs`](../src/hoarfrost/surface/tooltip/hover.rs),
[`warm.rs`](../src/hoarfrost/surface/tooltip/warm.rs),
[`place.rs`](../src/hoarfrost/surface/tooltip/place.rs),
[`widget.rs`](../src/hoarfrost/surface/tooltip/widget.rs)

### Geometry, layout and a11y (`geometry::*`, `layout::*`, `a11y`)

These modules are always compiled. They carry no widget and read no clock. Each is a set
of pure solvers or pure checkers over plain numbers.

#### `geometry::anchor` and `geometry::clamp` — placement and clamping

##### Overview

`geometry::anchor` places a card against a pointer, a control, or a cursor-proximal
trigger, and reports whether it had to flip or clamp. `geometry::clamp` keeps a movable
card's title bar reachable and its size inside a policy (`UIF-MOV-002`).

##### Prerequisites & Requirements

- Pass the viewport as a `Rectangle` in root coordinates.
- Pass a measured `card_size`. The solver does not measure text.
- Reuse `ClampRule::MOVABLE` unless the spec gives your card a different bound. The
  movable host reads its minimum from `SizePolicy::MOVABLE`, so a private floor would
  disagree with the solver.
- Read `Placement::was_flipped()` and `was_clamped()` instead of re-deriving them.

##### Syntax / Method Signature

```rust
pub enum Anchor {
    Pointer { point: Point },
    Control { rect: Rectangle, alignment: ControlAlignment },
    CursorProximal { point: Point, keyboard_invoked: bool },
}
pub enum VerticalAlign { Below, Above, Top, Bottom, Middle }
pub enum HorizontalAlign { Start, End, Center, Trailing, Leading }
pub struct ControlAlignment { pub vertical: VerticalAlign, pub horizontal: HorizontalAlign }
impl ControlAlignment {
    pub const fn new(vertical: VerticalAlign, horizontal: HorizontalAlign) -> Self;
    pub const fn below_start() -> Self;
    pub const fn below_end() -> Self;
    pub const fn submenu() -> Self;          // Top + Trailing
}
pub struct Placement {
    pub origin: Point,
    pub size: Size,
    pub flipped_horizontal: bool,
    pub flipped_vertical: bool,
    pub scrollable: bool,
    pub clamped_horizontal: bool,
    pub clamped_vertical: bool,
}
impl Placement {
    pub fn rect(&self) -> Rectangle;
    pub fn was_flipped(&self) -> bool;
    pub fn was_clamped(&self) -> bool;
    pub fn visible_height(&self, viewport: Rectangle) -> f32;
}
pub fn solve(anchor: Anchor, card_size: Size, viewport: Rectangle) -> Placement;

pub struct SizePolicy { pub min: Size, pub max: Option<Size> }
impl SizePolicy {
    pub const MOVABLE: Self;
    pub const fn bounded(min: Size, max: Size) -> Self;
    pub fn clamp(self, size: Size) -> Size;
}
pub struct ClampRule {
    pub min_visible_title: f32,
    pub top_bound: f32,
    pub title_bar_height: f32,
    pub size_policy: SizePolicy,
}
impl ClampRule { pub const MOVABLE: Self; }
pub enum ResizeHandle { TopLeft, Top, TopRight, Right, BottomRight, Bottom, BottomLeft, Left }
impl ResizeHandle {
    pub const ALL: [Self; 8];
    pub const fn drags_start_x(self) -> bool;
    pub const fn drags_end_x(self) -> bool;
    pub const fn drags_start_y(self) -> bool;
    pub const fn drags_end_y(self) -> bool;
}
pub fn clamp_position(position: Point, card_size: Size, viewport: Rectangle,
                      rule: ClampRule) -> Point;
pub fn drag_position(current: Point, delta: Vector, card_size: Size, viewport: Rectangle,
                     rule: ClampRule) -> Point;
pub fn clamp_rect(rect: Rectangle, viewport: Rectangle, rule: ClampRule) -> Rectangle;
pub fn clamp_size(rect: Rectangle, viewport: Rectangle, rule: ClampRule) -> Rectangle;
pub fn resize(rect: Rectangle, handle: ResizeHandle, delta: Vector, viewport: Rectangle,
              rule: ClampRule) -> Rectangle;
pub fn is_reachable(rect: Rectangle, viewport: Rectangle, rule: ClampRule) -> bool;
pub fn title_bar_rect(rect: Rectangle, rule: ClampRule) -> Rectangle;
pub fn visible_title_width(rect: Rectangle, viewport: Rectangle) -> f32;
```

##### Examples

```rust
use hoarfrost::geometry::anchor::{solve, Anchor, ControlAlignment};

let place = solve(Anchor::Control { rect: trigger, alignment: ControlAlignment::below_start() },
                  Size::new(280.0, 120.0), viewport);
if place.was_flipped() {
    // The card opened above the trigger. Paint the arrow on the other edge.
}
```

##### Type references

- `Anchor` — what the card hangs off. `CursorProximal` carries `keyboard_invoked`, which
  decides whether the card anchors to the pointer or to the focused control.
- `ControlAlignment` — the preferred side and cross-axis alignment, as one value.
- `Placement` — the solved origin and size, plus the flip and clamp flags.
- `ClampRule` — the four numbers a clamp needs. `ClampRule::MOVABLE` is the spec's rule.
- `ResizeHandle` — which edge or corner a drag pulls. The four `drags_*` predicates drive
  the axis maths, so no call site re-matches the variants.

##### Return values

- `solve` always returns a `Placement`. It never fails; it flips, then clamps.
- `visible_height` returns the part of the card inside the viewport, which is what tells
  you a card needs a scrollable body.
- `clamp_position` and `drag_position` return an origin, never a rect.
- `is_reachable` answers whether the title bar still has `min_visible_title` of width.

##### Side effects

None. Every function in this pair is pure.

##### References

[`geometry/anchor.rs`](../src/hoarfrost/geometry/anchor.rs),
[`geometry/clamp.rs`](../src/hoarfrost/geometry/clamp.rs)

---

#### `geometry::window_geometry` and `geometry::pane_solver` — window and pane maths

##### Overview

`geometry::window_geometry` holds the window's persisted geometry, the UI scale ladder,
the window layout solve, and the persistence seam (`UIF-WIN-001`). `geometry::pane_solver`
clamps a pane width against a viewport and a bound pair (`UIF-PANE-002`).

##### Prerequisites & Requirements

- Persist through `WindowStorage`. Tests MUST use `NoopWindowStorage` or
  `MemoryWindowStorage`, never a real user path.
- Build `UiScale` with `UiScale::new`. It clamps and quantizes; do not store a raw f32.
- `x` and `y` are `Option<f32>`. `None` means "let the window manager place the window".
- Solve the layout before you mount a pane. `right_pane_mounted` changes the split.

##### Syntax / Method Signature

```rust
pub const WINDOW_MIN_WIDTH: f32 = 640.0;
pub const WINDOW_MIN_HEIGHT: f32 = 480.0;
pub const WINDOW_DEFAULT_WIDTH: f32 = 1000.0;
pub const WINDOW_DEFAULT_HEIGHT: f32 = 700.0;
pub const CENTER_PANE_MIN_W: f32 = 240.0;
pub const LEFT_PANE_MIN_W: f32 = 150.0;
pub const RIGHT_PANE_MIN_W: f32 = 120.0;

pub struct UiScale(f32);
impl UiScale {
    pub const MIN: f32 = 0.75;
    pub const MAX: f32 = 2.00;
    pub const STEP: f32 = 0.10;
    pub const DEFAULT: Self;
    pub fn new(val: f32) -> Self;              // clamps and quantizes
    pub fn step_up(self) -> Self;
    pub fn step_down(self) -> Self;
    pub fn factor(self) -> f32;
    pub fn compose_with_dpi(self, os_dpi_scale: f32) -> f32;
}

pub struct WindowGeometry {
    pub x: Option<f32>, pub y: Option<f32>,
    pub width: f32, pub height: f32,
    pub left_split_ratio: f32, pub right_split_ratio: f32,
    pub ui_scale: UiScale,
}
pub struct SolvedWindowLayout {
    pub window_width: f32, pub window_height: f32,
    pub left_pane_width: f32, pub center_pane_width: f32, pub right_pane_width: f32,
}
pub fn solve_window_layout(requested_w: f32, requested_h: f32, left_ratio: f32,
                           right_ratio: f32, right_pane_mounted: bool) -> SolvedWindowLayout;

pub trait WindowStorage: Send + Sync {
    fn load_geometry(&self) -> Option<WindowGeometry>;
    fn save_geometry(&self, geometry: &WindowGeometry);
}
pub struct NoopWindowStorage;
pub struct MemoryWindowStorage;
impl MemoryWindowStorage {
    pub fn new() -> Self;
    pub fn with_geometry(geometry: WindowGeometry) -> Self;
}

pub fn solve_clamped_pane_width(viewport_w: f32, requested_w: f32, min_px: f32,
                                max_px: f32) -> f32;
pub fn auto_fit_pane_width(labels: &[&str], font_size: f32, icon_w: f32, padding: f32,
                           min_w: f32, max_w: f32) -> f32;
```

##### Examples

```rust
use hoarfrost::geometry::window_geometry::{MemoryWindowStorage, WindowStorage, UiScale};

let store = MemoryWindowStorage::new();
if let Some(g) = store.load_geometry() {
    let scale = UiScale::new(g.ui_scale.factor() + UiScale::STEP);
    store.save_geometry(&hoarfrost::geometry::window_geometry::WindowGeometry {
        ui_scale: scale, ..g
    });
}
```

##### Type references

- `UiScale` — the 0.75x–2.00x ladder in 0.10 steps. Non-finite input resolves to
  `DEFAULT`.
- `WindowGeometry` — the whole persisted window state as one value.
- `SolvedWindowLayout` — the three pane widths after clamping and the centre floor.
- `WindowStorage` — your persistence seam. It is `Send + Sync`, so a host can share it.

##### Return values

- `solve_window_layout` guarantees the centre pane keeps `CENTER_PANE_MIN_W`.
- `solve_clamped_pane_width` lets `min_px` win when `max_px < min_px`, and sanitises
  negative or non-finite input.
- `auto_fit_pane_width` returns a width inside `[min_w, max_w]`.
- `compose_with_dpi` returns the effective multiplier for the OS scale.

##### Side effects

- `NoopWindowStorage` ignores saves and returns `None` on load.
- `MemoryWindowStorage` keeps one slot behind a lock. Nothing touches the filesystem.

##### References

[`geometry/window_geometry.rs`](../src/hoarfrost/geometry/window_geometry.rs),
[`geometry/pane_solver.rs`](../src/hoarfrost/geometry/pane_solver.rs)

---

#### `layout::density`, `layout::emphasis`, `layout::footer`, `layout::optical`

##### Overview

These four modules turn layout rules into machine-checkable reports: one rung per row
(`UIF-PRIN-006` density), one focal point per container, the three ordered footer slots,
and optical glyph centring. Each is a pure checker over a small input struct.

##### Prerequisites & Requirements

- Report the real measured height of each control. A declared rung with no measurement
  hides the violation the check exists to find.
- Declare at most one `Emphasis::High` control per container scope.
- Order footer actions by `FooterSlot::ORDER`: `Utility`, then `Cancel`, then `Confirm`.
- A destructive footer plan MUST offer a `Cancel` and MUST start focus on it.
- Supply real `OpticalInset` measurements, or use `OpticalInset::ZERO` and accept
  geometric centring.

##### Syntax / Method Signature

```rust
// density
pub enum ControlDensity { Compact, Standard }
impl ControlDensity { pub fn from_height(height: f32) -> Option<Self>; }
pub enum DensityBand { Navbar, FormRow, OverflowToolbar, FilterBar, DialogFooter }
pub struct RowSlot<'a> { pub name: &'a str, pub height: f32 }
impl RowSlot<'_> { pub fn rung(&self) -> Option<ControlDensity>; }
pub struct DensityViolation<'a> {
    pub band: DensityBand, pub declared: ControlDensity, pub control: &'a str,
    pub height: f32, pub rung: Option<ControlDensity>,
}
pub struct DensityReport<'a> {
    pub band: DensityBand, pub declared: ControlDensity,
    pub violations: Vec<DensityViolation<'a>>, pub rung_spread: Vec<ControlDensity>,
}
impl DensityReport<'_> {
    pub fn is_homogeneous(&self) -> bool;
    pub fn mixes_rungs(&self) -> bool;
    pub fn off_rung_count(&self) -> usize;
}
pub fn check_row_density<'a>(band: DensityBand, declared: ControlDensity,
                              slots: &[RowSlot<'a>]) -> DensityReport<'a>;
pub fn overflow_toolbar_declared_density() -> Option<ControlDensity>;
pub fn form_field_declared_density() -> Option<ControlDensity>;
pub fn check_overflow_toolbar<'a>(controls: &[RowSlot<'a>]) -> Option<DensityReport<'a>>;
pub fn check_form_field_row<'a>(controls: &[RowSlot<'a>]) -> Option<DensityReport<'a>>;

// emphasis
pub enum Emphasis { High, Medium, Quiet }
pub enum ContainerScope { ModalFooter, ToolbarCluster, Card, Form }
pub struct EmphasisSlot<'a> { pub name: &'a str, pub emphasis: Emphasis }
impl EmphasisSlot<'_> {
    pub fn of_button_variant(name: &'a str, variant: ButtonVariant) -> Self;
    pub fn of_dialog_role(name: &'a str, role: DialogButtonRole) -> Self;
}
impl From<ButtonVariant> for Emphasis;
impl From<DialogButtonRole> for Emphasis;
pub struct FocalPointReport<'a> { pub scope: ContainerScope, pub high: Vec<EmphasisSlot<'a>>,
                                  pub total: usize }
impl FocalPointReport<'_> {
    pub fn is_single_focal_point(&self) -> bool;
    pub fn excess(&self) -> usize;
    pub fn high_names(&self) -> Vec<&'a str>;
}
pub fn check_focal_point<'a>(scope: ContainerScope, slots: &[EmphasisSlot<'a>])
    -> FocalPointReport<'a>;

// footer
pub enum FooterSlot { Utility, Cancel, Confirm }
impl FooterSlot {
    pub const ORDER: [Self; 3];
    pub const ALL: [Self; 3];
    pub const fn position(self) -> u8;
}
pub const GROUP_GAP: f32 = space::SM;
pub enum FooterAlignment { FarLeft, Right }
pub enum FooterKind { Plain, Destructive }
pub struct FooterAction<'a> {
    pub name: &'a str, pub slot: FooterSlot, pub emphasis: Emphasis,
    pub height: Option<f32>,
}
pub enum FooterViolation<'a> {
    CancelIsHighEmphasis { control: &'a str },
    ConfirmIsNotHighEmphasis { control: &'a str, actual: &'static str },
    MultipleHighEmphasis { controls: Vec<&'a str> },
    OutOfOrder { earlier: &'a str, later: &'a str },
    DestructiveWithoutInitialFocus,
    DestructiveWithoutCancel,
    InitialFocusUnknown { focused: &'a str },
    InitialFocusNotCancel { focused: &'a str, expected: &'a str },
}
pub struct FooterPlan<'a> {
    pub kind: FooterKind, pub density: ControlDensity, pub scope: ContainerScope,
    pub actions: Vec<FooterAction<'a>>, pub initial_focus: Option<&'a str>,
}
impl FooterPlan<'_> {
    pub fn focused_action(&self) -> Option<&FooterAction<'_>>;
    pub fn cancel_action(&self) -> Option<&FooterAction<'_>>;
    pub fn confirm_action(&self) -> Option<&FooterAction<'_>>;
    pub fn effective_height(&self, action: &FooterAction<'_>) -> f32;
    pub fn density_report(&self) -> DensityReport<'_>;
    pub fn validate(&self) -> Vec<FooterViolation<'_>>;
    pub fn is_valid(&self) -> bool;
}

// optical
pub const GLYPH_SLOT_SM: f32 = size::ICON_MD;
pub const GLYPH_SLOT_MD: f32 = size::HIT_TARGET_MIN;
pub struct OpticalInset { pub left: f32, pub top: f32, pub right: f32, pub bottom: f32 }
impl OpticalInset {
    pub const ZERO: Self;
    pub const fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self;
    pub fn exceeds(&self, slot_width: f32, slot_height: f32) -> bool;
    pub fn is_symmetric(&self) -> bool;
}
pub struct OpticalShift { pub x: f32, pub y: f32 }
pub fn optical_box(slot_width: f32, slot_height: f32, inset: OpticalInset) -> Rectangle;
pub fn optical_shift(inset: OpticalInset) -> OpticalShift;
```

##### Examples

```rust
use hoarfrost::layout::density::{check_row_density, ControlDensity, DensityBand, RowSlot};

let report = check_row_density(DensityBand::FilterBar, ControlDensity::Standard,
    &[RowSlot { name: "search", height: 32.0 }, RowSlot { name: "sort", height: 28.0 }]);
assert!(!report.is_homogeneous());        // 32 and 28 are different rungs
```

##### Type references

- `DensityBand` — the five rows the spec fixes a rung for.
- `RowSlot` — one control's name and measured height.
- `EmphasisSlot` — one control's declared emphasis. The two `of_*` constructors map a
  button variant or a dialog role onto it.
- `FooterPlan` — the whole footer as data, including initial focus. `validate` is the
  rule; `is_valid` is the assertion.
- `OpticalInset` — the four-edge gap between a glyph's em box and its ink.

##### Return values

- `check_row_density` returns a report even for a clean row. Read `violations`.
- `check_overflow_toolbar` and `check_form_field_row` return `None` when there is
  nothing to check, so an empty row is not reported as a violation.
- `check_focal_point` returns the report; `excess()` is how many `High` controls to drop.
- `validate` returns an empty `Vec` for a legal footer.
- `optical_shift` returns `(0.0, 0.0)` for `OpticalInset::ZERO`.

##### Side effects

None. Every function is pure and every report implements `Display`, so a failing test
prints a readable reason.

##### References

[`layout/density.rs`](../src/hoarfrost/layout/density.rs),
[`layout/emphasis.rs`](../src/hoarfrost/layout/emphasis.rs),
[`layout/footer.rs`](../src/hoarfrost/layout/footer.rs),
[`layout/optical.rs`](../src/hoarfrost/layout/optical.rs)

---

#### `a11y` — accessibility contracts and tab-stop rules

##### Overview

`a11y::contract` is the framework's accessibility model: a role plus the state a screen
reader needs. `a11y::project` derives a contract from a widget model, so the state cannot
drift from the widget. `a11y::tab_stop` checks the single-tab-stop rule for composites
(`UIF-PRIN-008`).

##### Prerequisites & Requirements

- Project from the model. Do not hand-build a contract for a widget that has a
  `project_*` function.
- Set only the fields your role uses. `is_bare()` catches a contract with no state at all.
- Declare the composite's category and edge behaviour honestly. `check_composite` reports
  a mismatch rather than guessing.
- `project_*` functions are gated by the same feature as their widget.

##### Syntax / Method Signature

```rust
pub enum A11yRole { Button, Checkbox, Radio, RadioGroup, Group, Menu, MenuItem,
                    MenuItemCheckBox, MenuItemRadio, Separator, Heading, TreeItem,
                    TextBox, ProgressBar }
pub enum Checked { Unchecked, Indeterminate, Checked }
pub enum A11yValue<'a> { Text(&'a str), Number(i64), Fraction(f32) }
pub struct A11yContract<'a> {
    pub role: A11yRole,
    pub name: Option<&'a str>,
    pub value: Option<A11yValue<'a>>,
    pub expanded: Option<bool>,
    pub selected: Option<bool>,
    pub checked: Option<Checked>,
    pub busy: Option<bool>,
    pub invalid: Option<bool>,
    pub disabled: Option<bool>,
}
impl A11yContract<'_> {
    pub fn filled_count(&self) -> usize;
    pub fn is_bare(&self) -> bool;
}

pub enum EdgeBehaviour { Wraps, Clamps }
pub enum CompositeCategory { BoundedGroup, SpatialCollection }
pub struct CompositeBoundary<'a> {
    pub id: &'a str,
    pub category: CompositeCategory,
    pub edge: EdgeBehaviour,
    pub member_count: usize,
    pub enabled: &'a [usize],
    pub active: Option<usize>,
}
impl CompositeBoundary<'_> {
    pub fn entry_member(&self) -> Option<usize>;
    pub fn next_within(&self, direction: RoveDirection) -> Option<usize>;
}
pub enum TabStopViolation<'a> {
    WrongEdgeBehaviour { id: &'a str, category: CompositeCategory,
                         declared: EdgeBehaviour, required: EdgeBehaviour },
    EnabledExceedsMembers { id: &'a str, member_count: usize, enabled_count: usize },
    ActiveMemberNotEnabled { id: &'a str, active: usize },
}
pub fn check_composite<'a>(boundary: &CompositeBoundary<'a>) -> Vec<TabStopViolation<'a>>;
pub fn check_roster<'a>(boundaries: &[CompositeBoundary<'a>]) -> Vec<TabStopViolation<'a>>;
pub fn total_tab_stops(boundaries: &[CompositeBoundary<'_>]) -> usize;
pub fn total_members(boundaries: &[CompositeBoundary<'_>]) -> usize;

// project_* — one per widget model (each gated by that widget's feature)
pub fn project_button(model: &ButtonModel) -> A11yContract<'_>;
pub fn project_checkbox(model: &CheckboxModel) -> A11yContract<'_>;
pub fn project_radio_option<Id>(option: &RadioOption<Id>, selected: bool) -> A11yContract<'_>;
pub fn project_radio_group<Id>(model: &RadioGroupModel<Id>) -> A11yContract<'_>;
pub fn project_segmented<Id>(model: &SegmentedModel<Id>) -> A11yContract<'_>;
pub fn project_segment<Id>(model: &SegmentedModel<Id>, index: usize) -> Option<A11yContract<'_>>;
pub fn project_menu_row(row: &Row) -> A11yContract<'_>;
pub fn project_menu_row_open(row: &Row, is_open: bool) -> A11yContract<'_>;
pub fn project_tree_row<Id>(row: &VisibleRow<Id>) -> A11yContract<'_>;
pub fn project_chip<Id>(model: &ChipModel<Id>) -> A11yContract<'_>;
pub fn project_text_input(model: &TextInputModel) -> A11yContract<'_>;
pub fn project_progress(model: &ProgressModel) -> A11yContract<'_>;
pub fn project_disclosure(toggle: &DisclosureToggle) -> A11yContract<'_>;
pub fn project_tab_item(tab: &TabItem, is_active: bool) -> A11yContract<'_>;
pub fn project_nav_rail_destination<DestId>(destination: &NavDestination<DestId>,
                                            is_selected: bool) -> A11yContract<'_>;
pub fn project_stepper_step<StepId>(step: &StepItem<StepId>, is_active: bool) -> A11yContract<'_>;
pub fn project_grid_tile<Id>(tile: &GridTile<Id>, is_selected: bool) -> A11yContract<'_>;
pub fn project_command_palette_item<ItemId>(item: &PaletteItem<ItemId>,
                                            is_selected: bool) -> A11yContract<'_>;
```

##### Examples

```rust
use hoarfrost::a11y::{check_composite, CompositeBoundary, CompositeCategory, EdgeBehaviour};

let boundary = CompositeBoundary {
    id: "tabs", category: CompositeCategory::BoundedGroup, edge: EdgeBehaviour::Wraps,
    member_count: 3, enabled: &[0, 2], active: Some(2),
};
assert!(check_composite(&boundary).is_empty());
```

##### Type references

- `A11yRole` — the role a screen reader announces. It is the framework's own vocabulary,
  not an iced type.
- `A11yValue` — the value channel: text, an integer, or a fraction for a progress bar.
- `CompositeCategory` — a bounded group (tabs, a radio group) or a spatial collection
  (a grid, a tree). The category fixes the required edge behaviour.
- `CompositeBoundary` — one composite's roster as data, so the rule is testable without a
  window.

##### Return values

- `is_bare()` is `true` when no state field is set, which is almost always a bug.
- `check_composite` returns an empty `Vec` for a legal composite.
- `total_tab_stops` counts one stop per composite, not one per member.
- `next_within` returns `None` when the boundary has no enabled member.

##### Side effects

None. Every function is pure.

##### References

[`a11y/contract.rs`](../src/hoarfrost/a11y/contract.rs),
[`a11y/project.rs`](../src/hoarfrost/a11y/project.rs),
[`a11y/tab_stop.rs`](../src/hoarfrost/a11y/tab_stop.rs)

### Utility Functions

These are the crate's pure helpers: the microcopy checkers, the validation pillar, the
verification gates, and the headless test harness. None of them owns state you must
drive.

#### `microcopy` — label rules as checkers (`UIF-PRIN-007`)

##### Overview

`microcopy` turns the spec's label rules into machine-checkable verdicts: the action
ellipsis, verb-first labels, sentence case, the closed literal pin table, tabular
numerals, and truncation with a full-text tooltip. Each checker takes a small input
struct and returns a report that implements `Display`.

##### Prerequisites & Requirements

- Run the checkers in your own tests over every label you ship. They are checkers, not
  runtime guards.
- Pass a `PromptBehaviour` with each action label. The label alone cannot say whether a
  prompt follows.
- Use `...` (three ASCII dots) as the action suffix. `U+2026` is reserved for truncation,
  placeholders, and breadcrumb elision.
- Pass your licensed proper nouns to `classify_with`. `DEFAULT_LICENSED` is a framework
  default, not a spec list.
- Do not invent a pin. `HOST_PIN_TABLE` is closed and matched by exact, case-sensitive
  equality.
- Truncate paths, URIs, and git refs in the middle. Truncate prose at the end.

##### Syntax / Method Signature

```rust
// action_label
pub const ASCII_ACTION_ELLIPSIS: &str = "...";
pub const UNICODE_ELLIPSIS: char = '\u{2026}';
pub enum PromptBehaviour { Immediate, LaunchesPrompt }
impl PromptBehaviour { pub const fn requires_action_ellipsis(self) -> bool; }
pub struct ActionLabel<'a> { pub label: &'a str, pub behaviour: PromptBehaviour }
pub enum EllipsisViolation<'a> {
    MissingActionEllipsis { label: &'a str },
    SpuriousActionEllipsis { label: &'a str },
    SpacedActionEllipsis { label: &'a str },
    UnicodeEllipsisAsActionSuffix { label: &'a str },
}
pub struct EllipsisVerdict<'a> {
    pub label: &'a str, pub behaviour: PromptBehaviour,
    pub violation: Option<EllipsisViolation<'a>>, pub exempt: bool,
}
impl EllipsisVerdict<'_> { pub fn is_compliant(&self) -> bool; }
pub fn ends_with_ascii_ellipsis(label: &str) -> bool;
pub fn ends_with_unicode_ellipsis(label: &str) -> bool;
pub fn is_spaced_ellipsis(label: &str) -> bool;
pub fn check_action_ellipsis<'a>(action: &ActionLabel<'a>) -> EllipsisVerdict<'a>;
pub const GENERIC_CTA_LABELS: [&str; 4] = ["ok", "okay", "yes", "no"];
pub fn is_generic_cta(label: &str) -> bool;
pub struct VerbFirstVerdict<'a> { pub label: &'a str, pub generic: bool, pub exempt: bool }
impl VerbFirstVerdict<'_> { pub fn is_compliant(&self) -> bool; }
pub fn check_verb_first(label: &str) -> VerbFirstVerdict<'_>;
pub fn check_action<'a>(action: &ActionLabel<'a>) -> (EllipsisVerdict<'a>, VerbFirstVerdict<'a>);

// case_rule
pub enum WordCase { Uncased, Lower, Title, Upper, Mixed }
impl WordCase { pub fn of(word: &str) -> Self; }
pub enum Capitalisation { Uncasable, Sentence, AllCaps, MidWordCapital,
                          FirstWordLowercase, TitleCase }
pub const SPEC_NAMED_LICENSED: [&str; 4] = ["Markdown", "PDF", "MCP", "JSON"];
pub const DEFAULT_LICENSED: &[&str] = /* AI, API, CSV, DAV, Git, GitHub, HTML, ID, JMAP,
    JSON, MCP, Markdown, OAuth, PDF, TOML, UI, URI, URL, Windows, XML, YAML */;
pub struct CaseReport<'a> {
    pub label: &'a str, pub capitalisation: Capitalisation,
    pub exempt: bool, pub pinned_by: Option<&'static str>,
}
impl CaseReport<'_> { pub fn is_compliant(&self) -> bool; }
pub fn classify_with(label: &str, licensed: &[&str]) -> Capitalisation;
pub fn classify(label: &str) -> Capitalisation;
pub fn check_label(label: &str) -> CaseReport<'_>;
pub fn check_label_with<'a>(label: &'a str, licensed: &[&str]) -> CaseReport<'a>;

// pins
pub enum PinExemption { SentenceCase, VerbFirst, Ellipsis }
pub struct PinnedLabel { pub literal: &'static str, pub pinned_by: &'static str,
                         pub exempts: &'static [PinExemption] }
pub const HOST_PIN_TABLE: &[PinnedLabel];      // nine literals, closed
pub fn pin_for(label: &str) -> Option<&'static PinnedLabel>;
pub fn is_pinned(label: &str) -> bool;
pub fn is_exempt(label: &str, exemption: PinExemption) -> bool;
pub const HOST_PIN_REQUIREMENTS: &str = "UI-009..UI-034, UI-062";

// tabular
pub enum TabularNums { Required, NotRequired }
pub enum ReadoutKind { Counter, Percentage, Timestamp, PaginationRange, NumericColumn }
pub struct NumericReadout<'a> {
    pub id: &'a str, pub kind: ReadoutKind, pub tabular: TabularNums,
    pub header_right_aligned: bool, pub cell_right_aligned: bool,
}
pub enum TabularViolation<'a> {
    NotTabular { id: &'a str, kind: ReadoutKind },
    HeaderNotRightAligned { id: &'a str },
    CellNotRightAligned { id: &'a str },
}
pub fn check_numeric_readout<'a>(readout: &NumericReadout<'a>) -> Vec<TabularViolation<'a>>;

// truncate
pub const ELLIPSIS: char = UNICODE_ELLIPSIS;
pub const SEGMENT_SEPARATORS: [char; 2] = ['/', '\\'];
pub enum TruncationMode { End, Middle }
impl TruncationMode { pub const fn slug(self) -> &'static str; }
pub struct Truncated { pub text: String, pub mode: TruncationMode, pub was_truncated: bool,
                       pub head_kept: bool, pub tail_preserved: bool }
pub fn truncate_end(text: &str, budget: usize) -> Truncated;
pub fn truncate_middle(text: &str, budget: usize) -> Truncated;
pub fn truncate(text: &str, budget: usize, mode: TruncationMode) -> Truncated;
pub struct LabelSlot<'a> { pub id: &'a str, pub text: &'a str, pub tooltip: Option<&'a str> }
pub enum TruncationTooltipViolation<'a> {
    MissingTooltip { id: &'a str, full: &'a str },
    TooltipNotFullText { id: &'a str, full: &'a str, tooltip: &'a str },
}
pub fn check_truncation_tooltip<'a>(slot: &LabelSlot<'a>, mode: TruncationMode,
                                    budget: usize) -> Vec<TruncationTooltipViolation<'a>>;
```

##### Examples

```rust
use hoarfrost::microcopy::action_label::{check_action_ellipsis, ActionLabel, PromptBehaviour};

let verdict = check_action_ellipsis(&ActionLabel { label: "Rename", behaviour: PromptBehaviour::LaunchesPrompt });
assert!(!verdict.is_compliant());     // a prompt follows, so the label needs "..."
```

##### Type references

- `ActionLabel` — a label plus what activating it does. Both halves are required.
- `Capitalisation` — the label-level classification the sentence-case rule reads.
- `WordCase` — the per-word classification `Capitalisation` is built from.
- `PinnedLabel` — one spec-pinned literal and the rules it escapes.
- `Truncated` — the shortened text plus what the truncation preserved.

##### Return values

- Every verdict exposes `is_compliant()`. A pinned label is compliant because `exempt`
  is set, not because the checker was skipped.
- `check_action` returns both verdicts at once, which is what a label audit loop wants.
- `truncate_middle` keeps the head and the tail and marks `tail_preserved`.
- `check_truncation_tooltip` returns an empty `Vec` when the text fits the budget.

##### Side effects

None. Every function is pure.

##### References

[`microcopy/action_label.rs`](../src/hoarfrost/microcopy/action_label.rs),
[`case_rule.rs`](../src/hoarfrost/microcopy/case_rule.rs),
[`pins.rs`](../src/hoarfrost/microcopy/pins.rs),
[`tabular.rs`](../src/hoarfrost/microcopy/tabular.rs),
[`truncate.rs`](../src/hoarfrost/microcopy/truncate.rs)

---

#### `validation` — reward early, punish late (`UIF-PRIN-009`, `UIF-PRIN-010`)

##### Overview

`validation` implements the validation pillar. `stance` tracks whether the user has
committed to a field. `disallowed` classifies characters that are never legal. `punish`
combines both into one decision. `recovery` snapshots an editor so a cancelled commit
restores exactly what the user typed.

##### Prerequisites & Requirements

- Track `EditStance` per field. Move it to `Touched` on `on_commit`, `blur`, or submit.
- Paint the invalid state only when `Punishment::painted()` is `true`. A pristine field
  that has never been committed to MUST NOT go red.
- Block submit whenever `Punishment::blocks_submit()` is `true`, even when nothing is
  painted.
- Refuse disallowed characters immediately. That is the one instant-punish case.
- Do not invent a `DisallowedClass`. The enum is closed and models one class only.
- Snapshot before a commit that can fail. Restore through `cancel_into_*`.

##### Syntax / Method Signature

```rust
pub enum EditStance { Pristine, Touched }
impl EditStance {
    pub const ALL: [Self; 2];
    pub const fn is_pristine(self) -> bool;
    pub const fn is_touched(self) -> bool;
}

pub const FILENAME_ILLEGAL_CHARS: &str = "/\\:*?\"<>|";
pub enum DisallowedClass { FilenameBasename }
impl DisallowedClass {
    pub const ALL: [Self; 1];
    pub const fn classify(self, character: char) -> Option<DisallowedKind>;
    pub fn first_offence(self, value: &str) -> Option<DisallowedChar>;
    pub fn offences(self, value: &str) -> impl Iterator<Item = DisallowedChar> + '_;
    pub fn contains_any(self, value: &str) -> bool;
}
pub enum DisallowedKind { IllegalPathCharacter, ControlCharacter }
impl DisallowedKind { pub const ALL: [Self; 2]; }
pub struct DisallowedChar { pub character: char, pub offset: usize, pub kind: DisallowedKind }

pub enum PunishReason { Valid, WithheldPristine, DisallowedImmediate, TouchedInvalid }
pub struct Punishment;            // fields are private; read the accessors
impl Punishment {
    pub const fn invalid(&self) -> bool;
    pub const fn painted(&self) -> bool;
    pub const fn reason(&self) -> PunishReason;
    pub const fn blocks_submit(&self) -> bool;
}
pub const fn punish(stance: EditStance, invalid: bool, disallowed: bool) -> Punishment;

pub struct ScrollOffset(f32);
impl ScrollOffset { pub fn new(pixels: f32) -> Self; pub fn is_zero(self) -> bool; }
pub struct RestoredFields;
impl RestoredFields { pub fn count(self) -> usize; }
pub struct EditSnapshot;
impl EditSnapshot {
    pub fn new(buffer: impl Into<String>, selection: Option<TextRange>,
               scroll: Option<ScrollOffset>) -> Self;
    pub fn from_text_input(model: &TextInputModel) -> Self;
    pub fn from_text_area(model: &TextAreaModel) -> Self;
    pub fn buffer(&self) -> &str;
    pub fn carried_count(&self) -> usize;
    pub fn restore_text_input(&self, model: &mut TextInputModel) -> RestoredFields;
    pub fn restore_text_area(&self, model: &mut TextAreaModel) -> RestoredFields;
}
pub struct RecoverySession;
impl RecoverySession {
    pub fn new() -> Self;
    pub fn is_open(&self) -> bool;
    pub fn begin(&mut self, snapshot: &EditSnapshot) -> GestureStart;
    pub fn begin_from_text_input(&mut self, model: &TextInputModel) -> GestureStart;
    pub fn begin_from_text_area(&mut self, model: &TextAreaModel) -> GestureStart;
    pub fn cancel(&mut self) -> Option<Cancel<EditSnapshot>>;
    pub fn cancel_into_text_input(&mut self, model: &mut TextInputModel) -> Option<RestoredFields>;
    pub fn cancel_into_text_area(&mut self, model: &mut TextAreaModel) -> Option<RestoredFields>;
    pub fn settle(&mut self) -> bool;
}
```

##### Examples

```rust
use hoarfrost::validation::{disallowed::DisallowedClass, punish::punish, EditStance};

let bad = DisallowedClass::FilenameBasename.first_offence("a/b.md");
let p = punish(EditStance::Pristine, true, bad.is_some());
assert!(p.invalid() && p.painted());      // an illegal character is punished at once
```

##### Type references

- `EditStance` — whether the user has committed to the field. Two states only.
- `DisallowedChar` — the offending character, its byte offset, and which rule it broke.
- `Punishment` — the whole decision: the fact, whether to paint, and the reason.
- `EditSnapshot` — buffer, selection, and scroll as one value.
- `RecoverySession` — one open commit gesture, with exactly one `Cancel`.

##### Return values

- `punish` is `const` and total. It never panics on any input combination.
- `WithheldPristine` means the value is invalid but nothing is painted. Submit is still
  blocked.
- `cancel*` methods return `None` when no gesture is open.
- `settle` returns whether it closed an open gesture.
- `RestoredFields::count()` says how many fields actually changed.

##### Side effects

- `RecoverySession::begin*` opens a gesture and remembers the snapshot.
- `cancel_into_*` writes the snapshot back into the model you pass.
- Nothing in this module reads a clock or the filesystem.

##### References

[`validation/stance.rs`](../src/hoarfrost/validation/stance.rs),
[`disallowed.rs`](../src/hoarfrost/validation/disallowed.rs),
[`punish.rs`](../src/hoarfrost/validation/punish.rs),
[`recovery.rs`](../src/hoarfrost/validation/recovery.rs)

---

#### `invariant_gate` — the verification gates (`UIF-VER-001..005`)

##### Overview

`invariant_gate` wraps the layout and microcopy rules in one result type so a test can
collect violations instead of panicking on the first one. Each `check_*` returns
`Result<(), InvariantViolation>`. `audit_invariants` runs the whole set and reports.

##### Prerequisites & Requirements

- Call the `check_*` functions from tests, not from a render path.
- Record every result into one `InvariantReport` and assert `is_clean()` once.
- Treat a violation as a build failure. The gate exists so a rule cannot be skipped
  silently.
- Pass `is_chrome` to `check_sentence_case`. Chrome labels and content labels take
  different rules.

##### Syntax / Method Signature

```rust
pub enum InvariantCategory { SentenceCase, ActionEllipsis, EmphasisFocalPoint, RowDensity,
                             TooltipObligation, SecretRedaction, ValidationPunishment }
pub struct InvariantViolation { pub category: InvariantCategory, pub target: String,
                                pub message: String }
impl InvariantViolation {
    pub fn new(category: InvariantCategory, target: impl Into<String>,
               message: impl Into<String>) -> Self;
}
pub struct InvariantReport { pub total_checked: usize, pub violations: Vec<InvariantViolation> }
impl InvariantReport {
    pub fn new() -> Self;
    pub fn record_check(&mut self);
    pub fn record_result(&mut self, result: Result<(), InvariantViolation>);
    pub fn is_clean(&self) -> bool;
    pub fn violation_count(&self) -> usize;
    pub fn describe(&self) -> String;
}

pub fn check_sentence_case(label: &str, is_chrome: bool) -> Result<(), InvariantViolation>;
pub fn check_action_ellipsis(label: &str, behaviour: PromptBehaviour) -> Result<(), InvariantViolation>;
pub fn check_navbar_trigger_ellipsis(trigger: &NavBarMenuTrigger) -> Result<(), InvariantViolation>;
pub fn check_emphasis_focal_point<'a>(scope: ContainerScope, slots: &[EmphasisSlot<'a>])
    -> Result<(), InvariantViolation>;
pub fn check_row_density<'a>(declared: Option<ControlDensity>, slots: &[RowSlot<'a>])
    -> Result<(), InvariantViolation>;
pub fn check_label_truncation_tooltip<'a>(slot: &LabelSlot<'a>) -> Result<(), InvariantViolation>;
pub fn check_icon_only_button_tooltip(is_icon_only: bool, tooltip_label: Option<&str>)
    -> Result<(), InvariantViolation>;
pub fn check_secret_redaction<T: fmt::Debug + fmt::Display>(secret: &T, plaintext: &str)
    -> Result<(), InvariantViolation>;
pub fn check_validation_transition(stance: EditStance, invalid: bool, disallowed: bool) -> Punishment;
pub fn audit_invariants() -> InvariantReport;
```

##### Examples

```rust
use hoarfrost::invariant_gate::{check_sentence_case, InvariantReport};

let mut report = InvariantReport::new();
for label in ["Save", "Rename..."] {
    report.record_result(check_sentence_case(label, true));
}
assert!(report.is_clean(), "{}", report.describe());
```

##### Type references

- `InvariantCategory` — which pillar a violation belongs to.
- `InvariantViolation` — category, target, and a human-readable message. `Display` is
  implemented, so a test failure prints the reason.
- `InvariantReport` — the accumulator. `total_checked` counts checks, so a clean report
  with zero checks is itself a signal.

##### Return values

- Every `check_*` returns `Ok(())` or one `Err(InvariantViolation)`.
- `check_validation_transition` returns a `Punishment`, not a `Result`. It asserts the
  punishment table by construction.
- `describe()` returns a multi-line report body.

##### Side effects

- `record_check` and `record_result` mutate the report only.
- `audit_invariants` builds a fresh report each call and caches nothing.

##### References

[`invariant_gate.rs`](../src/hoarfrost/invariant_gate.rs)

---

#### `token_lint` — the tokenization audit (`UIF-PRIN-001`)

##### Overview

`token_lint` scans the crate's own source and enforces two rules: no colour constructor
outside `token/color.rs`, and no bare numeric literal on a visually-named constant unless
it is a registered Surface Geometry Constant or an allow-listed entry. It is a build-time
gate, run from a test.

##### Prerequisites & Requirements

- Define colours only in `token/color.rs`. Use the four constructors listed in
  `COLOUR_CONSTRUCTORS` nowhere else.
- Bind a visual measurement to a token rung. If the spec fixes the number, register the
  constant in `SURFACE_GEOMETRY_CONSTANTS` with its spec reference.
- Name a visual constant so `VISUAL_NAME_MARKERS` matches it. A name like `_H` instead of
  `_HEIGHT` is invisible to the gate.
- Add an `ALLOW_LIST` entry only with a written reason. A stale entry fails the audit.
- Run `audit_crate()` in a test and assert `Audit::is_clean()`.

##### Syntax / Method Signature

```rust
pub enum Rule { ColourLiteral, UnboundVisualScalar }
pub enum ScalarClass { TokenBound, Zero, Ratio, SurfaceBound, Violation, Unreadable }
pub struct Violation { pub rule: Rule, pub file: String, pub line: usize,
                       pub code: String, pub detail: String }
impl Violation {
    pub fn key(&self) -> String;
    pub fn constant_name(&self) -> Option<&str>;
}
pub struct SourceScan {
    pub findings: Vec<Violation>, pub token_layer_colour_definitions: usize,
    pub token_bound: usize, pub zero: usize, pub ratio: usize,
    pub surface_bound: usize, pub unreadable: usize,
}
pub struct Scope<'a> {
    pub colour_definition_file: &'static str,
    pub visual_scope_prefixes: &'a [&'static str],
    pub surface_geometry: &'a [SurfaceGeometryConstant],
}
pub fn crate_scope<'a>() -> Scope<'a>;
pub const COLOUR_DEFINITION_FILE: &str = "token/color.rs";
pub const VISUAL_SCOPE_DIRS: [&str; 3] = ["widget/", "surface/", "composite/"];
pub const VISUAL_NAME_MARKERS: [&str; 10] = ["HEIGHT", "WIDTH", "PADDING", "RADIUS",
    "STROKE", "GAP", "SIZE", "INSET", "MARGIN", "SLOT"];
pub const COLOUR_CONSTRUCTORS: [&str; 4] = ["Color::from_rgb", "Color::from_rgba",
    "Color::from_rgb8", "Color::from_rgba8"];
pub struct SurfaceGeometryConstant { pub file: &'static str, pub name: &'static str,
                                     pub spec: &'static str }
pub const SURFACE_GEOMETRY_CONSTANTS: &[SurfaceGeometryConstant];
pub fn surface_geometry(file: &str, name: &str) -> Option<&'static SurfaceGeometryConstant>;
pub fn lookup_geometry<'s>(registry: &'s [SurfaceGeometryConstant], file: &str, name: &str)
    -> Option<&'s SurfaceGeometryConstant>;
pub struct AllowEntry { pub key: &'static str, pub reason: &'static str }
pub const ALLOW_LIST: &[AllowEntry];
pub struct Audit {
    pub violations: Vec<Violation>,
    pub suppressed: Vec<(Violation, &'static str)>,
    pub stale_allow_entries: Vec<&'static str>,
    pub stats: ScanStats,
}
impl Audit { pub fn is_clean(&self) -> bool; pub fn describe(&self) -> String; }
pub struct ScanStats {
    pub files_scanned: usize, pub rule_a_files: usize, pub rule_b_files: usize,
    pub token_layer_colour_definitions: usize, pub token_bound: usize, pub zero: usize,
    pub ratio: usize, pub surface_bound: usize, pub unreadable: usize,
}
pub fn scan_source_in(scope: &Scope<'_>, file: &str, text: &str) -> SourceScan;
pub fn scan_source(file: &str, text: &str) -> SourceScan;
pub fn scan_tree(root: &Path, scope: &Scope<'_>, allow: &[AllowEntry]) -> Audit;
pub fn scan_and_audit(root: &Path) -> Audit;
pub fn crate_root() -> PathBuf;
pub fn audit_crate() -> Audit;
pub fn render(audit: &Audit) -> String;
pub fn render_scoped(label: &str, colour_file: &str, audit: &Audit) -> String;
pub fn source_files(root: &Path) -> Vec<PathBuf>;
pub fn is_test_file(file: &str) -> bool;
pub fn in_visual_scope(file: &str) -> bool;
pub fn in_scope(file: &str, prefixes: &[&str]) -> bool;
pub fn is_visual_name(name: &str) -> bool;
pub fn redact_comments_and_strings(text: &str) -> String;
```

##### Examples

```rust
#[test]
fn crate_is_token_clean() {
    let audit = hoarfrost::token_lint::audit_crate();
    assert!(audit.is_clean(), "{}", audit.describe());
}
```

##### Type references

- `Rule` — which of the two rules a finding belongs to.
- `ScalarClass` — how a visually-named constant's expression was classified. `Zero` and
  `Ratio` are legal without registration.
- `SurfaceGeometryConstant` — one registered literal plus the spec section that fixes it.
- `AllowEntry` — one suppressed finding, keyed as `file:CONSTANT_NAME`, with a reason.
- `Audit` — violations, suppressions, stale allow entries, and counters.
- `ScanStats` — the counters that let a report say why it found nothing.

##### Return values

- `Audit::is_clean()` is `false` when anything is unallowed **or** when an allow entry
  matched nothing.
- `scan_source` never reads the filesystem. Pass it text you already hold.
- `unreadable` counts constants the parser could not read. Each is also a finding, so a
  clean scan cannot hide an unparsed constant.

##### Side effects

- `scan_tree`, `scan_and_audit`, and `audit_crate` read source files under a root path.
  They write nothing.
- `redact_comments_and_strings` is pure and is what stops a comment from tripping a rule.

##### References

[`token_lint.rs`](../src/hoarfrost/token_lint.rs)

---

#### `test_helpers` — the headless harness

##### Overview

`test_helpers` gives tests a real iced interface without a window. `Harness` carries
widget state across rebuilds and answers focus and pointer questions. `Capture` renders
frames and compares pixels. Both use iced's headless renderer.

##### Prerequisites & Requirements

- Enable the `test-helpers` feature, or run under `test`.
- Use `NoopWindowStorage`, `MemoryWindowStorage`, `MemoryGeometryStorage`, or a
  `tempfile::TempDir` for anything persistent. Tests MUST NOT touch `%APPDATA%` or
  `USERPROFILE`.
- Prefer `Harness::headless` for behaviour and `Harness::recording` for what was drawn.
- Build one `Capture` per test. Minting iced's headless renderer costs roughly 620 ms in
  a debug build.
- Use `Capture` frames for repaint assertions. Each frame gets a fresh cache, so frames
  do not share widget state.

##### Syntax / Method Signature

```rust
pub const VIEWPORT: Size = Size::new(400.0, 300.0);
pub type El<'a, Message, Renderer> = Element<'a, Message, Theme, Renderer>;
pub type Recorded = Vec<(Option<Id>, Rectangle)>;

pub struct Harness<'a, Message, Renderer>;
impl<'a, Message, Renderer> Harness<'a, Message, Renderer> {
    pub fn headless(view: impl Fn() -> El<'a, Message, HeadlessRenderer> + 'a) -> Self;
    pub fn recording(view: impl Fn() -> El<'a, Message, Recorder> + 'a) -> Self;
    pub fn new(renderer: Renderer, view: impl Fn() -> El<'a, Message, Renderer> + 'a) -> Self;
    pub fn set_view(&mut self, view: impl Fn() -> El<'a, Message, Renderer> + 'a);
    pub fn run<T>(&mut self, operation: Box<dyn Operation<T>>) -> Option<T>;
    pub fn focused(&mut self) -> Option<Id>;
    pub fn is_focused(&mut self, id: &Id) -> bool;
    pub fn focus_count(&mut self) -> operation::focusable::Count;
    pub fn registered(&mut self) -> Vec<Option<Id>>;
    pub fn recorded(&mut self) -> Vec<(Option<Id>, Rectangle)>;
    pub fn candidates(&mut self) -> Vec<Candidate>;
    pub fn focus(&mut self, id: &Id);
    pub fn unfocus(&mut self);
    pub fn iced_focus_next(&mut self);
    pub fn iced_focus_previous(&mut self);
    pub fn sync(&mut self, model: &FocusState);
    pub fn tab(&mut self, model: &mut FocusState, direction: Direction) -> Option<FocusId>;
    pub fn pointer(&mut self, events: &[Event], cursor: mouse::Cursor) -> Vec<Message>;
    pub fn hover(&mut self, at: Point) -> Vec<Message>;
    pub fn press(&mut self, at: Point) -> Vec<Message>;
    pub fn release(&mut self, at: Point) -> Vec<Message>;
    pub fn render(&mut self);
    pub fn paint(&mut self) -> Vec<Paint>;
    pub fn first_bounds(&mut self) -> Rectangle;
}

pub struct RecordFocusables;      // Operation<Recorded>: can Tab stop here?
pub struct RecordCandidates;      // every box and name, tagged by publishing hook
impl RecordCandidates { pub fn candidates(&self) -> Vec<Candidate>; }
pub struct Candidate;             // kind labels: container, scrollable, focusable,
                                  // text_input, text, custom
pub const ALL_KINDS: [&str; 6] = [KIND_CONTAINER, KIND_SCROLLABLE, KIND_FOCUSABLE,
    KIND_TEXT_INPUT, KIND_TEXT, KIND_CUSTOM];
pub struct Paint;
pub struct Recorder;

pub fn iced_id(label: &str) -> Id;
pub fn iced_ids(labels: &[&'static str]) -> Vec<Id>;
pub fn opt_ids(labels: &[&'static str]) -> Vec<Option<Id>>;
pub fn model_of(labels: &[&'static str]) -> (GroupId, FocusState);
pub fn model_with(group: &str, members: Vec<FocusMember>) -> (GroupId, FocusState);
pub fn wrapped(label: &str, enabled: bool) -> El<'static, (), HeadlessRenderer>;
pub fn focusable_tree(labels: &[&'static str]) -> El<'static, (), HeadlessRenderer>;
pub fn mixed_focusable_tree(entries: &[(&'static str, bool)]) -> El<'static, (), HeadlessRenderer>;
pub fn assert_agree<Message: Send + 'static, Renderer: renderer::Renderer>(
    harness: &mut Harness<'_, Message, Renderer>, model: &FocusState, step: &str);
pub fn focus_one<Message: Send + 'static, Renderer: renderer::Renderer>(
    harness: &mut Harness<'_, Message, Renderer>, label: &str);
pub fn moved(to: Point) -> Event;
pub fn pressed() -> Event;
pub fn released() -> Event;
pub fn centre(bounds: Rectangle) -> Point;

pub const PIXEL_SCALE: f32 = 2.0;
pub const INK_THRESHOLD: u32 = 90;
pub const SURFACE_TOLERANCE: u8 = 4;
pub const MIN_CHANGED_PIXELS: usize = 8;
pub fn headless_renderer() -> HeadlessRenderer;
pub fn screenshot<Message: 'static>(size: Size, scale: f32, background: Color,
                                    root: Element<'_, Message>) -> Vec<u8>;
pub fn ink_band(rgba: &[u8], physical_width: u32, scale: f32) -> Option<(f32, f32)>;
pub fn ink_band_between(rgba: &[u8], width: u32, scale: f32, y_min: f32, y_max: f32)
    -> Option<(f32, f32)>;
pub fn physical_width(size: Size, scale: f32) -> u32;
pub fn to_rgb(colour: Color) -> [u8; 3];

pub struct Capture;
impl Capture {
    pub fn new() -> Self;
    pub fn frame<Message: 'static>(&mut self, viewport: Size, root: Element<'_, Message>) -> Frame;
    pub fn frame_at<Message: 'static>(&mut self, viewport: Size, scale: f32,
                                      background: Color, root: Element<'_, Message>) -> Frame;
    pub fn frame_after<Message: 'static>(&mut self, viewport: Size, root: Element<'_, Message>,
                                         events: &[Event]) -> Frame;
    pub fn frame_after_at<Message: 'static>(&mut self, viewport: Size,
        root: Element<'_, Message>, events: &[Event], cursor: mouse::Cursor) -> Frame;
    pub fn click<Message: 'static>(&mut self, root: Element<'_, Message>, at: Point) -> Vec<Message>;
    pub fn click_and_frame<Message: 'static>(&mut self, viewport: Size,
        root: Element<'_, Message>, at: Point) -> (Frame, Vec<Message>);
    pub fn frame_and_messages_after<Message: 'static>(&mut self, viewport: Size,
        root: Element<'_, Message>, events: &[Event], cursor: mouse::Cursor)
        -> (Frame, Vec<Message>);
    pub fn renderer(&self) -> &HeadlessRenderer;
    pub fn renderer_mut(&mut self) -> &mut HeadlessRenderer;
}

pub struct Frame;
impl Frame {
    pub fn new(rgba: Vec<u8>, viewport: Size, scale: f32) -> Self;
    pub fn viewport(&self) -> Size;
    pub fn physical_size(&self) -> Size<u32>;
    pub fn scale(&self) -> f32;
    pub fn pixels(&self) -> &[u8];
    pub fn mask_against(&self, reference: Color, tolerance: u8) -> Vec<bool>;
    pub fn pixel(&self, x: f32, y: f32) -> [u8; 3];
    pub fn ink_count(&self, rect: Rectangle) -> usize;
    pub fn has_ink(&self, rect: Rectangle) -> bool;
    pub fn painted_count(&self, rect: Rectangle, reference: [u8; 3], tolerance: u8) -> usize;
    pub fn matching_count(&self, rect: Rectangle, reference: [u8; 3], tolerance: u8) -> usize;
    pub fn ink_bounds(&self, rect: Rectangle) -> Option<Rectangle>;
    pub fn is_uniform(&self, rect: Rectangle, tolerance: u8) -> bool;
    pub fn differing_pixels(&self, other: &Frame, rect: Rectangle) -> usize;
}
pub fn assert_repainted(before: &Frame, after: &Frame, rect: Rectangle, what: &str);
pub fn panic_text(outcome: &Result<(), Box<dyn Any + Send>>) -> String;
pub fn assert_panics(what: &str, probe: impl FnOnce()) -> String;
```

##### Examples

```rust
use hoarfrost::test_helpers::{focusable_tree, model_of, Harness};

let (_group, mut model) = model_of(&["a", "b", "c"]);
let mut harness = Harness::headless(|| focusable_tree(&["a", "b", "c"]));
assert_eq!(harness.tab(&mut model, Direction::Next).map(|id| id.as_str()), Some("a"));
```

##### Type references

- `Harness` — a stateful headless interface. Every method rebuilds first, so it observes
  the state a running application would have kept.
- `RecordFocusables` — answers "can Tab stop here?".
- `RecordCandidates` — answers "which boxes and names did the operation publish, and
  through which hook?". It implements every publishing hook, so a missing door is visible.
- `Capture` — one renderer, many frames. Frames do not share widget state.
- `Frame` — one captured RGBA buffer plus the pixel queries a paint test needs.

##### Return values

- `tab` returns the `FocusId` the model moved to, or `None`.
- `assert_agree` fails when the model and iced's own focus order disagree.
- `differing_pixels` counts changed pixels in a rect. `MIN_CHANGED_PIXELS` is the floor
  a repaint MUST clear.
- `ink_bounds` returns `None` when a rect holds no ink.
- `assert_panics` returns the panic text so a test can match on the message.

##### Side effects

- `Capture` and `headless_renderer` allocate a wgpu headless renderer.
- `screenshot` and `Capture` write nothing to disk. Return the bytes or assert on them.
- Nothing in this module touches real user paths.

##### References

[`test_helpers.rs`](../src/hoarfrost/test_helpers.rs)

## Configuration Reference

Hoarfrost has no runtime configuration object and no global settings. You configure it at
three points: the Cargo features you enable, the values you pass in, and the seams you
implement. Everything else is a compiled constant.

### Cargo features

`default = []`. The core is always compiled: `token`, `state`, `focus`, `bridge`,
`binding`, `identity`, `strings`, `geometry`, `layout`, `a11y`, `validation`,
`microcopy`, `invariant_gate`, and `token_lint`. Every widget and every surface is
opt-in, one feature per public item.

Enable the feature that matches the item you use. Do not enable `all` to make an import
error go away. You MAY enable `all` in a demo or test crate that compiles every widget on
purpose; the only cost is compile time.

**Atoms**

| Feature | Implies | Public item |
|---|---|---|
| `badge` | — | `widget::badge` |
| `avatar` | `badge` | `widget::avatar` |
| `button` | `tooltip` | `widget::button`, `widget::progress` |
| `progress` | `button` | alias for the progress half of `button` |
| `chip` | — | `widget::chip` |
| `color-picker` | — | `widget::color_picker` |
| `combo-box` | `menu`, `text-input` | `widget::combo_box` (select and combo box) |
| `date-picker` | — | `widget::date_picker` |
| `time-picker` | — | `widget::time_picker` |
| `hyperlink` | `button` | `widget::hyperlink` |
| `number-box` | `button`, `range-model`, `text-input` | `widget::number_box` |
| `password` | `text-input` | `widget::password` |
| `rating` | — | `widget::rating` |
| `search-box` | — | `widget::search_box` |
| `segmented` | `button` | `widget::segmented` |
| `slider` | `button`, `range-model` | `widget::slider` |
| `range-slider` | `range-model` | `widget::range_slider` |
| `split-button` | `button`, `menu` | `widget::split_button` |
| `text-input` | `button` | `widget::text_input` |
| `text-area` | `text-input` | `widget::text_area` |
| `toggle` | `button` | `widget::toggle` (checkbox, radio, switch) |
| `auto-suggest` | `menu`, `text-input`, `tooltip` | `widget::auto_suggest` |

**Composites**

| Feature | Implies | Public item |
|---|---|---|
| `breadcrumb` | `button`, `menu` | `composite::breadcrumb` |
| `card` | `button` | `composite::card` |
| `code-block` | — | `composite::code_block` |
| `command-palette` | `text-input` | `composite::command_palette` |
| `diff-viewer` | — | `composite::diff_viewer` |
| `empty-state` | `toast` | `composite::empty_state` |
| `expander` | `button` | `composite::expander` |
| `form-field` | `button` | `composite::form_field` |
| `grid-view` | `tree-view` | `composite::grid_view` |
| `info-bar` | `button`, `notification` | `composite::info_bar` |
| `keybind-recorder` | — | `composite::keybind_recorder` |
| `list-tile` | `button` | `composite::list_tile` |
| `navbar` | `tooltip` | `composite::navbar` |
| `navigation-rail` | `badge`, `range-model`, `tooltip` | `composite::navigation_rail` |
| `notification` | — | `composite::notification` |
| `overflow-toolbar` | `button`, `menu` | `composite::overflow_toolbar` |
| `pagination` | — | `composite::pagination` |
| `pips-pager` | — | `composite::pips_pager` |
| `scroll-view` | — | `composite::scroll_view` |
| `settings-row` | `button` | `composite::settings_row` |
| `side-sheet` | `zorder` | `composite::side_sheet` |
| `skeleton` | — | `composite::skeleton` |
| `split-divider` | `button` | `composite::split_divider` |
| `stepper` | — | `composite::stepper` |
| `tab-strip` | `menu`, `tooltip` | `composite::tab_strip` |
| `table` | `tree-view`, `virtual-list` | the `composite::table_*` family |
| `toast` | `info-bar`, `notification`, `tooltip`, `zorder` | `composite::toast` |
| `tree-row` | `button`, `list-tile`, `virtual-list` | `composite::tree_row` |
| `tree-view` | `virtual-list` | `composite::tree_view` |
| `virtual-list` | `scroll-view` | `composite::virtual_list` |

**Surfaces**

| Feature | Implies | Public item |
|---|---|---|
| `tooltip` | — | `surface::tooltip` |
| `menu` | `button`, `tooltip` | `surface::menu` |
| `zorder` | `menu` | `surface::zorder` |
| `modal` | `zorder` | `surface::modal`, `surface::modal_tier` |
| `dialog` | `button`, `zorder` | `surface::input_dialog`, `surface::dialog_button` |
| `movable` | `zorder` | `surface::movable`, `surface::drag_tier` |
| `popover` | `zorder` | `surface::popover` |

**Meta features**

| Feature | Implies | Purpose |
|---|---|---|
| `range-model` | — | the shared interval model behind `slider`, `range-slider`, `number-box`, `navigation-rail`. Depend on a widget instead. |
| `all` | every widget and surface feature | the whole framework |
| `gallery` | `all` | the `gallery` binary and the documentation sweep |
| `test-helpers` | — | publishes `test_helpers` in the public API |

`test-helpers` gates no extra crates. `iced_runtime`, `iced_renderer`, and `iced_futures`
are already non-optional dependencies, so the harness builds under `test`, under
`test-helpers`, or under both.

### Dependency requirements

| Crate | Version | Why |
|---|---|---|
| `iced` | `0.14.0`, features `wgpu`, `tiny-skia`, `fira-sans`, `advanced-shaping`, `advanced` | the host framework |
| `iced_futures` | `0.14.0` | the harness needs an executor `iced` does not expose |
| `iced_renderer` | `0.14.0` | the concrete headless renderer |
| `iced_runtime` | `0.14.0` | `UserInterface`, which `iced::advanced` does not re-export |

`advanced` is mandatory, not optional. Without it, `iced::advanced` is compiled out and
`Widget`, `Tree`, `layout`, and `operation::Focusable` are unreachable, so no custom
widget can be written at all. It adds no code of its own; it unlocks the re-export module.

`iced_test` and `proptest` are dev-dependencies only. They never enter a consumer's graph.

### Theme and palette

There is one palette and one constructor.

```rust
pub struct Palette { /* 23 iced::Color fields, snake_case */ }
impl Palette { pub const fn dark() -> Palette; }

pub struct EditorPalette { /* 3 fields */ }
impl EditorPalette { pub const fn inverted() -> EditorPalette; }
```

- Call `Palette::dark()` once and pass the value to each widget with `.palette(...)`.
  There is no global theme registry and no `Theme` type.
- Do not construct an `iced::Color` outside `token/color.rs`. `token_lint` fails the
  build when you do.
- Read every measurement from `token::{space, size, shape, typography}`. A bare literal
  on a visually-named constant fails the build unless it is registered.

### Timing configuration

Time arrives as a value. Nothing in the crate reads a wall clock inside a rule.

| Token | `name` | `ms` | Kind |
|---|---|---|---|
| `time::INSTANT` | `time.instant` | 0 | timer |
| `time::FAST` | `time.fast` | 100 | transition |
| `time::NORMAL` | `time.normal` | 180 | transition |
| `time::SLOW` | `time.slow` | 240 | transition |
| `time::DEBOUNCE_LIVE` | `time.debounce.live` | 200 | timer |
| `time::DELAY_SUBMENU` | `time.delay.submenu` | 150 | timer |
| `time::DELAY_TOOLTIP_OPEN` | `time.delay.tooltip_open` | 400 | timer |
| `time::DELAY_TOOLTIP_WARM` | `time.delay.tooltip_warm` | 100 | timer |
| `time::HOLD_COPIED` | `time.hold.copied` | 1500 | timer |
| `time::TOAST_AUTO_DISMISS` | `time.toast.auto_dismiss` | 5000 | timer |

```rust
pub struct DurationToken { pub name: &'static str, pub ms: u64,
                           pub easing: Option<Easing>, pub alias: Option<&'static str> }
pub enum Easing { Linear, EaseOutCubic, EaseInOutCubic }
pub const fn resolve_ms(intended: DurationToken, reduced_motion: bool) -> u64;
```

- Pass `reduced_motion` yourself. `resolve_ms` queries nothing.
- Under `reduced_motion`, a **transition** rung resolves to `0`. A **timer** rung keeps
  its duration either way, because zeroing a debounce or a hover delay would produce more
  churn, not less.
- Drive every timed rule with `surface::tooltip::TimeMs`. `TimeMs::now()` exists for call
  sites that genuinely need wall time; rules take it as an argument.

### Persistence seams

The crate never opens a file. You implement two traits.

```rust
pub trait WindowStorage: Send + Sync {
    fn load_geometry(&self) -> Option<WindowGeometry>;
    fn save_geometry(&self, geometry: &WindowGeometry);
}

pub trait GeometryStorage {
    fn load(&mut self, key: &str) -> Option<MovableGeometry>;
    fn save(&mut self, key: &str, geometry: MovableGeometry);
    fn clear(&mut self, key: &str);
}
```

| Implementation | Use |
|---|---|
| `NoopWindowStorage` | tests and ephemeral sessions. Ignores saves, returns `None`. |
| `MemoryWindowStorage` | in-memory window state. `with_geometry` seeds it. |
| `NoopGeometryStorage` | tests. Stores nothing. |
| `MemoryGeometryStorage` | in-memory card geometry. `keys()` lists what is stored. |

- Derive the movable key from `movable::storage_key(&id)`. Do not invent your own key
  format, or a renamed surface loses its saved position.
- `MovableGeometry::encode` / `decode` give a lossless text form. Serialise those four
  numbers with whatever you already use.
- Tests MUST NOT read or write real user paths. Use the noop or memory seams, or a
  `tempfile::TempDir`.

### Window and pane defaults

| Constant | Value | Meaning |
|---|---|---|
| `WINDOW_DEFAULT_WIDTH` | 1000 | default main window width |
| `WINDOW_DEFAULT_HEIGHT` | 700 | default main window height |
| `WINDOW_MIN_WIDTH` | 640 | minimum main window width |
| `WINDOW_MIN_HEIGHT` | 480 | minimum main window height |
| `CENTER_PANE_MIN_W` | 240 | central workspace floor |
| `LEFT_PANE_MIN_W` | 150 | left pane floor |
| `RIGHT_PANE_MIN_W` | 120 | right pane floor |
| `UiScale::MIN` / `MAX` / `STEP` | 0.75 / 2.00 / 0.10 | the UI scale ladder |
| `UiScale::DEFAULT` | 1.00 | the default scale |

Build a scale with `UiScale::new`. It clamps to `[0.75, 2.00]`, quantizes to the nearest
0.10 step, and resolves a non-finite input to `DEFAULT`.

### Surface geometry constants

These literals are fixed by the spec, so they are permitted as bare numbers. Each is
registered in `token_lint::SURFACE_GEOMETRY_CONSTANTS` with its spec reference, and
`surface_geometry(file, name)` looks one up.

| Area | Registered constants |
|---|---|
| Menu card | `MENU_CARD_WIDTH_MIN` 224, `MENU_CARD_WIDTH_MAX` 320 |
| Input dialog | `CARD_WIDTH` 320, `FOLDER_LIST_MAX_HEIGHT` 200 |
| Modals | `EDITOR_WIDTH_MIN` 520, `EDITOR_WIDTH_MAX` 960, `EDITOR_HEIGHT_MIN` 380, `EDITOR_HEIGHT_MAX` 760, `TOOLS_WIDTH` 720, `TOOLS_HEIGHT_MIN` 320, `BATCH_WIDTH` 560, `CONTENT_HEIGHT_MAX` 480, `ABOUT_WIDTH` 440, `ATTRIBUTIONS_WIDTH` 640, `ATTRIBUTIONS_HEIGHT` 480, `ATTRIBUTIONS_LIST_MAX_HEIGHT` 200 |
| Movable card | `CARD_DEFAULT_WIDTH` 680, `CARD_DEFAULT_HEIGHT` 440 |
| Tooltip | `MAX_WIDTH` 280 |
| Teaching popover | `WIDTH_MIN` 280, `WIDTH_MAX` 360 |
| Table header | `RESIZE_REACH` 10 (halved across the divider) |
| Auto-suggest | `FLYOUT_MAX_HEIGHT` 240 |
| Combo box | `FLYOUT_MAX_HEIGHT` 280, `MIN_WIDTH` 120, `MIN_WIDTH_COMPACT` 64 |
| Form field | `HEADER_HEIGHT` 20 |
| List tile | `TWO_LINE_HEIGHT` 44 |
| Toast | `WIDTH` 360 |
| Breadcrumb | `BAR_HEIGHT` 28, `SEGMENT_HEIGHT` 24, `SEGMENT_PADDING_X` 6, `SEGMENT_GAP` 4, `ELLIPSIS_TRIGGER_WIDTH` 28 |
| Overflow toolbar | `STRIP_HEIGHT` 28, `ITEM_HEIGHT` 24, `ITEM_GAP` 4, `TRIGGER_SIZE` 28, `TRIGGER_GAP` 8 |
| Side sheet | `DEFAULT_WIDTH` 360, `MIN_WIDTH` 280, `MAX_WIDTH` 520, `FOOTER_HEIGHT` 48 |
| Navigation rail | `COMPACT_WIDTH` 48, `EXPANDED_DEFAULT_WIDTH` 220, `EXPANDED_MIN_WIDTH` 180, `EXPANDED_MAX_WIDTH` 320 |
| Command palette | `CARD_WIDTH` 560, `CARD_HEIGHT` 380, `SEARCH_HEIGHT` 40, `ROW_DOUBLE_HEIGHT` 44 |
| Grid view | `DEFAULT_MIN_TILE_WIDTH` 160, `DEFAULT_TILE_HEIGHT` 120 |
| Code block, diff viewer | `GUTTER_WIDTH` 40 each |
| Date picker | `CALENDAR_FLYOUT_WIDTH` 256 |
| Time picker | `TIME_FLYOUT_WIDTH` 220, `TIME_FLYOUT_MAX_HEIGHT` 260 |
| Colour picker | `COLOR_FLYOUT_WIDTH` 240, `COLOR_WELL_WIDTH` 180, `COLOR_WELL_HEIGHT` 140 |
| Tab strip | `TAB_MIN_WIDTH` 96, `TAB_MAX_WIDTH` 220 |

Everything else on a visually-named constant MUST read a token rung.

### The token lint scope

`token_lint` inspects:

- Rule A, colour literals: the four `Color::from_*` constructors may appear only in
  `token/color.rs`.
- Rule B, unbound visual scalars: a `pub const` whose name contains any of `HEIGHT`,
  `WIDTH`, `PADDING`, `RADIUS`, `STROKE`, `GAP`, `SIZE`, `INSET`, `MARGIN`, `SLOT`, inside
  `widget/`, `surface/`, or `composite/`.

A visual constant is legal when it binds a token path, is zero, is a sub-unit ratio, or
is a registered Surface Geometry Constant. One entry sits on the allow-list:
`surface/input_dialog.rs:BUTTON_MIN_WIDTH`, with a written reason. A stale allow entry
fails the audit as surely as a violation.

### User-facing text

Every string a host renders verbatim lives in `strings.rs` as a `pub const`. Read labels
and validation messages from there. Do not restate them at the call site.

### Quality gate

Run these from the repository root before you call a change complete:

```
cargo check --workspace --all-features
cargo nextest run -p hoarfrost --all-features --status-level fail --show-progress none
cargo clippy --workspace --all-features -- -D warnings
cargo fmt --check
cargo doc --workspace --all-features --no-deps --quiet
```

Unit tests SHOULD live in separate `<file>_tests.rs` files and MUST run under
`cargo nextest`. Every change SHOULD be covered by a narrow integration test as well as a unit
test. Use `cargo nextest run --workspace` to cover `hoarfrost-demo` too.

## Error Handling

### Principles

1. A refusal is a value, never an exception. Every rule in this crate returns a variant
   that says what was refused and why.
2. Nothing in a paint path panics. Layout, style resolution, and drawing return a value
   for every input, including `NaN` and negative sizes.
3. Degenerate input is sanitised, not propagated. A non-finite width becomes the cap, a
   negative width becomes `0`, a non-finite progress fraction becomes no progress.
4. Contrast shields panic only in a `debug_assertions` build. They fire at the resolver
   that built the bad pair, so the failure names its author.
5. Match every refusal variant. A `_ => {}` arm on a `NoOp` hides the exact bug these
   types exist to surface.

### The refusal vocabulary

| Layer | Type | What it refuses |
|---|---|---|
| Validation | `PunishReason::WithheldPristine` | an invalid value that MUST NOT be painted yet |
| Validation | `DisallowedKind` | a character that is never legal |
| Recovery | `GestureStart::AlreadyOpen` | a second begin on an open gesture |
| Modal | `ModalOutcome::DismissRefused` | a dismissal the card will not honour |
| Modal | `ModalNoOp` | a key or pointer event the trap does not own |
| Dialog | `Verdict::Reject(ValidationMessage)` | a value that fails its rule |
| Dialog | `InputNoOp` | an event outside the dialog's ownership |
| Popover | `OpenRefusal` | a callout that may not open |
| Popover | `IgnoredReason` | an event the callout MUST NOT double-handle |
| Z-order | `EscapeRefusal` | an `Escape` no surface consumed |
| Z-order | `DeclineReason` | a surface that cannot step back |
| Menu | `MenuViolation` | a structurally illegal menu |
| Menu | `IgnoredReason` | a cascade request it cannot serve |
| Binding | `Activation::Suppressed` | a duplicate activation on a busy action |
| Binding | `CancelRefusal` | a cancel request for a non-cancellable action |
| Movable | `MovableNoOp` | a gesture or key the card does not own |
| Gate | `Result<(), InvariantViolation>` | a violated invariant |
| Lint | `token_lint::Violation` | a build-time tokenization breach |

### Validation: the punish pipeline

`punish` is the single decision point. Call it on every edit.

```rust
pub const fn punish(stance: EditStance, invalid: bool, disallowed: bool) -> Punishment;
```

| Stance | Invalid | Disallowed | `reason()` | `invalid()` | `painted()` | `blocks_submit()` |
|---|---|---|---|---|---|---|
| `Pristine` | no | no | `Valid` | false | false | false |
| `Pristine` | yes | no | `WithheldPristine` | true | false | true |
| `Pristine` | any | yes | `DisallowedImmediate` | true | true | true |
| `Touched` | no | no | `Valid` | false | false | false |
| `Touched` | yes | no | `TouchedInvalid` | true | true | true |
| any | any | yes | `DisallowedImmediate` | true | true | true |

- Read `painted()` before you turn a field red. A pristine field that has never been
  committed to stays calm.
- Read `blocks_submit()` even when nothing is painted. `WithheldPristine` still blocks.
- Do not implement your own version of this table. The four reasons are the whole rule.

### Disallowed characters

```rust
pub enum DisallowedClass { FilenameBasename }
pub struct DisallowedChar { pub character: char, pub offset: usize, pub kind: DisallowedKind }
pub enum DisallowedKind { IllegalPathCharacter, ControlCharacter }
pub const FILENAME_ILLEGAL_CHARS: &str = "/\\:*?\"<>|";
```

- Call `first_offence(value)` to get the character, its byte offset, and the rule it
  broke. Use `offences(value)` when you want every one.
- Refuse these immediately, at any stance. This is the only instant-punish class.
- The set is the Windows-reserved union plus both path separators, so a name survives a
  round trip through Windows.
- `DisallowedClass` is closed. Do not add a class such as "no spaces" or "no uppercase".
  Those are ordinary validation rules, and they belong in `Verdict`, not here.

### Recovery after a refused commit

```rust
pub struct RecoverySession;
impl RecoverySession {
    pub fn begin_from_text_input(&mut self, model: &TextInputModel) -> GestureStart;
    pub fn begin_from_text_area(&mut self, model: &TextAreaModel) -> GestureStart;
    pub fn cancel_into_text_input(&mut self, model: &mut TextInputModel) -> Option<RestoredFields>;
    pub fn cancel_into_text_area(&mut self, model: &mut TextAreaModel) -> Option<RestoredFields>;
    pub fn settle(&mut self) -> bool;
}
```

1. Snapshot before the commit that can fail. `from_text_input` carries buffer, selection,
   and scroll.
2. On rejection, call `cancel_into_*`. It restores buffer, selection, and scroll together,
   so the caret does not jump.
3. On success, call `settle()`. It closes the gesture and drops the snapshot.
4. `cancel*` returns `None` when no gesture is open. That is not an error; it means the
   gesture already settled.
5. `GestureStart::AlreadyOpen` means you began twice. Do not overwrite the first snapshot,
   or the restore lands on the wrong state.

### Modal and dialog errors

```rust
pub struct ModalError;   // code, message, optional location, optional recovery
pub enum ErrorLocation { Position { line: u32, column: u32 }, Region(String) }
pub enum Recovery { Retry, FocusControl(FocusId), Acknowledge }
pub enum ModalOutcome { Dismissed { cause }, DismissRefused { cause }, Absorbed { target },
                        FocusMoved { focused }, NoOp { reason } }
```

- Build a `ModalError` with a stable code and one `Recovery` affordance. An error with no
  recovery leaves the user stuck inside a modal trap.
- `take_retry()` returns the error once and clears the retry request, so a retry cannot
  fire twice.
- A dirty card answers `DismissRefused { cause }`. Confirm with the user, then call
  `close(...)`. Do not silently discard.
- `set_error` returns the previous error, so a chained failure does not lose the first.

For input dialogs:

```rust
pub enum Verdict { Accept, Reject(ValidationMessage) }
pub enum InputOutcome { Accepted { value }, Cancelled { cause }, ErrorRevealed { message },
                        Absorbed { target }, FocusMoved { focused }, NoOp { reason } }
```

- `set_value` returns the new `Verdict` and never closes the card.
- The validation slot is permanently reserved, so revealing a message never shifts the
  layout.
- Codes are stable: `INP-1001` empty, `INP-1002` edge whitespace, `INP-1003` control
  characters, `INP-1004` illegal basename characters, `INP-1005` unchanged name,
  `INP-1006` nothing to confirm, `INP-1007` no destination.

### Escape refusals

```rust
pub enum EscapeOutcome {
    Consumed { surface, kind, band, dismissed, declined_before },
    Ignored(EscapeRefusal),
}
pub enum EscapeRefusal { EmptyStack, AllDeclined { declined: Vec<Declination> } }
pub enum DeclineReason { NotOpen, NotFocused, CannotUnwind }
```

- `Ignored(EmptyStack)` is the defined no-op at the bottom of the stack. Do not treat it
  as an error and do not swallow the key.
- `AllDeclined { declined }` lists each surface and its reason, in the order it was asked.
  Use it to explain to the user why nothing closed.
- `Declined(NotFocused)` is normal. A modal that does not hold focus steps aside for the
  surface that does.
- `declined_before` on a `Consumed` outcome records surfaces that refused before the one
  that accepted. It is empty in the common case.
- A host with a drag-capable control MUST call `escape_with_pre_rank`. `Escape` on a live
  drag is step 1, and `SurfaceRegistry::escape` cannot see a drag at all.

### Activation suppression and busy actions

```rust
pub enum Activation { Fired, Suppressed(SuppressedReason) }
pub enum SuppressedReason { Busy, AwaitingProjection }
pub enum CancelRefusal { NotBusy, NotCancellable }
```

- Treat `Suppressed` as a normal outcome, not a failure. It is the guard that stops a
  double submit.
- `AwaitingProjection` means you activated before observing the pending state. Call
  `observe(pending)` every frame before you try to activate.
- `request_cancel()` returns `None` when the action is not cancellable. Surface that
  fact; do not pretend a cancel happened.
- The `fired()` and `suppressed()` counters exist for tests and diagnostics. Assert on
  them rather than on wall-clock timing.

### Gesture refusals

```rust
pub enum GestureStart { Started, AlreadyOpen }
pub struct Live<T> { /* value */ }
pub struct Commit<T> { /* value */ }
pub struct Cancel<T> { /* restored */ }
```

- A gesture emits any number of `Live` frames and exactly one `Commit` or `Cancel`.
- `cancel()` returns `None` when nothing is open. Do not restore state in that case.
- `Cancel<T>` carries the pre-gesture value. Restore from it; do not recompute it.
- `abort()` drops a gesture without emitting a tier. Use it only when the gesture should
  leave no trace.

### Menu structural violations

```rust
pub enum MenuViolation {
    MissingKey { row, label },
    DuplicateKey { row, key, first },
    DestructiveNotLast { row, group },
    MixedGroup { group, row },
    RadioOverSelected { group, rows },
}
```

- Call `Menu::validate()` in a test and assert the vector is empty. These are build-time
  errors about your data, not runtime conditions.
- `DestructiveNotLast` and `MixedGroup` protect one rule: a destructive action is never
  adjacent to a harmless one.
- `RadioOverSelected { rows }` names every selected member, so you can see the whole
  conflict at once.
- A cascade request it cannot serve returns `CascadeOutcome::ignored(reason)`. Check
  `is_ignored()` before you assume a card opened.

### Invariants as results

```rust
pub struct InvariantViolation { pub category: InvariantCategory, pub target: String,
                                pub message: String }
pub fn check_sentence_case(label: &str, is_chrome: bool) -> Result<(), InvariantViolation>;
```

- Every gate returns `Result<(), InvariantViolation>`. Collect into one `InvariantReport`
  and assert `is_clean()` once, so one bad label does not hide the next nine.
- `describe()` prints the whole report. Put it in the assert message.
- A report with `total_checked == 0` is a bug in your test, not a pass.

### Contrast shields

```rust
pub fn assert_fill_foreground_lock(background: Color, foreground: Color, palette: &Palette);
pub fn assert_contrast(fg: Color, bg: Color, min_ratio: f32, label: &str);
```

- Both are `debug_assertions` only. A release build paints the pair anyway.
- The real gate is the crate-wide contrast audit, which walks every pair every resolver
  can emit and fails the build. The asserts are a local shield, not the enforcement.
- `FillLockRole::Unlocked` means the background is not one of the locked solid fills, so
  the lock places no demand. It is still checked, so a later edit that swaps in a locked
  fill fails at construction.
- Thresholds: text `4.5`, active chrome `3.0`, coincident fill `3.0`, disabled text band
  `4.0`–`6.05`.

### Secret redaction

```rust
pub fn check_secret_redaction<T: fmt::Debug + fmt::Display>(secret: &T, plaintext: &str)
    -> Result<(), InvariantViolation>;
```

- Call it in a test for every type that can hold a secret. It fails when `Debug` or
  `Display` leaks the plaintext.
- Redact at the type, not at the log call. A redaction that depends on every call site
  remembering is a redaction that will be missed.

### Build-time failures

`token_lint` failures are not runtime errors. `Audit::is_clean()` is `false` when:

- a colour constructor appears outside `token/color.rs`;
- a visually-named constant holds a bare literal that is not registered or allow-listed;
- an allow-list entry matched nothing (`stale_allow_entries`).

Fix the source. Do not add an allow entry to make a test pass unless the spec genuinely
fixes the number, and then register it instead.

### What never happens

- No function in this crate panics on user input. Non-finite and negative values are
  sanitised at the boundary.
- No rule reads a wall clock, a filesystem, or an environment variable.
- No error type carries a stack trace or a wrapped I/O error. The crate performs no I/O.
- No panic message contains user text. Panic text is a rule name plus numeric values.

---

## Appendix A — Spec / code drift

This document describes the **code**. Where `doc/spec.md` or `README.md` disagrees, this
file follows the source tree. This appendix lists the known disagreements so you do not
implement against a stale signature.

Do not treat this list as a to-do. It is a reading aid.

### `surface::zorder`

| Spec (spec.md §5, lines 1594–1619) | Code | Effect |
|---|---|---|
| `pub struct SurfaceId(pub u64)` | `pub struct SurfaceId(String)` with `new(impl Into<String>)` and `From<&str>` | Ids are host-chosen strings, not numbers. |
| `pub struct FocusId(pub u64)` | string newtype, same shape | Same. |
| `pub struct NotificationId(pub u64)` | string newtype | Same. |
| `register(&mut self, id, band, host: Option<SurfaceId>, regions, escape_dismissable)` | `register(&mut self, id: SurfaceId, kind: SurfaceKind, regions: Vec<Rectangle>) -> &Surface` | The band comes from `SurfaceKind::band()`. There is no `host` parameter and no `escape_dismissable` parameter. |
| `enum EscapeDismissable { Always, WhenFocused, Never }` | **does not exist** | Dismissal policy lives in each host's `unwind_answer`, which returns `Declined(NotFocused)` or `Declined(CannotUnwind)`. |
| `enum EscapeOutcome { Consumed(SurfaceId), Suppressed(SurfaceId), Ignored }` | `Consumed { surface, kind, band, dismissed, declined_before }` \| `Ignored(EscapeRefusal)` | There is **no** `Suppressed` variant. "Consumed but still open" is `Consumed { dismissed: false }`. |
| `enum UnwindAnswer { Dismissed, Declined(DeclineReason) }` | `Unwound \| Dismissed \| Declined(DeclineReason)` | Code adds `Unwound`: consumed, still registered, and not a dismissal. |
| `Ignored` (unit) | `Ignored(EscapeRefusal)` with `EmptyStack` \| `AllDeclined { declined }` | The refusal carries who declined and why. |
| `unregister(&mut self, id: SurfaceId)` | `unregister(&mut self, id: &SurfaceId) -> Option<Surface>` | Borrowed id, returns the removed surface. |
| `route_pointer(&self, point) -> Option<SurfaceId>` | `-> PointerRoute` | The route also reports the bands passed through and whether the workspace was reached. |
| `escape(&mut self, impl FnMut(SurfaceId) -> UnwindAnswer)` | `escape<F: FnMut(&Surface) -> UnwindAnswer>` | The closure receives the whole `Surface`. |
| *(not in spec)* | `escape_with_pre_rank(...) -> PreRankedEscape` | Code implements `UIF-WIN-003` step 1, the pointer-captured drag pre-rank. |
| `ZBand` variants and discriminants | identical | No drift. |
| `DeclineReason` variants | identical | No drift. |

### `focus`

| Spec | Code | Effect |
|---|---|---|
| `focus(&mut self, id: FocusId) -> bool` | `focus(&mut self, id: &FocusId) -> bool` | Borrowed id. |
| `push_trap(&mut self, group: FocusGroupId)` | `push_trap(&mut self, group: GroupId)` | The type is `GroupId`, not `FocusGroupId`. |
| `pop_trap(&mut self) -> Option<FocusId>` | `pop_trap(&mut self) -> Option<TrapPop>` | `TrapPop` carries `group`, `recorded`, and the post-pop focus. |

### `geometry::anchor`

| Spec | Code | Effect |
|---|---|---|
| `Anchor::Control(Rectangle)` | `Anchor::Control { rect: Rectangle, alignment: ControlAlignment }` | The alignment travels with the anchor. |
| `solve(anchor, card, viewport, preferred: PreferredSide) -> Placement` | `solve(anchor, card_size, viewport) -> Placement` | There is no fourth parameter. Side preference is carried by `ControlAlignment`, or by `popover::PreferredSide::alignment()` for a callout. |
| `Placement { origin: Point, scrollable: bool }` | `Placement { origin, size, flipped_horizontal, flipped_vertical, scrollable, clamped_horizontal, clamped_vertical }` | Read the flip and clamp flags instead of re-deriving them. |

### `surface::input_dialog`

| Spec | Code | Effect |
|---|---|---|
| `InputLayout::solve(card: Rectangle, kind: InputDialogKind)` | `InputLayout::solve(card: Rectangle, control: &InputControl, buttons: FooterButtons)` | The type is `InputControl`, and footer button widths are a required input. |
| *(no error-visibility parameter)* | a permanently reserved `validation` rectangle in `InputLayout` | Consistent with the spec's intent, expressed as geometry rather than a flag. |

### `surface::movable`

| Spec | Code | Effect |
|---|---|---|
| `trait GeometryStorage { fn load(&self, key) -> Option<Rectangle>; fn save(&mut self, key, rect: Rectangle); }` | `fn load(&mut self, key: &str) -> Option<MovableGeometry>; fn save(&mut self, key: &str, geometry: MovableGeometry); fn clear(&mut self, key: str)` | `&mut self` on load, a `MovableGeometry` value instead of a `Rectangle`, and a `clear` method. |
| `FollowState::resolve(setting: FollowSetting, at_bottom)` | `FollowState::resolve(setting: AutoScrollSetting, at_bottom: bool)` | The enum is `AutoScrollSetting`. |

### `surface::modal`

| Spec | Code | Effect |
|---|---|---|
| `ModalError { pub message, pub location: Option<LineCol>, pub recovery: Option<Recovery> }` | a private struct built with `new(code, message)`, `.at(location)`, `.with_recovery(recovery)` | There is a `code` channel the spec does not mention, and the fields are read through accessors. |

### `token`

The spec and the README refer to token types that do not exist in the code.

| Claimed | Reality |
|---|---|
| `token::TypeScale` | `token::typography`, with `TypeRung` values such as `BODY`, `CAPTION`, `UI_COMPACT`, `TITLE_SM` |
| `token::Spacing` | `token::space`, with `XXS` … `XXL` constants |
| `token::Motion` | `token::time`, with `DurationToken` values and `resolve_ms` |
| `token::Theme` | **does not exist.** `token::color::Palette::dark()` is the only palette |
| `token::Elevation` | **does not exist.** Elevation is expressed through `Palette` surface fields |
| a multi-theme palette | `Palette` has 23 `iced::Color` fields and one constructor, `dark()` |

### `state`

| Claimed | Reality |
|---|---|
| a single flat state enum | `BaseState { Rest, Hovered, Pressed, Disabled, Invalid }` plus `VisualState { base, focused }`. Focus is additive, not a competitor. |
| `resolve(hover, press, focus, disabled)` | `resolve(input: InteractionInput) -> VisualState`, one struct argument |

### `bridge`

`bridge` exports only `widget_id`, `Direction`, `focus_operation`, `focus_task`,
`tab_operation`, and `tab_task`. There is no `BridgeAction` and no `BridgeEvent`.

### `widget::token` aliases and feature placement

- `progress` and `progress_model` compile under the `button` feature. The `progress`
  feature is an alias for `button`, not an independent module.
- `widget::focus` and `widget::focusable` are separate modules. `FocusPlacement::{Outer,
  Inset}` owns `bounds(control, width)`; `FocusIndicator` owns `bounds(control)`.

### `README.md` requirement labels

The README labels several composites with the wrong `UIF-COMP-*` id. The code's `//!`
comments are authoritative.

| Widget | README says | Code says |
|---|---|---|
| Pagination bar | `UIF-COMP-012` | `UIF-COMP-014` |
| Table | `UIF-COMP-011` | `UIF-COMP-007` |
| Breadcrumb bar | `UIF-COMP-004` | `UIF-COMP-008` |
| Toast / SnackBar | `UIF-COMP-009` | `UIF-COMP-002` |

### Unverifiable README claims

The README states "6,188 tests" and "89 of 92 requirements implemented". These counts are
not derived from anything in the source tree and were not re-counted for this document.
Treat them as a claim about the repository at the time the README was written, not as a
fact you can rely on.

### Dangling references

Module documentation across the crate cites `doc/planning/hoarfrost-tasks.md` for INC and
G-numbered decisions. That file is not present in the repository. The citations are
historical; the rules they justify are implemented and tested in the code.

### Composite modules with no `Widget` impl

These composite modules are resolvers and models only. Your application renders them:

`breadcrumb`, `card`, `expander`, `form_field`, `icon`, `info_bar`, `list_tile`,
`notification`, `overflow_toolbar`, `scroll_view`, `settings_row`, `table_cell`,
`table_column`, `table_header`, `table_view`, `tab_strip`, `toast`, `tree_row`,
`virtual_list`.

These own a `Widget` impl: `code_block`, `command_palette`, `diff_viewer`, `empty_state`,
`grid_view`, `keybind_recorder`, `navigation_rail`, `pagination`, `pips_pager`,
`side_sheet`, `skeleton`, `split_divider`, `stepper`.

### Duplicate names to watch

- `SelectionMode` exists twice with different shapes: `composite::segmented_model` uses
  `{ SingleSelect, MultiSelect }`; `composite::tree_view` uses `{ Single, ExtendedMulti }`.
- `DismissCause` exists in `surface::modal` (`Escape, Scrim, CloseButton, Programmatic`)
  and in `surface::input_dialog` (`Escape, CancelButton, Scrim, Programmatic`) and in
  `surface::popover` (`PrimaryAction, DismissButton, Escape, ClickOutside, Programmatic`).
  They are three distinct enums.
- `IgnoredReason` exists in `surface::menu::cascade` and in `surface::popover::host`.
- `clamp_width` exists in `surface::tooltip` (a free function) and as
  `PopoverStyle::clamp_width` (an associated function).
- `group_for(id: &SurfaceId) -> GroupId` exists in `surface::modal`,
  `surface::input_dialog`, and `surface::movable`. Each derives its own group namespace.
- `Desired`, `Cap`, `Zone`, `Candidate`, `Visibility`, `Activation`, `Recovery`, `Rule`,
  and `Violation` are each defined in more than one module. Always import through the
  module path shown in the entry that documents them.
