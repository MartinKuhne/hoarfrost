# hoarfrost SDK Reference

Machine-oriented reference for the `hoarfrost` crate, written for AI coding agents.

## Contents

- [Overview](#overview)
- [Architecture & Core Concepts](#architecture--core-concepts)
- [API Reference](#api-reference)
  - [Core Module / Class](#core-module--class)
  - [Utility Functions](#utility-functions)
- [Configuration Reference](#configuration-reference)
- [Error Handling](#error-handling)

---

## Overview

### Identity

| Field | Value |
|---|---|
| Crate name | `hoarfrost` |
| Version | `0.14.1` |
| Edition | Rust 2024 |
| License | MIT |
| Repository | https://github.com/MartinKuhne/hoarfrost |
| Categories / keywords | `gui` / `iced`, `ui`, `widgets`, `design-tokens` |
| Library path | [`src/hoarfrost/mod.rs`](file:///D:/hoarfrost/src/hoarfrost/mod.rs) (not `src/lib.rs`) |
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

The `advanced` iced feature is **REQUIRED**. Without it, `iced::advanced` is `cfg`-ed out and custom widgets cannot compile. You **MUST NOT** enable a mismatched `iced` feature set or remove `advanced` from your `iced` dependency: re-exported `iced` types on both sides of the crate boundary **MUST** match.

### Installation

`hoarfrost` is published on [crates.io](https://crates.io/crates/hoarfrost). Add it to your `Cargo.toml`:

```toml
[dependencies]
hoarfrost = "0.14.1"
iced = { version = "0.14.0", features = [
    "wgpu", "tiny-skia", "fira-sans", "advanced-shaping", "advanced",
] }
```

The default feature set does not include any default widget. Enable one Cargo feature per widget you instantiate; see [Configuration Reference](#configuration-reference).

### Scope

The SDK targets provides user interface elements across nine core areas: main window, navbar, multiple panes, tabbed windows, input dialogs, modal dialogs, movable overlays, fixed menus, and context menus.

### Terms used in this document

| Term | Meaning |
|---|---|
| `application` (or `caller`) | The consumer crate that integrates `hoarfrost` with `iced`. |
| `*Host controller` | An in-crate stateful coordinator struct (such as `ModalHost`, `ComboHost`, `NumberBoxHost`) that pairs a domain model with `Ephemeral<S>` state. |
| `style resolver` | A pure function `(variant, state, palette) -> style`. |
| `geometry solver` | A pure function computing layout rectangles, clamping, or scroll windows. |
| `surface` | An elevated overlay card in `surface::*` (`modal`, `input_dialog`, `movable`, `menu`, `popover`, `tooltip`). |
| `atom` | One single-control widget with a complete interaction state machine (`widget::*`). |
| `composite` | A multi-part control or layout container combining atoms and geometry (`composite::*`). |
| `model` | The application-owned, declarative value a widget reads each frame. |
| `rung` | One named step of a token scale (for example `space::MD`). |
| `tier` | One of the three callback classes: `Live`, `Commit`, `Cancel`. |

Normative keywords follow RFC 2119 (`**MUST**`, `**MUST NOT**`, `**SHOULD**`, `**SHOULD NOT**`, `**MAY**`).

### Ownership boundary - read this before you design an application

Every API in the crate obeys this boundary:

- This crate **MUST** supply sizing, frame, and interaction semantics only.
- The application **MUST** own surface lifecycle: open, close, and dismiss.
- The application **MUST** remain the single writer to `SurfaceRegistry`.
- The application **MUST** inject all persistence handlers.
- No application type appears in this API; the dependency points one way.
- A widget **MUST NOT** mutate the model it reads.
- A widget **MUST NOT** self-register, self-unregister, or close the topmost surface on its own.

### Requirement traceability

Requirement IDs from the functional specifications are included with this document. They are of no consequence for the library consumer, but make it easier to maintain this document.

| Spec series | Subject |
|---|---|
| `UIF-PRIN-001..009` | The nine normative design principles |
| `UIF-TOK-001..007` | Design tokens |
| `UIF-ATOM-*` | Atomic widgets, § 3 |
| `UIF-COMP-*` | Composite widgets, §3B |
| `UIF-WIN/NAV/PANE/TAB/INP/MOD/MOV/FMENU/CMENU/ZORD-*` | Core surfaces, §4 - §5 |
| `UIF-VER-001..005` | Verification gates, §6 |

---

## Architecture & Core Concepts

### Layer model

```text
token/      closed token set: colour, space, size, shape, typography, time, contrast
  |
widget/     atoms        = style resolver (pure) + iced widget (paints + drives state)
composite/  composites   = pure geometry solver / style resolver + iced widget or Element builder
surface/    surfaces     = *Host controller (pure state machine) + renderer
  |
cross-cutting: state, binding, focus, bridge, geometry, a11y, microcopy, layout,
               validation, identity, strings, invariant_gate, token_lint
```

You **MUST NOT** reach into unexported module internals, and call sites **MUST** stay qualified. `token`, `widget`, and `composite` do not glob-re-export their children because short rung names (`sm`, `md`, `lg`) collide across scales. Write `token::space::MD` and `token::size::CONTROL_MD`, not `MD`.

### C1 - Tokenization (`UIF-PRIN-001`)

Every visual and temporal number comes from `token/`. Colour, spacing, sizing, corner radius, stroke weight, typography, and duration are all declared inside `token/`. You **MUST NOT** place raw numeric visual literals in widget files.

An atom file **MAY** carry only the absence of a property (`0.0` stroke width or `Color::TRANSPARENT` fill) and spec-mandated opacity modifiers as named constants. `token_lint` enforces this rule; see [Utility Functions](#utility-functions).

### C2 - Style resolver and widget split

Each atom and composite separates style resolution from rendering:

- The **style resolver** is a pure function from `(variant, state, palette)` to a resolved style struct, requiring no window, renderer, or GPU. The `UIF-VER-001` and `UIF-VER-003` verification gates test style resolvers directly.
- The **widget** is the `iced::Element` that paints those resolved layers and drives state transitions from pointer and keyboard events.

You **MUST** preserve this split when adding or integrating widgets. Tables in this document explicitly mark modules that provide only a style resolver or model without an `iced::Widget`.

### C3 - The interaction state model (`UIF-ATOM-001`, `UIF-ATOM-002`, `UIF-VER-003`)

The resolved interaction state consists of two orthogonal parts:

- `BaseState` - the mutually exclusive visual state: `Rest`, `Hovered`, `Pressed`, `Disabled`, `Invalid`.
- `VisualState { base, focused }` - the `BaseState` plus an independent `focused` flag that controls the focus ring.

Precedence is `Disabled > Invalid > Pressed > Hovered > Rest`. `focused` resolves independently of `base`, except that `Disabled` forces `focused: false` so inert controls never display a focus ring.

Two rules govern edge cases:

- `pressed: true` with `hovered: false` resolves to `Pressed` to support pointer drag-in releases and synthetic accessibility presses.
- `Invalid` outranks `Pressed` and `Hovered` so validation errors remain visible during pointer and keyboard interaction.

### C4 - Ownership: `Controlled` and `Ephemeral` (`UIF-ATOM-000b` §3.0.4.1)

Two wrapper types enforce the ownership boundary between the application and the widget tree:

- `Controlled<'a, M>` - an immutable shared borrow of the application's domain model (`&'a M`) read by the widget each frame.
- `Ephemeral<S>` - transient interaction state owned by the widget tree or `*Host controller` (such as hover, open gestures, or in-flight drags).

Widgets accept models in one of three forms depending on their signature:

| Style | Used by | Signature shape |
|---|---|---|
| `Controlled<'_, M>` | `Button::controlled`, `TextInput::controlled`, `Checkbox`, `RadioGroup`, `ToggleSwitch`, `SegmentedButton`, `SplitButton`, `Hyperlink` | `Widget::controlled(model.controlled(), focus_id, emit)` |
| plain `&'a M` | `ComboBox`, `TabStrip`, `NavBar` | `widget(&model)` |
| by value (clone) | pickers, `RatingControl`, `Avatar`, `RangeSlider`, `GridView`, `SideSheet`, `NavigationRail`, `CommandPalette`, `Stepper`, `SplitDivider`, `KeybindRecorder`, `Skeleton`, `EmptyState` | `widget(model)` |

### C5 - The three callback tiers (`UIF-ATOM-000b` §3.0.4.2)

Live ticks, final commits, and cancellations use three distinct wrapper types that share no common trait:

| Tier | Type | Fires | Application **MUST** |
|---|---|---|---|
| Live | `Live<T>` | every tick of a continuous gesture | update transient preview state; **MUST NOT** perform blocking disk or network I/O |
| Commit | `Commit<T>` | exactly once, on settle or submit | write durable state and trigger side effects |
| Cancel | `Cancel<T>` | on `Escape` or lost capture | restore `cancel.into_restored()` (or `*cancel.restored()`) to revert the model |

Widgets surface these tiers in two ways:

1. **Per-callback builder methods** - pickers and range widgets expose `.on_change(f)`, `.on_commit(f)`, and `.on_cancel(f)`.
2. **Single `emit` closure over an `*Emission` enum** - controlled atoms take `emit: impl Fn(ButtonEmission) -> Option<Message> + 'a`, where returning `None` ignores an unhandled tier.

`LiveCoalescer<T>` retains only the newest pending `Live<T>` value per frame so slow subscribers never replay stale intermediate ticks.

### C6 - Busy actions and activation gating (`UIF-ATOM-000b` §3.0.4.3)

`ActionPending` projects whether an asynchronous action is running, and `ActivationGate` latches to prevent double activation:

- `ActionPending::{idle, indeterminate, determinate(progress), with_cancellable(bool)}` describes the current busy state without reading a system clock.
- `ActivationGate::try_activate(&pending) -> Activation` returns `Fired` or `Suppressed(SuppressedReason::{Busy, AwaitingProjection})`.
- `ActionPending::try_cancel() -> Result<CancelRequest, CancelRefusal::{NotBusy, NotCancellable}>` mints an opaque `CancelRequest` token required by cancellation handlers.

A busy button suppresses repeat activations and permanently reserves `SPINNER_SLOT` width so toggling `busy` never shifts layout (`UIF-PRIN-003`).

### C7 - Focus: model, bridge, traps, roving (`UIF-PRIN-005`, decision D-7)

`focus::FocusState` is a pure policy model of tab order, groups, and focus traps. `bridge` projects `FocusState` into iced's focus router:

- The bridge runs strictly one-way from `FocusState` to iced's router; production code **MUST NOT** read focus state back from iced.
- Applications **MUST** route `Tab` and `Shift+Tab` through `bridge::tab_task`, which wraps at group ends, skips disabled controls, and honours the active focus trap.
- When no control is focusable, `bridge::focus_task` emits an explicit `unfocus` operation.

### C8 - Z-bands and Escape ranks (`UIF-ZORD-001`, `UIF-WIN-003`)

Visual elevation (`ZBand`) and `Escape` unwind priority (`escape_rank`) use separate orderings:

| `ZBand` | Band index | Escape rank | Typical surfaces |
|---|---|---|---|
| `Workspace` | 0 | 0 | window, navbar, panes, tabs, rail, docked sheet |
| `UtilityOverlay` | 1 | 1 | movable overlays, modeless sheet, toast stack |
| `Menu` | 2 | **3** (unwinds first) | menus, flyouts, pickers, popovers |
| `Modal` | 3 | 2 | modals, input dialogs, command palette, modal sheet |

`SurfaceRegistry::escape` walks registered surfaces in descending Escape-rank order and stops at the first surface that consumes the key. `Declined(CannotUnwind)` consumes `Escape` and halts the walk so a dirty modal never closes a lower-rank surface behind it.

Because a pointer-captured drag is not a surface, applications with drag-capable controls **MUST** call `escape_with_pre_rank`, which cancels an active drag in `PreRankCoordinator` before walking `SurfaceRegistry`.

### C9 - Geometry solvers are pure

Every geometry solver in `geometry` and `composite` is a pure function: anchor placement, viewport clamping, pane widths, column widths, breadcrumb elision, toolbar overflow, virtual-list metrics, and grid column layout. Non-finite inputs (`NaN`, `±∞`) clamp to deterministic defaults. Form-field caption bands and `list_tile::slots` reserve fixed token geometry independent of dynamic text content (`UIF-PRIN-003`, decision D-8).

### C10 - Single tab-stop composites (`UIF-PRIN-008`)

Every composite widget exposes a single `Tab` stop and uses arrow keys for internal roving traversal via `widget::rove::rove_index`, which skips disabled items and wraps at boundaries. `a11y::tab_stop::check_composite` verifies this invariant in tests.

### C11 - Reward early, punish late (`UIF-PRIN-009`)

`validation::EditStance` tracks whether a field is `Pristine` or `Touched`. `validation::punish(stance, invalid, disallowed)` suppresses error borders and error captions on `Pristine` fields until the user commits, blurs, or submits (`note_commit`, `note_blur`, `note_submit`), while immediately flagging `DisallowedClass` characters.

### C12 - Time arrives as a value

No module in this crate reads the system clock except `tooltip::hover::TimeMs::now()`. Callers pass a frame timestamp (`TimeMs` or `elapsed_ms: u64`) into timing functions, and `token::time::resolve_ms` collapses motion transitions to `0` ms when `reduced_motion` is `true`.

### C13 - Verification gates (`UIF-VER-001..005`)

| Gate | What it proves | Crate support |
|---|---|---|
| `UIF-VER-001` | token discipline and WCAG contrast | `token::contrast`, `token_lint` |
| `UIF-VER-002` | geometry, hit targets, responsive containers | pure geometry solvers in `geometry`, `composite` |
| `UIF-VER-003` | state machine and callback lifecycle | `state::resolve`, `binding`, tier types |
| `UIF-VER-004` | resolved tokens and ordered draw layers | `Layer` structs, `test_helpers` |
| `UIF-VER-005` | microcopy and ergonomic invariants | `microcopy`, `layout`, `invariant_gate` |

Unit tests reside in sibling `<file>_tests.rs` sidecars and run via `cargo nextest run -p hoarfrost --all-features`.

### Cross-cutting API conventions

Every API Reference entry below uses eight level-4 subsections in fixed order: `Overview`, `Prerequisites & Requirements`, `Syntax / Method Signature`, `Examples`, `Type references`, `Return values`, `Side effects`, and `References`. Follow these seven rules across all modules:

1. **Model passing:** Check each widget signature to see whether it takes `Controlled<'_, M>`, `&'a M`, or `M` by value.
2. **Callback shapes:** Controlled atoms take one `emit: impl Fn(<X>Emission) -> Option<Message>` closure, whereas pickers, sliders, and pagers take `.on_change(...)`, `.on_commit(...)`, and `.on_cancel(...)` builders.
3. **Tier extraction:** Extract live ticks and settled commits via `.into_value()` (or `.value()`), and extract cancellation payloads via `.into_restored()` (or `.restored()`).
4. **Message bounds:** Your application `Message` enum **MUST** implement `Clone + Send + 'static` so widgets convert into `iced::Element` and `bridge` tasks compile.
5. **Controller outcomes:** Stateful `*Host controller` methods return an `*Outcome` struct carrying `pub emissions: Vec<*Emission>`; you **MUST** iterate `for e in outcome.emissions` in your `update` loop.
6. **Pure verification:** Call `resolve_style(...)` and `*_bounds(...)` directly in unit tests instead of spinning up a window renderer.
7. **Per-widget Cargo features:** Enable the corresponding Cargo feature in `Cargo.toml` before importing any gated `widget`, `composite`, or `surface` module.

### End-to-end iced application blueprint

```rust
use iced::{Element, Length, Task};
use hoarfrost::bridge::{tab_task, Direction};
use hoarfrost::focus::{FocusMember, FocusState as FocusRouter, GroupId};
use hoarfrost::surface::zorder::SurfaceRegistry;
use hoarfrost::token::{color::Palette, space};
use hoarfrost::widget::button::ButtonVariant;
use hoarfrost::widget::button_controller::{ButtonController, ButtonEmission};
use hoarfrost::widget::button_model::ButtonModel;
use hoarfrost::widget::button_widget::Button;
use hoarfrost::widget::number_box::StepDirection;
use hoarfrost::widget::number_box_host::{NumberBoxEmission, NumberBoxHost};
use hoarfrost::widget::range_model::RangeModel;
use hoarfrost::widget::text_input::{InputKind, TextInput};
use hoarfrost::widget::text_input_controller::{TextInputController as TextInputState, TextInputEmission};
use hoarfrost::widget::text_input_model::TextInputModel;

#[derive(Clone, Debug)]
pub enum Message {
    NameEmit(TextInputEmission),
    SaveEmit(ButtonEmission),
    StepCount(StepDirection),
    TabKey(bool),
}

pub struct State {
    pub name_model: TextInputModel,
    pub name_state: TextInputState,
    pub save_model: ButtonModel,
    pub save_ctrl: ButtonController,
    pub count_model: RangeModel,
    pub count_host: NumberBoxHost,
    pub focus: FocusRouter,
    pub surfaces: SurfaceRegistry,
    pub palette: Palette,
}

impl State {
    pub fn update(&mut self, msg: Message) -> Task<Message> {
        self.name_state.observe(&self.name_model);
        self.save_ctrl.observe(&self.save_model);
        match msg {
            Message::NameEmit(emission) => {
                if emission.advances_stance() { self.name_model.note_commit(); }
                match emission {
                    TextInputEmission::Changed(live) => self.name_model.value = live.into_value(),
                    TextInputEmission::Committed(commit)
                    | TextInputEmission::FocusCommitted(commit)
                    | TextInputEmission::Cleared(commit) => self.name_model.value = commit.into_value(),
                    TextInputEmission::Cancelled(cancel) => self.name_model.value = cancel.into_restored(),
                    TextInputEmission::FocusChanged(_) => {}
                }
            }
            Message::SaveEmit(ButtonEmission::Activated(commit)) => {
                let _settlement = commit.into_value();
                self.name_model.note_submit();
            }
            Message::SaveEmit(ButtonEmission::Cancelled(cancel)) => {
                let _restored = cancel.into_restored();
            }
            Message::SaveEmit(ButtonEmission::CancelRequested(_req)) => {}
            Message::StepCount(dir) => {
                let outcome = self.count_host.press_stepper(&self.count_model, dir);
                for e in outcome.emissions {
                    match e {
                        NumberBoxEmission::Changed(live) => self.count_model.set(live.into_value()),
                        NumberBoxEmission::Committed(commit) => self.count_model.set(commit.into_value()),
                        NumberBoxEmission::Cancelled(cancel) => self.count_model.set(cancel.into_restored()),
                    }
                }
            }
            Message::TabKey(shifted) => return tab_task(&mut self.focus, Direction::from_shift(shifted)),
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let input = TextInput::controlled(
            InputKind::TextEntry,
            self.name_model.controlled(),
            "form.name",
            |e| Some(Message::NameEmit(e)),
        )
        .palette(self.palette);
        let save = Button::controlled(
            self.save_model.controlled(),
            "Save",
            "form.save",
            |e| Some(Message::SaveEmit(e)),
        )
        .palette(self.palette);
        iced::widget::row![input, save].spacing(space::SM).width(Length::Fill).into()
    }
}
```

---

## API Reference

<a id="core-module--class"></a>

### `token` - design tokens

#### Overview

The `token` module provides pure design tokens for colour, typography, spacing, sizing, corner radii, stroke widths, and motion timing (`UIF-PRIN-001`, `UIF-TOK-001..007`). It performs no stateful adaptation or widget style resolution, exposing only constants and pure interpolation functions.

#### Prerequisites & Requirements

- No Cargo feature is required; `token` is always compiled.
- You **MUST** import submodules explicitly (`hoarfrost::token::color`, `::typography`, `::space`, `::size`, `::shape`, `::time`, `::contrast`) because `token` does not re-export its children.
- You **MUST NOT** hardcode numeric visual literals in widget or application layout code when a token rung exists (`UIF-PRIN-001`).

#### Syntax / Method Signature

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
pub enum Easing { Linear, EaseOutCubic, EaseInOutCubic }
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

#### Examples

```rust
use hoarfrost::token::color::Palette;
use hoarfrost::token::{shape, size, space};

let palette = Palette::dark();
let height = size::CONTROL_MD;            // 32.0
let gap = space::SM;                      // 8.0
let radius = shape::RADIUS_WIDGET;        // 4.0
```

#### Type references

`Palette` fields in declaration order, each mapping to one `color.*` spec token:

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

`Palette` provides `Palette::dark()` as its sole built-in constructor. Construct custom palettes by populating the 23 `iced::Color` fields directly.

#### Return values

`resolve_ms` returns duration in whole milliseconds, or `0` when `reduced_motion` is `true` and `intended` carries an easing curve. `eased_progress` returns a clamped progress fraction in `0.0..=1.0`.

#### Side effects

None. Every item is a compile-time constant or a pure function.

#### References

- [`token/color.rs`](file:///D:/hoarfrost/src/hoarfrost/token/color.rs)
- [`token/typography.rs`](file:///D:/hoarfrost/src/hoarfrost/token/typography.rs)
- [`token/space.rs`](file:///D:/hoarfrost/src/hoarfrost/token/space.rs), [`token/size.rs`](file:///D:/hoarfrost/src/hoarfrost/token/size.rs), [`token/shape.rs`](file:///D:/hoarfrost/src/hoarfrost/token/shape.rs)
- [`token/time.rs`](file:///D:/hoarfrost/src/hoarfrost/token/time.rs)
- [`token/contrast.rs`](file:///D:/hoarfrost/src/hoarfrost/token/contrast.rs)

---

### `token::contrast` - contrast audit rules

#### Overview

The `token::contrast` module provides pure design token contrast evaluators implementing WCAG 2.1 relative luminance, contrast ratios, alpha compositing, and solid-fill foreground lock rules (`UIF-TOK-006`, `UIF-VER-001`). Style resolvers and verification tests use these functions to audit foreground/background colour pairs without a renderer.

#### Prerequisites & Requirements

- No Cargo feature is required; import from `hoarfrost::token::contrast::*`.
- You **MUST** composite translucent colours over their backing surface with `composite_over` before calling `contrast_ratio` or `meets_*`.
- You **SHOULD** call `meets_*` and `fill_lock` in tests and verification gates, and you **SHOULD NOT** call `assert_*` on release-critical hot paths where custom third-party palettes are untrusted.

#### Syntax / Method Signature

```rust
pub const MIN_TEXT_CONTRAST: f32 = 4.5;
pub const MIN_CHROME_CONTRAST: f32 = 3.0;
pub const MIN_COINCIDENT_FILL_CONTRAST: f32 = 3.0;
pub const DISABLED_TEXT_MIN_CONTRAST: f32 = 4.0;
pub const DISABLED_TEXT_MAX_CONTRAST: f32 = 6.05;

pub enum ChromeClass {
    Judged,
    CoincidentWithFill,
    UnpaintedBackground,
    RestingDivider,
}

pub fn relative_luminance(color: Color) -> f32;
pub fn contrast_ratio(a: Color, b: Color) -> f32;
pub fn composite_over(fg: Color, bg: Color) -> Color;
pub fn meets_text_contrast(fg: Color, bg: Color) -> bool;
pub fn meets_chrome_contrast(a: Color, b: Color) -> bool;
pub fn meets_coincident_fill_contrast(fill: Color, surface: Color) -> bool;
pub fn meets_chrome_contrast_as(class: ChromeClass, a: Color, b: Color) -> bool;
pub fn meets_disabled_text_band(fg: Color, bg: Color) -> bool;

pub enum FillLockRole {
    Unlocked,
    LockedFill,
    ForegroundOnly,
}

pub enum FillLock {
    NotSolidFill,
    Locked,
    ForegroundNotPainted,
    ForegroundOnlyOnFill,
    WrongForegroundOnFill,
}

pub fn fill_lock_role(color: Color, palette: &Palette) -> FillLockRole;
pub fn fill_lock(background: Color, foreground: Color, palette: &Palette) -> FillLock;
pub fn meets_fill_foreground_lock(background: Color, foreground: Color, palette: &Palette) -> bool;

// Debug-only panics. They abort a debug build, never a release build.
pub fn assert_fill_foreground_lock(background: Color, foreground: Color, palette: &Palette);
pub fn assert_contrast(fg: Color, bg: Color, min_ratio: f32, label: &str);
```

#### Examples

```rust
use hoarfrost::token::color::Palette;
use hoarfrost::token::contrast::{meets_fill_foreground_lock, meets_text_contrast};

let p = Palette::dark();
assert!(meets_text_contrast(p.text_primary, p.surface_base));
assert!(meets_fill_foreground_lock(p.accent_fill, p.text_strong, &p));
```

#### Type references

- `Color` is `iced::Color` (`{ r, g, b, a }`, each `f32` in `0.0..=1.0`).
- `Palette` is `token::color::Palette`; see [`token`](#token--design-tokens).
- `ChromeClass` classifies a stroke or boundary (`Judged`, `CoincidentWithFill`, `UnpaintedBackground`, `RestingDivider`) so `meets_chrome_contrast_as` applies the corresponding threshold.
- `FillLockRole` classifies a palette colour into `Unlocked`, `LockedFill`, or `ForegroundOnly` under `UIF-TOK-006` rule 4.
- `FillLock` reports the rule-4 compliance verdict for a `(background, foreground)` pair.

#### Return values

Every `meets_*` function returns `bool`. `contrast_ratio` returns a symmetric ratio in `1.0..=21.0`. `fill_lock` returns a `FillLock` variant where `NotSolidFill`, `Locked`, and `ForegroundNotPainted` satisfy `is_compliant()`.

#### Side effects

`assert_fill_foreground_lock` and `assert_contrast` panic via `debug_assert!` when a pair violates the rule in debug builds, and compile to no-ops in release builds.

#### References

- [`token/contrast.rs`](file:///D:/hoarfrost/src/hoarfrost/token/contrast.rs)

---

### `state` - interaction state resolution

#### Overview

The `state` module provides the pure state resolver that converts raw interaction flags (`enabled`, `hovered`, `pressed`, `focused`, `invalid`) into a canonical `VisualState` (`UIF-ATOM-001`, `UIF-ATOM-002`, `UIF-VER-003`). It serves as the single owner of state precedence across all atomic and composite widgets.

#### Prerequisites & Requirements

- No Cargo feature is required; import `hoarfrost::state::{BaseState, InteractionInput, VisualState, resolve}`.
- Widgets and style resolvers **MUST** compute interaction precedence via `state::resolve` rather than branching on raw pointer or focus booleans directly.
- Callers **MUST** set `enabled: true` explicitly when constructing `InteractionInput`, because `InteractionInput::default()` sets `enabled: false` (`BaseState::Disabled`).

#### Syntax / Method Signature

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
pub const SNAPSHOT_STATE_NAMES: [&str; 6] = [
    SNAPSHOT_REST, SNAPSHOT_HOVERED, SNAPSHOT_PRESSED,
    SNAPSHOT_FOCUSED, SNAPSHOT_DISABLED, SNAPSHOT_INVALID,
];
```

#### Examples

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

#### Type references

Precedence for `base` is `Disabled > Invalid > Pressed > Hovered > Rest`. `focused` resolves independently except when `enabled` is `false`, which forces `focused: false`. `snapshot_label` maps `VisualState` to one of the six spec matrix row names (`SNAPSHOT_STATE_NAMES`).

#### Return values

`resolve` always returns a `VisualState` and never fails or panics.

#### Side effects

None. `resolve` is a `const fn` with no internal state.

#### References

- [`state.rs`](file:///D:/hoarfrost/src/hoarfrost/state.rs)

---

### `focus` - focus model

#### Overview

The `focus` module provides the pure focus policy model for tab order, logical focus groups, and modal focus traps (`UIF-PRIN-005`, `UIF-PRIN-008`). It stores no widget references and processes no raw GUI events; applications mutate `FocusState` and project it into iced via `bridge`.

#### Prerequisites & Requirements

- No Cargo feature is required; import `hoarfrost::focus::{FocusId, GroupId, FocusMember, FocusGroup, FocusState, TrapPop}`.
- You **MUST** assign stable, deterministic string identifiers to `FocusId` and `GroupId`, and you **MUST NOT** generate them from per-frame counters.
- When opening a modal surface, you **MUST** call `push_trap` after registering the modal's `FocusGroup` and call `pop_trap` before unregistering the group on close.

#### Syntax / Method Signature

```rust
pub struct FocusId(String);
impl FocusId {
    pub fn new(id: impl Into<String>) -> Self;
    pub fn as_str(&self) -> &str;
}
impl From<&str> for FocusId;
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

#### Examples

```rust
use hoarfrost::focus::{FocusMember, FocusState, GroupId};

let mut focus = FocusState::new();
let group = GroupId::new("demo.toolbar");
focus.register_group(
    group,
    vec![
        FocusMember::new("demo.toolbar.save"),
        FocusMember::disabled("demo.toolbar.undo"),
        FocusMember::new("demo.toolbar.redo"),
    ],
);

// Disabled members keep their slot in group order but traversal skips them.
assert_eq!(focus.focus_next().map(|id| id.as_str().to_owned()), Some("demo.toolbar.save".to_owned()));
assert_eq!(focus.focus_next().map(|id| id.as_str().to_owned()), Some("demo.toolbar.redo".to_owned()));
```

#### Type references

- `FocusId` and `GroupId` are string newtypes identifying individual focusable controls and ordered focus groups.
- `FocusMember` pairs a `FocusId` with an `enabled` boolean; disabled members retain their slot order while traversal skips them.
- `TrapPop` reports the popped trap's `group`, the `recorded` pre-trap focus, the `restored` focus, and any `lost` focus id that was no longer focusable.

#### Return values

- Traversal methods (`focus_next`, `focus_previous`, `focus_first_in_group`, `focus_last_in_group`, `move_within`) return `Option<FocusId>` with the newly focused control id, or `None` when no focusable member exists.
- `focus` returns `true` when `id` is registered, enabled, and within the active trap scope.
- `register_group`, `unregister_group`, `set_member_enabled`, and `clear` return the `Option<FocusId>` evicted from focus, if any.

#### Side effects

All `&mut self` methods mutate `FocusState` in memory only; you **MUST** execute a `bridge` task to synchronize iced's focus router.

#### References

- [`focus.rs`](file:///D:/hoarfrost/src/hoarfrost/focus.rs)

---

### `bridge` - focus model to iced operations

#### Overview

The `bridge` module provides one-way projection primitives that translate `FocusState` transitions into `iced::Task` and `iced::advanced::widget::Operation` commands (`UIF-PRIN-005`). It keeps iced's internal widget focus router synchronized with `FocusState` without reading focus state back from iced.

#### Prerequisites & Requirements

- No Cargo feature is required; import `hoarfrost::bridge::{Direction, focus_operation, focus_task, tab_operation, tab_task, widget_id}`.
- Your `Message` type **MUST** satisfy `Send + 'static`.
- You **MUST** pass the same `FocusId` string to both `FocusState` and the widget constructor so `bridge::widget_id` targets the matching iced widget.

#### Syntax / Method Signature

```rust
pub fn widget_id(focus_id: &FocusId) -> iced::advanced::widget::Id;

pub enum Direction { Forward, Backward }
impl Direction { pub const fn from_shift(shifted: bool) -> Self; }

pub fn focus_operation<Message: Send + 'static>(model: &FocusState) -> Box<dyn Operation<Message>>;
pub fn focus_task<Message: Send + 'static>(model: &FocusState) -> Task<Message>;
pub fn tab_operation<Message: Send + 'static>(model: &mut FocusState, direction: Direction) -> Box<dyn Operation<Message>>;
pub fn tab_task<Message: Send + 'static>(model: &mut FocusState, direction: Direction) -> Task<Message>;
```

#### Examples

```rust
use hoarfrost::bridge::{tab_task, Direction};
use hoarfrost::focus::FocusState;

fn handle_tab<Message: Send + 'static>(focus: &mut FocusState, shifted: bool) -> iced::Task<Message> {
    tab_task(focus, Direction::from_shift(shifted))
}
```

#### Type references

- `Direction` selects `Forward` (`Tab`) or `Backward` (`Shift+Tab`) traversal via `Direction::from_shift(shifted)`.
- `widget_id` converts a `FocusId` into the corresponding `iced::advanced::widget::Id`.

#### Return values

`focus_operation` and `tab_operation` return a boxed `Operation<Message>`. `focus_task` and `tab_task` wrap that operation in an `iced::Task<Message>` ready to return from `update`.

#### Side effects

`tab_operation` and `tab_task` advance `FocusState` in place before constructing the iced operation. `focus_operation` and `focus_task` read `FocusState` without mutating it.

#### References

- [`bridge.rs`](file:///D:/hoarfrost/src/hoarfrost/bridge.rs)

---

### `binding` - ownership, tiers, gestures, gating

#### Overview

The `binding` module provides the core ownership primitives (`Controlled`, `Ephemeral`), callback tier wrappers (`Live`, `Commit`, `Cancel`), gesture snapshots (`GestureSlot`), busy-action gates (`ActionPending`, `ActivationGate`), pre-rank drag capture (`PreRankCoordinator`), and container-width responsive solvers (`ResponsiveLayout`) (`UIF-ATOM-000b` §3.0.4.1 - §3.0.4.4). Every interactive atom, composite, and `*Host controller` builds its state lifecycle on these types.

#### Prerequisites & Requirements

- No Cargo feature is required; import from `hoarfrost::binding::*`.
- You **MUST NOT** execute blocking disk or network I/O inside a `Live<T>` handler (`UIF-ATOM-000b` §3.0.4.2).
- When handling `Cancel<T>`, you **MUST** restore the application's domain model to `cancel.into_restored()` (or `*cancel.restored()`).

#### Syntax / Method Signature

```rust
// Ownership (binding/controlled.rs)
pub struct Controlled<'a, M> { /* &'a M */ }
impl<'a, M> Controlled<'a, M> {
    pub fn new(model: &'a M) -> Self;
    pub fn get(&self) -> &'a M;
    pub fn map<N>(&self, project: impl FnOnce(&'a M) -> &'a N) -> Controlled<'a, N>;
}
impl<'a, M> Clone for Controlled<'a, M>;
impl<'a, M> Copy for Controlled<'a, M>;

pub struct Ephemeral<S> { /* S */ }
impl<S> Ephemeral<S> {
    pub fn new(state: S) -> Self;
    pub fn get(&self) -> &S;
    pub fn get_mut(&mut self) -> &mut S;
    pub fn set(&mut self, state: S) -> S;
    pub fn reset(&mut self) -> S where S: Default;
}

// Tiers (binding/tier.rs)
pub struct Live<T>;
pub struct Commit<T>;
impl<T> Live<T>   { pub fn new(value: T) -> Self; pub fn value(&self) -> &T; pub fn into_value(self) -> T; }
impl<T> Commit<T> { pub fn new(value: T) -> Self; pub fn value(&self) -> &T; pub fn into_value(self) -> T; }

pub struct LiveCoalescer<T>;
impl<T> LiveCoalescer<T> {
    pub fn new() -> Self;
    pub fn push(&mut self, value: T) -> bool;
    pub fn is_pending(&self) -> bool;
    pub fn coalesced(&self) -> usize;
    pub fn take(&mut self) -> Option<T>;
}

// Snapshots and gestures (binding/snapshot.rs)
pub struct Snapshot<T>;
impl<T> Snapshot<T> {
    pub fn capture(value: &T) -> Self where T: Clone;
    pub fn value(&self) -> &T;
    pub fn into_value(self) -> T;
    pub fn matches(&self, current: &T) -> bool where T: PartialEq;
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
    pub fn snapshot(&self) -> &Snapshot<T>;
    pub fn live(&self, value: T) -> Live<T>;
    pub fn commit(self, value: T) -> Commit<T>;
    pub fn cancel(self) -> Cancel<T>;
}

pub struct GestureSlot<T>;
impl<T> GestureSlot<T> {
    pub fn new() -> Self;
    pub fn is_open(&self) -> bool;
    pub fn begin(&mut self, current: &T) -> GestureStart where T: Clone;
    pub fn live(&self, value: T) -> Option<Live<T>>;
    pub fn commit(&mut self, value: T) -> Option<Commit<T>>;
    pub fn cancel(&mut self) -> Option<Cancel<T>>;
    pub fn abort(&mut self);
}

// Busy actions and activation gating (binding/pending.rs)
pub struct ActionPending;
impl ActionPending {
    pub const fn idle() -> Self;
    pub const fn indeterminate() -> Self;
    pub fn determinate(progress: f32) -> Self;
    pub const fn with_cancellable(self, cancellable: bool) -> Self;
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

// Drag pre-emption (binding/pre_rank.rs)
pub enum DragKind {
    Slider,
    RangeSlider,
    SplitDivider,
    ColumnResize,
    SideSheetResize,
    OverlayMove,
    OverlayResize,
    Scrollbar,
}
pub struct CaptureId(String);
impl CaptureId { pub fn new(id: impl Into<String>) -> Self; pub fn as_str(&self) -> &str; }
pub struct DragCapture;
impl DragCapture {
    pub fn new(id: impl Into<CaptureId>, kind: DragKind) -> Self;
    pub fn id(&self) -> &CaptureId;
    pub fn kind(&self) -> DragKind;
}
pub enum CaptureStart { Started, AlreadyHeld { held: DragCapture } }
impl CaptureStart { pub fn is_started(&self) -> bool; pub fn held_by(&self) -> Option<&DragCapture>; }
pub enum PreRankOutcome {
    Cancelled { capture: DragCapture },
    Stale { capture: DragCapture },
    NotCaptured,
}
impl PreRankOutcome {
    pub fn is_cancelled(&self) -> bool;
    pub fn proceed_with_rank_walk(&self) -> bool;
    pub fn capture(&self) -> Option<&DragCapture>;
}
pub struct PreRankCoordinator;
impl PreRankCoordinator {
    pub fn new() -> Self;
    pub fn is_captured(&self) -> bool;
    pub fn capture(&self) -> Option<&DragCapture>;
    pub fn begin(&mut self, capture: DragCapture) -> CaptureStart;
    pub fn release(&mut self, id: &CaptureId) -> bool;
    pub fn clear(&mut self) -> Option<DragCapture>;
    pub fn pre_rank_cancel<F>(&mut self, cancel: F) -> PreRankOutcome where F: FnOnce(&DragCapture) -> bool;
}

// Responsive rungs (binding/responsive.rs)
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

#### Examples

```rust
use hoarfrost::binding::{ActionPending, ActivationGate, GestureSlot, GestureStart};

// Suppress a second activation while the first action is still in flight.
let mut gate = ActivationGate::new();
let busy = ActionPending::indeterminate().with_cancellable(true);
assert!(gate.try_activate(&ActionPending::idle()).is_fired());
assert!(gate.try_activate(&busy).is_suppressed());

// Capture a pre-gesture value and restore it on Escape.
let mut slot = GestureSlot::new();
let mut width = 240.0_f32;
if let GestureStart::Started = slot.begin(&width) {
    width = 300.0;
    if let Some(cancel) = slot.cancel() {
        width = cancel.into_restored();
    }
}
assert_eq!(width, 240.0);
```

#### Type references

- `Controlled<'a, M>` wraps `&'a M` to enforce read-only widget access to application models.
- `Ephemeral<S>` wraps transient widget or controller state that drops when the widget tree rebuilds.
- `GestureSlot<T>` retains the outer snapshot across nested `begin` calls and consumes the open gesture on `commit` or `cancel`.
- `ActivationGate` latches on activation and suppresses subsequent presses with `SuppressedReason::{Busy, AwaitingProjection}` until `settle`.
- `PreRankCoordinator` tracks a single active pointer drag (`DragCapture`) so `Escape` cancels the drag before unwinding any `ZBand` surface.
- `ResponsiveLayout` resolves `Breakpoint::{Expanded, Compact}` and `LabelMode::{Full, IconOnly}` from allocated parent container width.

#### Return values

- `GestureSlot::{live, commit, cancel}` return `None` when no gesture is open.
- `ActionPending::try_cancel` returns `Ok(CancelRequest)` when busy and cancellable, or `Err(CancelRefusal::{NotBusy, NotCancellable})`.
- `ActivationGate::settle` returns `true` when an in-flight latch was cleared.
- `PreRankCoordinator::release` returns `true` when `id` matched the active capture.

#### Side effects

None. All types operate in memory without I/O, clocks, or global state.

#### References

- [`binding/controlled.rs`](file:///D:/hoarfrost/src/hoarfrost/binding/controlled.rs), [`binding/tier.rs`](file:///D:/hoarfrost/src/hoarfrost/binding/tier.rs), [`binding/snapshot.rs`](file:///D:/hoarfrost/src/hoarfrost/binding/snapshot.rs)
- [`binding/pending.rs`](file:///D:/hoarfrost/src/hoarfrost/binding/pending.rs), [`binding/pre_rank.rs`](file:///D:/hoarfrost/src/hoarfrost/binding/pre_rank.rs), [`binding/responsive.rs`](file:///D:/hoarfrost/src/hoarfrost/binding/responsive.rs)

---

### `identity` - addressable sub-part text ids

#### Overview

The `identity` module provides pure naming derivations and an operation publisher (`publish_text`) that expose custom widget sub-part text under deterministic `<widget-id>.<role>` identifiers. Test harnesses and accessibility queries use these derived IDs to inspect rendered text without reaching into private widget fields.

#### Prerequisites & Requirements

- No Cargo feature is required; import `hoarfrost::identity::*`.
- Custom widgets **MUST** call `publish_text` inside `Widget::operate` only when the parent widget has a named `Id` (`Id::new(...)`, not `Id::unique()`).
- Sub-part role names **MUST** come from `identity::role::*` (or `indexed_role`) so `split_sub_part_id` can parse them unambiguously.

#### Syntax / Method Signature

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

#### Examples

```rust
use hoarfrost::identity::{publish_text, role, sub_part_id};
use iced::advanced::widget::Id;

let base = Id::new("row.primary");
let title_id = sub_part_id(&base, role::TITLE);
assert!(title_id.is_some());

// Inside Widget::operate:
// publish_text(operation, &self.id, role::TITLE, title_bounds, "Workspace");
```

#### Type references

Sub-part identifiers follow the pattern `<widget-id>.<role>` or `<widget-id>.<role>-<index>`. `INDEX_SEPARATOR` is `'-'` so dotted widget IDs (`"a.b.c"`) round-trip through `split_sub_part_id` without ambiguity.

#### Return values

`custom_name` returns `None` for anonymous `Id::unique()` values. `sub_part_id` returns `None` when `base` has no custom name or `role_name` is empty. `split_sub_part_name` and `split_sub_part_id` return `Some((base_name, role_segment))` when the suffix is a recognised role.

#### Side effects

`publish_text` emits one `operation::text` entry into `operation` when `base` is named and `content` is non-empty; it never mutates widget layout or draw state.

#### References

- [`identity.rs`](file:///D:/hoarfrost/src/hoarfrost/identity.rs)

---

### `strings` - user-facing text catalogue

#### Overview

The `strings` module provides the pure compile-time catalogue of default English user-facing string constants (`pub const &str`) rendered by `hoarfrost` controls (`HF-001`, `RUST-021`). Isolating every default label, placeholder, and validation message in one module prevents hardcoded string literals across widget implementations.

#### Prerequisites & Requirements

- No Cargo feature is required; import `hoarfrost::strings::*`.
- Widget implementations **MUST** reference constants from `hoarfrost::strings` instead of embedding inline user-facing string literals (`HF-001`).
- Applications that localise user-facing text **MUST** pass translated strings into widget builder methods at the call site.

#### Syntax / Method Signature

All items are `pub const &str`:

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

Composite modules re-export the constants they consume (for example `composite::stepper::LABEL_NEXT`).

#### Examples

```rust
use hoarfrost::strings::{LABEL_FINISH, PLACEHOLDER_COMMANDS};

let finish_label = LABEL_FINISH;        // "Finish ✓"
let search_hint = PLACEHOLDER_COMMANDS; // "Type a command..."
```

#### Type references

Every item is `pub const &'static str`. This module defines no structs, enums, or functions.

#### Return values

Not applicable; this module exposes constants only.

#### Side effects

None.

#### References

- [`strings.rs`](file:///D:/hoarfrost/src/hoarfrost/strings.rs)

---
### `widget::button`, `widget::button_model`, `widget::button_controller`, `widget::button_widget` - Button Atom (`UIF-ATOM-001`, `UIF-ATOM-001a`)

#### Overview
The `widget::button_widget` module provides a **live `iced` widget (`Element<'a, Message>`)** (`Button`), while `widget::button`, `widget::button_model`, and `widget::button_controller` provide the **headless model/controller + style resolver** (`ButtonModel`, `ButtonController`, `ButtonStyle`, and `resolve_style`) implementing `UIF-ATOM-001` and `UIF-ATOM-001a` (spec §3.1). Together they enforce single-flight activation gating, three-tier settlement emissions (`Activated`, `Cancelled`, `CancelRequested`), adaptive contrast-safe focus rings, and mandatory tooltips on collapsed icon-only buttons.

#### Prerequisites & Requirements
- The application **MUST** supply a unique `FocusId` for every `Button` instance so the focus bridge can project keyboard focus onto the underlying `iced::advanced::widget::Id`.
- When constructing a controlled button via `Button::controlled`, the application **MUST** retain ownership of `ButtonModel` and pass a shared borrow wrapped in `Controlled<'a, ButtonModel>`.
- When a button renders in `ButtonDensity::Compact` or through `Button::icon_only` where no visible text label is present, the application **MUST** configure a non-empty tooltip string via `ButtonModel::with_tooltip` or `Button::with_tooltip`.
- When `ButtonModel::tracks_action()` is `true` (`busy || cancellable`), the application **MUST** call `ButtonController::observe(&model)` once per frame when driving `ButtonController` headlessly so the `ActivationGate` settles once `busy` transitions back to `false`.

#### Syntax / Method Signature
```rust
// widget::button
pub const HEIGHT_COMPACT: f32 = size::CONTROL_SM;
pub const HEIGHT_STANDARD: f32 = size::CONTROL_MD;
pub const HORIZONTAL_PADDING: f32 = space::MD;
pub const ICON_LABEL_GAP: f32 = space::SM;
pub const RADIUS: f32 = shape::RADIUS_WIDGET;
pub const BORDER_WIDTH: f32 = shape::BORDER_HAIRLINE;
pub const FOCUS_RING_WIDTH: f32 = shape::BORDER_FOCUS;
pub const ICON_ONLY_MIN_WIDTH: f32 = size::CONTROL_SM;
pub const ICON_ONLY_MIN_HEIGHT: f32 = size::CONTROL_SM;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ButtonStyle {
    pub background: Color,
    pub foreground: Color,
    pub border_color: Color,
    pub border_width: f32,
    pub focus_ring_color: Color,
    pub focus_ring_width: f32,
}

impl ButtonStyle {
    #[must_use]
    pub const fn is_transparent_background(&self) -> bool;
}

#[must_use]
pub fn resolve_style(variant: ButtonVariant, state: VisualState, palette: &Palette) -> ButtonStyle;

// widget::button_model
pub const SPINNER_SLOT: f32 = size::SPINNER_SM;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ButtonIcon {
    Glyph(&'static str),
    Reserved,
}

impl ButtonIcon {
    pub fn width(&self) -> f32;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonDensity {
    #[default]
    Standard,
    Compact,
}

impl ButtonDensity {
    #[must_use]
    pub const fn height(self) -> f32;
    pub fn breakpoint(self) -> Breakpoint;
    pub fn is_collapsed(self) -> bool;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ButtonModel {
    pub label: Option<String>,
    pub leading_icon: Option<ButtonIcon>,
    pub trailing_icon: Option<ButtonIcon>,
    pub variant: ButtonVariant,
    pub density: ButtonDensity,
    pub busy: bool,
    pub disabled: bool,
    pub cancellable: bool,
    pub tooltip: Option<String>,
}

pub type ControlledModel<'a> = Controlled<'a, ButtonModel>;

impl ButtonModel {
    pub fn new(label: impl Into<String>) -> Self;
    pub fn icon_only(icon: &'static str) -> Self;
    pub fn with_variant(self, variant: ButtonVariant) -> Self;
    pub fn leading(self, icon: ButtonIcon) -> Self;
    pub fn trailing(self, icon: ButtonIcon) -> Self;
    pub fn with_density(self, density: ButtonDensity) -> Self;
    pub fn with_tooltip(self, tooltip: impl Into<String>) -> Self;
    pub fn busy(self) -> Self;
    pub fn cancellable(self) -> Self;
    pub fn disabled(self) -> Self;
    pub fn controlled(&self) -> Controlled<'_, ButtonModel>;
    pub fn tracks_action(&self) -> bool;
    pub fn pending(&self) -> ActionPending;
    pub fn is_icon_only(&self, mode: LabelMode) -> bool;
    pub fn tooltip_for(&self, mode: LabelMode) -> Option<&str>;
    pub fn collapsed_min_width(&self) -> f32;
    pub fn expanded_min_width(&self) -> f32;
    pub fn measurement_with_label(&self, label_width: f32) -> Measurement;
    pub fn measurement(&self) -> Measurement;
    pub fn responsive(&self) -> ResponsiveLayout;
    pub fn responsive_to(&self, allocated: f32) -> ResponsiveLayout;
}

// widget::button_controller
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivationSource {
    Pointer,
    Keyboard,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ButtonSettlement {
    pub source: ActivationSource,
    pub activated: Option<Activation>,
    pub pending: ActionPending,
}

impl ButtonSettlement {
    #[must_use]
    pub fn undispatched(source: ActivationSource, pending: ActionPending) -> Self;
    #[must_use]
    pub fn dispatched(&self) -> bool;
}

#[derive(Clone)]
pub enum ButtonEmission {
    Activated(Commit<ButtonSettlement>),
    Cancelled(Cancel<ButtonSettlement>),
    CancelRequested(CancelRequest),
}

impl ButtonEmission {
    #[must_use]
    pub fn settlement(&self) -> Option<&ButtonSettlement>;
}

#[derive(Debug, Default)]
pub struct ButtonController { /* private fields */ }

impl ButtonController {
    #[must_use]
    pub fn new() -> Self;
    pub fn observe(&mut self, model: &ButtonModel);
    pub fn begin(&mut self, model: &ButtonModel, source: ActivationSource) -> Option<ButtonEmission>;
    pub fn commit(&mut self, model: &ButtonModel, source: ActivationSource) -> Option<ButtonEmission>;
    pub fn cancel(&mut self, model: &ButtonModel) -> Option<ButtonEmission>;
    #[must_use]
    pub fn gesture_open(&self) -> bool;
    #[must_use]
    pub fn last_begin(&self) -> Option<GestureStart>;
    #[must_use]
    pub fn is_pending(&self) -> bool;
    #[must_use]
    pub fn busy_observed(&self) -> bool;
    #[must_use]
    pub fn fired(&self) -> usize;
    #[must_use]
    pub fn suppressed(&self) -> usize;
    #[must_use]
    pub fn cancel_requested(&self) -> bool;
    pub fn reset(&mut self);
}

// widget::button_widget
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layer {
    pub bounds: Rectangle,
    pub fill: Color,
    pub border: Border,
}

impl Layer {
    #[must_use]
    pub fn quad(&self) -> renderer::Quad;
}

#[must_use]
pub fn background_layer(bounds: Rectangle, style: &ButtonStyle) -> Option<Layer>;
#[must_use]
pub fn focus_ring_layer(bounds: Rectangle, style: &ButtonStyle) -> Option<Layer>;
#[must_use]
pub fn busy_spinner_layer(
    bounds: Rectangle,
    padding: Padding,
    style: &ButtonStyle,
    busy: bool,
    palette: &Palette,
) -> Option<Layer>;

pub struct Button<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> { /* private fields */ }

impl<'a, Message, Theme, Renderer> Button<'a, Message, Theme, Renderer> {
    #[must_use]
    pub fn new(
        variant: ButtonVariant,
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
        on_press: Message,
        focus_id: impl Into<FocusId>,
    ) -> Self;
    #[must_use]
    pub fn controlled(
        model: Controlled<'a, ButtonModel>,
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
        focus_id: impl Into<FocusId>,
        emit: impl Fn(ButtonEmission) -> Option<Message> + 'a,
    ) -> Self;
    #[must_use]
    pub fn icon_only(
        variant: ButtonVariant,
        icon: impl Into<Element<'a, Message, Theme, Renderer>>,
        on_press: Message,
        focus_id: impl Into<FocusId>,
        tooltip: impl Into<String>,
    ) -> Self;
    #[must_use]
    pub fn with_tooltip(self, tooltip: impl Into<String>) -> Self;
    #[must_use]
    pub fn compact(self) -> Self;
    #[must_use]
    pub fn standard(self) -> Self;
    #[must_use]
    pub fn disabled(self) -> Self;
    #[must_use]
    pub fn enabled(self) -> Self;
    #[must_use]
    pub fn invalid(self) -> Self;
    #[must_use]
    pub fn palette(self, palette: Palette) -> Self;
    #[must_use]
    pub fn variant(&self) -> ButtonVariant;
    #[must_use]
    pub fn focus_id(&self) -> &FocusId;
    #[must_use]
    pub fn widget_id(&self) -> &Id;
    #[must_use]
    pub fn is_enabled(&self) -> bool;
    #[must_use]
    pub fn tooltip_text(&self) -> Option<&str>;
    #[must_use]
    pub fn into_element(self) -> Element<'a, Message, Theme, Renderer>
    where
        Message: 'a + Clone,
        Theme: 'a,
        Renderer: 'a + renderer::Renderer + iced::advanced::text::Renderer<Font = iced::Font>;
    #[must_use]
    pub fn is_icon_only(&self, content: Size) -> bool;
    #[must_use]
    pub fn tooltip_violation(&self, content: Size) -> Option<TooltipViolation>;
}
```

#### Examples
```rust
use hoarfrost::focus::FocusId;
use hoarfrost::widget::button::ButtonVariant;
use hoarfrost::widget::button_controller::{ActivationSource, ButtonController, ButtonEmission};
use hoarfrost::widget::button_model::ButtonModel;
use hoarfrost::widget::button_widget::Button;

#[derive(Clone, Debug)]
enum AppMsg {
    DeployCommitted( bool),
    DeployAbandoned,
    DeployCancelRequested,
}

// 1. Declarative model with cancellable in-flight tracking
let model = ButtonModel::new("Deploy")
    .with_variant(ButtonVariant::Primary)
    .cancellable();

// 2. Headless controller lifecycle
let mut ctrl = ButtonController::new();
ctrl.observe(&model);
let _ = ctrl.begin(&model, ActivationSource::Pointer);
if let Some(ButtonEmission::Activated(commit)) = ctrl.commit(&model, ActivationSource::Pointer) {
    assert!(commit.value().dispatched());
}

// 3. Live iced widget construction over the controlled model
let _element: iced::Element<'_, AppMsg> = Button::controlled(
    model.controlled(),
    iced::widget::text("Deploy"),
    FocusId::new("deploy-cta"),
    |emission| match emission {
        ButtonEmission::Activated(commit) => Some(AppMsg::DeployCommitted(commit.value().dispatched())),
        ButtonEmission::Cancelled(_cancel) => Some(AppMsg::DeployAbandoned),
        ButtonEmission::CancelRequested(_req) => Some(AppMsg::DeployCancelRequested),
    },
)
.into_element();
```

#### Type references
- `ButtonVariant` - Enumerates the four visual hierarchies (`Primary`, `Secondary`, `Ghost`, `Danger`) defined in spec §3.1.
- `ButtonStyle` - Holds the resolved background, foreground, border, and focus ring colors and widths for a single `(ButtonVariant, VisualState)` cell.
- `ButtonIcon` - Represents a leading or trailing icon slot (`Glyph(&'static str)` or `Reserved` spinner space).
- `ButtonDensity` - Selects between `Standard` (`32px`, `size.control.md`) and `Compact` (`28px`, `size.control.sm`) control heights.
- `ButtonModel` - Host-owned declarative configuration for a button's text, icons, variant, density, `busy`, `disabled`, `cancellable`, and `tooltip` state.
- `ActivationSource` - Identifies whether a button gesture originated from `Pointer` or `Keyboard`.
- `ButtonSettlement` - Snapshot payload capturing `source`, `activated: Option<Activation>`, and `pending: ActionPending` at gesture start and settlement.
- `ButtonEmission` - Three-tier sum type (`Activated(Commit<ButtonSettlement>)`, `Cancelled(Cancel<ButtonSettlement>)`, `CancelRequested(CancelRequest)`) emitted by `ButtonController` and `Button::controlled`.
- `ButtonController` - Ephemeral per-control state machine managing gesture linearity, double-click busy suppression, and stop requests.
- `Layer` - A resolved quad (`bounds`, `fill`, `border`) produced by `background_layer`, `focus_ring_layer`, and `busy_spinner_layer`.
- `Button` - The live, keyboard-focusable `iced` widget implementing `UIF-ATOM-001` and `UIF-ATOM-001a`.

#### Return values
- `resolve_style` returns a `ButtonStyle` guaranteed to satisfy the `UIF-TOK-006` solid-fill foreground lock and `3.0:1` adaptive focus ring contrast floor.
- `ButtonController::begin`, `commit`, and `cancel` return `Option<ButtonEmission>` representing at most one settlement or stop-request tier per gesture transition.
- `Button::into_element` returns an `iced::Element<'a, Message, Theme, Renderer>`, automatically wrapping the button in a `surface::tooltip::Tooltip` when `self.tooltip` is `Some`.

#### Side effects
- `resolve_style` and `ButtonModel` methods are pure and perform no I/O or mutation.
- `ButtonController` methods mutate only the controller's in-memory `GestureSlot` and `ActivationGate` counters.
- `Button::operate` registers the widget with `iced`'s focus router when enabled and unconditionally unfocuses it when disabled.

#### References
- Requirements: `UIF-ATOM-001`, `UIF-ATOM-001a`, `UIF-PRIN-003`, `UIF-PRIN-005`, `UIF-TOK-006`
- Source: [`button.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/button.rs), [`button_model.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/button_model.rs), [`button_controller.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/button_controller.rs), [`button_widget.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/button_widget.rs)

---

### `widget::text_input`, `widget::text_input_model`, `widget::text_input_controller` - Single-Line Text Input (`UIF-ATOM-002`, `UIF-ATOM-002a`)

#### Overview
The `widget::text_input` module provides both a **live `iced` widget (`Element<'a, Message>`)** (`TextInput`) and a pure **style and geometry resolver** (`resolve_input_style`, `text_area_bounds`, `trailing_slot_bounds`), while `widget::text_input_model` and `widget::text_input_controller` provide the **headless model/controller** (`TextInputModel`, `TextInputController`, `TextInputEmission`) implementing `UIF-ATOM-002` and `UIF-ATOM-002a` (spec §3.2). The atom permanently reserves its trailing icon slot so validation, search, clear, and reveal affordances never shift the editable text bounds (`UIF-PRIN-003`).

#### Prerequisites & Requirements
- The application **MUST** supply a `FocusId` when constructing `TextInput::new` or `TextInput::controlled` so focus routing and `Escape` session snapshots bind to a stable widget identity.
- When driving `TextInput::controlled`, the application **MUST** hold `TextInputModel` in its own state and pass `model.controlled()` each frame.
- To honor `UIF-PRIN-009` ("reward early, punish late"), the application **SHOULD** advance the model's `EditStance` via `note_commit`, `note_blur`, or `note_submit` when `TextInputEmission::advances_stance()` returns `true`.

#### Syntax / Method Signature
```rust
// widget::text_input
pub const HEIGHT_STANDARD: f32 = size::CONTROL_MD;
pub const HEIGHT_COMPACT: f32 = size::CONTROL_SM;
pub const HORIZONTAL_PADDING: f32 = space::MD;
pub const RADIUS: f32 = shape::RADIUS_WIDGET;
pub const SLOT_GAP: f32 = space::SM;
pub const SLOT_GLYPH_SIZE: f32 = size::ICON_MD;
pub const GLYPH_SEARCH: char = '🔍';
pub const GLYPH_CLEAR: char = '×';
pub const GLYPH_ERROR: char = '⚠';
pub const GLYPH_CHEVRON: char = '▾';
pub const GLYPH_REVEAL: char = '\u{1F441}';

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InputKind {
    TextEntry,
    Search,
    DropdownTrigger,
    Password,
    TextArea,
}

impl InputKind {
    #[must_use]
    pub const fn trailing_slot_width(self) -> f32;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotContent {
    Empty,
    Passive(char),
    Actionable(char),
    Toggle(char),
}

#[must_use]
pub const fn slot_content(kind: InputKind, has_value: bool, invalid: bool) -> SlotContent;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InputStyle {
    pub background: Color,
    pub text: Color,
    pub placeholder: Color,
    pub icon: Color,
    pub selection_background: Color,
    pub selection_foreground: Color,
    pub border_color: Color,
    pub border_width: f32,
    pub focus_ring_color: Color,
    pub focus_ring_width: f32,
}

#[must_use]
pub fn resolve_input_style(kind: InputKind, state: VisualState, palette: &Palette) -> InputStyle;
#[must_use]
pub fn text_area_bounds(bounds: Rectangle, kind: InputKind) -> Rectangle;
#[must_use]
pub fn trailing_slot_bounds(bounds: Rectangle, kind: InputKind) -> Rectangle;
#[must_use]
pub fn background_layer(bounds: Rectangle, style: &InputStyle) -> Option<Layer>;
#[must_use]
pub fn focus_ring_layer(bounds: Rectangle, style: &InputStyle) -> Option<Layer>;

pub struct TextInput<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> { /* private fields */ }

impl<'a, Message, Theme, Renderer> TextInput<'a, Message, Theme, Renderer>
where
    Message: Clone + 'a,
    Theme: iced::widget::text_input::Catalog + 'a,
    Renderer: iced::advanced::text::Renderer + 'a,
    Theme::Class<'a>: From<iced::widget::text_input::StyleFn<'a, Theme>>,
{
    #[must_use]
    pub fn new(
        kind: InputKind,
        placeholder: impl Into<String>,
        value: impl Into<String>,
        on_input: impl Fn(String) -> Message + 'a,
        focus_id: impl Into<FocusId>,
    ) -> Self;
    #[must_use]
    pub fn controlled(
        kind: InputKind,
        model: Controlled<'a, TextInputModel>,
        focus_id: impl Into<FocusId>,
        emit: impl Fn(TextInputEmission) -> Option<Message> + 'a,
    ) -> Self;
    #[must_use]
    pub fn compact(self) -> Self;
    #[must_use]
    pub fn standard(self) -> Self;
    #[must_use]
    pub fn width(self, width: impl Into<Length>) -> Self;
    #[must_use]
    pub fn disabled(self) -> Self;
    #[must_use]
    pub fn enabled(self) -> Self;
    #[must_use]
    pub fn invalid(self) -> Self;
    #[must_use]
    pub fn valid(self) -> Self;
    #[must_use]
    pub fn read_only(self) -> Self;
    #[must_use]
    pub fn on_clear(self, message: Message) -> Self;
    #[must_use]
    pub fn on_reveal(self, message: Message) -> Self;
    #[must_use]
    pub fn masked(self, masked: bool) -> Self;
    #[must_use]
    pub fn palette(self, palette: Palette) -> Self;
}

// widget::text_input_model
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextRange {
    pub start: usize,
    pub end: usize,
}

impl TextRange {
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self;
    #[must_use]
    pub const fn caret(offset: usize) -> Self;
    #[must_use]
    pub const fn len(&self) -> usize;
    #[must_use]
    pub const fn is_empty(&self) -> bool;
    #[must_use]
    pub const fn is_caret(&self) -> bool;
    #[must_use]
    pub const fn clamped_to(&self, value_len: usize) -> Self;
    #[must_use]
    pub fn text_of<'a>(&self, value: &'a str) -> Option<&'a str>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextInputModel {
    pub value: String,
    pub placeholder: String,
    pub selection: Option<TextRange>,
    pub validation: Option<String>,
    pub read_only: bool,
    pub disabled: bool,
    pub clearable: bool,
    pub disallowed: Option<DisallowedClass>,
}

impl TextInputModel {
    pub fn new(value: impl Into<String>) -> Self;
    #[must_use]
    pub fn empty() -> Self;
    pub fn with_placeholder(self, placeholder: impl Into<String>) -> Self;
    pub fn with_validation(self, message: impl Into<String>) -> Self;
    #[must_use]
    pub fn with_selection(self, selection: TextRange) -> Self;
    #[must_use]
    pub fn read_only(self) -> Self;
    #[must_use]
    pub fn disabled(self) -> Self;
    #[must_use]
    pub fn with_clearable(self, clearable: bool) -> Self;
    #[must_use]
    pub fn with_stance(self, stance: EditStance) -> Self;
    #[must_use]
    pub fn with_disallowed(self, disallowed: DisallowedClass) -> Self;
    #[must_use]
    pub fn controlled(&self) -> Controlled<'_, TextInputModel>;
    pub fn note_commit(&mut self);
    pub fn note_blur(&mut self);
    pub fn note_submit(&mut self);
    pub fn reset_stance(&mut self);
    #[must_use]
    pub fn punishment(&self) -> Punishment;
}

// widget::text_input_controller
#[derive(Clone)]
pub enum TextInputEmission {
    Changed(Live<String>),
    Committed(Commit<String>),
    FocusCommitted(Commit<String>),
    Cleared(Commit<String>),
    Cancelled(Cancel<String>),
    FocusChanged(bool),
}

impl TextInputEmission {
    #[must_use]
    pub fn value(&self) -> Option<&str>;
    #[must_use]
    pub const fn tier(&self) -> &'static str;
    #[must_use]
    pub const fn settles(&self) -> bool;
    #[must_use]
    pub const fn advances_stance(&self) -> bool;
}

#[derive(Debug, Default)]
pub struct TextInputController { /* private fields */ }

impl TextInputController {
    #[must_use]
    pub fn new() -> Self;
    #[must_use]
    pub fn with_value(value: impl Into<String>) -> Self;
    pub fn observe(&mut self, model: &TextInputModel);
    pub fn sync_value(&mut self, value: &str);
    pub fn note_edit(&mut self, value: String) -> bool;
    pub fn take_live(&mut self) -> Option<String>;
    pub fn commit(&mut self) -> Option<TextInputEmission>;
    pub fn revert(&mut self) -> Option<TextInputEmission>;
    pub fn clear(&mut self) -> Option<TextInputEmission>;
    pub fn note_focus(&mut self, focused: bool) -> Vec<TextInputEmission>;
    pub fn reset(&mut self);
}
```

#### Examples
```rust
use hoarfrost::focus::FocusId;
use hoarfrost::widget::text_input::{InputKind, TextInput};
use hoarfrost::widget::text_input_controller::TextInputEmission;
use hoarfrost::widget::text_input_model::TextInputModel;

#[derive(Clone, Debug)]
enum Msg {
    HostEdited(String),
    HostSettled(String),
    HostReverted(String),
}

let model = TextInputModel::new("localhost")
    .with_placeholder("Enter hostname");

let _input: iced::Element<'_, Msg> = TextInput::controlled(
    InputKind::TextEntry,
    model.controlled(),
    FocusId::new("hostname-input"),
    |emission| match emission {
        TextInputEmission::Changed(live) => Some(Msg::HostEdited(live.into_value())),
        TextInputEmission::Committed(commit)
        | TextInputEmission::FocusCommitted(commit)
        | TextInputEmission::Cleared(commit) => Some(Msg::HostSettled(commit.into_value())),
        TextInputEmission::Cancelled(cancel) => Some(Msg::HostReverted(cancel.into_restored())),
        TextInputEmission::FocusChanged(_) => None,
    },
)
.into();
```

#### Type references
- `InputKind` - Selects trailing-slot reservation and glyph behavior (`TextEntry`, `Search`, `DropdownTrigger`, `Password`, `TextArea`).
- `SlotContent` - Describes whether the trailing slot is `Empty`, `Passive(char)`, `Actionable(char)`, or `Toggle(char)`.
- `InputStyle` - Resolved colors and stroke widths for a `(InputKind, VisualState)` pair.
- `TextRange` - Byte-offset span (`start..end`) representing a caret or highlight inside a value.
- `TextInputModel` - Host-owned state for a single-line text field including `value`, `placeholder`, `selection`, `validation`, and `EditStance`.
- `TextInputEmission` - Tiered output sum type (`Changed`, `Committed`, `FocusCommitted`, `Cleared`, `Cancelled`, `FocusChanged`).
- `TextInputController` - Editing session engine capturing the pre-first-edit snapshot and coalescing `Live<String>` updates.
- `TextInput` - Live `iced` widget wrapping `iced::widget::text_input::TextInput` with Hoarfrost's five-state chrome and fixed trailing slot.

#### Return values
- `resolve_input_style` returns an `InputStyle` where `Invalid + focused` keeps the `2px` danger border and moves focus to an outer ring.
- `TextInputController::note_focus` returns `Vec<TextInputEmission>`, emitting `[FocusCommitted(v), FocusChanged(false)]` on blur when an editing session is open.

#### Side effects
- `TextInputModel::note_commit`, `note_blur`, and `note_submit` transition the model's internal `EditStance` to `EditStance::Touched`.
- `TextInput::update` intercepts keystrokes in controlled mode and publishes only the messages returned by `emit`.

#### References
- Requirements: `UIF-ATOM-002`, `UIF-ATOM-002a`, `UIF-PRIN-003`, `UIF-PRIN-009`
- Source: [`text_input.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/text_input.rs), [`text_input_model.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/text_input_model.rs), [`text_input_controller.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/text_input_controller.rs)

---

### `widget::text_area_model`, `widget::text_area_host` - Multi-Line Text Area (`UIF-ATOM-002e`)

#### Overview
The `widget::text_area_model` and `widget::text_area_host` modules provide a **headless model/controller + style resolver** (`TextAreaModel`, `TextAreaController`, `Growth`, `EditHistory`, and `TEXT_AREA_KIND`) for multi-line text wells (`UIF-ATOM-002e`, spec §3.2). They compute row-clamped auto-grow heights (`clamp(line_count, min_rows, max_rows) * 20px + 16px`), caret-following scroll offsets, bounded undo/redo snapshots, and distinct `Submitted` (`Ctrl+Enter`) versus `Committed` (blur) emissions.

#### Prerequisites & Requirements
- The application **MUST** call `TextAreaController::sync(&model)` once per frame before dispatching editing events so the underlying `TextInputController` tracks host changes when no session is open.
- The application **MUST** use `TextAreaController::submit(&model)` for `Ctrl+Enter` / `Cmd+Enter` and `TextAreaController::note_focus(&mut model, false)` (or `commit()`) for focus loss so explicit submission does not prematurely close the open `Escape` session.

#### Syntax / Method Signature
```rust
// widget::text_area_model
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct LineCol {
    pub line: u32,
    pub col: u32,
}

impl LineCol {
    #[must_use]
    pub const fn new(line: u32, col: u32) -> Self;
    #[must_use]
    pub const fn origin() -> Self;
    #[must_use]
    pub const fn is_origin(&self) -> bool;
    #[must_use]
    pub fn offset_in(self, text: &str) -> usize;
    #[must_use]
    pub fn from_offset(text: &str, offset: usize) -> Self;
    #[must_use]
    pub fn clamped_to(self, text: &str) -> Self;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ParseDiagnostic {
    pub at: LineCol,
    pub message: String,
}

impl ParseDiagnostic {
    #[must_use]
    pub fn new(at: LineCol, message: impl Into<String>) -> Self;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Growth {
    pub min_rows: u32,
    pub max_rows: Option<u32>,
}

impl Growth {
    #[must_use]
    pub const fn new(min_rows: u32, max_rows: Option<u32>) -> Self;
    #[must_use]
    pub const fn line_height() -> f32;
    #[must_use]
    pub const fn text_height(rows: u32) -> f32;
    #[must_use]
    pub const fn height_at(rows: u32) -> f32;
    #[must_use]
    pub fn min_height(&self) -> f32;
    #[must_use]
    pub fn max_height(&self) -> f32;
    #[must_use]
    pub fn height_for_lines(&self, lines: u32) -> f32;
    #[must_use]
    pub fn can_grow(&self, lines: u32) -> bool;
    #[must_use]
    pub fn rows_fitting(height: f32) -> u32;
    #[must_use]
    pub fn clamp_height(&self, height: f32) -> f32;
}

#[derive(Clone, Debug)]
pub struct EditHistory { /* private fields */ }

impl EditHistory {
    pub const DEFAULT_LIMIT: usize = 128;
    #[must_use]
    pub fn new() -> Self;
    #[must_use]
    pub fn with_limit(limit: usize) -> Self;
    pub fn record(&mut self, before: &str);
    #[must_use]
    pub fn can_undo(&self) -> bool;
    #[must_use]
    pub fn can_redo(&self) -> bool;
    #[must_use]
    pub fn undo(&mut self, current: &str) -> Option<String>;
    #[must_use]
    pub fn redo(&mut self, current: &str) -> Option<String>;
    #[must_use]
    pub fn undo_depth(&self) -> usize;
    #[must_use]
    pub fn redo_depth(&self) -> usize;
    pub fn clear(&mut self);
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextAreaModel { /* private fields */ }

impl TextAreaModel {
    #[must_use]
    pub fn new() -> Self;
    #[must_use]
    pub fn with_text(self, text: impl Into<String>) -> Self;
    #[must_use]
    pub fn with_growth(self, growth: Growth) -> Self;
    #[must_use]
    pub fn with_disabled(self, disabled: bool) -> Self;
    #[must_use]
    pub fn with_read_only(self, read_only: bool) -> Self;
    #[must_use]
    pub fn with_stance(self, stance: EditStance) -> Self;
    #[must_use]
    pub fn with_font_family(self, font_family: typography::FontFamily) -> Self;
    #[must_use]
    pub fn mono(self) -> Self;
    #[must_use]
    pub fn text(&self) -> &str;
    #[must_use]
    pub fn cursor(&self) -> LineCol;
    #[must_use]
    pub fn selection(&self) -> Option<TextRange>;
    #[must_use]
    pub fn growth(&self) -> Growth;
    #[must_use]
    pub fn is_dirty(&self) -> bool;
    #[must_use]
    pub fn validation(&self) -> Option<&ParseDiagnostic>;
    #[must_use]
    pub fn is_invalid(&self) -> bool;
    #[must_use]
    pub fn line_count(&self) -> u32;
    #[must_use]
    pub fn content_height(&self) -> f32;
    #[must_use]
    pub fn height(&self) -> f32;
    #[must_use]
    pub fn visible_text_height(&self) -> f32;
    #[must_use]
    pub fn max_scroll(&self) -> f32;
    #[must_use]
    pub fn is_scrollable(&self) -> bool;
    #[must_use]
    pub fn scroll_offset(&self) -> f32;
    pub fn set_scroll(&mut self, offset: f32) -> f32;
    pub fn scroll_to_show(&mut self, at: LineCol) -> f32;
    pub fn set_text(&mut self, text: impl Into<String>);
    pub fn set_cursor(&mut self, at: LineCol);
    pub fn set_selection(&mut self, selection: Option<TextRange>);
    pub fn set_validation(&mut self, validation: Option<ParseDiagnostic>);
    pub fn mark_clean(&mut self);
}

#[derive(Clone)]
pub enum TextAreaEmission {
    Changed(Live<String>),
    Committed(Commit<String>),
    Submitted(Commit<String>),
    Cancelled(Cancel<String>),
    CursorChanged(LineCol),
    Undone(String),
    Redone(String),
    FocusChanged(bool),
}

// widget::text_area_host
pub const TEXT_AREA_KIND: InputKind = InputKind::TextArea;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextAreaOutcome {
    pub emissions: Vec<TextAreaEmission>,
}

#[derive(Debug)]
pub struct TextAreaController { /* private fields */ }

impl TextAreaController {
    #[must_use]
    pub fn new() -> Self;
    #[must_use]
    pub fn with_value(text: impl Into<String>) -> Self;
    pub fn sync(&mut self, model: &TextAreaModel);
    pub fn note_edit(&mut self, model: &mut TextAreaModel, text: String) -> TextAreaOutcome;
    #[must_use]
    pub fn submit(&mut self, model: &TextAreaModel) -> TextAreaOutcome;
    pub fn commit(&mut self) -> TextAreaOutcome;
    pub fn cancel(&mut self, model: &mut TextAreaModel) -> TextAreaOutcome;
    pub fn undo(&mut self, model: &mut TextAreaModel) -> TextAreaOutcome;
    pub fn redo(&mut self, model: &mut TextAreaModel) -> TextAreaOutcome;
    pub fn note_cursor(&mut self, model: &mut TextAreaModel, at: LineCol) -> TextAreaOutcome;
    pub fn note_focus(&mut self, model: &mut TextAreaModel, focused: bool) -> TextAreaOutcome;
    pub fn reset(&mut self);
}
```

#### Examples
```rust
use hoarfrost::widget::text_area_host::TextAreaController;
use hoarfrost::widget::text_area_model::{Growth, TextAreaEmission, TextAreaModel};

let mut model = TextAreaModel::new()
    .with_growth(Growth::new(3, Some(6)))
    .mono();
let mut ctrl = TextAreaController::new();
ctrl.sync(&model);

let outcome = ctrl.note_edit(&mut model, "SELECT *\nFROM events;".to_owned());
for e in outcome.emissions {
    if let TextAreaEmission::Changed(live) = e {
        assert_eq!(live.value(), "SELECT *\nFROM events;");
    }
}

let submit_outcome = ctrl.submit(&model);
for e in submit_outcome.emissions {
    if let TextAreaEmission::Submitted(commit) = e {
        assert_eq!(commit.value(), "SELECT *\nFROM events;");
    }
}
```

#### Type references
- `LineCol` - Zero-based `(line, col)` coordinate in character columns with clamped UTF-8 byte-offset conversions.
- `ParseDiagnostic` - Validation error payload pairing a `LineCol` coordinate with an error message.
- `Growth` - Row-count floor and optional ceiling (`min_rows`, `max_rows`) computing outer well heights from `typography::BODY.line_height` and `space::SM`.
- `EditHistory` - Bounded undo/redo ledger of pre-edit whole-buffer snapshots (`DEFAULT_LIMIT = 128`).
- `TextAreaModel` - Declarative state of a multi-line buffer, cursor, selection, growth band, scroll offset, and validation diagnostic.
- `TextAreaEmission` - Sum type covering `Changed(Live<String>)`, `Committed(Commit<String>)`, `Submitted(Commit<String>)`, `Cancelled(Cancel<String>)`, `CursorChanged(LineCol)`, `Undone(String)`, `Redone(String)`, and `FocusChanged(bool)`.
- `TextAreaOutcome` - Wrapper holding `pub emissions: Vec<TextAreaEmission>` returned by `TextAreaController` operations.
- `TextAreaController` - Host-side controller composing `TextInputController` and `EditHistory` for multi-line editing.

#### Return values
- `Growth::height_for_lines` returns the outer well height in logical pixels (`76.0` for a 3-row minimum well).
- `TextAreaController` methods return a `TextAreaOutcome` whose `emissions` field contains the ordered `TextAreaEmission` values produced by the step.

#### Side effects
- `TextAreaController::note_edit`, `cancel`, `undo`, `redo`, and `note_cursor` mutate the passed `&mut TextAreaModel` in place to keep buffer text, dirty state, and scroll offset synchronized with the controller.

#### References
- Requirements: `UIF-ATOM-002e`, `UIF-PRIN-001`, `UIF-PRIN-009`
- Source: [`text_area_model.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/text_area_model.rs), [`text_area_host.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/text_area_host.rs)

---

### `widget::password_model`, `widget::password_host` - Password Input (`UIF-ATOM-002c`)

#### Overview
The `widget::password_model` and `widget::password_host` modules provide a **headless model/controller + style resolver** (`SecretInput`, `PasswordInputModel`, `PasswordController`, `PasswordEmission`, and `mirror`) for secret entry (`UIF-ATOM-002c`, spec §3.2) that projects into the live `widget::text_input::TextInput` widget. `SecretInput` redacts `Debug` and `Display` output to `•` (`U+2022`) grapheme masks so tiered emissions **MUST NOT** leak plaintext secrets into logs or panic traces.

#### Prerequisites & Requirements
- The application **MUST NOT** log or format `SecretInput::expose()` in diagnostic output; callers **MUST** use `SecretInput::expose()` or `SecretInput::into_exposed()` only at the authentication boundary.
- The application **MUST** route focus edges through `PasswordInputModel::note_focus(focused)` so losing focus automatically resets `revealed` to `false` (enforcing `revealed => focused`).
- To render a live password widget, the application **MUST** project `PasswordInputModel` via `password_host::mirror(&model)` into `TextInput::controlled(PASSWORD_KIND, ...)` and wire `.masked(!model.is_revealed())` and `.on_reveal(...)`.

#### Syntax / Method Signature
```rust
// widget::password_model
pub const SECRET_MASK: char = '\u{2022}';

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SecretInput(String);

impl SecretInput {
    #[must_use]
    pub fn new(plaintext: impl Into<String>) -> Self;
    #[must_use]
    pub fn empty() -> Self;
    #[must_use]
    pub fn expose(&self) -> &str;
    #[must_use]
    pub fn into_exposed(self) -> String;
    #[must_use]
    pub fn len(&self) -> usize;
    #[must_use]
    pub fn is_empty(&self) -> bool;
    #[must_use]
    pub fn masked(&self) -> String;
    #[must_use]
    pub fn masked_len(&self) -> usize;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PasswordInputModel { /* private fields */ }

impl PasswordInputModel {
    #[must_use]
    pub fn new() -> Self;
    #[must_use]
    pub fn with_placeholder(self, placeholder: impl Into<String>) -> Self;
    #[must_use]
    pub fn with_value(self, value: SecretInput) -> Self;
    #[must_use]
    pub fn with_disabled(self, disabled: bool) -> Self;
    #[must_use]
    pub fn with_read_only(self, read_only: bool) -> Self;
    #[must_use]
    pub fn with_stance(self, stance: EditStance) -> Self;
    #[must_use]
    pub fn value(&self) -> &SecretInput;
    #[must_use]
    pub fn masked_len(&self) -> usize;
    #[must_use]
    pub fn revealed_value(&self) -> Option<&str>;
    #[must_use]
    pub fn validation(&self) -> Option<&str>;
    #[must_use]
    pub fn placeholder(&self) -> &str;
    #[must_use]
    pub fn is_invalid(&self) -> bool;
    #[must_use]
    pub fn is_revealed(&self) -> bool;
    #[must_use]
    pub fn is_focused(&self) -> bool;
    #[must_use]
    pub fn is_disabled(&self) -> bool;
    #[must_use]
    pub fn is_read_only(&self) -> bool;
    #[must_use]
    pub fn slot_for(&self) -> SlotContent;
    #[must_use]
    pub fn can_reveal(&self) -> bool;
    #[must_use]
    pub fn toggle_revealed(&mut self) -> bool;
    #[must_use]
    pub fn note_focus(&mut self, focused: bool) -> bool;
    pub fn set_value(&mut self, value: SecretInput);
    pub fn set_validation(&mut self, validation: Option<String>);
}

#[derive(Clone)]
pub enum PasswordEmission {
    Changed(Live<SecretInput>),
    Committed(Commit<SecretInput>),
    Cancelled(Cancel<SecretInput>),
    Revealed(bool),
    FocusChanged(bool),
}

impl PasswordEmission {
    #[must_use]
    pub const fn tier(&self) -> &'static str;
    #[must_use]
    pub const fn settles(&self) -> bool;
    #[must_use]
    pub const fn carries_secret(&self) -> bool;
}

// widget::password_host
pub const PASSWORD_KIND: InputKind = InputKind::Password;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PasswordOutcome {
    pub emissions: Vec<PasswordEmission>,
}

#[derive(Clone, Debug, Default)]
pub struct PasswordController { /* private fields */ }

impl PasswordController {
    #[must_use]
    pub fn new() -> Self;
    pub fn observe(&mut self, model: &PasswordInputModel) -> bool;
    pub fn toggle_reveal(&mut self, model: &mut PasswordInputModel) -> PasswordOutcome;
    #[must_use]
    pub fn translate(emission: TextInputEmission) -> Option<PasswordEmission>;
    #[must_use]
    pub const fn reveals(&self) -> usize;
    #[must_use]
    pub const fn refusals(&self) -> usize;
    #[must_use]
    pub const fn masked_frames(&self) -> usize;
    #[must_use]
    pub const fn exposed_frames(&self) -> usize;
    pub fn reset(&mut self);
}

#[must_use]
pub fn mirror(model: &PasswordInputModel) -> TextInputModel;
```

#### Examples
```rust
use hoarfrost::focus::FocusId;
use hoarfrost::widget::password_host::{self, PASSWORD_KIND, PasswordController};
use hoarfrost::widget::password_model::{PasswordEmission, PasswordInputModel, SecretInput};
use hoarfrost::widget::text_input::TextInput;

#[derive(Clone, Debug)]
enum AuthMsg {
    PasswordEvent(PasswordEmission),
    ToggleRevealClicked,
}

let mut pw_model = PasswordInputModel::new()
    .with_placeholder("Master password")
    .with_value(SecretInput::new("hunter2"));
let mut pw_ctrl = PasswordController::new();

// Focus is required before reveal is permitted (`revealed => focused`)
let _ = pw_model.note_focus(true);
let outcome = pw_ctrl.toggle_reveal(&mut pw_model);
for e in outcome.emissions {
    assert_eq!(e, PasswordEmission::Revealed(true));
}

// Project `PasswordInputModel` into `TextInput` via `password_host::mirror`
let mirrored = password_host::mirror(&pw_model);
let _widget: iced::Element<'_, AuthMsg> = TextInput::controlled(
    PASSWORD_KIND,
    mirrored.controlled(),
    FocusId::new("master-password"),
    |text_emission| PasswordController::translate(text_emission).map(AuthMsg::PasswordEvent),
)
.masked(!pw_model.is_revealed())
.on_reveal(AuthMsg::ToggleRevealClicked)
.into();
```

#### Type references
- `SecretInput` - Redacted string wrapper whose `Debug` and `Display` implementations output `•` (`SECRET_MASK`) per grapheme instead of plaintext.
- `PasswordInputModel` - Host-owned model enforcing the `revealed => focused` security invariant and non-clearable password slot rules.
- `PasswordEmission` - Tiered output sum type (`Changed(Live<SecretInput>)`, `Committed(Commit<SecretInput>)`, `Cancelled(Cancel<SecretInput>)`, `Revealed(bool)`, `FocusChanged(bool)`).
- `PasswordOutcome` - Outcome wrapper carrying `pub emissions: Vec<PasswordEmission>`.
- `PasswordController` - Host-side reveal policy auditor and `TextInputEmission -> Option<PasswordEmission>` translator.

#### Return values
- `PasswordController::translate` converts a `TextInputEmission` into `Some(PasswordEmission)` with `SecretInput` payloads, or `None` if `TextInputEmission::Cleared` is encountered.
- `password_host::mirror` returns a `TextInputModel` configured with `clearable: false` and the current validation/stance state.

#### Side effects
- `PasswordController::toggle_reveal` mutates `PasswordInputModel`'s `revealed` flag only when `model.can_reveal()` is `true`; otherwise it increments `refusals` and emits nothing.

#### References
- Requirements: `UIF-ATOM-002c`, `UIF-PRIN-003`, `UIF-PRIN-009`
- Source: [`password_model.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/password_model.rs), [`password_host.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/password_host.rs)

---

### `widget::number_box`, `widget::number_box_host` - Number Box (`UIF-ATOM-005`)

#### Overview
The `widget::number_box` and `widget::number_box_host` modules provide a **headless model/controller + style resolver** (`NumberBoxHost`, `NumberBoxStyle`, `StepperShape`, `StepDirection`, and `resolve_style`) for numeric text entries with trailing dual steppers (`UIF-ATOM-005`, spec §3.5). They pair with `widget::range_model::RangeModel` to support stacked (`28x16px` `▲`/`▼`) or side-by-side (`24x24px` `[−]`/`[+]`) stepper geometries, deterministic caller-clocked press-and-hold repeats, and unit-suffix-aware text parsing.

#### Prerequisites & Requirements
- The application **MUST** supply a `RangeModel` configured with a positive `step` (via `RangeModel::with_step`) for stepper clicks, holds, and arrow/page keys to move the value.
- Because Hoarfrost **MUST NOT** read wall-clock time internally, the application **MUST** pass both `elapsed_ms` and a non-zero `repeat_interval_ms` to `NumberBoxHost::hold_stepper` during press-and-hold gestures.

#### Syntax / Method Signature
```rust
// widget::number_box
pub const ENTRY_HEIGHT: f32 = size::CONTROL_MD;
pub const STEPPER_WIDTH: f32 = size::CONTROL_SM;
pub const STEP_BUTTON_SIZE: f32 = size::HIT_TARGET_MIN;
pub const GLYPH_SIZE: f32 = size::ICON_MD;
pub const RADIUS: f32 = shape::RADIUS_WIDGET;
pub const BORDER_WIDTH: f32 = shape::BORDER_HAIRLINE;
pub const GLYPH_UP: char = '▲';
pub const GLYPH_DOWN: char = '▼';
pub const GLYPH_MINUS: char = '−';
pub const GLYPH_PLUS: char = '+';

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StepperShape {
    Stacked,
    SideBySide,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StepDirection {
    Decrement,
    Increment,
}

impl StepDirection {
    #[must_use]
    pub const fn delta(self) -> i64;
    #[must_use]
    pub const fn glyph(self, shape: StepperShape) -> char;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumberBoxStyle {
    pub entry: InputStyle,
    pub stepper_background: Color,
    pub stepper_border: Color,
    pub stepper_glyph: Color,
}

#[must_use]
pub fn resolve_style(state: VisualState, palette: &Palette) -> NumberBoxStyle;
#[must_use]
pub fn control_rect(origin: Point, width: f32) -> Rectangle;
#[must_use]
pub const fn stepper_width(shape: StepperShape) -> f32;
#[must_use]
pub fn stepper_rect_for(shape: StepperShape, bounds: Rectangle) -> Rectangle;
#[must_use]
pub fn stepper_rect(bounds: Rectangle) -> Rectangle;
#[must_use]
pub fn entry_rect_for(shape: StepperShape, bounds: Rectangle) -> Rectangle;
#[must_use]
pub fn entry_rect(bounds: Rectangle) -> Rectangle;
#[must_use]
pub fn step_button_rects(bounds: Rectangle, shape: StepperShape) -> [Rectangle; 2];
#[must_use]
pub fn step_at(bounds: Rectangle, shape: StepperShape, point: Point) -> Option<StepDirection>;
#[must_use]
pub fn entry_layer_for(shape: StepperShape, bounds: Rectangle, style: &NumberBoxStyle) -> Option<Layer>;
#[must_use]
pub fn entry_focus_ring_layer_for(shape: StepperShape, bounds: Rectangle, style: &NumberBoxStyle) -> Option<Layer>;
#[must_use]
pub fn stepper_layer_for(shape: StepperShape, bounds: Rectangle, style: &NumberBoxStyle) -> Layer;
#[must_use]
pub fn step_button_layers(bounds: Rectangle, style: &NumberBoxStyle, shape: StepperShape) -> [Layer; 2];
#[must_use]
pub fn stepper_border_chrome_class(border: Color, palette: &Palette) -> ChromeClass;

// widget::number_box_host
#[derive(Clone)]
pub enum NumberBoxEmission {
    Changed(Live<f64>),
    Committed(Commit<f64>),
    Cancelled(Cancel<f64>),
}

impl NumberBoxEmission {
    #[must_use]
    pub fn is_value_change(&self) -> bool;
    #[must_use]
    pub const fn tier(&self) -> &'static str;
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NumberBoxOutcome {
    pub emissions: Vec<NumberBoxEmission>,
}

#[derive(Debug, Clone, Default)]
pub struct NumberBoxHost { /* private fields */ }

impl NumberBoxHost {
    #[must_use]
    pub fn new() -> Self;
    #[must_use]
    pub fn is_open(&self) -> bool;
    #[must_use]
    pub const fn held_direction(&self) -> Option<StepDirection>;
    #[must_use]
    pub const fn current(&self) -> Option<f64>;
    #[must_use]
    pub const fn steps(&self) -> i64;
    #[must_use]
    pub fn snapshot(&self) -> Option<f64>;
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
```

#### Examples
```rust
use hoarfrost::widget::number_box::StepDirection;
use hoarfrost::widget::number_box_host::{NumberBoxEmission, NumberBoxHost};
use hoarfrost::widget::range_model::RangeModel;

let model = RangeModel::new(0.0, 100.0, 10.0)
    .with_step(5.0)
    .with_unit_suffix("px");
let mut host = NumberBoxHost::new();

// Initial press steps immediately and opens the gesture at 10.0
let press_out = host.press_stepper(&model, StepDirection::Increment);
for e in press_out.emissions {
    if let NumberBoxEmission::Changed(live) = e {
        assert_eq!(*live.value(), 15.0);
    }
}

// Holding for 250ms at a 100ms repeat interval adds 2 more steps (to 25.0)
let hold_out = host.hold_stepper(&model, StepDirection::Increment, 250, 100);
for e in hold_out.emissions {
    if let NumberBoxEmission::Changed(live) = e {
        assert_eq!(*live.value(), 25.0);
    }
}

// Releasing settles the gesture with a single Commit
let release_out = host.release_stepper(&model);
for e in release_out.emissions {
    if let NumberBoxEmission::Committed(commit) = e {
        assert_eq!(commit.into_value(), 25.0);
    }
}
```

#### Type references
- `StepperShape` - Selects between `Stacked` (`28x32px` zone with two `28x16px` halves) and `SideBySide` (`48x32px` zone with two `24x24px` buttons).
- `StepDirection` - Direction of a step (`Decrement` or `Increment`).
- `NumberBoxStyle` - Resolved `InputStyle` for the numeric entry plus background, border, and glyph colors for the stepper block.
- `NumberBoxEmission` - Tiered output sum type (`Changed(Live<f64>)`, `Committed(Commit<f64>)`, `Cancelled(Cancel<f64>)`).
- `NumberBoxOutcome` - Wrapper holding `pub emissions: Vec<NumberBoxEmission>`.
- `NumberBoxHost` - Ephemeral state machine managing stepper holds, repeat cadence math, keyboard steps, and text-entry commits.

#### Return values
- `step_at` returns `Some(StepDirection)` when a pointer coordinate falls inside either stepper button, or `None` when inside the text entry or outside the control.
- `NumberBoxHost::commit_text` returns a `Committed(Commit<f64>)` emission when `model.parse(text)` succeeds, or `NumberBoxOutcome::noop()` when the text is unparseable.

#### Side effects
- `NumberBoxHost` methods mutate only the host's internal gesture slot, step counter, and held-button flags; `RangeModel` is read-only and updated by the application upon receiving `NumberBoxEmission`.

#### References
- Requirements: `UIF-ATOM-005`, `UIF-TOK-006`
- Source: [`number_box.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/number_box.rs), [`number_box_host.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/number_box_host.rs), [`range_model.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/range_model.rs)

---

### `widget::toggle`, `widget::toggle_model`, `widget::toggle_widget`, `widget::rove` - Checkbox, RadioGroup & ToggleSwitch (`UIF-ATOM-003`)

#### Overview
The `widget::toggle_widget` module provides three **live `iced` widgets (`Element<'a, Message>`)** (`Checkbox`, `RadioGroup`, and `ToggleSwitch`), while `widget::toggle`, `widget::toggle_model`, and `widget::rove` provide the **headless model/controller + style resolver** (`CheckboxModel`, `RadioGroupModel`, `SwitchModel`, `ToggleStyle`, `resolve_style`, and `rove_index`) implementing `UIF-ATOM-003` (spec §3.3). All three controls share a single polarity-driven paint matrix (`TogglePolarity::Off` vs `TogglePolarity::On`), deterministic tri-state checkbox transitions, and a wrapping roving-focus walk over enabled options.

#### Prerequisites & Requirements
- The application **MUST** supply a `FocusId` when constructing `Checkbox`, `RadioGroup`, or `ToggleSwitch`.
- For parent tri-state checkboxes, the application **MUST** pass a `ChildSummary` via `Checkbox::with_children` (or `CheckboxModel::toggle(children)`) so clicking an `Indeterminate` parent transitions deterministically to `Unchecked` when all enabled children are already checked, and to `Checked` otherwise.
- When calling `widget::rove::rove_index` directly, the caller **MUST** pass `enabled` indices in strictly ascending display order.

#### Syntax / Method Signature
```rust
// widget::rove
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RoveDirection {
    Forward,
    Backward,
}

#[must_use]
pub fn rove_index(
    enabled: &[usize],
    from: Option<usize>,
    direction: RoveDirection,
) -> Option<usize>;

// widget::toggle
pub const BORDER_WIDTH: f32 = shape::BORDER_HAIRLINE;
pub const FOCUS_RING_WIDTH: f32 = shape::BORDER_FOCUS;
pub const LABEL_GAP: f32 = space::SM;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToggleControl {
    Checkbox,
    Radio,
    Switch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TogglePolarity {
    Off,
    On,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToggleStyle {
    pub indicator_fill: Color,
    pub indicator_border: Color,
    pub indicator_border_width: f32,
    pub mark: Color,
    pub label: Color,
    pub focus_ring_color: Color,
    pub focus_ring_width: f32,
}

#[must_use]
pub fn resolve_style(polarity: TogglePolarity, state: VisualState, palette: &Palette) -> ToggleStyle;
#[must_use]
pub fn indicator_chrome_class(border: Color, fill: Color, palette: &Palette) -> ChromeClass;
#[must_use]
pub const fn indicator_size(control: ToggleControl) -> (f32, f32);
#[must_use]
pub const fn indicator_radius(control: ToggleControl) -> f32;
#[must_use]
pub fn hit_size(control: ToggleControl, label_width: f32) -> (f32, f32);
#[must_use]
pub fn label_box(control: ToggleControl, row: Rectangle) -> Rectangle;
#[must_use]
pub fn indicator_box(control: ToggleControl, row: Rectangle) -> Rectangle;
#[must_use]
pub fn indicator_layer(indicator: Rectangle, control: ToggleControl, style: &ToggleStyle) -> Layer;
#[must_use]
pub fn mark_layer(
    indicator: Rectangle,
    control: ToggleControl,
    polarity: TogglePolarity,
    style: &ToggleStyle,
) -> Option<Layer>;
#[must_use]
pub fn focus_ring_layer(
    indicator: Rectangle,
    control: ToggleControl,
    style: &ToggleStyle,
) -> Option<Layer>;

// widget::toggle_model
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ChildSummary {
    pub enabled: usize,
    pub checked: usize,
}

impl ChildSummary {
    pub const LEAF: Self = Self { enabled: 0, checked: 0 };
    #[must_use]
    pub const fn all_checked(self) -> bool;
    #[must_use]
    pub const fn none_checked(self) -> bool;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CheckState {
    #[default]
    Unchecked,
    Indeterminate,
    Checked,
}

impl CheckState {
    #[must_use]
    pub const fn polarity(self) -> TogglePolarity;
    #[must_use]
    pub const fn next(self) -> Self;
    #[must_use]
    pub const fn next_with(self, children: ChildSummary) -> Self;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckboxModel {
    pub state: CheckState,
    pub label: String,
    pub description: Option<String>,
    pub disabled: bool,
}

impl CheckboxModel {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self;
    #[must_use]
    pub fn with_state(self, state: CheckState) -> Self;
    #[must_use]
    pub fn with_description(self, description: impl Into<String>) -> Self;
    #[must_use]
    pub fn disabled(self) -> Self;
    #[must_use]
    pub fn toggle(&self, children: ChildSummary) -> Option<Commit<CheckState>>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RadioOption<Id> {
    pub id: Id,
    pub label: String,
    pub disabled: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RadioOrientation {
    #[default]
    Vertical,
    HorizontalWrap,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RadioGroupModel<Id> {
    pub options: Vec<RadioOption<Id>>,
    pub selected: Option<Id>,
    pub orientation: RadioOrientation,
    pub disabled: bool,
}

impl<Id> RadioGroupModel<Id> {
    #[must_use]
    pub fn new(options: Vec<RadioOption<Id>>) -> Self;
    #[must_use]
    pub fn with_selected(self, id: Id) -> Self;
    #[must_use]
    pub fn with_orientation(self, orientation: RadioOrientation) -> Self;
    #[must_use]
    pub fn disabled(self) -> Self;
    #[must_use]
    pub fn rove(&self, from: Option<usize>, direction: RoveDirection) -> Option<usize>;
    #[must_use]
    pub fn rove_and_select(&self, from: Option<usize>, key: ToggleKey) -> Option<Roving<Id>>
    where
        Id: Clone;
    #[must_use]
    pub fn select(&self, id: &Id) -> Option<Commit<Id>>
    where
        Id: PartialEq + Clone;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwitchModel {
    pub on: bool,
    pub label: String,
    pub on_text: Option<String>,
    pub off_text: Option<String>,
    pub busy: bool,
    pub disabled: bool,
}

impl SwitchModel {
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self;
    #[must_use]
    pub fn with_on(self, on: bool) -> Self;
    #[must_use]
    pub fn with_texts(self, on_text: impl Into<String>, off_text: impl Into<String>) -> Self;
    #[must_use]
    pub fn busy(self) -> Self;
    #[must_use]
    pub fn disabled(self) -> Self;
    #[must_use]
    pub const fn polarity(&self) -> TogglePolarity;
    #[must_use]
    pub fn toggle(&self) -> Option<Commit<bool>>;
    #[must_use]
    pub fn pending(&self) -> ActionPending;
}

// widget::toggle_widget
pub struct Checkbox<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> { /* private fields */ }
impl<'a, Message, Theme, Renderer> Checkbox<'a, Message, Theme, Renderer> {
    #[must_use]
    pub fn controlled(
        model: Controlled<'a, CheckboxModel>,
        focus_id: impl Into<FocusId>,
        emit: impl Fn(Commit<CheckState>) -> Option<Message> + 'a,
    ) -> Self;
    #[must_use]
    pub fn standalone(
        label: impl Into<String>,
        state: CheckState,
        focus_id: impl Into<FocusId>,
        on_toggle: impl Fn(CheckState) -> Option<Message> + 'a,
    ) -> Self;
    #[must_use]
    pub fn with_children(self, children: ChildSummary) -> Self;
    #[must_use]
    pub fn with_palette(self, palette: Palette) -> Self;
    #[must_use]
    pub fn disabled(self, disabled: bool) -> Self;
}

pub struct RadioGroup<'a, Message, Id = String, Theme = iced::Theme, Renderer = iced::Renderer> { /* private fields */ }
impl<'a, Message, Id, Theme, Renderer> RadioGroup<'a, Message, Id, Theme, Renderer> {
    #[must_use]
    pub fn controlled(
        model: Controlled<'a, RadioGroupModel<Id>>,
        focus_id: impl Into<FocusId>,
        emit: impl Fn(Commit<Id>) -> Option<Message> + 'a,
    ) -> Self;
    #[must_use]
    pub fn with_palette(self, palette: Palette) -> Self;
}

pub struct ToggleSwitch<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> { /* private fields */ }
impl<'a, Message, Theme, Renderer> ToggleSwitch<'a, Message, Theme, Renderer> {
    #[must_use]
    pub fn controlled(
        model: Controlled<'a, SwitchModel>,
        focus_id: impl Into<FocusId>,
        emit: impl Fn(Commit<bool>) -> Option<Message> + 'a,
    ) -> Self;
    #[must_use]
    pub fn standalone(
        label: impl Into<String>,
        on: bool,
        focus_id: impl Into<FocusId>,
        on_toggle: impl Fn(bool) -> Option<Message> + 'a,
    ) -> Self;
    #[must_use]
    pub fn with_palette(self, palette: Palette) -> Self;
    #[must_use]
    pub fn disabled(self, disabled: bool) -> Self;
    #[must_use]
    pub fn busy(self, busy: bool) -> Self;
    #[must_use]
    pub fn with_texts(self, on_text: impl Into<String>, off_text: impl Into<String>) -> Self;
}
```

#### Examples
```rust
use hoarfrost::binding::Controlled;
use hoarfrost::focus::FocusId;
use hoarfrost::widget::rove::{RoveDirection, rove_index};
use hoarfrost::widget::toggle_model::{CheckState, CheckboxModel, SwitchModel};
use hoarfrost::widget::toggle_widget::{Checkbox, ToggleSwitch};

#[derive(Clone, Debug)]
enum Message {
    CheckCommitted(CheckState),
    NotifCommitted(bool),
}

// Roving focus helper wraps and skips disabled indices
assert_eq!(rove_index(&[0, 2, 4], Some(2), RoveDirection::Forward), Some(4));
assert_eq!(rove_index(&[0, 2, 4], Some(4), RoveDirection::Forward), Some(0));

let cb_model = CheckboxModel::new("Enable telemetry").with_state(CheckState::Unchecked);
let _cb: iced::Element<'_, Message> = Checkbox::controlled(
    Controlled::new(&cb_model),
    FocusId::new("telemetry-cb"),
    |c| Some(Message::CheckCommitted(c.into_value())),
)
.into();

let sw_model = SwitchModel::new("Desktop notifications").with_on(true);
let _sw: iced::Element<'_, Message> = ToggleSwitch::controlled(
    Controlled::new(&sw_model),
    FocusId::new("notif-sw"),
    |c| Some(Message::NotifCommitted(c.into_value())),
)
.into();
```

#### Type references
- `RoveDirection` - Direction of roving focus travel (`Forward` for `Down`/`Right`, `Backward` for `Up`/`Left`).
- `ToggleControl` - Identifies the indicator geometry (`Checkbox` `16x16px` rounded box, `Radio` `16x16px` circle, `Switch` `36x20px` pill track).
- `TogglePolarity` - Selects the §3.3 paint row (`Off` for unchecked/unselected/off, `On` for checked/indeterminate/selected/on).
- `ToggleStyle` - Resolved colors and border/ring widths for a toggle indicator and its inline label.
- `ChildSummary` - Counts of `enabled` and `checked` child items under a tri-state parent checkbox.
- `CheckState` - Tri-state checkbox value (`Unchecked`, `Indeterminate`, `Checked`).
- `CheckboxModel`, `RadioGroupModel<Id>`, `SwitchModel` - Application-owned models for the three toggle controls.
- `Checkbox`, `RadioGroup`, `ToggleSwitch` - Live, keyboard-focusable `iced` widgets implementing `UIF-ATOM-003`.

#### Return values
- `CheckboxModel::toggle`, `RadioGroupModel::select`, and `SwitchModel::toggle` return `Option<Commit<T>>`, returning `None` when disabled, when `SwitchModel::busy` is `true`, or when re-clicking an already-selected radio option.
- `indicator_layer`, `mark_layer`, and `focus_ring_layer` compose the checkbox, switch, and radio quad layers for any indicator rectangle and `ToggleStyle`.

#### Side effects
- `Checkbox::operate`, `RadioGroup::operate`, and `ToggleSwitch::operate` publish both the indicator container box (in all states) and the inline label text to `iced`'s operation stream, while registering as focusable only when enabled.

#### References
- Requirements: `UIF-ATOM-003`, `UIF-PRIN-003`, `UIF-PRIN-004`, `UIF-TOK-006`
- Source: [`toggle.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/toggle.rs), [`toggle_model.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/toggle_model.rs), [`toggle_widget.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/toggle_widget.rs), [`rove.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/rove.rs)

---

### `widget::combo_box`, `widget::combo_model`, `widget::combo_host` - ComboBox & Select (`UIF-ATOM-004`)

#### Overview
The `widget::combo_box` module provides two **live `iced` widgets (`Element<'a, Message>`)** (`ComboBox` and stateless `Select`) plus trigger style/geometry resolvers (`resolve_trigger`, `chevron_color`), while `widget::combo_model` and `widget::combo_host` provide the **headless model/controller** (`ComboBoxModel`, `ComboHost`, `ComboEmission`) implementing `UIF-ATOM-004` (spec §3.4). `ComboHost` delegates flyout focus trapping and row navigation to `surface::menu::cascade::MenuCascade` and retains a pre-open `ComboSnapshot<Id>` so `Escape` restores both the previous selection and filter query.

#### Prerequisites & Requirements
- Option identifier types `Id` used with `ComboBoxModel<Id>`, `ComboHost<Id>`, `ComboBox`, and `Select` **MUST** implement `Clone + fmt::Display + PartialEq`.
- When driving `ComboHost` with a `MenuCascade`, the application **MUST** call `ComboHost::sync(&model, &mut focus)` once per frame and project `host.is_open()` back onto the model via `ComboBoxModel::with_open(host.is_open())`.

#### Syntax / Method Signature
```rust
// widget::combo_model
pub const FLYOUT_MAX_HEIGHT: f32 = 280.0;
pub const FLYOUT_GAP: f32 = crate::token::space::XS;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectOption<Id> {
    pub id: Id,
    pub label: String,
    pub enabled: bool,
}

impl<Id> SelectOption<Id> {
    #[must_use]
    pub fn new(id: Id, label: impl Into<String>) -> Self;
    #[must_use]
    pub const fn disabled(self) -> Self;
    #[must_use]
    pub const fn with_enabled(self, enabled: bool) -> Self;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CaseHandling { #[default] Insensitive, Sensitive }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MatchMode { #[default] Contains, Prefix }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FilterPolicy {
    pub case: CaseHandling,
    pub mode: MatchMode,
    pub keep_selected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComboSnapshot<Id> {
    pub selected: Option<Id>,
    pub filter: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComboBoxModel<Id> { /* private fields */ }

impl<Id: Clone + fmt::Display + PartialEq> ComboBoxModel<Id> {
    #[must_use]
    pub fn new(items: Vec<SelectOption<Id>>, placeholder: impl Into<String>) -> Self;
    #[must_use]
    pub const fn editable(self) -> Self;
    #[must_use]
    pub const fn read_only(self) -> Self;
    #[must_use]
    pub const fn disabled(self) -> Self;
    #[must_use]
    pub fn with_selected(self, id: impl Into<Option<Id>>) -> Self;
    #[must_use]
    pub fn with_filter(self, query: impl Into<Option<String>>) -> Self;
    #[must_use]
    pub fn with_validation(self, message: impl Into<String>) -> Self;
    #[must_use]
    pub const fn with_filter_policy(self, policy: FilterPolicy) -> Self;
    #[must_use]
    pub fn with_menu_id(self, id: impl Into<String>) -> Self;
    #[must_use]
    pub const fn with_open(self, open: bool) -> Self;
    #[must_use]
    pub fn items(&self) -> &[SelectOption<Id>];
    #[must_use]
    pub const fn selected(&self) -> Option<&Id>;
    #[must_use]
    pub fn display_label(&self) -> &str;
    #[must_use]
    pub fn filtered(&self) -> Vec<&SelectOption<Id>>;
    #[must_use]
    pub fn snapshot(&self) -> ComboSnapshot<Id>;
}

// widget::combo_host
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ComboKey { Down, Up, Enter, Space, Home, End, Escape }

#[derive(Clone)]
pub enum ComboEmission<Id> {
    OpenChanged(bool),
    FilterChanged(Live<String>),
    Committed(Commit<Id>),
    Cancelled(Cancel<ComboSnapshot<Id>>),
}

impl<Id> ComboEmission<Id> {
    #[must_use]
    pub fn is_value_change(&self) -> bool;
}

#[derive(Clone, Debug)]
pub struct ComboOutcome<Id> {
    pub cascade: CascadeOutcome,
    pub emissions: Vec<ComboEmission<Id>>,
}

#[derive(Debug)]
pub struct ComboHost<Id> { /* private fields */ }

impl<Id: Clone + fmt::Display + PartialEq> ComboHost<Id> {
    #[must_use]
    pub fn new(control: FocusId) -> Self;
    #[must_use]
    pub fn is_open(&self) -> bool;
    pub fn open(&mut self, model: &ComboBoxModel<Id>, focus: &mut FocusState) -> ComboOutcome<Id>;
    pub fn close(&mut self, focus: &mut FocusState) -> ComboOutcome<Id>;
    pub fn toggle(&mut self, model: &ComboBoxModel<Id>, focus: &mut FocusState) -> ComboOutcome<Id>;
    pub fn on_filter_typed(&mut self, model: &ComboBoxModel<Id>, query: String, focus: &mut FocusState) -> ComboOutcome<Id>;
    pub fn key(&mut self, key: ComboKey, model: &ComboBoxModel<Id>, focus: &mut FocusState) -> ComboOutcome<Id>;
    pub fn escape(&mut self, focus: &mut FocusState) -> ComboOutcome<Id>;
    pub fn sync(&mut self, model: &ComboBoxModel<Id>, focus: &mut FocusState) -> ComboOutcome<Id>;
    pub fn reset(&mut self);
}

// widget::combo_box
pub const HEIGHT_STANDARD: f32 = size::CONTROL_MD;
pub const HEIGHT_COMPACT: f32 = size::CONTROL_SM;
pub const MIN_WIDTH: f32 = 120.0;
pub const MIN_WIDTH_COMPACT: f32 = 64.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ComboRung { Standard, Compact }

#[must_use]
pub fn resolve_trigger(state: VisualState, palette: &Palette) -> InputStyle;
#[must_use]
pub fn chevron_color(state: VisualState, open: bool, palette: &Palette) -> Color;

pub struct ComboBox<'a, Message, Id> { /* private fields */ }

impl<'a, Message, Id: Clone + fmt::Display + PartialEq> ComboBox<'a, Message, Id> {
    #[must_use]
    pub fn new(model: &'a ComboBoxModel<Id>) -> Self;
    #[must_use]
    pub const fn with_rung(self, rung: ComboRung) -> Self;
    #[must_use]
    pub fn with_width(self, width: impl Into<Length>) -> Self;
    #[must_use]
    pub fn with_compact_width(self, width: impl Into<Length>) -> Self;
    #[must_use]
    pub const fn with_palette(self, palette: Palette) -> Self;
    #[must_use]
    pub fn on_toggle_open(self, msg: Message) -> Self;
    #[must_use]
    pub fn on_press(self, msg: Message) -> Self;
    #[must_use]
    pub fn on_select(self, f: impl Fn(Id) -> Message + 'a) -> Self;
}

pub struct Select<'a, T, Message> { /* private fields */ }

impl<'a, T, Message> Select<'a, T, Message>
where
    T: Clone + PartialEq + fmt::Display + 'a,
    Message: Clone + 'a,
{
    #[must_use]
    pub fn new(
        options: &'a [T],
        selected: Option<&'a T>,
        on_select: impl Fn(T) -> Message + 'a,
    ) -> Self;
    #[must_use]
    pub fn with_placeholder(self, placeholder: &'a str) -> Self;
    #[must_use]
    pub const fn with_rung(self, rung: ComboRung) -> Self;
    #[must_use]
    pub fn with_width(self, width: impl Into<Length>) -> Self;
    #[must_use]
    pub const fn with_palette(self, palette: Palette) -> Self;
}

#[must_use]
pub fn combo_box<'a, Message, Id: Clone + fmt::Display + PartialEq>(
    model: &'a ComboBoxModel<Id>,
) -> ComboBox<'a, Message, Id>;

#[must_use]
pub fn select<'a, T, Message>(
    options: &'a [T],
    selected: Option<&'a T>,
    on_select: impl Fn(T) -> Message + 'a,
) -> Select<'a, T, Message>
where
    T: Clone + PartialEq + fmt::Display + 'a,
    Message: Clone + 'a;
```

#### Examples
```rust
use hoarfrost::focus::{FocusId, FocusState};
use hoarfrost::widget::combo_box::{ComboBox, ComboRung, Select};
use hoarfrost::widget::combo_host::{ComboEmission, ComboHost, ComboKey};
use hoarfrost::widget::combo_model::{ComboBoxModel, SelectOption};

#[derive(Clone, Debug)]
enum Msg {
    ToggleCombo,
    Picked(&'static str),
}

let model = ComboBoxModel::new(
    vec![
        SelectOption::new("utf8", "UTF-8"),
        SelectOption::new("utf16", "UTF-16 LE"),
    ],
    "Select encoding...",
)
.with_selected("utf8");

let mut focus = FocusState::new(vec![FocusId::new("encoding-combo")]);
let mut host = ComboHost::new(FocusId::new("encoding-combo"));

// Open and commit with keyboard
let _ = host.open(&model, &mut focus);
let outcome = host.key(ComboKey::Enter, &model, &mut focus);
for e in outcome.emissions {
    if let ComboEmission::Committed(commit) = e {
        assert_eq!(commit.into_value(), "utf8");
    }
}

// Live model-backed ComboBox widget and stateless Select widget
let _combo_el: iced::Element<'_, Msg> = ComboBox::new(&model)
    .with_rung(ComboRung::Standard)
    .on_toggle_open(Msg::ToggleCombo)
    .on_select(Msg::Picked)
    .into();

let encodings = ["UTF-8", "UTF-16 LE"];
let _select_el: iced::Element<'_, Msg> = Select::new(&encodings, Some(&encodings[0]), Msg::Picked)
    .with_placeholder("Encoding...")
    .with_rung(ComboRung::Compact)
    .into();
```

#### Type references
- `SelectOption<Id>` - A single choice in a `ComboBoxModel` carrying `id`, `label`, and `enabled`.
- `FilterPolicy` - Configures `CaseHandling` (`Insensitive`/`Sensitive`), `MatchMode` (`Contains`/`Prefix`), and `keep_selected` (`true` by default to preserve the `✓` row).
- `ComboSnapshot<Id>` - Pre-open state (`selected: Option<Id>`, `filter: Option<String>`) restored on `ComboEmission::Cancelled`.
- `ComboBoxModel<Id>` - Application-owned model for a closed select or editable combobox.
- `ComboKey` - Seven-key control vocabulary (`Down`, `Up`, `Enter`, `Space`, `Home`, `End`, `Escape`).
- `ComboEmission<Id>` - Tiered output sum type (`OpenChanged(bool)`, `FilterChanged(Live<String>)`, `Committed(Commit<Id>)`, `Cancelled(Cancel<ComboSnapshot<Id>>)`).
- `ComboOutcome<Id>` - Pairs the menu engine's `CascadeOutcome` with `Vec<ComboEmission<Id>>`.
- `ComboHost<Id>` - Per-control state machine coordinating `MenuCascade`, `GestureSlot<ComboSnapshot<Id>>`, and the working filter copy.
- `ComboRung` - Selects between `Standard` (`32px`, `min_width = 120px`) and `Compact` (`28px`, `min_width = 64px`) trigger heights.
- `ComboBox` and `Select` - Live `iced` widgets for model-driven and stateless single-selection dropdowns.

#### Return values
- `ComboHost::open`, `key`, `on_filter_typed`, and `escape` return a `ComboOutcome<Id>` containing both the menu cascade transition and any `ComboEmission<Id>` tiers.
- `chevron_color` returns `palette.text_strong` when hovered or open, `palette.text_primary` when focused and closed, and `palette.text_muted` at rest.

#### Side effects
- `ComboHost::open`, `close`, `key`, and `escape` mutate the passed `&mut FocusState` to push or pop the flyout's focus trap and return focus to the trigger on `Escape`.

#### References
- Requirements: `UIF-ATOM-004`, `UIF-PRIN-004`
- Source: [`combo_box.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/combo_box.rs), [`combo_model.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/combo_model.rs), [`combo_host.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/combo_host.rs)

---

### `widget::auto_suggest_model`, `widget::auto_suggest_host` - Auto-Suggest Box (`UIF-ATOM-002d`)

#### Overview
The `widget::auto_suggest_model` and `widget::auto_suggest_host` modules provide a **headless model/controller + style resolver** (`AutoSuggestModel`, `AutoSuggestHost`, `SuggestionItem`, `AutoSuggestCommit`, and `AutoSuggestEmission`) implementing `UIF-ATOM-002d` (spec §3.2). `AutoSuggestHost` joins `TextInputController` and `MenuCascade` with a caller-clocked `200ms` trailing-edge debounce timer (`DEBOUNCE_MS`), monotonic generation tokens that reject stale out-of-order provider responses, and a two-stage `Escape` unwind (dismiss flyout first, clear query second).

#### Prerequisites & Requirements
- Candidate identifier types `Id` **MUST** implement `Clone + fmt::Display + PartialEq`.
- The application **MUST** pass the current frame timestamp `TimeMs` to `AutoSuggestHost::note_edit` and `AutoSuggestHost::tick` so the trailing-edge debounce window can expire without internal wall-clock reads.
- When answering `AutoSuggestEmission::QueryRequested { generation, query }`, the application **MUST** pass the exact `generation` token back into `AutoSuggestHost::apply_suggestions` so superseded responses are dropped.
- The application **MUST** treat `model.highlighted_index` and `model.busy` as machine-projected read-only fields managed by `AutoSuggestHost`.

#### Syntax / Method Signature
```rust
// widget::auto_suggest_model
pub const FLYOUT_MAX_HEIGHT: f32 = 240.0;
pub const FLYOUT_ROW_HEIGHT: f32 = size::CONTROL_MD;
pub const FLYOUT_GAP: f32 = space::XS;
pub const FLYOUT_RADIUS: f32 = shape::RADIUS_WIDGET;
pub const DEBOUNCE_MS: u64 = time::DEBOUNCE_LIVE.ms;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SuggestionItem<Id> {
    pub id: Id,
    pub label: String,
}

impl<Id> SuggestionItem<Id> {
    #[must_use]
    pub fn new(id: Id, label: impl Into<String>) -> Self;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CommitSource {
    Enter,
    Click,
    Tab,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutoSuggestCommit<Id> {
    pub id: Id,
    pub source: CommitSource,
    pub label: String,
}

impl<Id> AutoSuggestCommit<Id> {
    #[must_use]
    pub fn new(id: Id, source: CommitSource, label: impl Into<String>) -> Self;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutoSuggestModel<Id> {
    pub query: String,
    pub suggestions: Vec<SuggestionItem<Id>>,
    pub highlighted_index: Option<usize>,
    pub busy: bool,
    pub disabled: bool,
}

impl<Id> AutoSuggestModel<Id> {
    #[must_use]
    pub fn new() -> Self;
    #[must_use]
    pub fn with_query(self, query: impl Into<String>) -> Self;
    #[must_use]
    pub fn with_suggestions(self, suggestions: Vec<SuggestionItem<Id>>) -> Self;
    #[must_use]
    pub fn disabled(self) -> Self;
    #[must_use]
    pub fn has_suggestions(&self) -> bool;
    #[must_use]
    pub fn query_is_blank(&self) -> bool;
    #[must_use]
    pub fn suggestion_at(&self, index: usize) -> Option<&SuggestionItem<Id>>;
    #[must_use]
    pub fn highlighted(&self) -> Option<&SuggestionItem<Id>>;
}

// widget::auto_suggest_host
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AutoSuggestKey { Up, Down, Enter, Tab, Escape }

#[derive(Clone, Debug)]
pub enum AutoSuggestEmission<Id> {
    QueryChanged(Live<String>),
    QueryRequested {
        generation: u64,
        query: String,
    },
    HighlightChanged(usize),
    Committed(AutoSuggestCommit<Id>),
    QueryCommitted(Commit<String>),
    QueryCleared(Commit<String>),
    Dismissed,
    OpenChanged(bool),
    FocusChanged(bool),
}

impl<Id> AutoSuggestEmission<Id> {
    #[must_use]
    pub const fn tier(&self) -> &'static str;
    #[must_use]
    pub const fn settles(&self) -> bool;
    #[must_use]
    pub const fn name(&self) -> &'static str;
}

#[derive(Debug)]
pub struct AutoSuggestHost<Id> { /* private fields */ }

impl<Id: Clone + fmt::Display + PartialEq> AutoSuggestHost<Id> {
    #[must_use]
    pub fn new(control: FocusId) -> Self;
    #[must_use]
    pub fn with_debounce(control: FocusId, debounce_ms: u64) -> Self;
    #[must_use]
    pub fn is_open(&self) -> bool;
    #[must_use]
    pub fn is_busy(&self) -> bool;
    #[must_use]
    pub fn is_debouncing(&self) -> bool;
    #[must_use]
    pub fn highlighted(&self) -> Option<usize>;
    #[must_use]
    pub const fn generation(&self) -> u64;
    #[must_use]
    pub const fn stale_dropped(&self) -> usize;
    #[must_use]
    pub const fn provider_passes(&self) -> usize;
    pub fn note_edit(
        &mut self,
        model: &mut AutoSuggestModel<Id>,
        value: String,
        now: TimeMs,
    ) -> Vec<AutoSuggestEmission<Id>>;
    pub fn tick(
        &mut self,
        model: &mut AutoSuggestModel<Id>,
        now: TimeMs,
    ) -> Vec<AutoSuggestEmission<Id>>;
    pub fn apply_suggestions(
        &mut self,
        model: &mut AutoSuggestModel<Id>,
        generation: u64,
        suggestions: Vec<SuggestionItem<Id>>,
        focus: &mut FocusState,
    ) -> Vec<AutoSuggestEmission<Id>>;
    pub fn key(
        &mut self,
        model: &mut AutoSuggestModel<Id>,
        key: AutoSuggestKey,
        focus: &mut FocusState,
    ) -> Vec<AutoSuggestEmission<Id>>;
    pub fn escape(
        &mut self,
        model: &mut AutoSuggestModel<Id>,
        focus: &mut FocusState,
    ) -> Vec<AutoSuggestEmission<Id>>;
    pub fn click(
        &mut self,
        model: &mut AutoSuggestModel<Id>,
        index: usize,
        focus: &mut FocusState,
    ) -> Vec<AutoSuggestEmission<Id>>;
    pub fn note_focus(
        &mut self,
        model: &mut AutoSuggestModel<Id>,
        focused: bool,
        focus: &mut FocusState,
    ) -> Vec<AutoSuggestEmission<Id>>;
    pub fn sync(
        &mut self,
        model: &mut AutoSuggestModel<Id>,
        focus: &mut FocusState,
    ) -> Vec<AutoSuggestEmission<Id>>;
    pub fn reset(&mut self);
}
```

#### Examples
```rust
use hoarfrost::focus::{FocusId, FocusState};
use hoarfrost::surface::tooltip::TimeMs;
use hoarfrost::widget::auto_suggest_host::{AutoSuggestEmission, AutoSuggestHost, AutoSuggestKey};
use hoarfrost::widget::auto_suggest_model::{
    AutoSuggestCommit, AutoSuggestModel, CommitSource, DEBOUNCE_MS, SuggestionItem,
};

let mut model: AutoSuggestModel<&'static str> = AutoSuggestModel::new();
let mut host = AutoSuggestHost::new(FocusId::new("cmd-suggest"));
let mut focus = FocusState::new(vec![FocusId::new("cmd-suggest")]);

// 1. User types "git"; debounce window arms at t = 0ms
let _ = host.note_edit(&mut model, "git".to_owned(), TimeMs::from_millis(0));

// 2. Frame tick at t = 200ms closes the debounce window and requests suggestions
let tick_emissions = host.tick(&mut model, TimeMs::from_millis(DEBOUNCE_MS));
let gen = match &tick_emissions[0] {
    AutoSuggestEmission::QueryRequested { generation, query } => {
        assert_eq!(query, "git");
        *generation
    }
    other => panic!("unexpected: {other:?}"),
};

// 3. Provider responds with matching generation token; flyout opens and highlights row 0
let _ = host.apply_suggestions(
    &mut model,
    gen,
    vec![SuggestionItem::new("git-status", "git status")],
    &mut focus,
);

// 4. Pressing Enter commits the highlighted suggestion
let key_emissions = host.key(&mut model, AutoSuggestKey::Enter, &mut focus);
assert_eq!(
    key_emissions[0],
    AutoSuggestEmission::Committed(AutoSuggestCommit::new(
        "git-status",
        CommitSource::Enter,
        "git status",
    ))
);
```

#### Type references
- `SuggestionItem<Id>` - Completion candidate holding a stable `id: Id` and display `label: String`.
- `CommitSource` - Identifies how a candidate was committed (`Enter`, `Click`, or `Tab`).
- `AutoSuggestCommit<Id>` - Commit payload carrying `id`, `source`, and the settled `label`.
- `AutoSuggestModel<Id>` - Declarative state holding `query`, `suggestions`, `highlighted_index`, `busy`, and `disabled`.
- `AutoSuggestKey` - Control-level key vocabulary (`Up`, `Down`, `Enter`, `Tab`, `Escape`).
- `AutoSuggestEmission<Id>` - Nine-arm emission enum covering live query edits, debounced provider requests, highlight motion, candidate/query commits, two-stage `Escape`, flyout openness, and focus edges.
- `AutoSuggestHost<Id>` - State machine coordinating `TextInputController`, `MenuCascade`, debounce timing, generation validation, and dismissed-query suppression.

#### Return values
- `AutoSuggestHost::tick` returns `vec![AutoSuggestEmission::QueryRequested { generation, query }]` once `now.elapsed_since(armed_at) >= self.debounce_ms`, or an empty `Vec` while waiting.
- `AutoSuggestHost::apply_suggestions` returns an empty `Vec` and increments `stale_dropped` if `generation` does not match the currently in-flight request.

#### Side effects
- `AutoSuggestHost` methods mutate `model.query`, `model.suggestions`, `model.highlighted_index`, and `model.busy` in place to keep the model synchronized with the editing session, in-flight request, and `MenuCascade` focus.

#### References
- Requirements: `UIF-ATOM-002d`, `UIF-PRIN-001`, `UIF-TOK-007`
- Source: [`auto_suggest_model.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/auto_suggest_model.rs), [`auto_suggest_host.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/auto_suggest_host.rs)

---

### `widget::date_picker` - DatePicker, DateRangePicker & CalendarView (`UIF-ATOM-010`)

#### Overview
The `widget::date_picker` module provides **live `iced` widgets (`Element<'a, Message>`)** (`DatePicker`, `DateRangePicker`, `CalendarView`, and constructors `date_picker`, `date_range_picker`, `calendar_view`) along with a **headless civil-date model and grid calculator** (`DatePickerModel`, `CivilDate`, `YearMonth`, `Weekday`, `DateSelection`, `CalendarViewMode`, `displayed_month_cells`) implementing `UIF-ATOM-010` (spec §3.10). It supports single-date and date-range selection (`start <= end` invariant), `Days`/`Months`/`Years` drill-down views, `min_date`/`max_date` clamping, and a `280px`-wide 7-column calendar flyout.

#### Prerequisites & Requirements
- Any `DateSelection::DateRange` passed to `DatePickerModel` **MUST** satisfy `start <= end`; `DatePickerModel::validate_invariants()` asserts this ordering alongside `min_date <= max_date`.
- The application **MUST** wire `on_open_change`, `on_month_navigate`, `on_view_mode_change`, `on_change`, `on_commit`, and `on_cancel` on `DatePicker` (or `DateRangePicker` / `CalendarView`) to persist user navigation and selection changes in its model.

#### Syntax / Method Signature
```rust
pub const CALENDAR_FLYOUT_WIDTH: f32 = 280.0;
pub const CALENDAR_CELL_SIZE: f32 = 36.0;
pub const CALENDAR_HEADER_HEIGHT: f32 = 36.0;
pub const WEEKDAY_ROW_HEIGHT: f32 = 24.0;
pub const CALENDAR_GRID_COLUMNS: usize = 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CivilDate {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl CivilDate {
    #[must_use]
    pub const fn new(year: i32, month: u8, day: u8) -> Option<Self>;
    #[must_use]
    pub const fn is_leap_year(year: i32) -> bool;
    #[must_use]
    pub const fn days_in_month(year: i32, month: u8) -> u8;
    #[must_use]
    pub const fn weekday(self) -> Weekday;
    #[must_use]
    pub const fn add_days(self, delta: i32) -> Self;
    #[must_use]
    pub const fn add_months(self, delta: i32) -> Self;
    #[must_use]
    pub const fn add_years(self, delta: i32) -> Self;
    #[must_use]
    pub fn format_iso(self) -> String;
    #[must_use]
    pub fn parse_iso(input: &str) -> Option<Self>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct YearMonth {
    pub year: i32,
    pub month: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Weekday { Mon, Tue, Wed, Thu, Fri, Sat, Sun }

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CalendarViewMode { #[default] Days, Months, Years }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DateSelection {
    SingleDate(Option<CivilDate>),
    DateRange(Option<(CivilDate, CivilDate)>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatePickerModel {
    pub selection: DateSelection,
    pub displayed_month: YearMonth,
    pub min_date: Option<CivilDate>,
    pub max_date: Option<CivilDate>,
    pub first_day_of_week: Weekday,
    pub view_mode: CalendarViewMode,
    pub open: bool,
    pub validation: Option<String>,
    pub disabled: bool,
}

impl DatePickerModel {
    #[must_use]
    pub const fn new(displayed_month: YearMonth) -> Self;
    #[must_use]
    pub const fn with_single_date(self, date: Option<CivilDate>) -> Self;
    #[must_use]
    pub const fn with_date_range(self, range: Option<(CivilDate, CivilDate)>) -> Option<Self>;
    #[must_use]
    pub const fn with_bounds(self, min_date: Option<CivilDate>, max_date: Option<CivilDate>) -> Option<Self>;
    #[must_use]
    pub const fn with_first_day_of_week(self, weekday: Weekday) -> Self;
    #[must_use]
    pub const fn with_view_mode(self, mode: CalendarViewMode) -> Self;
    #[must_use]
    pub const fn with_open(self, open: bool) -> Self;
    #[must_use]
    pub fn with_validation(self, validation: impl Into<String>) -> Self;
    #[must_use]
    pub const fn with_disabled(self, disabled: bool) -> Self;
    #[must_use]
    pub const fn is_date_in_bounds(&self, date: CivilDate) -> bool;
    #[must_use]
    pub const fn clamp_date(&self, date: CivilDate) -> CivilDate;
    #[must_use]
    pub const fn validate_invariants(&self) -> bool;
    #[must_use]
    pub fn trigger_text(&self) -> String;
}

pub struct DatePicker<'a, Message> { /* private fields */ }
pub type DateRangePicker<'a, Message> = DatePicker<'a, Message>;
pub type CalendarView<'a, Message> = DatePicker<'a, Message>;

impl<'a, Message> DatePicker<'a, Message> {
    #[must_use]
    pub fn new(model: DatePickerModel) -> Self;
    #[must_use]
    pub fn with_id(self, id: impl Into<iced::advanced::widget::Id>) -> Self;
    #[must_use]
    pub fn widget_id(&self) -> &iced::advanced::widget::Id;
    #[must_use]
    pub fn with_today(self, today: impl Into<Option<CivilDate>>) -> Self;
    #[must_use]
    pub fn with_palette(self, palette: Palette) -> Self;
    #[must_use]
    pub fn on_open_change(self, f: impl Fn(bool) -> Message + 'a) -> Self;
    #[must_use]
    pub fn on_view_mode_change(self, f: impl Fn(CalendarViewMode) -> Message + 'a) -> Self;
    #[must_use]
    pub fn on_month_navigate(self, f: impl Fn(YearMonth) -> Message + 'a) -> Self;
    #[must_use]
    pub fn on_change(self, f: impl Fn(Live<DateSelection>) -> Message + 'a) -> Self;
    #[must_use]
    pub fn on_commit(self, f: impl Fn(Commit<DateSelection>) -> Message + 'a) -> Self;
    #[must_use]
    pub fn on_cancel(self, f: impl Fn(Cancel<DateSelection>) -> Message + 'a) -> Self;
    #[must_use]
    pub const fn model(&self) -> &DatePickerModel;
    #[must_use]
    pub const fn size(&self) -> Size<Length>;
}

#[must_use]
pub fn date_picker<'a, Message>(model: DatePickerModel) -> DatePicker<'a, Message>;
#[must_use]
pub fn date_range_picker<'a, Message>(model: DatePickerModel) -> DateRangePicker<'a, Message>;
#[must_use]
pub fn calendar_view<'a, Message>(model: DatePickerModel) -> CalendarView<'a, Message>;
```

#### Examples
```rust
use hoarfrost::widget::date_picker::{
    CivilDate, DatePickerModel, DateSelection, YearMonth, date_picker, date_range_picker,
};

#[derive(Clone, Debug)]
enum Msg {
    DateCommitted(DateSelection),
    DateCancelled(DateSelection),
}

let ym = YearMonth { year: 2026, month: 10 };
let start = CivilDate::new(2026, 10, 22).unwrap();
let end = CivilDate::new(2026, 10, 29).unwrap();

// Single-date picker
let single_model = DatePickerModel::new(ym).with_single_date(Some(start));
let _single_el: iced::Element<'_, Msg> = date_picker(single_model)
    .with_today(start)
    .on_commit(|c| Msg::DateCommitted(c.into_value()))
    .on_cancel(|c| Msg::DateCancelled(c.into_restored()))
    .into();

// Date-range picker (`DateSelection::DateRange` with `start <= end`)
let range_model = DatePickerModel::new(ym)
    .with_date_range(Some((start, end)))
    .unwrap();
let _range_el: iced::Element<'_, Msg> = date_range_picker(range_model)
    .on_commit(|c| Msg::DateCommitted(c.into_value()))
    .on_cancel(|c| Msg::DateCancelled(c.into_restored()))
    .into();
```

#### Type references
- `CivilDate` - Proleptic Gregorian calendar date (`year`, `month`, `day`) with leap-year, weekday, arithmetic, and ISO-8601 (`YYYY-MM-DD`) formatting/parsing helpers.
- `YearMonth` - Displayed calendar page (`year`, `month`).
- `Weekday` - Day of the week (`Mon` .. `Sun`).
- `CalendarViewMode` - Flyout drill-down mode (`Days`, `Months`, `Years`).
- `DateSelection` - Either `SingleDate(Option<CivilDate>)` or `DateRange(Option<(CivilDate, CivilDate)>)`.
- `DatePickerModel` - Declarative state for single-date pickers, date-range pickers (`with_date_range`), and inline/open calendar views (`with_open(true)`).
- `DatePicker`, `DateRangePicker`, `CalendarView` - Live `iced` widgets rendering the `32px` trigger and `280px` calendar flyout.

#### Return values
- `CivilDate::new`, `YearMonth::new`, `DatePickerModel::with_date_range`, and `DatePickerModel::with_bounds` return `Option<Self>`, returning `None` if calendar bounds or `start <= end` invariants are violated.
- `date_picker`, `date_range_picker`, and `calendar_view` return a `DatePicker<'a, Message>` convertible into `iced::Element<'a, Message>`.

#### Side effects
- `DatePicker::update` manages internal focus, hovered day cells, range-anchor selection state, and text-entry buffers inside `iced`'s widget tree (`DatePickerState`) and publishes tier callbacks through `Shell::publish`.

#### References
- Requirements: `UIF-ATOM-010`, `UIF-TOK-006`
- Source: [`date_picker.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/date_picker.rs)

---

### `widget::time_picker` - TimePicker (`UIF-ATOM-011`)

#### Overview
The `widget::time_picker` module provides a **live `iced` widget (`Element<'a, Message>`)** (`TimePicker`) and a **headless civil-time model** (`TimePickerModel`, `CivilTime`, `AmPm`, `TimeSegment`, `DrumColumnKind`, `active_columns`) implementing `UIF-ATOM-011` (spec §3.11). It supports inline keyboard-navigable numeric segments (`HH:MM`, `HH:MM:SS`, 12-hour `AM`/`PM` or 24-hour) and a multi-column scrollable drum flyout with `minute_increment` snapping and `Escape` revert.

#### Prerequisites & Requirements
- The application **MUST** construct `CivilTime` via `CivilTime::new(hour, minute, second)` (which validates `hour < 24`, `minute < 60`, `second < 60`) and configure `TimePickerModel::with_minute_increment` with a divisor of `60` (`1`, `5`, `10`, `15`, `20`, or `30`).
- When mounting `TimePicker`, the application **MUST** wire `on_commit` (and optionally `on_change`, `on_cancel`, and `on_open_change`) to update its stored `TimePickerModel`.

#### Syntax / Method Signature
```rust
pub const DRUM_COLUMN_WIDTH: f32 = 56.0;
pub const DRUM_ROW_HEIGHT: f32 = size::CONTROL_MD;
pub const DRUM_VISIBLE_ROWS: usize = 7;
pub const DRUM_FLYOUT_HEIGHT: f32 = 224.0;
pub const TRIGGER_HEIGHT: f32 = size::CONTROL_MD;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CivilTime {
    hour: u8,
    minute: u8,
    second: u8,
}

impl CivilTime {
    #[must_use]
    pub const fn new(hour: u8, minute: u8, second: u8) -> Option<Self>;
    #[must_use]
    pub const fn hour(self) -> u8;
    #[must_use]
    pub const fn minute(self) -> u8;
    #[must_use]
    pub const fn second(self) -> u8;
    #[must_use]
    pub const fn to_12h(self) -> (u8, AmPm);
    #[must_use]
    pub const fn from_12h(hour12: u8, am_pm: AmPm, minute: u8, second: u8) -> Option<Self>;
    #[must_use]
    pub const fn with_hour(self, hour: u8) -> Option<Self>;
    #[must_use]
    pub const fn with_minute(self, minute: u8) -> Option<Self>;
    #[must_use]
    pub const fn with_second(self, second: u8) -> Option<Self>;
    #[must_use]
    pub const fn step_hours(self, delta: i32) -> Self;
    #[must_use]
    pub const fn step_minutes(self, delta: i32) -> Self;
    #[must_use]
    pub const fn step_seconds(self, delta: i32) -> Self;
    #[must_use]
    pub const fn toggle_am_pm(self) -> Self;
    #[must_use]
    pub const fn snap_minute(self, increment: u8) -> Self;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AmPm { Am, Pm }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimeSegment { Hour, Minute, Second, AmPm }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DrumColumnKind { Hour, Minute, Second, AmPm }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimePickerModel {
    pub value: Option<CivilTime>,
    pub use_24_hour: bool,
    pub show_seconds: bool,
    pub minute_increment: u8,
    pub open: bool,
    pub validation: Option<String>,
    pub disabled: bool,
}

impl TimePickerModel {
    #[must_use]
    pub const fn new() -> Self;
    #[must_use]
    pub const fn with_value(self, value: Option<CivilTime>) -> Self;
    #[must_use]
    pub const fn with_24_hour(self, use_24_hour: bool) -> Self;
    #[must_use]
    pub const fn with_seconds(self, show_seconds: bool) -> Self;
    #[must_use]
    pub const fn with_minute_increment(self, increment: u8) -> Self;
    #[must_use]
    pub const fn with_open(self, open: bool) -> Self;
    #[must_use]
    pub fn with_validation(self, validation: Option<String>) -> Self;
    #[must_use]
    pub const fn with_disabled(self, disabled: bool) -> Self;
    #[must_use]
    pub fn format_display(&self) -> String;
}

pub struct TimePicker<'a, Message> { /* private fields */ }

impl<'a, Message> TimePicker<'a, Message> {
    #[must_use]
    pub fn new(model: TimePickerModel) -> Self;
    #[must_use]
    pub fn with_id(self, id: impl Into<iced::advanced::widget::Id>) -> Self;
    #[must_use]
    pub fn widget_id(&self) -> &iced::advanced::widget::Id;
    #[must_use]
    pub fn palette(self, palette: Palette) -> Self;
    #[must_use]
    pub fn on_open_change(self, callback: impl Fn(bool) -> Message + 'a) -> Self;
    #[must_use]
    pub fn on_change(self, callback: impl Fn(Live<CivilTime>) -> Message + 'a) -> Self;
    #[must_use]
    pub fn on_commit(self, callback: impl Fn(Commit<CivilTime>) -> Message + 'a) -> Self;
    #[must_use]
    pub fn on_cancel(self, callback: impl Fn(Cancel<Option<CivilTime>>) -> Message + 'a) -> Self;
    #[must_use]
    pub const fn model(&self) -> &TimePickerModel;
    pub fn model_mut(&mut self) -> &mut TimePickerModel;
    #[must_use]
    pub const fn size(&self) -> Size<Length>;
}
```

#### Examples
```rust
use hoarfrost::widget::time_picker::{CivilTime, TimePicker, TimePickerModel};

#[derive(Clone, Debug)]
enum Msg {
    TimePreview(CivilTime),
    TimeCommitted(CivilTime),
    TimeCancelled(Option<CivilTime>),
}

let initial = CivilTime::new(14, 30, 0).unwrap();
let model = TimePickerModel::new()
    .with_value(Some(initial))
    .with_24_hour(true)
    .with_minute_increment(15);

let _picker: iced::Element<'_, Msg> = TimePicker::new(model)
    .on_change(|live| Msg::TimePreview(live.into_value()))
    .on_commit(|commit| Msg::TimeCommitted(commit.into_value()))
    .on_cancel(|cancel| Msg::TimeCancelled(cancel.into_restored()))
    .into();
```

#### Type references
- `CivilTime` - Validated time of day (`0..=23` hour, `0..=59` minute, `0..=59` second) with 12h/24h conversion and wrapping step helpers.
- `AmPm` - 12-hour period indicator (`Am`, `Pm`).
- `TimeSegment` - Inline trigger keyboard segment (`Hour`, `Minute`, `Second`, `AmPm`).
- `DrumColumnKind` - Column identifier inside the drum flyout (`Hour`, `Minute`, `Second`, `AmPm`).
- `TimePickerModel` - Declarative state for time format (`use_24_hour`, `show_seconds`), `minute_increment`, `open`, `validation`, and `disabled`.
- `TimePicker` - Live `iced` widget rendering the segmented time trigger and `224px`-high drum flyout.

#### Return values
- `CivilTime::new` and `CivilTime::from_12h` return `Option<CivilTime>`, returning `None` for out-of-range components.
- `TimePickerModel::format_display` returns the formatted trigger string (or placeholder `"--:--"` / `"--:--:--"` when `value` is `None`).

#### Side effects
- `TimePicker::update` tracks active inline segments, two-digit numeric entry accumulation, and pre-open `Option<CivilTime>` snapshots inside `TimePickerState`.

#### References
- Requirements: `UIF-ATOM-011`, `UIF-TOK-002`
- Source: [`time_picker.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/time_picker.rs)

---
### `widget::color_picker` - color selection and swatch trigger (`UIF-ATOM-012`)

#### Overview
The `widget::color_picker` module provides live interactive `iced` widgets (`Element<'a, Message>`) through `ColorPicker` and `ColorSwatchButton` (constructed via `color_picker` and `color_swatch_button`), backed by `ColorPickerModel` and geometry resolver `ColorFlyoutLayout`. It implements §3.12 (`UIF-ATOM-012`, `UIF-TOK-006`) with a 16×16px preview well over a neutral checkerboard, an anchored 240px card flyout with a 2D Saturation/Value canvas, 1D Hue and Alpha scrub rails, hex text entry, and a live WCAG 2.1 contrast readout chip.

#### Prerequisites & Requirements
- When constructing `ColorPicker` or `ColorSwatchButton`, applications **MUST** provide a distinct widget identifier via `with_id` if more than one instance appears in the layout tree to ensure correct keyboard focus routing.
- Applications **MUST** handle the `Live<Rgba8>`, `Commit<Rgba8>`, and `Cancel<Rgba8>` tiers appropriately: continuous updates during mouse dragging **MUST** remain lightweight, while costly application operations **MUST NOT** execute until `on_commit` is emitted.
- On `Escape` or Cancel button activation, the picker **MUST** revert its internal color state to `pre_open_snapshot` recorded at flyout opening, and callers **MUST** restore this value.
- When `contrast_reference` is supplied, contrast calculation **MUST** conform to WCAG 2.1 relative luminance standards, displaying `AA`, `AAA`, or `Fail` badges.

#### Syntax / Method Signature
```rust
// String constants & glyphs (§3.12, [RUST-021])
pub const HEX_PREFIX: &str = "#";
pub const FONT_FAMILY_MONO: &str = "monospace";
pub const BADGE_AA: &str = "AA";
pub const BADGE_AAA: &str = "AAA";
pub const BADGE_FAIL: &str = "Fail";
pub const ICON_PASS: &str = "✓";
pub const ICON_WARN: &str = "⚠";
pub const CHEVRON_DOWN: &str = "▾";
pub const CHEVRON_UP: &str = "▴";
pub const DEFAULT_COLOR_PICKER_ID: &str = "hoarfrost.color_picker";
pub const DEFAULT_COLOR_SWATCH_BUTTON_ID: &str = "hoarfrost.color_swatch_button";

// Layout & geometry constants (§3.12, UIF-ATOM-012, UIF-TOK-002, UIF-PRIN-001 §1.1)
pub const COLOR_FLYOUT_WIDTH: f32 = 240.0;
pub const COLOR_WELL_WIDTH: f32 = 180.0;
pub const COLOR_WELL_HEIGHT: f32 = 140.0;
pub const TRIGGER_HEIGHT_SM: f32 = size::CONTROL_SM; // 28.0px
pub const TRIGGER_HEIGHT_MD: f32 = size::CONTROL_MD; // 32.0px
pub const PREVIEW_WELL_SIZE: f32 = size::SLIDER_THUMB; // 16.0px
pub const CHECKERBOARD_CELL_SIZE: f32 = space::XS; // 4.0px
pub const SWATCH_SIZE: f32 = size::HIT_TARGET_MIN; // 24.0px
pub const SLIDER_TRACK_HEIGHT: f32 = size::SLIDER_TRACK_HEIGHT; // 4.0px
pub const THUMB_SIZE: f32 = size::SLIDER_THUMB; // 16.0px
pub const CARD_PADDING: f32 = space::MD; // 12.0px
pub const CARD_RADIUS: f32 = shape::RADIUS_CARD; // 6.0px
pub const INNER_BORDER_WIDTH: f32 = shape::BORDER_HAIRLINE; // 1.0px
pub const DUAL_RING_OUTER_WIDTH: f32 = shape::BORDER_FOCUS; // 2.0px
pub const DUAL_RING_INNER_WIDTH: f32 = shape::BORDER_HAIRLINE; // 1.0px
pub const SWATCH_GAP: f32 = space::XS; // 4.0px
pub const SECTION_GAP: f32 = space::SM; // 8.0px
pub const CONTRAST_CHIP_HEIGHT: f32 = size::BADGE_CHIP_HEIGHT; // 24.0px

// Color Data Models & Conversions (§3.12, UIF-ATOM-012)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Rgba8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba8 {
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self;
    #[must_use]
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self;
    #[must_use]
    pub const fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self;
    #[must_use]
    pub fn from_hex(s: &str) -> Option<Self>;
    #[must_use]
    pub fn to_hex_rgb(&self) -> String;
    #[must_use]
    pub fn to_hex_rgba(&self) -> String;
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Hsva {
    pub h: f32, // 0.0..=360.0
    pub s: f32, // 0.0..=1.0
    pub v: f32, // 0.0..=1.0
    pub a: f32, // 0.0..=1.0
}

/// Type alias for HSV representations without explicit alpha distinction (§3.12).
pub type Hsv = Hsva;

impl Hsva {
    #[must_use]
    pub fn new(h: f32, s: f32, v: f32, a: f32) -> Self;
    #[must_use]
    pub fn from_hsv(h: f32, s: f32, v: f32) -> Self;
    #[must_use]
    pub fn from_hsva(h: f32, s: f32, v: f32, a: f32) -> Self;
    #[must_use]
    pub fn to_rgba8(&self) -> Rgba8;
    #[must_use]
    pub fn from_rgba8(rgba: Rgba8) -> Self;
}

/// Type alias for settled color commit payloads (§3.12, UIF-ATOM-012).
pub type ColorCommit = Commit<Rgba8>;

#[must_use]
pub fn default_preset_swatches() -> Vec<Rgba8>;

// Declarative Model (§3.12, UIF-ATOM-012)
#[derive(Debug, Clone, PartialEq)]
pub struct ColorPickerModel {
    pub value: Rgba8,
    pub hsv: Hsva,
    pub hex_input: String,
    pub alpha_enabled: bool,
    pub preset_swatches: Vec<Rgba8>,
    pub contrast_reference: Option<Rgba8>,
    pub open: bool,
    pub disabled: bool,
    pub pre_open_snapshot: Option<Rgba8>,
    pub validation_error: Option<String>,
}

impl ColorPickerModel {
    #[must_use]
    pub fn new(value: Rgba8) -> Self;
    #[must_use]
    pub fn with_alpha(mut self, enabled: bool) -> Self;
    #[must_use]
    pub fn with_preset_swatches(mut self, swatches: impl IntoIterator<Item = Rgba8>) -> Self;
    #[must_use]
    pub fn with_contrast_reference(mut self, reference: Rgba8) -> Self;
    #[must_use]
    pub fn with_open(mut self, open: bool) -> Self;
    #[must_use]
    pub fn with_disabled(mut self, disabled: bool) -> Self;
    pub fn set_rgba(&mut self, rgba: Rgba8);
    pub fn set_hsva(&mut self, hsva: Hsva);
    pub fn set_hex_input(&mut self, hex: &str) -> bool;
    pub fn open_flyout(&mut self);
    pub fn close_flyout(&mut self);
    pub fn cancel(&mut self) -> Rgba8;
    pub fn commit(&mut self) -> Rgba8;
    #[must_use]
    pub fn format_display(&self) -> String;
}

// Layout Calculator (§3.12, UIF-ATOM-012)
#[derive(Debug, Clone)]
pub struct ColorFlyoutLayout {
    pub card_bounds: Rectangle,
    pub sv_well_bounds: Rectangle,
    pub hue_slider_bounds: Rectangle,
    pub alpha_slider_bounds: Option<Rectangle>,
    pub hex_input_bounds: Rectangle,
    pub contrast_chip_bounds: Option<Rectangle>,
    pub preset_swatch_bounds: Vec<Rectangle>,
    pub cancel_button_bounds: Rectangle,
    pub apply_button_bounds: Rectangle,
}

impl ColorFlyoutLayout {
    #[must_use]
    pub fn compute(top_left: Point, model: &ColorPickerModel) -> Self;
}

// Standalone Trigger Widget (§3.12, UIF-ATOM-012)
pub struct ColorSwatchButton<'a, Message> { /* private fields */ }

impl<'a, Message> ColorSwatchButton<'a, Message> {
    #[must_use]
    pub fn new(model: ColorPickerModel) -> Self;
    #[must_use]
    pub fn with_id(mut self, id: impl Into<iced::advanced::widget::Id>) -> Self;
    #[must_use]
    pub fn widget_id(&self) -> &iced::advanced::widget::Id;
    #[must_use]
    pub const fn with_height(mut self, height: f32) -> Self;
    #[must_use]
    pub fn on_press(mut self, on_press: impl Fn() -> Message + 'a) -> Self;
}

#[must_use]
pub fn color_swatch_button<'a, Message>(
    model: &ColorPickerModel,
) -> ColorSwatchButton<'a, Message>;

// Interactive Color Picker Widget (§3.12, UIF-ATOM-012)
pub struct ColorPicker<'a, Message> { /* private fields */ }

impl<'a, Message> ColorPicker<'a, Message> {
    #[must_use]
    pub fn new(model: ColorPickerModel) -> Self;
    #[must_use]
    pub fn with_id(mut self, id: impl Into<iced::advanced::widget::Id>) -> Self;
    #[must_use]
    pub fn widget_id(&self) -> &iced::advanced::widget::Id;
    #[must_use]
    pub const fn with_trigger_height(mut self, height: f32) -> Self;
    #[must_use]
    pub fn on_change(mut self, on_change: impl Fn(Live<Rgba8>) -> Message + 'a) -> Self;
    #[must_use]
    pub fn on_commit(mut self, on_commit: impl Fn(Commit<Rgba8>) -> Message + 'a) -> Self;
    #[must_use]
    pub fn on_cancel(mut self, on_cancel: impl Fn(Cancel<Rgba8>) -> Message + 'a) -> Self;
    #[must_use]
    pub fn on_open_change(mut self, on_open_change: impl Fn(Live<bool>) -> Message + 'a) -> Self;
    #[must_use]
    pub const fn model(&self) -> &ColorPickerModel;
    pub fn model_mut(&mut self) -> &mut ColorPickerModel;
}

#[must_use]
pub fn color_picker<'a, Message>(model: ColorPickerModel) -> ColorPicker<'a, Message>;
```

#### Examples
```rust
use hoarfrost::binding::{Cancel, Commit, Live};
use hoarfrost::widget::color_picker::{
    ColorCommit, ColorPickerModel, ColorSwatchButton, Rgba8, color_picker, color_swatch_button,
};
use iced::Element;

#[derive(Clone, Debug)]
enum Message {
    ColorScrubbed(Rgba8),
    ColorCommitted(Rgba8),
    ColorRolledBack(Rgba8),
    FlyoutToggled(bool),
    SwatchPressed,
}

// 1. Build model with contrast reference against window background
let model = ColorPickerModel::new(Rgba8::from_rgb(59, 130, 246))
    .with_alpha(true)
    .with_contrast_reference(Rgba8::from_rgb(18, 18, 18));

// 2. Full color picker widget with flyout
let picker: Element<'_, Message> = color_picker(model.clone())
    .with_id("accent_color_picker")
    .on_change(|live: Live<Rgba8>| Message::ColorScrubbed(live.into_value()))
    .on_commit(|commit: ColorCommit| Message::ColorCommitted(commit.into_value()))
    .on_cancel(|cancel: Cancel<Rgba8>| Message::ColorRolledBack(cancel.into_restored()))
    .on_open_change(|open: Live<bool>| Message::FlyoutToggled(open.into_value()))
    .into();

// 3. Standalone color swatch button trigger
let swatch: Element<'_, Message> = color_swatch_button(&model)
    .with_id("palette_swatch_trigger")
    .on_press(|| Message::SwatchPressed)
    .into();
```

#### Type references
- `Rgba8` - 8-bit RGBA color representation (`0..=255` channels) with hex parsing/formatting and bidirectional conversions to `iced::Color`.
- `Hsva` - Hue, Saturation, Value, and Alpha color representation (`0.0..=360.0` hue, `0.0..=1.0` s/v/a).
- `Hsv` - Type alias for `Hsva` for HSV representations.
- `ColorCommit` - Type alias for `Commit<Rgba8>` linear wrapper emitted when color selection settles.
- `ColorPickerModel` - Declarative state holding current color, HSV representation, hex input buffer, alpha toggle, presets, and rollback snapshot.
- `ColorFlyoutLayout` - Computed geometric layout bounds for the 240px card flyout, well, rails, buttons, and preset swatches.
- `ColorSwatchButton` - Standalone trigger button displaying a 16×16px preview well over checkerboard and uppercase monospace hex label.
- `ColorPicker` - Full live interactive widget combining trigger button and modal anchored card flyout.

#### Return values
- `Rgba8::from_hex` returns `Some(Rgba8)` for valid 6- or 8-digit hex strings (`#RRGGBB`, `#RRGGBBAA`), or `None` on invalid length, non-ASCII, or signed input.
- `ColorPickerModel::set_hex_input` returns `true` if parsed successfully, or `false` (setting `validation_error = Some(ERR_INVALID_HEX)`) on failure.
- `ColorPickerModel::cancel` reverts `value` and `hsv` to `pre_open_snapshot`, closes the flyout, and returns the restored `Rgba8`.
- `ColorPickerModel::commit` closes the flyout, clears the snapshot, and returns the committed `Rgba8`.

#### Side effects
- Opening the flyout records `pre_open_snapshot = Some(self.value)`.
- Escape key presses or clicking Cancel restores `pre_open_snapshot` and closes the flyout.
- Clicking Apply or pressing Enter commits the current color and closes the flyout.
- Scrubbing on the SV canvas or Hue/Alpha rails updates `value`, `hsv`, and `hex_input` synchronously and emits `Live<Rgba8>`.

#### References
- Requirements: `UIF-ATOM-012`, `UIF-TOK-006`, `UIF-PRIN-001`, `UIF-PRIN-005`
- Source: [`color_picker.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/color_picker.rs)

---

### `widget::slider` and `widget::slider_host` - continuous value slider (`UIF-ATOM-005`)

#### Overview
The `widget::slider` and `widget::slider_host` modules provide a headless model/host + style resolver (`SliderHost`, `RangeModel`, `SliderStyle`, and `resolve_style`) rather than a live `iced` widget (`Element<'a, Message>`). Together they implement §3.5 (`UIF-ATOM-005`) for continuous and discrete value scrubbing, providing drift-free arithmetic, 4px rail / 16px thumb geometry, and linear three-tier gesture emissions (`Live`, `Commit`, `Cancel`).

#### Prerequisites & Requirements
- Applications **MUST** pass a valid `RangeModel` reference into `SliderHost` during gesture processing (`press`, `drag`, `release`, `key`, `blur`).
- Scrub updates during dragging **MUST** emit `SliderEmission::Changed` carrying `Live<f64>`, whereas expensive persistence side effects **MUST NOT** trigger until `SliderEmission::Committed` carrying `Commit<f64>` is produced upon pointer release.
- When the user presses `Escape` during an active scrub, the host **MUST** emit `SliderEmission::Cancelled` carrying `Cancel<f64>` with the pre-drag snapshot, and the caller **MUST** restore this pre-drag value.
- When `RangeModel::disabled_flag()` is `true`, all gesture inputs **MUST** return `SliderOutcome::noop()`.

#### Syntax / Method Signature
```rust
// widget::range_model (§3.5, UIF-ATOM-005)
pub const PAGE_STEP_MULTIPLIER: i64 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RangeKey {
    Decrease, // Left / Down (-1 step)
    Increase, // Right / Up (+1 step)
    PageDown, // -10 steps
    PageUp,   // +10 steps
    Home,     // min
    End,      // max
}

#[derive(Clone, Debug, PartialEq)]
pub struct RangeModel { /* value, min, max, step, precision, unit_suffix, validation, disabled */ }

/// Semantic alias for range model when backing a slider control (§3.5, UIF-ATOM-005).
pub type SliderModel = RangeModel;

impl RangeModel {
    #[must_use]
    pub fn new(value: f64, min: f64, max: f64) -> Self;
    #[must_use]
    pub fn with_step(mut self, step: Option<f64>) -> Self;
    #[must_use]
    pub fn with_precision(mut self, precision: u8) -> Self;
    #[must_use]
    pub fn with_unit_suffix(mut self, unit: impl Into<String>) -> Self;
    #[must_use]
    pub fn with_validation(mut self, message: impl Into<String>) -> Self;
    #[must_use]
    pub fn with_disabled(mut self, disabled: bool) -> Self;
    #[must_use]
    pub const fn value(&self) -> f64;
    #[must_use]
    pub const fn min(&self) -> f64;
    #[must_use]
    pub const fn max(&self) -> f64;
    #[must_use]
    pub const fn step(&self) -> Option<f64>;
    #[must_use]
    pub const fn precision(&self) -> u8;
    #[must_use]
    pub fn unit_suffix(&self) -> Option<&str>;
    #[must_use]
    pub fn validation(&self) -> Option<&str>;
    #[must_use]
    pub const fn disabled_flag(&self) -> bool;
    #[must_use]
    pub fn ordered_bounds(&self) -> (f64, f64);
    #[must_use]
    pub fn low(&self) -> f64;
    #[must_use]
    pub fn high(&self) -> f64;
    #[must_use]
    pub fn is_inverted(&self) -> bool;
    #[must_use]
    pub fn is_degenerate(&self) -> bool;
    #[must_use]
    pub fn clamp(&self, value: f64) -> f64;
    #[must_use]
    pub fn clamped_value(&self) -> f64;
    #[must_use]
    pub fn effective_step(&self) -> Option<f64>;
    #[must_use]
    pub fn snap(&self, value: f64) -> f64;
    #[must_use]
    pub fn step_by(&self, value: f64, delta: i64) -> Option<f64>;
    #[must_use]
    pub fn apply_key(&self, key: RangeKey) -> Option<f64>;
    #[must_use]
    pub fn fraction(&self) -> f32;
    #[must_use]
    pub fn value_at_fraction(&self, fraction: f32) -> f64;
    #[must_use]
    pub fn display(&self) -> String;
    #[must_use]
    pub fn parse(&self, text: &str) -> Option<f64>;
}

// widget::slider_host (§3.5, UIF-ATOM-005)
#[derive(Clone)]
pub enum SliderEmission {
    /// The scrub moved - `on_change(f64)`, the `Live` tier.
    Changed(Live<f64>),
    /// The value settled - `on_commit(f64)`, the `Commit` tier.
    Committed(Commit<f64>),
    /// The gesture was abandoned - `Escape`, the `Cancel` tier, carrying pre-drag value.
    Cancelled(Cancel<f64>),
}

impl SliderEmission {
    #[must_use]
    pub fn is_value_change(&self) -> bool;
    #[must_use]
    pub const fn tier(&self) -> &'static str;
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SliderOutcome {
    pub emissions: Vec<SliderEmission>,
}

impl SliderOutcome {
    #[must_use]
    pub fn noop() -> Self;
    #[must_use]
    pub fn emissions(emissions: Vec<SliderEmission>) -> Self;
    #[must_use]
    pub fn is_noop(&self) -> bool;
}

#[derive(Debug)]
pub struct SliderHost { /* private fields */ }

impl SliderHost {
    #[must_use]
    pub fn new() -> Self;
    #[must_use]
    pub fn is_open(&self) -> bool;
    #[must_use]
    pub const fn is_capturing(&self) -> bool;
    #[must_use]
    pub const fn hovered(&self) -> bool;
    #[must_use]
    pub const fn pressed(&self) -> bool;
    #[must_use]
    pub fn snapshot(&self) -> Option<f64>;
    #[must_use]
    pub const fn current(&self) -> Option<f64>;
    #[must_use]
    pub const fn live_ticks(&self) -> usize;
    #[must_use]
    pub const fn committed(&self) -> bool;
    #[must_use]
    pub const fn cancelled(&self) -> bool;
    #[must_use]
    pub const fn settled(&self) -> bool;
    #[must_use]
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

// widget::slider (paint & geometry resolvers) (§3.5, UIF-ATOM-005)
pub const TRACK_HEIGHT: f32 = size::SLIDER_TRACK_HEIGHT; // 4.0px
pub const THUMB_SIZE: f32 = size::SLIDER_THUMB; // 16.0px
pub const THUMB_HIT_SIZE: f32 = size::HIT_TARGET_MIN; // 24.0px
pub const THUMB_RING_WIDTH: f32 = shape::BORDER_FOCUS; // 2.0px

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliderStyle {
    pub rail_unfilled: Color,
    pub rail_border: Color,
    pub rail_filled: Color,
    pub thumb_fill: Color,
    pub thumb_border: Color,
    pub thumb_border_width: f32,
    pub tick: Color,
    pub focus_ring_color: Color,
    pub focus_ring_width: f32,
}

#[must_use]
pub fn resolve_style(state: VisualState, mount: Color, palette: &Palette) -> SliderStyle;
#[must_use]
pub fn track_rect(bounds: Rectangle) -> Rectangle;
#[must_use]
pub fn thumb_travel(track: Rectangle) -> f32;
#[must_use]
pub fn thumb_center_x(track: Rectangle, fraction: f32) -> f32;
#[must_use]
pub fn fraction_at_x(track: Rectangle, x: f32) -> f32;
#[must_use]
pub fn thumb_rect(track: Rectangle, fraction: f32) -> Rectangle;
#[must_use]
pub fn hit_rect(track: Rectangle, fraction: f32) -> Rectangle;
#[must_use]
pub fn tick_fractions(count: usize) -> Vec<f32>;
#[must_use]
pub fn rail_layer(track: Rectangle, style: &SliderStyle) -> Layer;
#[must_use]
pub fn filled_layer(track: Rectangle, style: &SliderStyle, fraction: f32) -> Option<Layer>;
#[must_use]
pub fn thumb_layer(track: Rectangle, style: &SliderStyle, fraction: f32) -> Layer;
#[must_use]
pub fn tick_layers(track: Rectangle, style: &SliderStyle, count: usize) -> Vec<Layer>;
#[must_use]
pub fn focus_ring_layer(track: Rectangle, style: &SliderStyle, fraction: f32) -> Option<Layer>;
```

#### Examples
```rust
use hoarfrost::binding::{Cancel, Commit, Live};
use hoarfrost::widget::range_model::{RangeKey, RangeModel};
use hoarfrost::widget::slider_host::{SliderEmission, SliderHost, SliderOutcome};

let mut model = RangeModel::new(50.0, 0.0, 100.0)
    .with_step(Some(5.0))
    .with_precision(1)
    .with_unit_suffix("%");

let mut host = SliderHost::new();

// 1. Pointer press down at 75% fraction along the rail
let press_outcome = host.press(&model, 0.75);

// 2. Drag scrub along the track
let drag_outcome = host.drag(&model, 0.82);

// 3. Process emissions using .into_value() / .into_restored() scalar unpackers
for e in drag_outcome.emissions {
    match e {
        SliderEmission::Changed(live) => {
            let preview_f64: f64 = live.into_value();
            let preview_f32: f32 = preview_f64 as f32;
            println!("Live preview: {preview_f32}");
        }
        SliderEmission::Committed(commit) => {
            let settled_f64: f64 = commit.into_value();
            let settled_f32: f32 = settled_f64 as f32;
            println!("Settled value: {settled_f32}");
        }
        SliderEmission::Cancelled(cancel) => {
            let restored_f64: f64 = cancel.into_restored();
            let restored_f32: f32 = restored_f64 as f32;
            println!("Restored pre-drag value: {restored_f32}");
        }
    }
}

// 4. Release gesture to settle
let release_outcome = host.release(&model);
for e in release_outcome.emissions {
    if let SliderEmission::Committed(commit) = e {
        let final_value = commit.into_value();
        model = RangeModel::new(final_value, model.min(), model.max());
    }
}
```

#### Type references
- `RangeModel` (aliased as `SliderModel`) - Shared continuous interval model holding value, bounds `[min, max]`, step size, precision, and validation text.
- `RangeKey` - Keyboard commands (`Decrease`, `Increase`, `PageDown`, `PageUp`, `Home`, `End`).
- `SliderEmission` - Three-tier interaction emission (`Changed(Live<f64>)`, `Committed(Commit<f64>)`, `Cancelled(Cancel<f64>)`).
- `SliderOutcome` - Result container wrapping ordered `Vec<SliderEmission>`.
- `SliderHost` - Pure gesture state machine managing scrub capture, hover, press, drag, key stepping, and rollback snapshots.
- `SliderStyle` - Paint descriptor holding tokens for rail, fill, thumb, tick marks, and focus rings.

#### Return values
- `SliderHost::press`, `drag`, `release`, `cancel`, `blur`, and `key` return `SliderOutcome`.
- `RangeModel::clamped_value` returns `value` clamped within ordered `[min, max]`.
- `RangeModel::fraction` returns the thumb position in `0.0..=1.0` travel.
- `RangeModel::step_by` returns re-derived stepped values avoiding floating point drift.

#### Side effects
- `SliderHost::press` records `captured_value = Some(before)` before moving the value.
- `SliderHost::release` emits `Committed` and clears capture.
- `SliderHost::cancel` emits `Cancelled` with the pre-drag snapshot and resets the gesture slot.

#### References
- Requirements: `UIF-ATOM-005`, `UIF-TOK-002`, `UIF-TOK-006`, `UIF-PRIN-005`
- Source: [`slider.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/slider.rs), [`slider_host.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/slider_host.rs), [`range_model.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/range_model.rs)

---

### `widget::range_slider` and `widget::range_slider_model` - dual-thumb range slider (`UIF-ATOM-005b`)

#### Overview
The `widget::range_slider` module provides a live interactive `iced` widget (`Element<'a, Message>`) via `RangeSlider` (and `range_slider`), backed by `RangeSliderModel` from `widget::range_slider_model`. It implements §3.5 (`UIF-ATOM-005b`, `UIF-ATOM-005`) for dual-thumb interval selection with non-crossing invariant enforcement (`lower <= upper - min_span`), drift-free keyboard stepping, and three-tier gesture lifecycle callbacks.

#### Prerequisites & Requirements
- The model **MUST** maintain the invariant `lower <= upper - min_span`; callers **MUST NOT** initialize or update bounds such that thumbs cross.
- When embedding in focus trees, callers **MUST** configure a base `FocusId` via `with_focus_id` so that `.lower` and `.upper` sub-thumb focus endpoints are deterministically registered.
- Scrub listeners **MUST** handle `on_change` for visual scrub previews and **MUST NOT** trigger persistent mutations or remote requests until `on_commit` fires.
- Escape key presses during an active drag **MUST** trigger `on_cancel` restoring pre-gesture thumb positions.

#### Syntax / Method Signature
```rust
// widget::range_slider_model (§3.5, UIF-ATOM-005b)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RangeThumb {
    Lower,
    Upper,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RangeSliderModel {
    pub lower: f64,
    pub upper: f64,
    pub min: f64,
    pub max: f64,
    pub min_span: f64,
    pub step: Option<f64>,
    pub precision: u8,
    pub unit_suffix: Option<String>,
    pub validation: Option<String>,
    pub disabled: bool,
}

impl RangeSliderModel {
    #[must_use]
    pub fn new(min: f64, max: f64, lower: f64, upper: f64) -> Self;
    #[must_use]
    pub fn with_min_span(mut self, min_span: f64) -> Self;
    #[must_use]
    pub fn with_step(mut self, step: Option<f64>) -> Self;
    #[must_use]
    pub fn with_precision(mut self, precision: u8) -> Self;
    #[must_use]
    pub fn with_unit_suffix(mut self, unit: impl Into<String>) -> Self;
    #[must_use]
    pub fn with_validation(mut self, message: impl Into<String>) -> Self;
    #[must_use]
    pub fn with_disabled(mut self, disabled: bool) -> Self;
    #[must_use]
    pub fn low(&self) -> f64;
    #[must_use]
    pub fn high(&self) -> f64;
    #[must_use]
    pub fn ordered_bounds(&self) -> (f64, f64);
    #[must_use]
    pub fn effective_min_span(&self) -> f64;
    #[must_use]
    pub fn effective_step(&self) -> Option<f64>;
    #[must_use]
    pub fn clamp_lower(&self, val: f64) -> f64;
    #[must_use]
    pub fn clamp_upper(&self, val: f64) -> f64;
    #[must_use]
    pub fn clamped_range(&self) -> (f64, f64);
    #[must_use]
    pub fn snap(&self, value: f64) -> f64;
    #[must_use]
    pub fn lower_fraction(&self) -> f32;
    #[must_use]
    pub fn upper_fraction(&self) -> f32;
    #[must_use]
    pub fn value_at_fraction(&self, fraction: f32) -> f64;
    #[must_use]
    pub fn disambiguate_thumb(&self, click_fraction: f32) -> RangeThumb;
    #[must_use]
    pub fn step_thumb(&self, thumb: RangeThumb, key: RangeKey) -> (f64, f64);
    #[must_use]
    pub fn format_value(&self, val: f64) -> String;
    #[must_use]
    pub fn display(&self) -> (String, String);
}

// widget::range_slider (§3.5, UIF-ATOM-005b)
pub type RangeChange<'a, Message> = Box<dyn Fn(Live<(f64, f64)>) -> Option<Message> + 'a>;
pub type RangeCommit<'a, Message> = Box<dyn Fn(Commit<(f64, f64)>) -> Option<Message> + 'a>;
pub type RangeCancel<'a, Message> = Box<dyn Fn(Cancel<(f64, f64)>) -> Option<Message> + 'a>;

pub struct RangeSlider<'a, Message> { /* private fields */ }

impl<'a, Message> RangeSlider<'a, Message> {
    #[must_use]
    pub fn new(model: RangeSliderModel) -> Self;

    #[must_use]
    pub fn controlled(
        model: Controlled<'a, RangeSliderModel>,
        focus_id: impl Into<FocusId>,
        emit: impl Fn(Commit<(f64, f64)>) -> Option<Message> + 'a,
    ) -> Self;

    #[must_use]
    pub fn with_focus_id(mut self, focus_id: impl Into<FocusId>) -> Self;

    #[must_use]
    pub fn with_palette(mut self, palette: Palette) -> Self;

    #[must_use]
    pub fn on_change<F>(mut self, on_change: F) -> Self
    where
        F: Fn(Live<(f64, f64)>) -> Option<Message> + 'a;

    #[must_use]
    pub fn on_commit<F>(mut self, on_commit: F) -> Self
    where
        F: Fn(Commit<(f64, f64)>) -> Option<Message> + 'a;

    #[must_use]
    pub fn on_cancel<F>(mut self, on_cancel: F) -> Self
    where
        F: Fn(Cancel<(f64, f64)>) -> Option<Message> + 'a;

    #[must_use]
    pub fn model(&self) -> &RangeSliderModel;

    #[must_use]
    pub fn palette(&self) -> &Palette;

    #[must_use]
    pub fn focus_id(&self) -> &FocusId;
}

#[must_use]
pub fn range_slider<'a, Message>(model: RangeSliderModel) -> RangeSlider<'a, Message>;
```

#### Examples
```rust
use hoarfrost::binding::{Cancel, Commit, Live};
use hoarfrost::widget::range_slider::{RangeSlider, range_slider};
use hoarfrost::widget::range_slider_model::RangeSliderModel;
use iced::Element;

#[derive(Clone, Debug)]
enum Message {
    IntervalScrubbing((f64, f64)),
    IntervalCommitted((f64, f64)),
    IntervalRestored((f64, f64)),
}

let model = RangeSliderModel::new(0.0, 100.0, 20.0, 80.0)
    .with_min_span(10.0)
    .with_step(Some(1.0))
    .with_unit_suffix("ms");

let slider: Element<'_, Message> = RangeSlider::new(model)
    .with_focus_id("playback_range")
    .on_change(|live: Live<(f64, f64)>| Some(Message::IntervalScrubbing(live.into_value())))
    .on_commit(|commit: Commit<(f64, f64)>| Some(Message::IntervalCommitted(commit.into_value())))
    .on_cancel(|cancel: Cancel<(f64, f64)>| Some(Message::IntervalRestored(cancel.into_restored())))
    .into();
```

#### Type references
- `RangeThumb` - Target thumb enum (`Lower`, `Upper`) identifying the active drag or focus point.
- `RangeSliderModel` - Declarative model holding interval bounds, current lower/upper values, `min_span`, step, and precision.
- `RangeSlider` - Live interactive `iced` widget rendering the dual-thumb track, connecting keyboard focus and mouse gestures to three-tier callbacks.
- `RangeChange`, `RangeCommit`, `RangeCancel` - Boxed callback types taking `Live<(f64, f64)>`, `Commit<(f64, f64)>`, and `Cancel<(f64, f64)>`.

#### Return values
- `RangeSliderModel::clamped_range` returns `(lower, upper)` guaranteed to satisfy bounds and `min_span`.
- `RangeSliderModel::disambiguate_thumb` returns which thumb targets a click based on proximity.
- `RangeSliderModel::display` returns formatted `(lower_str, upper_str)` value readouts.

#### Side effects
- Dragging updates internal `RangeSliderState` positions and emits `on_change`.
- Pointer release settles the gesture and emits `on_commit`.
- Pressing Escape reverts both thumb positions to the pre-drag snapshot and emits `on_cancel`.

#### References
- Requirements: `UIF-ATOM-005b`, `UIF-ATOM-005`, `UIF-PRIN-005`
- Source: [`range_slider.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/range_slider.rs), [`range_slider_model.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/range_slider_model.rs)

---

### `widget::rating` - star / step rating (`UIF-ATOM-013`)

#### Overview
The `widget::rating` module provides a live interactive `iced` widget (`Element<'a, Message>`) via `RatingControl` (constructed via `rating_control`), backed by the declarative data model `RatingModel` and pure layout resolvers. It implements §3.13 (`UIF-ATOM-013`) for star rating rows supporting whole-step and half-step granularity, live hover previews, clear-on-reclick, and three-tier callbacks (`Live`, `Commit`, `Cancel`).

#### Prerequisites & Requirements
- If a rating control is marked read-only via `with_read_only(true)`, it **MUST NOT** accept keyboard focus, hover previews, or mouse click interactions.
- When `allow_clear` is set to `true`, clicking the currently active star rating or pressing `Delete`/`Backspace` **MUST** clear the rating value to `None`.
- Applications **MUST NOT** commit rating changes during hover preview; only explicit mouse release or `Enter`/`Space` key presses **MUST** fire `on_commit`.

#### Syntax / Method Signature
```rust
pub const STAR_FILLED: &str = "★";
pub const STAR_EMPTY: &str = "☆";
pub const DEFAULT_MAX_RATING: u8 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum RatingStep {
    Half,
    #[default]
    Whole,
}

impl RatingStep {
    #[must_use]
    pub const fn increment(self) -> f32;
}

#[derive(Clone, Debug, PartialEq)]
pub struct RatingModel {
    pub value: Option<f32>,
    pub hover_preview: Option<f32>,
    pub max: u8,
    pub step: RatingStep,
    pub allow_clear: bool,
    pub read_only: bool,
    pub disabled: bool,
}

impl RatingModel {
    #[must_use]
    pub fn new(max: u8) -> Self;
    #[must_use]
    pub fn with_value(mut self, value: impl Into<Option<f32>>) -> Self;
    #[must_use]
    pub fn with_hover_preview(mut self, hover_preview: impl Into<Option<f32>>) -> Self;
    #[must_use]
    pub fn with_step(mut self, step: RatingStep) -> Self;
    #[must_use]
    pub fn with_allow_clear(mut self, allow_clear: bool) -> Self;
    #[must_use]
    pub fn with_read_only(mut self, read_only: bool) -> Self;
    #[must_use]
    pub fn with_disabled(mut self, disabled: bool) -> Self;
    #[must_use]
    pub fn effective_value(&self) -> Option<f32>;
    #[must_use]
    pub fn total_width(&self) -> f32;
}

// Geometry resolvers (§3.13, UIF-ATOM-013)
#[must_use]
pub fn total_rating_width(max: u8) -> f32;
#[must_use]
pub fn star_slot_bounds(bounds: Rectangle, slot_index: usize) -> Rectangle;
#[must_use]
pub fn value_at_position(
    bounds: Rectangle,
    pos: Point,
    max: u8,
    step: RatingStep,
) -> Option<(usize, f32)>;

// Live Widget (§3.13, UIF-ATOM-013)
pub struct RatingControl<'a, Message> { /* private fields */ }

/// Semantic alias for RatingControl widget (§3.13, UIF-ATOM-013).
pub type RatingWidget<'a, Message> = RatingControl<'a, Message>;

impl<'a, Message> RatingControl<'a, Message> {
    #[must_use]
    pub fn new(model: RatingModel) -> Self;
    #[must_use]
    pub fn on_hover_preview(
        mut self,
        callback: impl Fn(Live<Option<f32>>) -> Message + 'a,
    ) -> Self;
    #[must_use]
    pub fn on_commit(mut self, callback: impl Fn(Commit<Option<f32>>) -> Message + 'a) -> Self;
    #[must_use]
    pub fn on_cancel(mut self, callback: impl Fn(Cancel<Option<f32>>) -> Message + 'a) -> Self;
    #[must_use]
    pub fn palette(mut self, palette: Palette) -> Self;
    #[must_use]
    pub fn id(mut self, id: iced::widget::Id) -> Self;
    #[must_use]
    pub fn widget_id(&self) -> &iced::widget::Id;
    #[must_use]
    pub fn size(&self) -> Size<Length>;
    #[must_use]
    pub fn model(&self) -> &RatingModel;
}

#[must_use]
pub fn rating_control<'a, Message>(model: RatingModel) -> RatingControl<'a, Message>;
```

#### Examples
```rust
use hoarfrost::binding::{Cancel, Commit, Live};
use hoarfrost::widget::rating::{RatingModel, RatingStep, rating_control};
use iced::Element;

#[derive(Clone, Debug)]
enum Message {
    HoveredRating(Option<f32>),
    RatingCommitted(Option<f32>),
    RatingCancelled(Option<f32>),
}

let model = RatingModel::new(5)
    .with_value(Some(4.0))
    .with_step(RatingStep::Half)
    .with_allow_clear(true);

let control: Element<'_, Message> = rating_control(model)
    .on_hover_preview(|live: Live<Option<f32>>| Message::HoveredRating(live.into_value()))
    .on_commit(|commit: Commit<Option<f32>>| Message::RatingCommitted(commit.into_value()))
    .on_cancel(|cancel: Cancel<Option<f32>>| Message::RatingCancelled(cancel.into_restored()))
    .into();
```

#### Type references
- `RatingStep` - Stepping granularity (`Half` for 0.5 increments, `Whole` for 1.0 increments).
- `RatingModel` - Declarative model configuration holding star count, current value, hover preview, step mode, and flags.
- `RatingControl` (aliased as `RatingWidget`) - Live interactive widget displaying star glyphs and handling hover/press/key events.
- `star_slot_bounds` - Geometry resolver computing the bounding rectangle of an individual star slot.
- `value_at_position` - Geometry resolver converting pointer coordinates to rating step values.

#### Return values
- `RatingModel::effective_value` returns `hover_preview` if active, otherwise the settled `value`.
- `value_at_position` returns `Some((slot_index, rating_value))` within bounds or `None` outside.

#### Side effects
- Hovering across star slots updates internal hover preview state and fires `on_hover_preview`.
- Clicking or pressing Enter commits the selected rating and emits `on_commit`.
- Pressing Escape clears the active hover preview, restores the pre-gesture value, and emits `on_cancel`.

#### References
- Requirements: `UIF-ATOM-013`, `UIF-TOK-002`, `UIF-PRIN-005`
- Source: [`rating.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/rating.rs)

---

### `widget::segmented` and `widget::segmented_model` - segmented button strip (`UIF-ATOM-001b`)

#### Overview
The `widget::segmented` module provides a live interactive `iced` widget (`Element<'a, Message>`) through `SegmentedButton`, backed by `SegmentedModel` and style/geometry resolvers from `widget::segmented_model`. It implements §3.1b (`UIF-ATOM-001b`) for cohesive segmented button strips and toggle buttons, supporting single- and multi-selection, roving tab stop keyboard navigation (`UIF-PRIN-005`), and adaptive perimeter border styling.

#### Prerequisites & Requirements
- A segmented button strip **MUST** present exactly one tab stop to the focus system, using arrow keys (`Left`/`Right` or `Up`/`Down`) to rove focus internally via `SegmentedModel::rove`.
- When building a toggle button, callers **MUST** invoke `SegmentedModel::toggle_button(on)` on `impl SegmentedModel<&'static str>` with segment ID `TOGGLE_SEGMENT_ID` (`"toggle"`).
- Toggling a segment **MUST** emit `Commit<BTreeSet<Id>>` containing the complete new selection set, and callers **MUST NOT** assume delta emissions.
- When `disabled()` is called on the model, all segments **MUST** be rendered in the disabled visual state and reject input.

#### Syntax / Method Signature
```rust
// widget::segmented_model (§3.1b, UIF-ATOM-001b)
pub const TOGGLE_SEGMENT_ID: &str = "toggle";
pub const TOGGLE_DEFAULT_LABEL: &str = "Toggle";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum SelectionMode {
    #[default]
    SingleSelect,
    MultiSelect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoveDirection {
    Forward,
    Backward,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentItem<Id> {
    pub id: Id,
    pub label: String,
    pub enabled: bool,
}

impl<Id> SegmentItem<Id> {
    #[must_use]
    pub fn new(id: Id, label: impl Into<String>) -> Self;
    #[must_use]
    pub fn disabled(mut self) -> Self;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentedModel<Id> {
    pub segments: Vec<SegmentItem<Id>>,
    pub selected: BTreeSet<Id>,
    pub mode: SelectionMode,
    pub density: ButtonDensity,
    pub disabled: bool,
}

impl<Id> SegmentedModel<Id> {
    #[must_use]
    pub fn new(segments: Vec<SegmentItem<Id>>) -> Self;
    #[must_use]
    pub fn multi(segments: Vec<SegmentItem<Id>>) -> Self;
    #[must_use]
    pub fn with_selected(mut self, id: Id) -> Self;
    #[must_use]
    pub fn with_selection(mut self, selected: BTreeSet<Id>) -> Self;
    #[must_use]
    pub fn with_mode(mut self, mode: SelectionMode) -> Self;
    #[must_use]
    pub fn with_density(mut self, density: ButtonDensity) -> Self;
    #[must_use]
    pub fn disabled(mut self) -> Self;
    #[must_use]
    pub fn segment_enabled(&self, index: usize) -> bool;
    #[must_use]
    pub fn enabled_indices(&self) -> Vec<usize>;
    #[must_use]
    pub fn index_of(&self, id: &Id) -> Option<usize>;
    #[must_use]
    pub fn is_selected(&self, id: &Id) -> bool;
    #[must_use]
    pub fn selected_count(&self) -> usize;
}

impl SegmentedModel<&'static str> {
    /// Construct a one-segment toggle button strip (§3.1b, UIF-ATOM-001b).
    #[must_use]
    pub fn toggle_button(on: bool) -> Self;
}

impl<Id: PartialEq> SegmentedModel<Id> {
    /// Whether a toggle button strip is in its on position (§3.1b, UIF-ATOM-001b).
    #[must_use]
    pub fn is_on(&self) -> bool;
}

impl<Id: Clone + Ord> SegmentedModel<Id> {
    #[must_use]
    pub fn rove(&self, from: Option<usize>, direction: RoveDirection) -> Option<usize>;
    #[must_use]
    pub fn toggle(&self, id: &Id) -> Option<Commit<BTreeSet<Id>>>;
    #[must_use]
    pub fn toggle_index(&self, index: usize) -> Option<Commit<BTreeSet<Id>>>;
}

// widget::segmented (paint & geometry resolvers) (§3.1b, UIF-ATOM-001b)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentPolarity {
    Unselected,
    Selected,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SegmentStyle {
    pub background: Color,
    pub foreground: Color,
    pub border_color: Color,
    pub border_width: f32,
    pub focus_ring_color: Color,
    pub focus_ring_width: f32,
    pub radius: iced::border::Radius,
}

#[must_use]
pub fn resolve_style(
    polarity: SegmentPolarity,
    state: VisualState,
    palette: &Palette,
) -> SegmentStyle;
#[must_use]
pub fn resolve_strip_ring(
    state: VisualState,
    mount: Color,
    palette: &Palette,
) -> Option<(Color, f32)>;
#[must_use]
pub fn segment_width(label_width: f32) -> f32;
#[must_use]
pub fn strip_width(label_widths: &[f32]) -> f32;
#[must_use]
pub fn segment_bounds(strip: Rectangle, index: usize, label_widths: &[f32]) -> Rectangle;
#[must_use]
pub fn check_slot(segment: Rectangle) -> Rectangle;
#[must_use]
pub fn label_bounds(segment: Rectangle) -> Rectangle;
#[must_use]
pub fn perimeter_class(style: &SegmentStyle, palette: &Palette) -> ChromeClass;
#[must_use]
pub fn segment_radius(index: usize, count: usize) -> iced::border::Radius;

// Live SegmentedButton Widget (§3.1b, UIF-ATOM-001b)
pub struct SegmentedButton<'a, Message, Id = String> { /* private fields */ }

impl<'a, Message, Id> SegmentedButton<'a, Message, Id> {
    #[must_use]
    pub fn controlled(
        model: Controlled<'a, SegmentedModel<Id>>,
        focus_id: impl Into<FocusId>,
        emit: impl Fn(BTreeSet<Id>) -> Option<Message> + 'a,
    ) -> Self;
    #[must_use]
    pub fn palette(mut self, palette: Palette) -> Self;
    #[must_use]
    pub fn with_palette(self, palette: Palette) -> Self;
    #[must_use]
    pub fn focus_id(&self) -> &FocusId;
    #[must_use]
    pub fn widget_id(&self) -> &WidgetId;
    #[must_use]
    pub fn model(&self) -> &SegmentedModel<Id>;
    #[must_use]
    pub fn palette_ref(&self) -> &Palette;
}
```

#### Examples
```rust
use std::collections::BTreeSet;
use hoarfrost::binding::controlled::Controlled;
use hoarfrost::widget::segmented::SegmentedButton;
use hoarfrost::widget::segmented_model::{SegmentItem, SegmentedModel};
use iced::Element;

#[derive(Clone, Debug)]
enum Message {
    ViewModeSelected(BTreeSet<String>),
    ToggleChanged(bool),
}

// 1. Multi-segment strip
let segments = vec![
    SegmentItem::new("grid".to_string(), "Grid View"),
    SegmentItem::new("list".to_string(), "List View"),
    SegmentItem::new("compact".to_string(), "Compact"),
];
let strip_model = SegmentedModel::new(segments).with_selected("grid".to_string());
let controlled_strip = Controlled::new(&strip_model);

let strip_widget: Element<'_, Message> = SegmentedButton::controlled(
    controlled_strip,
    "view_mode_strip",
    |selected| Some(Message::ViewModeSelected(selected)),
)
.into();

// 2. Degenerate ToggleButton
let toggle_model = SegmentedModel::toggle_button(true);
assert!(toggle_model.is_on());
```

#### Type references
- `SegmentItem<Id>` - One segment entry holding id, label, and individual enabled flag.
- `SegmentedModel<Id>` - Declarative model managing segments, selection set `BTreeSet<Id>`, selection mode, and roving index.
- `SelectionMode` - SingleSelect or MultiSelect operational mode.
- `SegmentPolarity` - Unselected vs Selected style input.
- `SegmentedButton` - Live `iced` widget rendering cohesive segments with roving tab stop keyboard focus.
- `SegmentStyle` - Resolved colors and corner radii for one segment slot.

#### Return values
- `SegmentedModel::toggle` returns `Some(Commit<BTreeSet<Id>>)` containing the entire new selection set, or `None` if disabled.
- `SegmentedModel::rove` returns the next enabled segment index in the requested direction, wrapping at ends.
- `SegmentedModel::is_on` returns `true` if `TOGGLE_SEGMENT_ID` is present in the selection.

#### Side effects
- Arrow key presses update the focused segment index within `SegmentedState`.
- Segment activation dispatches a callback with the full updated `BTreeSet<Id>`.

#### References
- Requirements: `UIF-ATOM-001b`, `UIF-PRIN-005`
- Source: [`segmented.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/segmented.rs), [`segmented_model.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/segmented_model.rs)

---

### `widget::split_button` - primary action with flyout trigger (`UIF-ATOM-001c`)

#### Overview
The `widget::split_button` module provides a live `iced` widget (`Element<'a, Message>`) via `SplitButton`, backed by `SplitButtonModel`, `SplitButtonHost`, and pure layout/paint resolvers. It implements §3.1c (`UIF-ATOM-001c`) pairing a primary push action with an anchored dropdown flyout menu, preserving independent hit zones, single-flight action gating, and five-variant settlement emissions.

#### Prerequisites & Requirements
- While the split button's flyout menu is open, the menu card **MUST** be registered as an active surface (`SurfaceKind::MenuCard`) with the focus manager.
- When the user presses `Escape` while the flyout menu is open, the control **MUST** close the menu without triggering primary action cancellation.
- When driving the control headlessly via `SplitButtonHost`, callers **MUST** call `SplitButtonHost::observe(&model)` once per frame to maintain synchronization with action gating states.
- If the primary half is configured as `busy`, subsequent primary clicks **MUST NOT** dispatch a second command until settled.

#### Syntax / Method Signature
```rust
// widget::split_button_model (§3.1c, UIF-ATOM-001c)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitMenuRow<ActionId> {
    pub id: ActionId,
    pub label: String,
    pub shortcut: Option<String>,
}

impl<ActionId> SplitMenuRow<ActionId> {
    #[must_use]
    pub fn new(id: ActionId, label: impl Into<String>) -> Self;
    #[must_use]
    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self;
    #[must_use]
    pub fn to_row(&self) -> Row;
}

#[derive(Clone, Debug, PartialEq)]
pub struct SplitButtonModel<ActionId> { /* label, menu_rows, leading_icon, variant, density, busy, disabled, cancellable, tooltip */ }

impl<ActionId: Clone + fmt::Display> SplitButtonModel<ActionId> {
    #[must_use]
    pub fn new(label: impl Into<String>, menu_rows: Vec<SplitMenuRow<ActionId>>) -> Self;
    #[must_use]
    pub fn primary(label: impl Into<String>, menu_rows: Vec<SplitMenuRow<ActionId>>) -> Self;
    #[must_use]
    pub fn with_menu_id(mut self, id: impl Into<String>) -> Self;
    #[must_use]
    pub fn with_menu_rows(mut self, menu_rows: Vec<SplitMenuRow<ActionId>>) -> Self;
    #[must_use]
    pub fn with_leading_icon(mut self, icon: ButtonIcon) -> Self;
    #[must_use]
    pub fn with_density(mut self, density: ButtonDensity) -> Self;
    #[must_use]
    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self;
    #[must_use]
    pub fn busy(mut self) -> Self;
    #[must_use]
    pub fn disabled(mut self) -> Self;
    #[must_use]
    pub fn cancellable(mut self) -> Self;
    #[must_use]
    pub fn label(&self) -> &str;
    #[must_use]
    pub fn variant(&self) -> ButtonVariant;
    #[must_use]
    pub fn density(&self) -> ButtonDensity;
    #[must_use]
    pub fn pending(&self) -> ActionPending;
    #[must_use]
    pub fn tracks_action(&self) -> bool;
    #[must_use]
    pub fn has_navigable_menu(&self) -> bool;
    #[must_use]
    pub fn menu(&self) -> Menu;
    #[must_use]
    pub fn action_at(&self, index: usize) -> Option<&ActionId>;
}

// widget::split_button_host (§3.1c, UIF-ATOM-001c)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitZone {
    Primary,
    Menu,
    Outside,
}

#[derive(Clone)]
pub enum SplitButtonEmission<ActionId> {
    PrimaryActivated(Commit<ButtonSettlement>),
    PrimaryCancelled(Cancel<ButtonSettlement>),
    StopRequested(CancelRequest),
    MenuToggled(bool),
    MenuSelected(ActionId),
}

impl<ActionId> SplitButtonEmission<ActionId> {
    #[must_use]
    pub fn settlement(&self) -> Option<&ButtonSettlement>;
    #[must_use]
    pub fn is_primary(&self) -> bool;
}

pub struct SplitButtonOutcome<ActionId> {
    pub emissions: Vec<SplitButtonEmission<ActionId>>,
}

pub struct SplitButtonHost { /* private fields */ }

impl SplitButtonHost {
    #[must_use]
    pub fn new(control: FocusId) -> Self;
    pub fn observe<ActionId: Clone + fmt::Display>(&mut self, model: &SplitButtonModel<ActionId>);
    #[must_use]
    pub fn is_menu_open(&self) -> bool;
    #[must_use]
    pub fn half_state(&self, half: SplitZone, focused: bool) -> VisualState;
    pub fn hover(&mut self, zone: SplitZone);
    pub fn pointer_leave(&mut self);
    pub fn press<ActionId: Clone + fmt::Display>(
        &mut self,
        zone: SplitZone,
        model: &SplitButtonModel<ActionId>,
    ) -> SplitButtonOutcome<ActionId>;
    pub fn release<ActionId: Clone + fmt::Display>(
        &mut self,
        zone: SplitZone,
        model: &SplitButtonModel<ActionId>,
        focus: &mut FocusState,
    ) -> SplitButtonOutcome<ActionId>;
    pub fn activate_primary<ActionId: Clone + fmt::Display>(
        &mut self,
        model: &SplitButtonModel<ActionId>,
    ) -> SplitButtonOutcome<ActionId>;
    pub fn open_menu<ActionId: Clone + fmt::Display>(
        &mut self,
        model: &SplitButtonModel<ActionId>,
        focus: &mut FocusState,
    ) -> SplitButtonOutcome<ActionId>;
    pub fn close_menu<ActionId>(&mut self, focus: &mut FocusState) -> SplitButtonOutcome<ActionId>;
    pub fn menu_key<ActionId: Clone + fmt::Display>(
        &mut self,
        key: MenuKey,
        model: &SplitButtonModel<ActionId>,
        focus: &mut FocusState,
    ) -> SplitButtonOutcome<ActionId>;
    pub fn escape<ActionId: Clone + fmt::Display>(
        &mut self,
        model: &SplitButtonModel<ActionId>,
        focus: &mut FocusState,
    ) -> SplitButtonOutcome<ActionId>;
    #[must_use]
    pub fn settlement(&self) -> Option<&ButtonSettlement>;
    #[must_use]
    pub fn is_primary(&self) -> bool;
    pub fn reset(&mut self);
}

// widget::split_button (paint & geometry resolvers) (§3.1c, UIF-ATOM-001c)
#[must_use]
pub fn divider_x(width: f32) -> f32;
#[must_use]
pub fn zone_at(x: f32, width: f32) -> SplitZone;
#[must_use]
pub fn primary_zone(box_: Rectangle) -> Rectangle;
#[must_use]
pub fn menu_zone(box_: Rectangle) -> Rectangle;
#[must_use]
pub fn divider_bounds(box_: Rectangle) -> Rectangle;
#[must_use]
pub fn control_width(label_width: f32) -> f32;
#[must_use]
pub fn label_bounds(primary: Rectangle) -> Rectangle;
#[must_use]
pub fn chevron_bounds(menu: Rectangle) -> Rectangle;
#[must_use]
pub fn height_for_density(density: ButtonDensity) -> f32;
#[must_use]
pub fn resolve_style(
    variant: ButtonVariant,
    primary_state: VisualState,
    menu_state: VisualState,
    palette: &Palette,
) -> SplitStyle;
#[must_use]
pub fn chevron_color(style: &SplitStyle) -> Color;

// Live SplitButton Widget (§3.1c, UIF-ATOM-001c)
pub struct SplitButton<'a, Message, ActionId = String> { /* private fields */ }

impl<'a, Message, ActionId> SplitButton<'a, Message, ActionId> {
    #[must_use]
    pub fn controlled(
        model: Controlled<'a, SplitButtonModel<ActionId>>,
        focus_id: impl Into<FocusId>,
        emit: impl Fn(SplitButtonEmission<ActionId>) -> Option<Message> + 'a,
    ) -> Self;
    #[must_use]
    pub fn palette(mut self, palette: Palette) -> Self;
    #[must_use]
    pub fn with_palette(self, palette: Palette) -> Self;
    #[must_use]
    pub fn focus_id(&self) -> &FocusId;
    #[must_use]
    pub fn widget_id(&self) -> &iced::advanced::widget::Id;
    #[must_use]
    pub fn model(&self) -> &SplitButtonModel<ActionId>;
    #[must_use]
    pub fn palette_ref(&self) -> &Palette;
}
```

#### Examples
```rust
use hoarfrost::binding::controlled::Controlled;
use hoarfrost::widget::split_button::SplitButton;
use hoarfrost::widget::split_button_host::SplitButtonEmission;
use hoarfrost::widget::split_button_model::{SplitButtonModel, SplitMenuRow};
use iced::Element;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Action {
    Save,
    SaveAs,
    ExportPdf,
}

#[derive(Clone, Debug)]
enum Message {
    PrimarySaved,
    MenuOptionChosen(Action),
    MenuToggled(bool),
}

let rows = vec![
    SplitMenuRow::new(Action::SaveAs, "Save As…").with_shortcut("Ctrl+Shift+S"),
    SplitMenuRow::new(Action::ExportPdf, "Export as PDF"),
];

let model = SplitButtonModel::primary("Save", rows);
let controlled = Controlled::new(&model);

let split: Element<'_, Message> = SplitButton::controlled(
    controlled,
    "save_split_button",
    |emission| match emission {
        SplitButtonEmission::PrimaryActivated(_commit) => Some(Message::PrimarySaved),
        SplitButtonEmission::MenuSelected(action) => Some(Message::MenuOptionChosen(action)),
        SplitButtonEmission::MenuToggled(open) => Some(Message::MenuToggled(open)),
        _ => None,
    },
)
.with_palette(hoarfrost::token::color::Palette::dark())
.into();
```

#### Type references
- `SplitMenuRow<ActionId>` - Flyout menu row carrying action identifier, display label, and optional keyboard shortcut.
- `SplitButtonModel<ActionId>` - Declarative model defining primary action label, density, variant, and dropdown menu rows.
- `SplitZone` - Zone enumeration (`Primary`, `Menu`, `Outside`) resolving pointer click coordinates.
- `SplitButtonEmission<ActionId>` - Five-variant emission enum (`PrimaryActivated`, `PrimaryCancelled`, `StopRequested`, `MenuToggled`, `MenuSelected`).
- `SplitButtonHost` - Headless state machine tracking independent half hover/press states and menu surface registration.
- `SplitButton` - Live `iced` widget rendering the split control with hairline divider and anchored menu flyout.

#### Return values
- `SplitButtonHost::press`, `release`, and `activate_primary` return `SplitButtonOutcome<ActionId>` containing triggered emissions.
- `zone_at` returns `SplitZone::Primary` or `SplitZone::Menu` based on whether the coordinate is left or right of the divider.

#### Side effects
- Clicking the menu trigger opens the flyout menu and pushes `SurfaceKind::MenuCard` to `FocusState`.
- Escape key presses close the menu flyout and restore keyboard focus to the split button.

#### References
- Requirements: `UIF-ATOM-001c`, `UIF-PRIN-001`, `UIF-PRIN-005`
- Source: [`split_button.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/split_button.rs), [`split_button_model.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/split_button_model.rs), [`split_button_host.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/split_button_host.rs)

---

### `widget::badge` - status badges and counter tags (`UIF-ATOM-008`)

#### Overview
The `widget::badge` module provides a headless model (`BadgeModel`) and style resolver (`resolve_style`, `BadgeStyle`) without building an interactive `iced` widget (`Element<'a, Message>`). It implements §3.8 (`UIF-ATOM-008`) for non-interactive status marks including 6px notification dots, numeric count pills (with a 99+ ceiling), and semantic status chips over three distinct tone channels.

#### Prerequisites & Requirements
- A numeric count badge **MUST** use tabular figures and **MUST** clamp numbers greater than `COUNT_CEILING` (99) to the formatted string `"99+"`.
- A status chip **MUST** include a leading status glyph icon (`StatusIcon`), and tone alone **MUST NOT** be relied upon to convey meaning (`UIF-TOK-006`).
- Because badges are non-interactive status marks, badge styles **MUST NOT** define or respond to hover, pressed, or keyboard focus visual states.

#### Syntax / Method Signature
```rust
// Layout & geometry constants (§3.8, UIF-ATOM-008)
pub const DOT_SIZE: f32 = 6.0;
pub const COUNT_PILL_HEIGHT: f32 = 18.0;
pub const COUNT_PILL_MIN_WIDTH: f32 = 18.0;
pub const STATUS_CHIP_HEIGHT: f32 = size::BADGE_CHIP_HEIGHT; // 24.0px
pub const COUNT_CEILING: u32 = 99;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StatusIcon {
    Check,
    Warning,
    Error,
    Info,
    Neutral,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum BadgeTone {
    #[default]
    Neutral,
    Accent,
    Success,
    Warning,
    Danger,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BadgeKind {
    Dot,
    Count { n: u32 },
    Status { icon: StatusIcon, label: &'static str },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BadgeModel { /* kind, tone */ }

impl BadgeModel {
    #[must_use]
    pub fn dot() -> Self;
    #[must_use]
    pub fn count(n: u32) -> Self;
    #[must_use]
    pub fn status(icon: StatusIcon, label: &'static str) -> Self;
    #[must_use]
    pub fn with_tone(mut self, tone: BadgeTone) -> Self;
    #[must_use]
    pub fn kind(&self) -> &BadgeKind;
    #[must_use]
    pub fn tone(&self) -> BadgeTone;
    #[must_use]
    pub fn is_empty(&self) -> bool;
    #[must_use]
    pub fn text(&self) -> Option<String>;
}

#[must_use]
pub fn format_count(n: u32) -> String;

// Resolver structures (§3.8, UIF-ATOM-008)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToneColors {
    pub dot: Color,
    pub chip_border: Color,
    pub chip_text: Color,
    pub pill_fill: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BadgeStyle {
    pub bounds_size: Size,
    pub fill: Color,
    pub border_color: Color,
    pub border_width: f32,
    pub corner_radius: f32,
    pub text_color: Color,
    pub font_weight: iced::font::Weight,
    pub shows_text: bool,
    pub shows_icon: bool,
}

#[must_use]
pub fn resolve_style(model: &BadgeModel, palette: &Palette) -> BadgeStyle;
#[must_use]
pub fn resolve_tone(tone: BadgeTone, palette: &Palette) -> ToneColors;
#[must_use]
pub fn pill_width(style: &BadgeStyle, text_width: f32) -> f32;
#[must_use]
pub fn chip_width(style: &BadgeStyle, glyph_width: f32, text_width: f32) -> f32;
#[must_use]
pub fn badge_chrome_class(style: &BadgeStyle, palette: &Palette) -> ChromeClass;
```

#### Examples
```rust
use hoarfrost::token::color::Palette;
use hoarfrost::widget::badge::{BadgeStyle, pill_width, resolve_style};
use hoarfrost::widget::badge_model::{BadgeModel, BadgeTone, StatusIcon};

let palette = Palette::dark();

// 1. Unread notification dot
let dot_model = BadgeModel::dot().with_tone(BadgeTone::Danger);
let dot_style: BadgeStyle = resolve_style(&dot_model, &palette);

// 2. Tabular count pill (folded past ceiling)
let count_model = BadgeModel::count(142).with_tone(BadgeTone::Accent);
let count_style = resolve_style(&count_model, &palette);
let computed_width = pill_width(&count_style, 24.0);
assert_eq!(count_model.text(), Some("99+".to_string()));

// 3. Semantic status chip
let status_model = BadgeModel::status(StatusIcon::Check, "Verified").with_tone(BadgeTone::Success);
let status_style = resolve_style(&status_model, &palette);
assert!(status_style.shows_icon);
```

#### Type references
- `BadgeKind` - Variant discriminator (`Dot`, `Count { n }`, or `Status { icon, label }`).
- `BadgeTone` - Semantic color role (`Neutral`, `Accent`, `Success`, `Warning`, `Danger`).
- `StatusIcon` - Leading icon glyph for status badges (`Check`, `Warning`, `Error`, `Info`, `Neutral`).
- `BadgeModel` - Declarative badge configuration.
- `ToneColors` - Tone channel resolution splitting dot stroke, chip border, chip text, and pill body fill.
- `BadgeStyle` - Fully resolved paint attributes (fill, border, corner radius, text typography).

#### Return values
- `format_count` returns decimal string for `n <= 99`, or `"99+"` for `n > 99`.
- `resolve_style` returns a complete `BadgeStyle` ready for direct canvas or quad painting.
- `pill_width` and `chip_width` return horizontal layout dimensions enclosing measured text and padding.

#### Side effects
- None; all functions in `widget::badge` are pure resolvers without global or local state mutations.

#### References
- Requirements: `UIF-ATOM-008`, `UIF-TOK-006`
- Source: [`badge.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/badge.rs), [`badge_model.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/badge_model.rs)

---

### `widget::chip` - removable chips and filter tags (`UIF-ATOM-006`)

#### Overview
The `widget::chip` module provides a headless model (`ChipModel<Id>`) and style/layout resolver (`resolve_style`, `ChipStyle`) without building a live `iced` widget (`Element<'a, Message>`). It implements §3.6 (`UIF-ATOM-006`) for assist, filter, and input chips across compact (24px) and standard (32px) heights, supporting leading icons, trailing remove hit targets, and selection toggle states.

#### Prerequisites & Requirements
- Filter chips **MUST** track binary selection state (`selected`), whereas assist chips **MUST NOT** maintain selected polarity.
- Removable chips **MUST** provide a dedicated remove hit target slot via `remove_slot`, and clicking within this boundary **MUST** yield `ChipEvent::Remove` rather than `ChipEvent::Select`.
- When resolving chip style via `resolve_style`, polarity **MUST** be supplied as `ChipPolarity::Selected` only for filter chips where `is_selected()` is `true`.

#### Syntax / Method Signature
```rust
// widget::chip_model (§3.6, UIF-ATOM-006)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChipVariant {
    Assist,
    Filter,
    Input,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ChipHeight {
    Compact, // 24.0px
    #[default]
    Standard, // 32.0px
}

impl ChipHeight {
    #[must_use]
    pub const fn height(self) -> f32;
    #[must_use]
    pub const fn icon_size(self) -> f32;
    #[must_use]
    pub const fn horizontal_padding(self) -> f32;
    #[must_use]
    pub const fn gap(self) -> f32;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChipIcon {
    Glyph(&'static str),
    None,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChipModel<Id> { /* id, label, variant, height, leading, selected, removable, disabled */ }

impl<Id> ChipModel<Id> {
    #[must_use]
    pub fn assist(id: Id, label: impl Into<String>) -> Self;
    #[must_use]
    pub fn filter(id: Id, label: impl Into<String>, selected: bool) -> Self;
    #[must_use]
    pub fn input(id: Id, label: impl Into<String>, removable: bool) -> Self;
    #[must_use]
    pub fn new(id: Id, label: impl Into<String>, variant: ChipVariant) -> Self;
    #[must_use]
    pub fn with_height(mut self, height: ChipHeight) -> Self;
    #[must_use]
    pub fn with_leading(mut self, icon: ChipIcon) -> Self;
    #[must_use]
    pub fn disabled(mut self) -> Self;
    #[must_use]
    pub fn id(&self) -> &Id;
    #[must_use]
    pub fn label(&self) -> &str;
    #[must_use]
    pub fn is_selected(&self) -> bool;
    #[must_use]
    pub fn is_removable(&self) -> bool;
    #[must_use]
    pub fn controlled(&self) -> Controlled<'_, Self>;
    #[must_use]
    pub fn respond(&self, gesture: ChipGesture) -> Option<ChipEvent<Id>>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChipGesture {
    Click,
    RemoveClick,
    DeleteKey,
    EnterKey,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChipEvent<Id> {
    Select(Id),
    Deselect(Id),
    Remove(Id),
    Activate(Id),
}

// widget::chip (resolvers) (§3.6, UIF-ATOM-006)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChipPolarity {
    Unselected,
    Selected,
}

impl ChipPolarity {
    #[must_use]
    pub fn of<Id>(model: &ChipModel<Id>) -> Self;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChipStyle {
    pub background: Color,
    pub foreground: Color,
    pub border_color: Color,
    pub border_width: f32,
    pub radius: iced::border::Radius,
    pub focus_ring_color: Color,
    pub focus_ring_width: f32,
    pub height: f32,
    pub shows_leading: bool,
    pub shows_remove: bool,
}

#[must_use]
pub fn resolve_style(
    polarity: ChipPolarity,
    height: ChipHeight,
    state: VisualState,
    palette: &Palette,
) -> ChipStyle;
#[must_use]
pub fn with_variant(style: ChipStyle, is_assist: bool, is_removable: bool) -> ChipStyle;
#[must_use]
pub fn resolve_model<Id>(
    model: &ChipModel<Id>,
    input: InteractionInput,
    palette: &Palette,
) -> ChipStyle;
#[must_use]
pub fn chip_chrome_class(style: &ChipStyle, palette: &Palette) -> ChromeClass;
#[must_use]
pub fn chip_width(style: &ChipStyle, leading_width: f32, text_width: f32) -> f32;
#[must_use]
pub fn remove_slot(style: &ChipStyle, bounds: Rectangle) -> Option<Rectangle>;
#[must_use]
pub fn border_of(style: &ChipStyle) -> Border;
```

#### Examples
```rust
use hoarfrost::state::VisualState;
use hoarfrost::token::color::Palette;
use hoarfrost::widget::chip::{ChipPolarity, ChipStyle, remove_slot, resolve_style};
use hoarfrost::widget::chip_model::{ChipEvent, ChipGesture, ChipHeight, ChipModel};
use iced::Rectangle;

let palette = Palette::dark();

// 1. Filter chip responding to selection click
let filter = ChipModel::filter("tag_rust", "Rust", true).with_height(ChipHeight::Compact);
let event = filter.respond(ChipGesture::Click);
assert_eq!(event, Some(ChipEvent::Deselect("tag_rust")));

// 2. Resolve paint style
let style: ChipStyle = resolve_style(
    ChipPolarity::of(&filter),
    ChipHeight::Compact,
    VisualState::default(),
    &palette,
);

// 3. Removable input chip slot layout
let input_chip = ChipModel::input("email_recipient", "alice@example.com", true);
let bounds = Rectangle::new(iced::Point::ORIGIN, iced::Size::new(140.0, 32.0));
let close_box = remove_slot(&style, bounds);
assert!(close_box.is_some());
```

#### Type references
- `ChipVariant` - Operational mode (`Assist`, `Filter`, or `Input`).
- `ChipHeight` - Height preset (`Compact` at 24px or `Standard` at 32px).
- `ChipModel<Id>` - Declarative model managing id, label, variant, leading icon, and selection/removal flags.
- `ChipGesture` - Interaction input event (`Click`, `RemoveClick`, `DeleteKey`, `EnterKey`).
- `ChipEvent<Id>` - Resulting semantic event (`Select`, `Deselect`, `Remove`, `Activate`).
- `ChipStyle` - Fully resolved paint attributes including background, border, and slot visibility flags.

#### Return values
- `ChipModel::respond` returns `Some(ChipEvent<Id>)` based on the gesture kind and active chip variant.
- `remove_slot` returns `Some(Rectangle)` defining the hit box for the trailing close button on removable chips, or `None`.
- `chip_width` computes total rendered width enclosing padding, optional leading icon, label text, and close slot.

#### Side effects
- None; the model and resolvers operate purely without internal mutation.

#### References
- Requirements: `UIF-ATOM-006`, `UIF-TOK-002`, `UIF-TOK-006`
- Source: [`chip.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/chip.rs), [`chip_model.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/chip_model.rs)

---

### `widget::avatar` - user and entity avatars (`UIF-ATOM-013`)

#### Overview
The `widget::avatar` module provides a live `iced` widget (`Element<'a, Message>`) via `Avatar` (and constructor `avatar`), backed by the declarative data model `AvatarModel`. It implements §3.13 (`UIF-ATOM-013`) for user and entity avatars with automatic three-stage graceful fallback (image URI → initials → generic fallback glyph), circle or rounded square geometry, and semantic presence badges.

#### Prerequisites & Requirements
- If `display_name` contains multiple words, initials **MUST** be derived from the first letters of the first and last words; single-word names **MUST** produce a single initial letter.
- Image loading failures or missing URIs **MUST NOT** panic or display broken UI; the avatar **MUST** fallback smoothly to `FallbackStage::Initials` or `FallbackStage::FallbackGlyph`.
- Presence status indicators **MUST** render as corner badge dots with appropriate high-contrast borders separating the indicator from the avatar face.

#### Syntax / Method Signature
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum AvatarSize {
    Sm, // 24.0px diameter
    #[default]
    Md, // 32.0px diameter
    Lg, // 40.0px diameter
}

impl AvatarSize {
    #[must_use]
    pub const fn diameter(self) -> f32;
    #[must_use]
    pub const fn font_size(self) -> f32;
    #[must_use]
    pub const fn badge_size(self) -> f32;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum AvatarShape {
    #[default]
    Circle,
    RoundedSquare,
}

impl AvatarShape {
    #[must_use]
    pub fn corner_radius(self, diameter: f32) -> shape::Radius;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FallbackStage {
    Image,
    Initials,
    FallbackGlyph,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AvatarModel {
    pub display_name: String,
    pub initials: Option<String>,
    pub image_uri: Option<String>,
    pub presence: Option<BadgeTone>,
    pub size: AvatarSize,
    pub shape: AvatarShape,
}

impl AvatarModel {
    #[must_use]
    pub fn new(display_name: impl Into<String>) -> Self;
    #[must_use]
    pub fn with_initials(mut self, initials: impl Into<String>) -> Self;
    #[must_use]
    pub fn with_image_uri(mut self, uri: impl Into<String>) -> Self;
    #[must_use]
    pub fn with_presence(mut self, presence: impl Into<Option<BadgeTone>>) -> Self;
    #[must_use]
    pub fn with_size(mut self, size: AvatarSize) -> Self;
    #[must_use]
    pub fn with_shape(mut self, shape: AvatarShape) -> Self;
    #[must_use]
    pub fn fallback_stage(&self) -> FallbackStage;
}

#[must_use]
pub fn extract_initials(display_name: &str) -> String;
#[must_use]
pub fn resolve_presence_color(tone: BadgeTone, palette: &Palette) -> Color;

// Live Avatar Widget (§3.13, UIF-ATOM-013)
#[derive(Clone, Debug)]
pub struct Avatar { /* private model, palette */ }

impl Avatar {
    #[must_use]
    pub fn new(model: AvatarModel) -> Self;
    #[must_use]
    pub fn palette(mut self, palette: Palette) -> Self;
    #[must_use]
    pub fn model(&self) -> &AvatarModel;
    #[must_use]
    pub fn size(&self) -> Size<Length>;
}

impl<'a, Message> Avatar {
    #[must_use]
    pub fn controlled(
        model: Controlled<'a, AvatarModel>,
        _focus_id: impl Into<FocusId>,
    ) -> Self {
        Self::new(model.as_ref().clone())
    }
}

#[must_use]
pub fn avatar(model: AvatarModel) -> Avatar;
```

#### Examples
```rust
use hoarfrost::widget::avatar::{Avatar, AvatarShape, AvatarSize, extract_initials};
use hoarfrost::widget::avatar_model::AvatarModel;
use hoarfrost::widget::badge_model::BadgeTone;
use iced::Element;

assert_eq!(extract_initials("Jane Doe"), "JD");

let model = AvatarModel::new("Jane Doe")
    .with_size(AvatarSize::Lg)
    .with_shape(AvatarShape::Circle)
    .with_presence(Some(BadgeTone::Success));

let avatar_elem: Element<'_, ()> = Avatar::new(model)
    .palette(hoarfrost::token::color::Palette::dark())
    .into();
```

#### Type references
- `AvatarSize` - Predefined diameter presets (`Sm` = 24px, `Md` = 32px, `Lg` = 40px).
- `AvatarShape` - Boundary clipping shape (`Circle` or `RoundedSquare`).
- `FallbackStage` - Graceful fallback chain state (`Image`, `Initials`, or `FallbackGlyph`).
- `AvatarModel` - Declarative data model containing display name, image URI, presence status, and shape.
- `Avatar` - Live `iced` widget rendering the image/initials canvas and presence badge indicator.

#### Return values
- `extract_initials` returns uppercase initials parsed from whitespace-separated words.
- `AvatarModel::fallback_stage` returns `Image` if an image URI is present, `Initials` if letters can be extracted, or `FallbackGlyph` as a final fallback.
- `Avatar::size` returns the fixed bounding `Size<Length>` matching `model.size.diameter()`.

#### Side effects
- None; the avatar is a presentation widget without user event mutations.

#### References
- Requirements: `UIF-ATOM-013`, `UIF-ATOM-008`
- Source: [`avatar.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/avatar.rs)

---

### `widget::hyperlink` - navigational text links (`UIF-ATOM-001d`)

#### Overview
The `widget::hyperlink` module provides a live interactive `iced` widget (`Element<'a, Message>`) via `Hyperlink`, backed by `HyperlinkModel` and pure geometry/style resolvers. It implements §3.1d (`UIF-ATOM-001d`) for internal and external navigation links, featuring hover underline transitions, status-bar URI previews, keyboard activation gating, and contrast-safe focus rings.

#### Prerequisites & Requirements
- When a hyperlink points to an external destination, it **MUST** set `external = true` to render the trailing external link glyph and signal external browsing behavior.
- The crate **MUST NOT** invoke OS-level browser openers directly; activation **MUST** emit `HyperlinkEmission::Opened(Commit<HyperlinkTarget>)` to allow the host application to supervise URL navigation.
- When disabled (`enabled = false`), mouse clicks and keyboard activations **MUST NOT** emit `Opened` or `HoverUri` events.

#### Syntax / Method Signature
```rust
// widget::hyperlink_model (§3.1d, UIF-ATOM-001d)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HyperlinkTarget {
    pub uri: String,
    pub external: bool,
}

impl HyperlinkTarget {
    #[must_use]
    pub fn new(uri: impl Into<String>, external: bool) -> Self;
}

#[derive(Clone, Debug)]
pub enum HyperlinkEmission {
    /// The link was activated - `on_open`, carrying the target.
    Opened(Commit<HyperlinkTarget>),
    /// The status-bar preview changed - `on_hover_uri`. `None` clears it.
    HoverUri(Option<String>),
}

impl HyperlinkEmission {
    #[must_use]
    pub fn uri(&self) -> Option<&str>;
    #[must_use]
    pub fn is_open(&self) -> bool;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HyperlinkModel { /* label, target, enabled */ }

impl HyperlinkModel {
    #[must_use]
    pub fn new(label: impl Into<String>, uri: impl Into<String>) -> Self;
    #[must_use]
    pub fn external(label: impl Into<String>, uri: impl Into<String>) -> Self;
    #[must_use]
    pub fn target(&self) -> HyperlinkTarget;
    #[must_use]
    pub fn activate(&self) -> Option<Commit<HyperlinkTarget>>;
    #[must_use]
    pub fn hover_uri(&self) -> Option<String>;
    #[must_use]
    pub fn hover_entered(&self) -> Option<HyperlinkEmission>;
    #[must_use]
    pub fn hover_cleared() -> HyperlinkEmission;
}

// widget::hyperlink (paint & geometry resolvers) (§3.1d, UIF-ATOM-001d)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HyperlinkStyle {
    pub foreground: Color,
    pub underline: Option<Color>,
    pub focus_ring_color: Color,
    pub focus_ring_width: f32,
    pub shows_external_glyph: bool,
}

#[must_use]
pub fn resolve_style(state: VisualState, mount: Color, palette: &Palette) -> HyperlinkStyle;
#[must_use]
pub fn control_size(label_width: f32) -> Size;
#[must_use]
pub fn label_bounds(control: Rectangle) -> Rectangle;
#[must_use]
pub fn underline_bounds(control: Rectangle) -> Rectangle;
#[must_use]
pub fn pill_layer(bounds: Rectangle, style: &HyperlinkStyle) -> Option<Layer>;
#[must_use]
pub fn underline_layer(bounds: Rectangle, style: &HyperlinkStyle) -> Option<Layer>;
#[must_use]
pub fn focus_ring_layer(bounds: Rectangle, style: &HyperlinkStyle) -> Option<Layer>;

// Live Hyperlink Widget (§3.1d, UIF-ATOM-001d)
pub struct Hyperlink<'a, Message> { /* private fields */ }

impl<'a, Message> Hyperlink<'a, Message> {
    #[must_use]
    pub fn controlled(
        model: Controlled<'a, HyperlinkModel>,
        focus_id: impl Into<FocusId>,
        emit: impl Fn(HyperlinkEmission) -> Option<Message> + 'a,
    ) -> Self;
    #[must_use]
    pub fn standalone(
        label: impl Into<String>,
        uri: impl Into<String>,
        focus_id: impl Into<FocusId>,
        on_press: impl Fn(HyperlinkTarget) -> Option<Message> + 'a,
    ) -> Self;
    #[must_use]
    pub fn mount(mut self, mount: Color) -> Self;
    #[must_use]
    pub fn palette(mut self, palette: Palette) -> Self;
    #[must_use]
    pub fn external(mut self, external: bool) -> Self;
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self;
    #[must_use]
    pub fn label(&self) -> &str;
    #[must_use]
    pub fn uri(&self) -> &str;
    #[must_use]
    pub fn is_external(&self) -> bool;
    #[must_use]
    pub fn is_enabled(&self) -> bool;
    #[must_use]
    pub fn focus_id(&self) -> &FocusId;
    #[must_use]
    pub fn widget_id(&self) -> &iced::advanced::widget::Id;
}
```

#### Examples
```rust
use hoarfrost::binding::controlled::Controlled;
use hoarfrost::widget::hyperlink::Hyperlink;
use hoarfrost::widget::hyperlink_model::{HyperlinkEmission, HyperlinkModel, HyperlinkTarget};
use iced::Element;

#[derive(Clone, Debug)]
enum Message {
    OpenUrl(String),
    PreviewUrl(Option<String>),
}

let model = HyperlinkModel::external("Documentation", "https://example.com/docs");
let controlled = Controlled::new(&model);

let link: Element<'_, Message> = Hyperlink::controlled(
    controlled,
    "docs_link",
    |emission| match emission {
        HyperlinkEmission::Opened(commit) => {
            let target: HyperlinkTarget = commit.into_value();
            Some(Message::OpenUrl(target.uri))
        }
        HyperlinkEmission::HoverUri(maybe_uri) => Some(Message::PreviewUrl(maybe_uri)),
    },
)
.into();
```

#### Type references
- `HyperlinkTarget` - URI destination string paired with an `external` indicator flag.
- `HyperlinkEmission` - Interaction output enum (`Opened(Commit<HyperlinkTarget>)` or `HoverUri(Option<String>)`).
- `HyperlinkModel` - Declarative model defining link label, target URI, and enabled state.
- `HyperlinkStyle` - Paint attributes defining foreground color, underline stroke, and focus ring.
- `Hyperlink` - Live `iced` widget rendering the clickable text link and external navigation icon.

#### Return values
- `HyperlinkModel::activate` returns `Some(Commit<HyperlinkTarget>)` when enabled, or `None` if disabled.
- `HyperlinkModel::hover_entered` returns `Some(HyperlinkEmission::HoverUri(Some(uri)))`.
- `HyperlinkModel::hover_cleared` returns `HyperlinkEmission::HoverUri(None)`.

#### Side effects
- Hovering over the link renders an underline and emits `HoverUri(Some(uri))`.
- Clicking or pressing Enter dispatches `Opened(Commit<HyperlinkTarget>)`.

#### References
- Requirements: `UIF-ATOM-001d`, `UIF-PRIN-005`
- Source: [`hyperlink.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/hyperlink.rs), [`hyperlink_model.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/hyperlink_model.rs)

---

### `widget::search_box` - search input with instant clear (`UIF-PANE-003`)

#### Overview
The `widget::search_box` module provides a live interactive `iced` widget (`Element<'a, Message>`) via `SearchBox`, backed by `SearchBoxModel`. It implements `UIF-PANE-003` and `UIF-PRIN-006` combining a 32px standard text input with a trailing 28×28px action button that dynamically swaps between a passive search glyph `🔍` and an actionable clear button `×`.

#### Prerequisites & Requirements
- When the search query is non-empty or `is_active_override` is `true`, the trailing action button **MUST** display `GLYPH_CLEAR` (`'×'`) and emit `on_clear` when pressed.
- When the query is empty and no filter override is active, the trailing button **MUST** display `GLYPH_SEARCH` (`'🔍'`) and act as a passive glyph.
- Clearing the search box via the clear button **MUST NOT** trigger `on_submit`; it **MUST** emit `on_clear` so the host application resets the query and filter states cleanly.

#### Syntax / Method Signature
```rust
pub const SEARCH_INPUT_HEIGHT: f32 = size::CONTROL_MD; // 32.0px
pub const SEARCH_ACTION_SIZE: f32 = size::CONTROL_SM; // 28.0px
pub const SEARCH_PLACEHOLDER: &str = "Search content…";
pub const GLYPH_SEARCH: char = '🔍';
pub const GLYPH_CLEAR: char = '×';

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchBoxModel {
    pub query: String,
    pub is_active_override: bool,
}

impl SearchBoxModel {
    #[must_use]
    pub fn new(query: impl Into<String>) -> Self;
    #[must_use]
    pub fn with_active(mut self, active: bool) -> Self;
    #[must_use]
    pub fn is_active(&self) -> bool;
    #[must_use]
    pub fn trailing_glyph(&self) -> char;
}

pub struct SearchBox<'a, Message> { /* private fields */ }

impl<'a, Message: 'a + Clone> SearchBox<'a, Message> {
    #[must_use]
    pub fn new(query: impl Into<String>) -> Self;
    #[must_use]
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self;
    #[must_use]
    pub fn with_active(mut self, active: bool) -> Self;
    #[must_use]
    pub fn with_palette(mut self, palette: Palette) -> Self;
    #[must_use]
    pub fn on_input(mut self, f: impl Fn(String) -> Message + 'a) -> Self;
    #[must_use]
    pub fn on_submit(mut self, msg: Message) -> Self;
    #[must_use]
    pub fn on_clear(mut self, msg: Message) -> Self;
}

impl<'a, Message: 'a + Clone> From<SearchBox<'a, Message>> for Element<'a, Message> {
    fn from(search_box: SearchBox<'a, Message>) -> Self;
}
```

#### Examples
```rust
use hoarfrost::widget::search_box::SearchBox;
use iced::Element;

#[derive(Clone, Debug)]
enum Message {
    QueryChanged(String),
    SearchExecuted,
    QueryCleared,
}

let search: Element<'_, Message> = SearchBox::new("query text")
    .with_placeholder("Search documentation…")
    .on_input(Message::QueryChanged)
    .on_submit(Message::SearchExecuted)
    .on_clear(Message::QueryCleared)
    .into();
```

#### Type references
- `SearchBoxModel` - Declarative state holding current query text and active filter override flag.
- `SearchBox` - Live `iced` composite widget pairing an inner text entry field with a trailing clear/search button.
- `GLYPH_SEARCH` / `GLYPH_CLEAR` - Visual state glyph constants (`🔍` vs `×`).

#### Return values
- `SearchBoxModel::is_active` returns `true` if `query` is not empty or `is_active_override` is `true`.
- `SearchBoxModel::trailing_glyph` returns `'×'` when active, or `'🔍'` when idle.

#### Side effects
- Text typing dispatches `on_input`.
- Pressing Enter in the input field dispatches `on_submit`.
- Clicking the trailing action button while active dispatches `on_clear`.

#### References
- Requirements: `UIF-PANE-003`, `UIF-PRIN-006`
- Source: [`search_box.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/search_box.rs)

---

### `widget::progress` - progress bars and activity rings (`UIF-ATOM-009`)

#### Overview
The `widget::progress` module provides a headless model (`ProgressModel`) and pure paint resolvers (`resolve_style`, `track_layer`, `fill_layer`, `spinner_style`) without building an interactive `iced` widget (`Element<'a, Message>`). It implements §3.9 (`UIF-ATOM-009`) for determinate and indeterminate progress bars and activity spinners across `Active`, `Paused`, and `Error` statuses, enforcing clock-free determinism and OS reduced-motion adaptability.

#### Prerequisites & Requirements
- Determinate progress instances **MUST** enforce `completed <= total` and calculate a valid finite fraction in `[0.0, 1.0]`.
- When OS reduced motion is active, indeterminate progress indicators **MUST NOT** animate continuous movement; they **MUST** render as a static striped block via `indeterminate_segment`.
- If a progress operation can be cancelled by the user, the model **MUST** set `with_cancellable(true)` so enclosing containers can provide an abort affordance.

#### Syntax / Method Signature
```rust
// Layout & geometry constants (§3.9, UIF-ATOM-009)
pub const BAR_HEIGHT: f32 = size::PROGRESS_BAR_HEIGHT; // 4.0px
pub const INDETERMINATE_FRACTION: f32 = 0.35; // 35% of track width
pub const SPINNER_SMALL_DIAMETER: f32 = size::SPINNER_SM; // 16.0px
pub const SPINNER_LARGE_DIAMETER: f32 = size::SPINNER_LG; // 32.0px

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ProgressMode {
    #[default]
    Determinate,
    Indeterminate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ProgressStatus {
    #[default]
    Active,
    Paused,
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProgressModel {
    pub completed: u64,
    pub total: u64,
    pub mode: ProgressMode,
    pub status: ProgressStatus,
    pub label: Option<&'static str>,
    pub cancellable: bool,
}

impl ProgressModel {
    #[must_use]
    pub fn determinate(completed: u64, total: u64) -> Self;
    #[must_use]
    pub fn indeterminate() -> Self;
    #[must_use]
    pub fn with_status(mut self, status: ProgressStatus) -> Self;
    #[must_use]
    pub fn with_label(mut self, label: &'static str) -> Self;
    #[must_use]
    pub fn with_cancellable(mut self, cancellable: bool) -> Self;
    #[must_use]
    pub fn with_mode(mut self, mode: ProgressMode) -> Self;
    #[must_use]
    pub fn with_completed(mut self, completed: u64) -> Self;
    #[must_use]
    pub fn mode(&self) -> ProgressMode;
    #[must_use]
    pub fn status(&self) -> ProgressStatus;
    #[must_use]
    pub fn label(&self) -> Option<&'static str>;
    #[must_use]
    pub fn cancellable(&self) -> bool;
    #[must_use]
    pub fn is_indeterminate(&self) -> bool;
    #[must_use]
    pub fn fraction(&self) -> Option<f32>;
}

// Paint & geometry resolvers (§3.9, UIF-ATOM-009)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProgressStyle {
    pub track_unfilled: Color,
    pub track_border: Color,
    pub fill: Color,
    pub status: ProgressStatus,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SegmentStyle {
    pub fill: Color,
    pub border: Color,
    pub striped: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpinnerStyle {
    pub arc_color: Color,
    pub arc_width: f32,
    pub diameter: f32,
}

#[must_use]
pub fn resolve_style(model: &ProgressModel, palette: &Palette) -> ProgressStyle;
#[must_use]
pub fn indeterminate_segment(
    track: Rectangle,
    phase: f32,
    reduced_motion: bool,
    style: &ProgressStyle,
) -> SegmentStyle;
#[must_use]
pub fn spinner_style(large: bool, inside_solid: bool, palette: &Palette) -> SpinnerStyle;
#[must_use]
pub fn track_layer(bounds: Rectangle, style: &ProgressStyle) -> Layer;
#[must_use]
pub fn fill_layer(bounds: Rectangle, style: &ProgressStyle, fraction: f32) -> Option<Layer>;
#[must_use]
pub fn segment_layer(bounds: Rectangle, segment: &SegmentStyle) -> Layer;
#[must_use]
pub fn progress_chrome_class(_fill: Color, _palette: &Palette) -> ChromeClass;
#[must_use]
pub fn spinner_arc_chrome_class(arc: Color, palette: &Palette) -> ChromeClass;
```

#### Examples
```rust
use hoarfrost::token::color::Palette;
use hoarfrost::widget::progress::{ProgressStyle, fill_layer, resolve_style, track_layer};
use hoarfrost::widget::progress_model::{ProgressModel, ProgressStatus};
use iced::Rectangle;

let palette = Palette::dark();

// 1. Determinate progress
let model = ProgressModel::determinate(45, 100)
    .with_status(ProgressStatus::Active)
    .with_cancellable(true);

let fraction = model.fraction().unwrap_or(0.0);
assert_eq!(fraction, 0.45);

// 2. Resolve style and layers
let style: ProgressStyle = resolve_style(&model, &palette);
let bounds = Rectangle::new(iced::Point::ORIGIN, iced::Size::new(200.0, 4.0));
let _track = track_layer(bounds, &style);
let _fill = fill_layer(bounds, &style, fraction);
```

#### Type references
- `ProgressMode` - Operational mode (`Determinate` or `Indeterminate`).
- `ProgressStatus` - Semantic state (`Active`, `Paused`, or `Error`).
- `ProgressModel` - Declarative model tracking completed units, total units, status, mode, and cancellable flag.
- `ProgressStyle` - Resolved paint tokens for unfilled rail and indicator fill.
- `SegmentStyle` - Resolved style for indeterminate moving or striped segment blocks.
- `SpinnerStyle` - Arc color, line stroke width, and diameter for circular activity rings.

#### Return values
- `ProgressModel::fraction` returns `Some(f32)` in `0.0..=1.0` for determinate progress, or `None` for indeterminate mode.
- `fill_layer` returns `Some(Layer)` representing the filled indicator quad, or `None` if fraction is zero or non-finite.

#### Side effects
- None; progress resolvers are clock-free pure functions parameterized by host-provided phase and progress numbers.

#### References
- Requirements: `UIF-ATOM-009`, `UIF-TOK-002`, `UIF-TOK-006`
- Source: [`progress.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/progress.rs), [`progress_model.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/progress_model.rs)

---
### `widget::focus` and `widget::focusable` - focus rendering (`UIF-PRIN-005`)

#### Overview

The `widget::focus` module provides a headless style and geometry resolver (`FocusIndicator`, `FocusPlacement`) for outer focus rings and inset focus outlines, while `widget::focusable` provides a live `iced` widget (`Focusable`, `Element<'a, Message>`) that bridges `iced` focus state with `hoarfrost::focus::FocusId`. Both modules enforce the `3:1` WCAG 2.2 SC 1.4.11 non-text contrast floor against the surrounding surface (`UIF-PRIN-005`).

#### Prerequisites & Requirements

- The caller **MUST** enable the `iced` cargo feature (`#[cfg(feature = "iced")]`).
- The caller **MUST** supply a `&Palette` from `hoarfrost::token::color::Palette` when resolving ring colors.
- The caller **MUST** pass a unique `FocusId` to `focusable` so keyboard traversal and focus traps can target the control.

#### Syntax / Method Signature

```rust
// widget::focus
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusPlacement {
    Outer,
    Inset,
}

impl FocusPlacement {
    pub const fn stroke(self) -> f32;
    pub fn bounds(self, control: Rectangle, width: f32) -> Rectangle;
    pub const fn radius(self, radius: f32, width: f32) -> f32;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FocusIndicator {
    pub color: Color,
    pub width: f32,
    pub placement: FocusPlacement,
}

impl FocusIndicator {
    pub const fn outer(color: Color) -> Self;
    pub const fn inset(color: Color) -> Self;
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

// widget::focusable
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct FocusableState { /* private fields */ }

impl FocusableState {
    pub const fn new() -> Self;
    pub const fn is_focused(&self) -> bool;
}

pub struct Focusable<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> { /* private fields */ }

impl<'a, Message, Theme, Renderer> Focusable<'a, Message, Theme, Renderer> {
    pub fn new(
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
        id: FocusId,
        enabled: bool,
    ) -> Self;
    pub fn focus_id(&self) -> &FocusId;
    pub fn widget_id(&self) -> &iced::widget::Id;
    pub fn is_enabled(&self) -> bool;
}

pub fn focusable<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
    id: FocusId,
    enabled: bool,
) -> Element<'a, Message, Theme, Renderer>;
```

#### Examples

```rust
use hoarfrost::focus::FocusId;
use hoarfrost::token::color::Palette;
use hoarfrost::widget::focus::{self, FocusPlacement};
use hoarfrost::widget::focusable::focusable;

let palette = Palette::dark();
let indicator = focus::resolve_focus_indicator_for_background(
    palette.surface_base,
    &palette,
    FocusPlacement::Outer,
);
assert!(indicator.width > 0.0);

let wrapped: iced::Element<'_, ()> = focusable(
    iced::widget::text("Focusable item"),
    FocusId::new("item-1"),
    true,
);
```

#### Type references

- `FocusPlacement` - Ring placement relative to the control bounds (`Outer` 2px ring vs `Inset` 1px outline for clipped cells).
- `FocusIndicator` - Resolved color, stroke width, and placement for a focus indicator.
- `FocusableState` - Widget-tree state tracking whether the wrapped element holds focus.
- `Focusable` - Wrapper widget that draws an outer focus ring when focused and enabled.

#### Return values

- `resolve_ring_color` and `resolve_ring_color_for_background` return `palette.accent_border` when it clears `3:1` against the background, or `palette.text_strong` otherwise.
- `focusable` returns a live `Element<'a, Message, Theme, Renderer>`.

#### Side effects

- None. `Focusable` only reads and updates its own `FocusableState` node inside the `iced` widget tree.

#### References

- Implementation: [`src/hoarfrost/widget/focus.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/focus.rs), [`src/hoarfrost/widget/focusable.rs`](file:///D:/hoarfrost/src/hoarfrost/widget/focusable.rs)
- Specification: `UIF-PRIN-005`

---

### `composite::card`, `composite::icon`, `composite::skeleton` - card, icon, and skeleton containers (`UIF-COMP-005`, `UIF-COMP-014`)

#### Overview

The `composite::card` and `composite::icon` modules provide headless style and geometry resolvers (`CardStyle`, `Layer`, `Icon`), while `composite::skeleton` provides both a live `iced` widget (`Skeleton`, `Element<'a, Message>`) and a headless style resolver (`SkeletonStyle`). Card elevation is strictly decorative (`UIF-PRIN-005`), and skeleton placeholders pulse between `surface.raised` and `surface.hover` or freeze under `reduced_motion` (`UIF-MOTION-002`).

#### Prerequisites & Requirements

- The caller **MUST** enable the `iced` cargo feature (`#[cfg(feature = "iced")]`).
- The caller **MUST** supply a `&Palette` from `hoarfrost::token::color::Palette` when resolving card or skeleton styles.
- The application **MUST NOT** use `CardElevation` alone to communicate selection, focus, or active status (`UIF-PRIN-005`).

#### Syntax / Method Signature

```rust
// composite::card
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CardElevation {
    #[default]
    Rest,
    Elevated,
}

impl CardElevation {
    pub const fn is_active_indicator(self) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CardStyle {
    pub background: Color,
    pub border_color: Color,
    pub border_width: f32,
    pub radius: f32,
    pub padding: f32,
}

pub fn resolve_style(elevation: CardElevation, palette: &Palette) -> CardStyle;
pub fn content_bounds(bounds: Rectangle, style: &CardStyle) -> Rectangle;
pub fn card_layer(bounds: Rectangle, style: &CardStyle) -> Layer;

// composite::icon
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Icon(pub String);

impl Icon {
    pub fn new(s: impl Into<String>) -> Self;
}

// composite::skeleton
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Skeleton {
    pub width: f32,
    pub height: f32,
    pub radius: f32,
    pub phase: f32,
    pub reduced_motion: bool,
}

impl Skeleton {
    pub fn new(width: f32, height: f32) -> Self;
    pub fn row(height: f32) -> Self;
    pub fn circle(diameter: f32) -> Self;
    pub fn with_radius(mut self, radius: f32) -> Self;
    pub fn with_phase(mut self, phase: f32) -> Self;
    pub fn with_reduced_motion(mut self, reduced_motion: bool) -> Self;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkeletonStyle {
    pub background: Color,
    pub border_color: Color,
    pub border_width: f32,
    pub radius: f32,
}

pub fn skeleton(width: f32, height: f32) -> Skeleton;
pub fn resolve_style(skeleton: &Skeleton, palette: &Palette) -> SkeletonStyle;
pub fn interpolate_color(from: Color, to: Color, t: f32) -> Color;
```

#### Examples

```rust
use hoarfrost::composite::card::{self, CardElevation};
use hoarfrost::composite::skeleton::{self, skeleton};
use hoarfrost::token::color::Palette;

let palette = Palette::dark();
let style = card::resolve_style(CardElevation::Elevated, &palette);
assert!(!CardElevation::Elevated.is_active_indicator());

let skel = skeleton(180.0, 16.0).with_phase(0.5).with_reduced_motion(false);
let skel_style = skeleton::resolve_style(&skel, &palette);
let element: iced::Element<'_, ()> = skel.into();
```

#### Type references

- `CardElevation` - Decorative surface elevation tier (`Rest` vs `Elevated`).
- `CardStyle` - Resolved background, `1px border.subtle` border, `6px` card radius, and `16px` padding.
- `Icon` - Strongly typed icon glyph or symbol wrapper used across composite controls.
- `Skeleton` - Shimmering loading placeholder that converts directly into an `iced::Element` or resolves headlessly via `skeleton::resolve_style`.
- `SkeletonStyle` - Resolved interpolated fill color, border color, border width, and corner radius for a `Skeleton`.

#### Return values

- `card::resolve_style` returns `CardStyle` with `radius = 6.0` (`shape::RADIUS_CARD`) and `padding = 16.0` (`space::LG`).
- `card::content_bounds` returns the inner `Rectangle` inset by `style.padding` (clamped at `0.0`).
- `skeleton::resolve_style` returns a static `surface.raised` fill when `reduced_motion` is `true`, or a triangle-wave interpolation between `surface.raised` and `surface.hover` when `false`.

#### Side effects

- None. All functions are pure calculations.

#### References

- Implementation: [`src/hoarfrost/composite/card.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/card.rs), [`src/hoarfrost/composite/icon.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/icon.rs), [`src/hoarfrost/composite/skeleton.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/skeleton.rs)
- Specification: `UIF-COMP-005`, `UIF-COMP-014`

---

### `composite::expander` and `composite::form_field` - disclosure and form wrappers (`UIF-COMP-001`, `UIF-COMP-003`)

#### Overview

The `composite::expander` module provides a headless disclosure and accordion state model (`Expander`, `Accordion`) and header style/geometry resolver (`HeaderStyle`, `HeaderSlots`), while `composite::form_field` provides both live `iced` layout wrappers (`stacked`, `inline`, `stacked_reserved` returning `Element<'a, Message>`) and headless header/caption resolvers (`HeaderParts`, `CaptionParts`).

#### Prerequisites & Requirements

- The caller **MUST** enable the `iced` cargo feature (`#[cfg(feature = "iced")]`).
- The caller **MUST** supply a `&Palette` from `hoarfrost::token::color::Palette`.
- When calling `form_field::resolve_caption`, the caller **MUST** pass `valid: true` when the input is valid and `valid: false` when the input is invalid (for example, `!model.is_invalid()`).

#### Syntax / Method Signature

```rust
// composite::expander
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AccordionPolicy {
    #[default]
    Independent,
    SingleOpen,
}

impl AccordionPolicy {
    pub const fn closes_siblings(self) -> bool;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HeaderRung {
    #[default]
    Standard,
    Compact,
}

impl HeaderRung {
    pub const fn height(self) -> f32;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisclosureKey {
    Toggle,
    Expand,
    Collapse,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expander { /* private fields */ }

impl Expander {
    pub fn new(id: impl Into<String>, title: impl Into<String>, open: bool) -> Self;
    pub fn with_enabled(self, enabled: bool) -> Self;
    pub fn id(&self) -> &str;
    pub fn title(&self) -> &str;
    pub const fn is_open(&self) -> bool;
    pub const fn is_enabled(&self) -> bool;
    pub fn toggle(&mut self) -> Option<Commit<bool>>;
    pub fn set_open(&mut self, open: bool) -> Option<Commit<bool>>;
    pub fn on_key(&mut self, key: DisclosureKey) -> Option<Commit<bool>>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Accordion { /* private fields */ }

impl Accordion {
    pub fn new(policy: AccordionPolicy, sections: Vec<Expander>) -> Self;
    pub const fn policy(&self) -> AccordionPolicy;
    pub fn sections(&self) -> &[Expander];
    pub fn toggle(&mut self, index: usize) -> Vec<(usize, Commit<bool>)>;
    pub fn on_key(&mut self, index: usize, key: DisclosureKey) -> Vec<(usize, Commit<bool>)>;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeaderStyle {
    pub background: Color,
    pub border_color: Color,
    pub border_width: f32,
    pub text_color: Color,
    pub chevron_color: Color,
    pub chevron_degrees: f32,
    pub focus_ring: Option<FocusIndicator>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeaderSlots {
    pub header: Rectangle,
    pub chevron: Rectangle,
    pub label: Rectangle,
    pub trailing: Rectangle,
}

pub const fn chevron_rotation(open: bool) -> f32;
pub fn resolve_header_style(
    visual: VisualState,
    open: bool,
    surface: Color,
    palette: &Palette,
) -> HeaderStyle;
pub fn header_slots(bounds: Rectangle, trailing_width: f32) -> HeaderSlots;
pub const fn total_height_at(rung: HeaderRung, open: bool, content_height: f32) -> f32;
pub const fn total_height(open: bool, content_height: f32) -> f32;
pub fn header_layer(bounds: Rectangle, style: &HeaderStyle) -> Layer;

// composite::form_field
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OverflowPolicy {
    #[default]
    Clip,
    Wrap,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeaderParts<'a> {
    pub label: &'a str,
    pub required_marker: Option<&'static str>,
    pub optional_tag: Option<&'static str>,
    pub unit_suffix: Option<&'a str>,
    pub label_color: Color,
    pub marker_color: Color,
    pub meta_color: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CaptionParts<'a> {
    pub glyph: Option<&'static str>,
    pub text: Option<&'a str>,
    pub color: Color,
    pub is_error: bool,
}

impl<'a> CaptionParts<'a> {
    pub fn rendered(&self) -> Option<String>;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StackedSlots {
    pub header: Rectangle,
    pub control: Rectangle,
    pub caption: Rectangle,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InlineSlots {
    pub label: Rectangle,
    pub control: Rectangle,
}

pub fn resolve_header<'a>(
    label: &'a str,
    required: bool,
    unit_suffix: Option<&'a str>,
    palette: &Palette,
) -> HeaderParts<'a>;
pub fn resolve_caption<'a>(
    valid: bool,
    help: Option<&'a str>,
    error: Option<&'a str>,
    palette: &Palette,
) -> CaptionParts<'a>;
pub const fn stacked_height() -> f32;
pub const fn reserved_band_height(reserved: f32, policy: OverflowPolicy) -> f32;
pub fn stacked_slots(origin_x: f32, origin_y: f32, width: f32, control_height: f32) -> StackedSlots;
pub fn inline_slots(
    origin_x: f32,
    origin_y: f32,
    label_width: f32,
    control_width: f32,
    row_height: f32,
) -> InlineSlots;
pub fn stacked<'a, Message: 'a>(
    header: HeaderParts<'a>,
    control: Element<'a, Message>,
    caption: CaptionParts<'a>,
) -> Element<'a, Message>;
pub fn stacked_reserved<'a, Message: 'a>(
    header: HeaderParts<'a>,
    control: Element<'a, Message>,
    caption: CaptionParts<'a>,
    reserved_height: f32,
    policy: OverflowPolicy,
) -> Element<'a, Message>;
pub fn inline<'a, Message: 'a>(
    header: HeaderParts<'a>,
    control: Element<'a, Message>,
    label_width: f32,
) -> Element<'a, Message>;
```

#### Examples

```rust
use hoarfrost::composite::form_field;
use hoarfrost::token::color::Palette;
use hoarfrost::widget::text_input::text_input;
use hoarfrost::widget::text_input_model::TextInputModel;

let palette = Palette::dark();
let mut model = TextInputModel::new("");
model.set_invalid(true);

let header = form_field::resolve_header("Name", true, None, &palette);
let caption = form_field::resolve_caption(
    !model.is_invalid(),
    Some("Required"),
    Some("Name cannot be empty"),
    &palette,
);
let input: iced::Element<'_, ()> = text_input(&model, |_| (), None, None, &palette).into();
let field: iced::Element<'_, ()> = form_field::stacked(header, input, caption);
```

#### Type references

- `AccordionPolicy` - Sibling collapse policy (`Independent` vs `SingleOpen`).
- `HeaderRung` - Disclosure header height tier (`Standard` 32px vs `Compact` 28px).
- `DisclosureKey` - Keyboard action (`Toggle` on Space/Enter, `Expand` on Right, `Collapse` on Left).
- `Expander` - Headless state model for a single collapsible disclosure section.
- `Accordion` - Vertically stacked group of `Expander` sections governed by `AccordionPolicy`.
- `HeaderParts` - Resolved form label, required/optional marker, unit suffix, and token colors.
- `CaptionParts` - Resolved bottom caption slot holding either help text or a `"⚠ "`-prefixed validation error (`UIF-PRIN-003`).
- `OverflowPolicy` - Multi-line error behavior inside a reserved caption band (`Clip` vs `Wrap`).

#### Return values

- `Expander::toggle`, `set_open`, and `on_key` return `Some(Commit<bool>)` only when the section is enabled and its open state changes.
- `Accordion::toggle` and `on_key` return a `Vec<(usize, Commit<bool>)>` containing the toggled section and any sibling closed by `AccordionPolicy::SingleOpen`.
- `form_field::stacked`, `stacked_reserved`, and `inline` return a live `Element<'a, Message>`.

#### Side effects

- `Expander` and `Accordion` mutate their internal boolean states in place when toggled.

#### References

- Implementation: [`src/hoarfrost/composite/expander.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/expander.rs), [`src/hoarfrost/composite/form_field.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/form_field.rs)
- Specification: `UIF-COMP-001`, `UIF-COMP-003`

---

### `composite::list_tile`, `composite::settings_row`, `composite::tree_row` - row geometry and styling (`UIF-COMP-001`, `UIF-COMP-005`, `UIF-COMP-006`)

#### Overview

The `composite::list_tile`, `composite::settings_row`, and `composite::tree_row` modules provide headless slot-geometry solvers and visual-style resolvers (`TileSlots`, `TileStyle`, `RowGeometry`, `TreeRowSlots`, `TreeRowStyle`, `Layer`) and do not construct standalone `iced` widgets directly. All three modules enforce fixed row heights, `3px` active selection bars (`UIF-PRIN-005`), and minimum hit targets (`24x24px` interactive leading slots).

#### Prerequisites & Requirements

- The caller **MUST** enable the `iced` cargo feature (`#[cfg(feature = "iced")]`).
- The caller **MUST** supply a `&Palette` from `hoarfrost::token::color::Palette`.
- When rendering deep hierarchy trees in narrow panes, the caller **MUST** clamp indentation via `tree_row::indent_geometry` so total indent never exceeds `40%` of the row width (`UIF-COMP-006`).

#### Syntax / Method Signature

```rust
// composite::list_tile
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TileLines {
    #[default]
    Single,
    Two,
}

impl TileLines {
    pub const fn has_subtitle(self) -> bool;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LeadingExtent {
    #[default]
    Small,
    Large,
}

impl LeadingExtent {
    pub const fn pixels(self) -> f32;
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Leading {
    #[default]
    None,
    Icon(LeadingExtent),
    Checkbox,
    Radio,
}

impl Leading {
    pub const fn visual_extent(self) -> f32;
    pub const fn hit_extent(self, interactive: bool) -> f32;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TileSlots {
    pub row: Rectangle,
    pub selection_bar: Rectangle,
    pub leading_visual: Option<Rectangle>,
    pub leading_hit: Option<Rectangle>,
    pub text: Rectangle,
    pub title: Rectangle,
    pub subtitle: Option<Rectangle>,
    pub trailing: Option<Rectangle>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TileStyle {
    pub background: Color,
    pub show_selection_bar: bool,
    pub selection_bar_color: Color,
    pub title_color: Color,
    pub title_weight: typography::Weight,
    pub subtitle_color: Color,
    pub focus_ring: Option<FocusIndicator>,
}

pub const fn tile_height(lines: TileLines) -> f32;
pub fn tile_slots(
    bounds: Rectangle,
    lines: TileLines,
    leading: Leading,
    leading_interactive: bool,
    trailing_width: f32,
) -> TileSlots;
pub fn resolve_style(
    selected: bool,
    disabled: bool,
    base: BaseState,
    focused: bool,
    surface: Color,
    palette: &Palette,
) -> TileStyle;
pub fn row_layer(bounds: Rectangle, style: &TileStyle) -> Option<Layer>;
pub fn selection_bar_layer(slots: &TileSlots, style: &TileStyle) -> Option<Layer>;

// composite::settings_row
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RowGeometry {
    pub row: Rectangle,
    pub text_column: Rectangle,
    pub title: Rectangle,
    pub description: Option<Rectangle>,
    pub control: Rectangle,
    pub separator: Rectangle,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RowStyle {
    pub title_color: Color,
    pub description_color: Color,
    pub separator_color: Color,
}

pub fn row_geometry(
    origin_x: f32,
    origin_y: f32,
    width: f32,
    has_description: bool,
    control_width: f32,
    control_height: f32,
) -> RowGeometry;
pub fn resolve_style(disabled: bool, palette: &Palette) -> RowStyle;
pub fn separator_layer(geom: &RowGeometry, style: &RowStyle) -> Layer;

// composite::tree_row
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IndentGeometry {
    pub step: f32,
    pub total_indent: f32,
    pub clamped: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RowPadding {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TreeRowSlots {
    pub row: Rectangle,
    pub selection_bar: Rectangle,
    pub indent: IndentGeometry,
    pub guide_x: Vec<f32>,
    pub chevron_hit: Rectangle,
    pub chevron_glyph: Rectangle,
    pub leading_visual: Option<Rectangle>,
    pub leading_hit: Option<Rectangle>,
    pub label: Rectangle,
    pub trailing: Option<Rectangle>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TreeRowStyle {
    pub background: Color,
    pub show_selection_bar: bool,
    pub selection_bar_color: Color,
    pub label_color: Color,
    pub label_weight: typography::Weight,
    pub chevron_color: Color,
    pub guide_color: Color,
    pub focus_outline: FocusIndicator,
}

pub fn indent_allowance(row_width: f32) -> f32;
pub fn indent_geometry(depth: usize, row_width: f32) -> IndentGeometry;
pub fn guide_positions(levels: usize, step: f32) -> Vec<f32>;
pub fn row_padding(indent_width: f32) -> RowPadding;
pub fn row_slots(
    row: Rectangle,
    depth: usize,
    leading: Leading,
    interactive: bool,
    trailing_width: f32,
) -> TreeRowSlots;
pub fn content_width(depth: usize, label_width: f32) -> f32;
pub fn resolve_style(
    selected: bool,
    disabled: bool,
    warning: bool,
    base: BaseState,
    surface: Color,
    palette: &Palette,
) -> TreeRowStyle;
pub fn row_layer(bounds: Rectangle, style: &TreeRowStyle) -> Option<Layer>;
pub fn selection_bar_layer(bar: Rectangle, style: &TreeRowStyle) -> Option<Layer>;
pub fn guide_layers(slots: &TreeRowSlots, style: &TreeRowStyle) -> Vec<Layer>;
```

#### Examples

```rust
use hoarfrost::composite::list_tile::{Leading, LeadingExtent};
use hoarfrost::composite::tree_row;
use hoarfrost::state::BaseState;
use hoarfrost::token::color::Palette;
use iced::Rectangle;

let palette = Palette::dark();
let bounds = Rectangle::new(iced::Point::ORIGIN, iced::Size::new(280.0, 28.0));
let slots = tree_row::row_slots(
    bounds,
    2,
    Leading::Icon(LeadingExtent::Small),
    false,
    0.0,
);
let style = tree_row::resolve_style(
    true,
    false,
    false,
    BaseState::Rest,
    palette.surface_base,
    &palette,
);
let guides = tree_row::guide_layers(&slots, &style);
assert_eq!(guides.len(), 2);
```

#### Type references

- `TileLines` - Row line-count selector (`Single` 32px vs `Two` 48px with subtitle).
- `LeadingExtent` - Optical icon size (`Small` 16px vs `Large` 20px).
- `Leading` - Optional leading slot kind (`None`, `Icon(LeadingExtent)`, `Checkbox`, `Radio`).
- `TileSlots` - Solved bounding rectangles for a list tile's selection bar, leading slot, text stack, and trailing slot.
- `TileStyle` - Resolved colors, font weights, and focus indicator for a list tile.
- `RowGeometry` - Solved two-column geometry for a settings row and its `1px` bottom separator.
- `IndentGeometry` - Effective per-level step, total indentation width, and clamp flag (`<= 40%` row width).
- `TreeRowSlots` - Solved rectangles and vertical guide X coordinates for a hierarchical tree row.
- `TreeRowStyle` - Resolved colors, font weight, guide line color, and inset focus outline for a tree row.

#### Return values

- `list_tile::tile_height` returns `32.0` for `TileLines::Single` and `48.0` for `TileLines::Two`.
- `tree_row::indent_geometry` returns an `IndentGeometry` with `clamped = true` whenever `depth * 16.0` exceeds `0.4 * row_width`.
- `tree_row::guide_layers` returns one `1px` vertical `Layer` per ancestor depth level.

#### Side effects

- None. All functions are pure geometric and color resolvers.

#### References

- Implementation: [`src/hoarfrost/composite/list_tile.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/list_tile.rs), [`src/hoarfrost/composite/settings_row.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/settings_row.rs), [`src/hoarfrost/composite/tree_row.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/tree_row.rs)
- Specification: `UIF-COMP-001`, `UIF-COMP-005`, `UIF-COMP-006`

---

### `composite::scroll_view` and `composite::virtual_list` - virtualized scrolling (`UIF-COMP-004`, `UIF-COMP-006`)

#### Overview

The `composite::scroll_view` and `composite::virtual_list` modules provide headless viewport math, scrollbar-track geometry, linear thumb-drag gesture engines, and `O(1)` fixed-row windowing models (`ViewportWindow`, `ThumbGeometry`, `ThumbDrag`, `VirtualList`) and do not construct standalone `iced` widgets directly.

#### Prerequisites & Requirements

- The caller **MUST** enable the `iced` cargo feature (`#[cfg(feature = "iced")]`).
- The caller **MUST** supply a `&Palette` from `hoarfrost::token::color::Palette` when resolving thumb, track, or tick colors.
- Dense list, tree, and table containers **MUST** use `LayoutMode::Reserved` (`10px` permanent gutter) so scrollbar appearance never reflows inner content (`UIF-COMP-004`).

#### Syntax / Method Signature

```rust
// composite::scroll_view
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Axis {
    #[default]
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LayoutMode {
    #[default]
    Overlay,
    Reserved,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TrackPhase {
    #[default]
    Hidden,
    Rest,
    Hovered,
    Dragged,
}

impl TrackPhase {
    pub const fn thumb_thickness(self) -> f32;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TickKind {
    Selection,
    DiagnosticWarning,
    DiagnosticError,
    SearchMatch,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollTick {
    pub position: f32,
    pub kind: TickKind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThumbGeometry {
    pub offset: f32,
    pub length: f32,
    pub scrollable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrackSlots {
    pub content: Rectangle,
    pub gutter: Rectangle,
    pub thumb: Option<Rectangle>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewportWindow {
    pub start: usize,
    pub end: usize,
    pub first_visible: usize,
    pub visible_count: usize,
}

#[derive(Debug)]
pub struct ThumbDrag { /* private fields */ }

impl ThumbDrag {
    pub fn new(offset: f32) -> Self;
    pub fn offset(&self) -> f32;
    pub fn is_active(&self) -> bool;
    pub fn begin(&mut self) -> GestureStart;
    pub fn drag(&mut self, offset: f32, content_len: f32, viewport_len: f32) -> Option<Live<f32>>;
    pub fn release(&mut self) -> Option<Commit<f32>>;
    pub fn cancel(&mut self) -> Option<Cancel<f32>>;
}

pub fn max_scroll(content_len: f32, viewport_len: f32) -> f32;
pub fn clamp_scroll(offset: f32, content_len: f32, viewport_len: f32) -> f32;
pub fn thumb_geometry(content_len: f32, viewport_len: f32, scroll_offset: f32) -> ThumbGeometry;
pub fn track_slots(
    viewport: Rectangle,
    axis: Axis,
    mode: LayoutMode,
    phase: TrackPhase,
    thumb: ThumbGeometry,
) -> TrackSlots;
pub fn thumb_color(phase: TrackPhase, palette: &Palette) -> Color;
pub fn track_background(phase: TrackPhase, palette: &Palette) -> Option<Color>;
pub fn tick_color(kind: TickKind, palette: &Palette) -> Color;
pub fn tick_rect(gutter: Rectangle, axis: Axis, tick: ScrollTick) -> Rectangle;
pub fn fixed_row_window(
    total_rows: usize,
    row_height: f32,
    scroll_offset: f32,
    viewport_height: f32,
    overscan: usize,
) -> ViewportWindow;
pub fn scroll_into_view(
    row: usize,
    row_height: f32,
    scroll_offset: f32,
    viewport_height: f32,
) -> f32;

// composite::virtual_list
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VirtualList {
    pub total_rows: usize,
    pub row_height: f32,
    pub overscan: usize,
    pub viewport_height: f32,
    pub scroll_offset: f32,
    pub content_max_width: f32,
    pub viewport_width: f32,
    pub scroll_x: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MountedRow {
    pub index: usize,
    pub y: f32,
    pub height: f32,
}

impl VirtualList {
    pub fn new(total_rows: usize, row_height: f32, viewport_height: f32) -> Self;
    pub const fn with_overscan(self, overscan: usize) -> Self;
    pub const fn with_horizontal(self, content_max_width: f32, viewport_width: f32) -> Self;
    pub const fn content_height(&self) -> f32;
    pub fn max_scroll_y(&self) -> f32;
    pub fn max_scroll_x(&self) -> f32;
    pub fn needs_horizontal_scroll(&self) -> bool;
    pub fn set_scroll_y(&mut self, offset: f32) -> f32;
    pub fn scroll_by_y(&mut self, delta: f32) -> f32;
    pub fn set_scroll_x(&mut self, offset: f32) -> f32;
    pub fn set_total_rows(&mut self, total_rows: usize) -> f32;
    pub fn set_viewport(&mut self, width: f32, height: f32);
    pub fn window(&self) -> ViewportWindow;
    pub fn mounted_count(&self) -> usize;
    pub fn mounted_rows(&self) -> Vec<MountedRow>;
    pub fn row_at_y(&self, content_y: f32) -> Option<usize>;
    pub fn ensure_row_visible(&mut self, row: usize) -> f32;
    pub fn step_focus(&mut self, current: usize, delta: isize) -> Option<usize>;
    pub fn page_rows(&self) -> usize;
}
```

#### Examples

```rust
use hoarfrost::composite::scroll_view::{self, Axis, LayoutMode, TrackPhase};
use hoarfrost::composite::virtual_list::VirtualList;
use iced::Rectangle;

let mut vlist = VirtualList::new(10_000, 28.0, 280.0);
vlist.ensure_row_visible(500);
let win = vlist.window();
assert!(win.start <= 500 && 500 < win.end);

let thumb = scroll_view::thumb_geometry(vlist.content_height(), 280.0, vlist.scroll_offset);
let slots = scroll_view::track_slots(
    Rectangle::with_size(iced::Size::new(400.0, 280.0)),
    Axis::Vertical,
    LayoutMode::Reserved,
    TrackPhase::Rest,
    thumb,
);
assert_eq!(slots.content.width, 390.0);
```

#### Type references

- `Axis` - Scrollbar orientation (`Vertical` vs `Horizontal`).
- `LayoutMode` - Gutter allocation policy (`Overlay` floats over content; `Reserved` deducts `10px`).
- `TrackPhase` - Interactive scrollbar phase (`Hidden` 0px, `Rest` 6px, `Hovered` 10px, `Dragged` 10px).
- `TickKind` - Semantic category for a `2px` scrollbar annotation tick.
- `ScrollTick` - Normalized `[0.0, 1.0]` position and `TickKind` for a gutter annotation marker.
- `ThumbGeometry` - Solved thumb offset, length (`>= 24px` minimum), and scrollability flag.
- `ViewportWindow` - Half-open `[start, end)` row range including overscan buffers.
- `ThumbDrag` - Linear gesture state machine emitting `Live<f32>`, `Commit<f32>`, and `Cancel<f32>`.
- `VirtualList` - Fixed-row virtualized viewport calculator capping mounted rows to `visible + 2 * overscan`.
- `MountedRow` - Index, Y coordinate, and height of a single mounted row inside the active window.

#### Return values

- `thumb_geometry` enforces `MIN_THUMB_LENGTH` (`24.0px`) and returns `scrollable = false` when `content_len <= viewport_len`.
- `ThumbDrag::cancel` restores the pre-drag scroll offset and returns `Some(Cancel<f32>)`.
- `VirtualList::mounted_rows` returns at most `ceil(viewport_height / row_height) + 2 * overscan` rows regardless of `total_rows`.

#### Side effects

- `ThumbDrag` and `VirtualList` mutate their own offset/gesture fields in place.

#### References

- Implementation: [`src/hoarfrost/composite/scroll_view.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/scroll_view.rs), [`src/hoarfrost/composite/virtual_list.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/virtual_list.rs)
- Specification: `UIF-COMP-004`, `UIF-COMP-006`

---

### `composite::table_column`, `composite::table_header`, `composite::table_cell`, `composite::table_view` - data tables (`UIF-COMP-007`)

#### Overview

The `composite::table_column`, `composite::table_header`, and `composite::table_cell` modules provide headless column-width solvers, sort-cycle state machines, and cell-style resolvers, while `composite::table_view` provides a headless state model (`TableModel`) and view builder (`TableView`) that renders into a live `iced` widget (`Element<'a, Message>`) via `TableView::render`.

#### Prerequisites & Requirements

- The caller **MUST** enable the `iced` cargo feature (`#[cfg(feature = "iced")]`).
- Row identifiers `Id` **MUST** implement `Clone + Ord`.
- Column identifiers **MUST** use `ColId` (`u16`).
- Numeric and timestamp columns **MUST** use `CellAlign::Right` with `FontFamily::Mono` so decimal points and digits align vertically (`UIF-COMP-007`).

#### Syntax / Method Signature

```rust
// composite::table_column
pub type ColId = u16;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WidthPolicy {
    Fixed(f32),
    Content(f32),
    Proportional(f32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellAlign {
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ColumnSpec {
    pub id: ColId,
    pub label: String,
    pub width_policy: WidthPolicy,
    pub min_width: f32,
    pub max_width: Option<f32>,
    pub align: CellAlign,
    pub sortable: bool,
    pub resizable: bool,
}

impl ColumnSpec {
    pub fn new(id: ColId, label: impl Into<String>, width_policy: WidthPolicy) -> Self;
    pub const fn with_bounds(self, min_width: f32, max_width: Option<f32>) -> Self;
    pub const fn with_align(self, align: CellAlign) -> Self;
    pub const fn with_sortable(self, sortable: bool) -> Self;
    pub const fn with_resizable(self, resizable: bool) -> Self;
    pub fn effective_min(&self) -> f32;
    pub fn clamp_width(&self, width: f32) -> f32;
    pub fn base_width(&self) -> f32;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidthRegime {
    Surplus,
    Deficit,
    Oversubscribed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SolvedColumns {
    pub widths: Vec<f32>,
    pub x_offsets: Vec<f32>,
    pub total_width: f32,
    pub effective_viewport: f32,
    pub regime: WidthRegime,
    pub horizontal_overflow: bool,
}

pub fn solve_widths(
    columns: &[ColumnSpec],
    overrides: &[Option<f32>],
    available_width: f32,
    reserve_gutter: bool,
) -> SolvedColumns;

// composite::table_header
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl SortDirection {
    pub const fn glyph(self) -> &'static str;
}

pub const fn next_sort_state(current: Option<SortDirection>) -> Option<SortDirection>;
pub fn apply_sort_click(
    active: Option<(ColId, SortDirection)>,
    clicked: ColId,
    sortable: bool,
) -> Option<Option<(ColId, SortDirection)>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeaderPress {
    Resize { column: usize },
    Sort { column: usize },
    Nothing,
}

pub fn classify_header_press(
    columns: &[ColumnSpec],
    solved: &SolvedColumns,
    content_x: f32,
) -> HeaderPress;

#[derive(Debug)]
pub struct ColumnResize { /* private fields */ }

impl ColumnResize {
    pub fn new(column: ColId, spec: ColumnSpec, start_width: f32) -> Self;
    pub const fn column(&self) -> ColId;
    pub const fn width(&self) -> f32;
    pub fn is_active(&self) -> bool;
    pub fn begin(&mut self, pointer_x: f32) -> GestureStart;
    pub fn drag(&mut self, pointer_x: f32) -> Option<Live<f32>>;
    pub fn release(&mut self) -> Option<Commit<f32>>;
    pub fn cancel(&mut self) -> Option<Cancel<f32>>;
}

// composite::table_cell
#[derive(Clone, Debug, PartialEq)]
pub enum CellContent {
    Text { value: String },
    Mono { value: String },
    Badge { label: String, color: Color },
    Progress { fraction: f32, label: Option<String> },
    Action { id: String, label: String, enabled: bool },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CellAction {
    Invoke(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellBand {
    Base,
    Raised,
    Hover,
    Selected,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellStyle {
    pub background: Color,
    pub text_color: Color,
    pub bottom_border: Color,
    pub focus_outline: Option<FocusIndicator>,
}

impl CellStyle {
    pub fn resolve(band: CellBand, focused: bool, disabled: bool, palette: &Palette) -> Self;
}

// composite::table_view
#[derive(Clone, Debug, PartialEq)]
pub struct TableRow<Id> {
    pub id: Id,
    pub cells: Vec<CellContent>,
    pub disabled: bool,
}

impl<Id> TableRow<Id> {
    pub fn new(id: Id, cells: Vec<CellContent>) -> Self;
    pub const fn with_disabled(self, disabled: bool) -> Self;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TableFocus<Id> {
    Header { column: ColId },
    Body { row: Id, column: ColId },
}

#[derive(Clone, Debug, PartialEq)]
pub enum TableEvent<Id> {
    SelectionChanged(BTreeSet<Id>),
    CellAction(Id, ColId, CellAction),
    SortChanged(Option<(ColId, SortDirection)>),
    ColumnResizeLive(ColId, f32),
    ColumnResizeCommit(ColId, f32),
    ColumnResizeCancel(ColId, f32),
}

#[derive(Clone, Debug)]
pub struct TableModel<Id: Clone + Ord> { /* private fields */ }

impl<Id: Clone + Ord> TableModel<Id> {
    pub fn new(
        columns: Vec<ColumnSpec>,
        rows: Vec<TableRow<Id>>,
        mode: TreeSelectionMode,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Self;
    pub const fn with_zebra(self, zebra: bool) -> Self;
    pub fn columns(&self) -> &[ColumnSpec];
    pub fn rows(&self) -> &[TableRow<Id>];
    pub fn solved_columns(&self) -> &SolvedColumns;
    pub const fn sort(&self) -> Option<(ColId, SortDirection)>;
    pub const fn selection(&self) -> &BTreeSet<Id>;
    pub const fn focus(&self) -> Option<&TableFocus<Id>>;
    pub const fn virtual_list(&self) -> &VirtualList;
    pub fn set_viewport(&mut self, width: f32, height: f32);
    pub fn set_scroll_y(&mut self, offset: f32) -> f32;
    pub fn set_scroll_x(&mut self, offset: f32) -> f32;
    pub fn click_header(&mut self, col_id: ColId) -> Option<Commit<Option<(ColId, SortDirection)>>>;
    pub fn set_column_width(&mut self, col_id: ColId, width: f32) -> Option<f32>;
    pub fn click_row(&mut self, id: &Id, shift: bool, ctrl: bool) -> Option<Commit<BTreeSet<Id>>>;
    pub fn select_all(&mut self) -> Option<Commit<BTreeSet<Id>>>;
    pub fn focus_cell(&mut self, row: Id, column: ColId) -> bool;
    pub fn move_focus(&mut self, d_row: isize, d_col: isize) -> Option<TableFocus<Id>>;
    pub fn activate_focused_cell(&mut self) -> Option<Commit<(Id, ColId, CellAction)>>;
}

pub struct TableView<'a, Id: Clone + Ord> {
    pub model: &'a TableModel<Id>,
}

impl<'a, Id: Clone + Ord> TableView<'a, Id> {
    pub const fn new(model: &'a TableModel<Id>) -> Self;
    pub fn render<Message: 'a + Clone>(
        &self,
        palette: &Palette,
        on_event: impl Fn(TableEvent<Id>) -> Message + 'a,
    ) -> Element<'a, Message>;
}
```

#### Examples

```rust
use hoarfrost::composite::table_cell::CellContent;
use hoarfrost::composite::table_column::{CellAlign, ColumnSpec, WidthPolicy};
use hoarfrost::composite::table_view::{TableEvent, TableModel, TableRow, TableView};
use hoarfrost::composite::tree_view::TreeSelectionMode;
use hoarfrost::token::color::Palette;

let cols = vec![
    ColumnSpec::new(0, "Host", WidthPolicy::Proportional(1.0)),
    ColumnSpec::new(1, "Latency", WidthPolicy::Fixed(96.0)).with_align(CellAlign::Right),
];
let rows = vec![TableRow::new(
    1u32,
    vec![
        CellContent::Text { value: "edge-01".into() },
        CellContent::Mono { value: "1.4 ms".into() },
    ],
)];
let model = TableModel::new(cols, rows, TreeSelectionMode::Single, 480.0, 240.0);
let palette = Palette::dark();
let element: iced::Element<'_, TableEvent<u32>> =
    TableView::new(&model).render(&palette, |ev| ev);
```

#### Type references

- `ColId` - Column identifier alias (`u16`).
- `WidthPolicy` - Column sizing policy (`Fixed(f32)`, `Content(f32)`, or `Proportional(f32)`).
- `ColumnSpec` - Column definition including label, alignment, sortability, and resize bounds.
- `SolvedColumns` - Solved per-column pixel widths, X offsets, and `WidthRegime`.
- `SortDirection` - Tri-state sort direction (`Ascending` `"▲"`, `Descending` `"▼"`, or `None`).
- `HeaderPress` - Hit-test classification (`Resize`, `Sort`, or `Nothing`) giving the `10px` resize grab zone priority over sort clicks.
- `ColumnResize` - Linear drag gesture state machine for column border resizing.
- `CellContent` - Typed cell payload (`Text`, `Mono`, `Badge`, `Progress`, or `Action`).
- `CellBand` - Row surface band (`Base`, `Raised`, `Hover`, or `Selected`).
- `CellStyle` - Resolved cell background, text color, bottom border, and inset focus outline.
- `TableFocus` - Active 2D keyboard focus coordinate (`Header { column }` or `Body { row, column }`).
- `TableEvent` - Unified event enum emitted by `TableView::render`.
- `TableModel` - Headless state model combining column solver, sort state, selection, 2D focus, and `VirtualList`.

#### Return values

- `solve_widths` deducts the `10px` scrollbar gutter when `reserve_gutter` is `true` and sets `horizontal_overflow = true` only in `WidthRegime::Oversubscribed`.
- `TableView::render` returns a live `Element<'a, Message>`.

#### Side effects

- `TableModel` methods mutate selection, sort state, column width overrides, and scroll offsets in place.

#### References

- Implementation: [`src/hoarfrost/composite/table_column.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/table_column.rs), [`src/hoarfrost/composite/table_header.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/table_header.rs), [`src/hoarfrost/composite/table_cell.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/table_cell.rs), [`src/hoarfrost/composite/table_view.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/table_view.rs)
- Specification: `UIF-COMP-007`

---

### `composite::tree_view` - hierarchical trees (`UIF-COMP-006`)

#### Overview

The `composite::tree_view` module provides a headless hierarchical tree state model (`TreeModel`) and a view builder (`TreeView`) that renders into a live `iced` widget (`Element<'a, Message>`) via `TreeView::render` or `TreeView::render_with_footer`. It supports single and extended multi-selection, WAI-ARIA TreeView keyboard navigation, ancestor-matched filter projection, and an optional pinned search footer.

#### Prerequisites & Requirements

- The caller **MUST** enable the `iced` cargo feature (`#[cfg(feature = "iced")]`).
- Node identifiers `Id` **MUST** implement `Clone + Ord`.
- The caller **MUST** supply a `&Palette` from `hoarfrost::token::color::Palette` when rendering.

#### Syntax / Method Signature

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreeSelectionMode {
    #[default]
    Single,
    ExtendedMulti,
}

impl TreeSelectionMode {
    pub const fn is_multi(self) -> bool;
}

#[derive(Clone, Debug, PartialEq)]
pub struct TreeNode<Id> {
    pub id: Id,
    pub label: String,
    pub children: Vec<TreeNode<Id>>,
    pub expandable: bool,
    pub disabled: bool,
    pub warning: bool,
    pub leading: Leading,
}

impl<Id> TreeNode<Id> {
    pub fn leaf(id: Id, label: impl Into<String>) -> Self;
    pub fn branch(id: Id, label: impl Into<String>, children: Vec<TreeNode<Id>>) -> Self;
    pub const fn with_disabled(self, disabled: bool) -> Self;
    pub const fn with_warning(self, warning: bool) -> Self;
    pub const fn with_leading(self, leading: Leading) -> Self;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlatNode<Id> {
    pub id: Id,
    pub label: String,
    pub depth: usize,
    pub is_branch: bool,
    pub is_open: bool,
    pub disabled: bool,
    pub warning: bool,
    pub leading: Leading,
    pub parent: Option<Id>,
    pub matched_filter: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TreeEvent<Id> {
    SelectionChanged(BTreeSet<Id>),
    Activate(Id),
    ExpandToggle(Id, bool),
    ContextMenu(BTreeSet<Id>, Point),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeKey {
    Down,
    Up,
    Right,
    Left,
    Home,
    End,
    PageDown,
    PageUp,
    Enter,
    Space,
    SelectAll,
}

#[derive(Clone, Debug)]
pub struct TreeModel<Id: Clone + Ord> { /* private fields */ }

impl<Id: Clone + Ord> TreeModel<Id> {
    pub fn new(
        roots: Vec<TreeNode<Id>>,
        expanded: BTreeSet<Id>,
        mode: TreeSelectionMode,
        viewport_height: f32,
    ) -> Self;
    pub fn visible(&self) -> &[FlatNode<Id>];
    pub fn visible_count(&self) -> usize;
    pub const fn selection(&self) -> &BTreeSet<Id>;
    pub const fn focused(&self) -> Option<&Id>;
    pub const fn expanded(&self) -> &BTreeSet<Id>;
    pub fn is_expanded(&self, id: &Id) -> bool;
    pub const fn virtual_list(&self) -> &VirtualList;
    pub fn filter(&self) -> &str;
    pub fn set_filter(&mut self, query: impl Into<String>);
    pub fn toggle_expand(&mut self, id: &Id) -> Option<Commit<(Id, bool)>>;
    pub fn click_row(&mut self, id: &Id, shift: bool, ctrl: bool) -> Option<Commit<BTreeSet<Id>>>;
    pub fn right_click_row(
        &mut self,
        id: &Id,
        point: Point,
    ) -> Option<(Option<Commit<BTreeSet<Id>>>, Commit<(BTreeSet<Id>, Point)>)>;
    pub fn select_all(&mut self) -> Option<Commit<BTreeSet<Id>>>;
    pub fn on_key(&mut self, key: TreeKey, shift: bool, ctrl: bool) -> Vec<TreeEvent<Id>>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchFooterConfig {
    pub placeholder: String,
    pub query: String,
    pub match_count: Option<usize>,
}

impl SearchFooterConfig {
    pub fn new(query: impl Into<String>) -> Self;
    pub fn with_placeholder(self, placeholder: impl Into<String>) -> Self;
    pub const fn with_match_count(self, count: usize) -> Self;
}

pub struct TreeView<'a, Id: Clone + Ord> {
    pub model: &'a TreeModel<Id>,
    pub row_width: f32,
    pub search_footer: Option<SearchFooterConfig>,
}

impl<'a, Id: Clone + Ord> TreeView<'a, Id> {
    pub const fn new(model: &'a TreeModel<Id>, row_width: f32) -> Self;
    pub fn with_search_footer(self, footer: impl Into<SearchFooterConfig>) -> Self;
    pub const fn has_search_footer(&self) -> bool;
    pub const fn search_footer(&self) -> Option<&SearchFooterConfig>;
    pub fn render<Message: 'a + Clone>(
        &self,
        palette: &Palette,
        on_event: impl Fn(TreeEvent<Id>) -> Message + 'a,
    ) -> Element<'a, Message>;
    pub fn render_with_footer<Message: 'a + Clone>(
        &self,
        palette: &Palette,
        on_event: impl Fn(TreeEvent<Id>) -> Message + 'a,
        footer_input: Element<'a, Message>,
    ) -> Element<'a, Message>;
}
```

#### Examples

```rust
use std::collections::BTreeSet;
use hoarfrost::composite::tree_view::{
    TreeEvent, TreeKey, TreeModel, TreeNode, TreeSelectionMode, TreeView,
};
use hoarfrost::token::color::Palette;

let roots = vec![TreeNode::branch(
    "src",
    "src",
    vec![TreeNode::leaf("lib.rs", "lib.rs")],
)];
let mut expanded = BTreeSet::new();
expanded.insert("src");

let mut tree = TreeModel::new(roots, expanded, TreeSelectionMode::Single, 280.0);
let events = tree.on_key(TreeKey::Down, false, false);
assert!(!events.is_empty());

let palette = Palette::dark();
let element: iced::Element<'_, TreeEvent<&str>> =
    TreeView::new(&tree, 260.0).render(&palette, |ev| ev);
```

#### Type references

- `TreeSelectionMode` - Selection cardinality (`Single` vs `ExtendedMulti`).
- `TreeNode` - Declarative hierarchy node (`leaf` or `branch`).
- `FlatNode` - Flattened visible row projection with depth, expansion state, parent pointer, and filter-match flag.
- `TreeEvent` - Output event (`SelectionChanged`, `Activate`, `ExpandToggle`, or `ContextMenu`).
- `TreeKey` - Keyboard navigation and activation command for `TreeModel::on_key`.
- `TreeModel` - Headless state machine managing expansion, filter projection, selection, roving focus, and `VirtualList` windowing.
- `SearchFooterConfig` - Configuration for the optional `36px` bottom search footer.
- `TreeView` - View builder rendering the visible window of a `TreeModel` into an `iced::Element`.

#### Return values

- `TreeModel::on_key` returns a `Vec<TreeEvent<Id>>` (0, 1, or 2 events depending on focus movement and selection changes).
- `TreeView::render` and `TreeView::render_with_footer` return a live `Element<'a, Message>`.

#### Side effects

- `TreeModel` mutates its flattened projection, expansion set, selection set, focus, and scroll offset in place.

#### References

- Implementation: [`src/hoarfrost/composite/tree_view.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/tree_view.rs)
- Specification: `UIF-COMP-006`

---

### `composite::tab_strip`, `composite::navbar`, `composite::navigation_rail` - workspace navigation (`UIF-TAB-001..003`, `UIF-NAV-001..003`, `UIF-COMP-009`)

#### Overview

The `composite::tab_strip` and `composite::navbar` modules provide headless state models (`TabStripModel`, `NavBarModel`) with rendering helpers (`tab_strip`, `navbar`) that produce live `iced` widgets (`Element<'a, Message>`), while `composite::navigation_rail` provides a live `iced` widget (`NavigationRail`, `Element<'a, Message>`) backed by `NavigationRailModel`.

#### Prerequisites & Requirements

- The caller **MUST** enable the `iced` cargo feature (`#[cfg(feature = "iced")]`).
- When `TabStripModel::request_close` returns `TabCloseVerdict::RequiresConfirmation { tab_id, title }`, the application **MUST** prompt the user before calling `confirm_close` (`UIF-TAB-003`).
- A navbar menu trigger with `PromptBehaviour::LaunchesPrompt` **MUST** end with an unspaced ASCII ellipsis (`"..."`) and **MUST NOT** use a spaced ellipsis (`" ..."`) (`UIF-NAV-002`).

#### Syntax / Method Signature

```rust
// composite::tab_strip
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabItemModel {
    pub id: String,
    pub title: String,
    pub dirty: bool,
    pub pinned: bool,
    pub icon: Option<String>,
}

impl TabItemModel {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self;
    pub fn with_dirty(self, dirty: bool) -> Self;
    pub fn with_pinned(self, pinned: bool) -> Self;
    pub fn with_icon(self, icon: impl Into<String>) -> Self;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TabCloseVerdict {
    CloseImmediately { new_active_id: Option<String> },
    RequiresConfirmation { tab_id: String, title: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TabStripModel {
    pub tabs: Vec<TabItemModel>,
    pub active_id: Option<String>,
    pub scroll_offset: f32,
    pub overflow_menu_open: bool,
}

impl TabStripModel {
    pub fn new(tabs: Vec<TabItemModel>, active_id: Option<String>) -> Self;
    pub fn select(&mut self, id: &str) -> bool;
    pub fn request_close(&mut self, id: &str) -> TabCloseVerdict;
    pub fn confirm_close(&mut self, id: &str) -> Option<String>;
    pub fn external_deletion_sync(&mut self, deleted_id: &str) -> Option<String>;
    pub fn reorder(&mut self, from_index: usize, to_index: usize) -> bool;
    pub fn visible_and_overflow(&self, available_width: f32) -> (Vec<&TabItemModel>, Vec<&TabItemModel>);
}

pub fn tab_strip<'a, Message: Clone + 'a>(
    model: &'a TabStripModel,
    on_select: impl Fn(String) -> Message + 'a,
    on_close_request: impl Fn(String) -> Message + 'a,
    on_overflow_toggle: Message,
) -> Element<'a, Message>;

// composite::navbar
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptBehaviour {
    Direct,
    LaunchesPrompt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavBarMenuTrigger {
    pub id: String,
    pub label: String,
    pub prompt_behaviour: PromptBehaviour,
}

impl NavBarMenuTrigger {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        prompt_behaviour: PromptBehaviour,
    ) -> Self;
    pub fn validate_ellipsis(&self) -> Result<(), String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavBarToolButton {
    pub id: String,
    pub icon_glyph: String,
    pub tooltip: String,
    pub active: bool,
}

impl NavBarToolButton {
    pub fn new(
        id: impl Into<String>,
        icon_glyph: impl Into<String>,
        tooltip: impl Into<String>,
    ) -> Self;
    pub fn with_active(self, active: bool) -> Self;
    pub fn has_valid_tooltip(&self) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavBarModel {
    pub menu_triggers: Vec<NavBarMenuTrigger>,
    pub tool_buttons: Vec<NavBarToolButton>,
    pub context_title: String,
    pub active_menu_id: Option<String>,
}

impl NavBarModel {
    pub fn new(context_title: impl Into<String>) -> Self;
    pub fn with_menus(self, menus: Vec<NavBarMenuTrigger>) -> Self;
    pub fn with_tools(self, tools: Vec<NavBarToolButton>) -> Self;
    pub fn validate(&self) -> Result<(), Vec<String>>;
}

pub fn navbar<'a, Message: Clone + 'a>(
    model: &'a NavBarModel,
    on_menu_click: impl Fn(String) -> Message + 'a,
    on_tool_click: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message>;

// composite::navigation_rail
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavDestination<DestId> {
    pub id: DestId,
    pub icon: Icon,
    pub label: String,
    pub badge_count: Option<u32>,
    pub disabled: bool,
}

impl<DestId> NavDestination<DestId> {
    pub fn new(id: DestId, icon: impl Into<Icon>, label: impl Into<String>) -> Self;
    pub const fn badge(self, count: u32) -> Self;
    pub const fn disabled(self, disabled: bool) -> Self;
    pub fn formatted_badge(&self) -> Option<String>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct NavigationRailModel<DestId> {
    pub destinations: Vec<NavDestination<DestId>>,
    pub bottom_destinations: Vec<NavDestination<DestId>>,
    pub active_id: DestId,
    pub expanded: bool,
    pub width: f32,
}

impl<DestId: Clone + PartialEq> NavigationRailModel<DestId> {
    pub fn new(
        destinations: Vec<NavDestination<DestId>>,
        bottom_destinations: Vec<NavDestination<DestId>>,
        active_id: DestId,
    ) -> Self;
    pub fn with_expanded(self, expanded: bool) -> Self;
    pub fn with_width(self, width: f32) -> Self;
    pub fn set_expanded(&mut self, expanded: bool);
    pub fn set_width(&mut self, width: f32);
    pub fn step_focus(&self, current: &DestId, delta: isize) -> Option<DestId>;
    pub fn home_destination(&self) -> Option<DestId>;
    pub fn end_destination(&self) -> Option<DestId>;
}

pub struct NavigationRail<'a, Message, DestId, Theme = iced::Theme, Renderer = iced::Renderer> { /* private fields */ }

impl<'a, Message, DestId: 'a + Clone + PartialEq, Theme, Renderer>
    NavigationRail<'a, Message, DestId, Theme, Renderer>
{
    pub fn new(model: NavigationRailModel<DestId>) -> Self;
    pub fn on_select(self, handler: impl Fn(Commit<DestId>) -> Message + 'a) -> Self;
    pub fn on_expand_toggle(self, handler: impl Fn(Commit<bool>) -> Message + 'a) -> Self;
    pub fn on_resize_change(self, handler: impl Fn(Live<f32>) -> Message + 'a) -> Self;
    pub fn on_resize_commit(self, handler: impl Fn(Commit<f32>) -> Message + 'a) -> Self;
    pub fn on_resize_cancel(self, handler: impl Fn(Cancel<f32>) -> Message + 'a) -> Self;
    pub fn palette(self, palette: Palette) -> Self;
    pub fn model(&self) -> &NavigationRailModel<DestId>;
    pub fn current_width(&self) -> f32;
    pub fn is_expanded(&self) -> bool;
    pub fn toggle_rect(&self, bounds: Rectangle) -> Rectangle;
    pub fn hit_destination(
        &self,
        bounds: Rectangle,
        pos: Point,
        scroll_offset: f32,
    ) -> Option<(usize, &NavDestination<DestId>)>;
}

pub fn navigation_rail<'a, Message, DestId: 'static + Clone + PartialEq, Theme, Renderer>(
    model: NavigationRailModel<DestId>,
) -> NavigationRail<'a, Message, DestId, Theme, Renderer>;
```

#### Examples

```rust
use hoarfrost::composite::navbar::{NavBarMenuTrigger, PromptBehaviour};
use hoarfrost::composite::tab_strip::{TabCloseVerdict, TabItemModel, TabStripModel};

let mut tabs = TabStripModel::new(
    vec![TabItemModel::new("a.rs", "a.rs").with_dirty(true)],
    Some("a.rs".into()),
);
if let TabCloseVerdict::RequiresConfirmation { tab_id, title } = tabs.request_close("a.rs") {
    assert_eq!(tab_id, "a.rs");
    assert_eq!(title, "a.rs");
}

let trigger = NavBarMenuTrigger::new("file.open", "Open...", PromptBehaviour::LaunchesPrompt);
assert!(trigger.validate_ellipsis().is_ok());
```

#### Type references

- `TabItemModel` - Single tab metadata including `id`, `title`, `dirty` indicator, and `pinned` flag.
- `TabCloseVerdict` - Outcome of `TabStripModel::request_close` (`CloseImmediately` vs `RequiresConfirmation` for dirty tabs).
- `TabStripModel` - Document tab strip state model enforcing pinned-left ordering and adjacent fallback activation on close.
- `PromptBehaviour` - Declares whether a navbar item executes immediately (`Direct`) or opens a modal dialog (`LaunchesPrompt`).
- `NavBarMenuTrigger` - Top-bar menu trigger validated for ellipsis rules (`"..."`).
- `NavBarToolButton` - Icon-only top-bar button requiring a non-empty tooltip.
- `NavDestination` - Single navigation rail destination with icon, label, and optional badge count (`99+` cap).
- `NavigationRailModel` - State model for the collapsible left navigation rail (`48px` compact vs `160px..280px` expanded).
- `NavigationRail` - Live `iced` widget supporting destination selection, expand/collapse toggle, and right-edge drag resizing.

#### Return values

- `TabStripModel::confirm_close` and `external_deletion_sync` remove the target tab and return `Option<String>` containing the newly active tab's ID (`self.active_id.clone()`), or `None` when no tabs remain.
- `NavBarMenuTrigger::validate_ellipsis` returns `Ok(())` when `LaunchesPrompt` ends with `"..."` (and not `" ..."`) or when `Direct` does not end with `"..."`.

#### Side effects

- `TabStripModel::request_close` immediately removes clean tabs (`dirty == false`) from `self.tabs`, whereas dirty tabs remain in `self.tabs` until `confirm_close` is called.

#### References

- Implementation: [`src/hoarfrost/composite/tab_strip.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/tab_strip.rs), [`src/hoarfrost/composite/navbar.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/navbar.rs), [`src/hoarfrost/composite/navigation_rail.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/navigation_rail.rs)
- Specification: `UIF-TAB-001`, `UIF-TAB-002`, `UIF-TAB-003`, `UIF-NAV-001`, `UIF-NAV-002`, `UIF-NAV-003`, `UIF-COMP-009`

---

### `composite::notification`, `composite::info_bar`, `composite::toast` - inline alerts and toast stack (`UIF-COMP-002`)

#### Overview

The `composite::notification`, `composite::info_bar`, and `composite::toast` modules provide headless state machines, layout geometries, and draw-layer resolvers (`NotificationRecord`, `InfoBarLayout`, `ToastStack`) and do not construct standalone `iced` widgets directly. Inline info bars occupy Band 0 document flow, while toasts stack in Band 4 (`ZBand::Toast`) with hover/focus timer pause and `Warning`/`Danger` persistence (`UIF-COMP-002`).

#### Prerequisites & Requirements

- The caller **MUST** enable the `iced` cargo feature (`#[cfg(feature = "iced")]`).
- The caller **MUST** supply a `&Palette` from `hoarfrost::token::color::Palette`.
- The application **MUST NOT** auto-dismiss `Severity::Warning` or `Severity::Danger` notifications (`effective_auto_dismiss_ms` unconditionally returns `None` for both severities).

#### Syntax / Method Signature

```rust
// composite::notification
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotificationAction<ActionId> {
    pub id: ActionId,
    pub label: String,
}

impl<ActionId> NotificationAction<ActionId> {
    pub fn new(id: ActionId, label: impl Into<String>) -> Self;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DismissReason {
    User,
    Action,
    Timeout,
    Overflow,
    Escape,
    Host,
}

impl DismissReason {
    pub const fn acknowledged(self) -> bool;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotificationRecord<ActionId> {
    pub id: u64,
    pub severity: Severity,
    pub title: String,
    pub message: Option<String>,
    pub action: Option<NotificationAction<ActionId>>,
    pub dismissible: bool,
    pub auto_dismiss_ms: Option<u64>,
}

impl<ActionId> NotificationRecord<ActionId> {
    pub fn new(id: u64, severity: Severity, title: impl Into<String>) -> Self;
    pub fn with_message(self, message: impl Into<String>) -> Self;
    pub fn with_action(self, action: NotificationAction<ActionId>) -> Self;
    pub const fn with_dismissible(self, dismissible: bool) -> Self;
    pub const fn with_auto_dismiss_ms(self, ms: Option<u64>) -> Self;
    pub const fn effective_auto_dismiss_ms(&self) -> Option<u64>;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NotificationPalette {
    pub fill: Color,
    pub accent: Color,
    pub border: Color,
    pub title: Color,
    pub message: Color,
    pub glyph: &'static str,
}

pub fn resolve_palette(severity: Severity, palette: &Palette) -> NotificationPalette;

// composite::info_bar
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InfoBarLayout {
    pub bounds: Rectangle,
    pub accent_bar: Rectangle,
    pub icon: Rectangle,
    pub text_block: Rectangle,
    pub title: Rectangle,
    pub message: Option<Rectangle>,
    pub action: Option<Rectangle>,
    pub dismiss: Option<Rectangle>,
}

pub fn min_height(has_message: bool) -> f32;
pub fn layout<ActionId>(
    origin: Point,
    width: f32,
    record: &NotificationRecord<ActionId>,
    action_label_width: f32,
) -> InfoBarLayout;
pub fn layers(layout: &InfoBarLayout, palette: &Palette, severity: Severity) -> Vec<Layer>;
pub fn user_dismiss<ActionId>(record: &NotificationRecord<ActionId>) -> Option<Commit<DismissReason>>;
pub fn invoke_action<ActionId: Clone>(
    record: &NotificationRecord<ActionId>,
) -> Option<(Commit<ActionId>, Commit<DismissReason>)>;

// composite::toast
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimerState {
    Never,
    Running { remaining_ms: u64 },
    Paused { remaining_ms: u64 },
    Expired,
}

impl TimerState {
    pub const fn remaining_ms(self) -> Option<u64>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToastEntry<ActionId> {
    pub record: NotificationRecord<ActionId>,
    pub timer: TimerState,
    pub hovered: bool,
    pub focused: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PushOutcome<ActionId> {
    Shown {
        evicted: Option<Box<ToastEntry<ActionId>>>,
    },
    Queued,
}

#[derive(Clone, Debug, Default)]
pub struct ToastStack<ActionId> { /* private fields */ }

impl<ActionId: Clone> ToastStack<ActionId> {
    pub const fn new() -> Self;
    pub fn visible(&self) -> &[ToastEntry<ActionId>];
    pub fn queued_len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn push(&mut self, record: NotificationRecord<ActionId>) -> PushOutcome<ActionId>;
    pub fn set_hovered(&mut self, id: u64, hovered: bool) -> bool;
    pub fn set_focused(&mut self, id: u64, focused: bool) -> bool;
    pub fn tick(&mut self, elapsed_ms: u64) -> Vec<(u64, Commit<DismissReason>)>;
    pub fn dismiss(
        &mut self,
        id: u64,
        reason: DismissReason,
    ) -> Option<(NotificationRecord<ActionId>, Commit<DismissReason>)>;
    pub fn dismiss_topmost_on_escape(&mut self) -> Option<(u64, Commit<DismissReason>)>;
    pub fn invoke_action(
        &mut self,
        id: u64,
    ) -> Option<(Commit<ActionId>, Commit<DismissReason>)>;
    pub fn layout(&self, viewport: Rectangle) -> Vec<(u64, InfoBarLayout)>;
}
```

#### Examples

```rust
use hoarfrost::composite::notification::{DismissReason, NotificationRecord};
use hoarfrost::composite::toast::{PushOutcome, ToastStack};
use hoarfrost::token::color::Severity;

let mut stack: ToastStack<&str> = ToastStack::new();
let danger = NotificationRecord::new(1, Severity::Danger, "Disk full")
    .with_auto_dismiss_ms(Some(5_000));
assert_eq!(danger.effective_auto_dismiss_ms(), None);

let outcome = stack.push(danger);
assert!(matches!(outcome, PushOutcome::Shown { evicted: None }));
assert!(stack.tick(10_000).is_empty());

let dismissed = stack.dismiss_topmost_on_escape();
assert_eq!(dismissed.unwrap().1.into_inner(), DismissReason::Escape);
```

#### Type references

- `NotificationAction` - Single optional CTA button descriptor (`id` and `label`).
- `DismissReason` - Reason a notification closed (`User`, `Action`, `Timeout`, `Overflow`, `Escape`, or `Host`).
- `NotificationRecord` - Severity, title, optional body message, optional CTA, and auto-dismiss policy for an alert.
- `NotificationPalette` - Resolved fill, `3px` accent bar color, border color, text colors, and redundant severity glyph (`ⓘ`, `✓`, `⚠`, `✓`).
- `InfoBarLayout` - Solved rectangles for an inline banner or toast card.
- `TimerState` - Auto-dismiss countdown state (`Never`, `Running`, `Paused`, or `Expired`).
- `ToastEntry` - Mounted toast combining a `NotificationRecord` with its `TimerState` and hover/focus flags.
- `PushOutcome` - Result of `ToastStack::push` (`Shown { evicted }` vs `Queued`).
- `ToastStack` - Bounded Band-4 toast manager (`MAX_VISIBLE = 3`) with FIFO overflow queue.

#### Return values

- `NotificationRecord::effective_auto_dismiss_ms` returns `None` whenever `severity` is `Severity::Warning` or `Severity::Danger`, regardless of `auto_dismiss_ms`.
- `ToastStack::push` evicts the oldest visible `Info`/`Success` toast when full, or returns `PushOutcome::Queued` when all 3 visible slots hold non-evictable `Warning`/`Danger` toasts.

#### Side effects

- `ToastStack` mutates its visible slice and FIFO queue in place and automatically promotes queued toasts when slots open.

#### References

- Implementation: [`src/hoarfrost/composite/notification.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/notification.rs), [`src/hoarfrost/composite/info_bar.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/info_bar.rs), [`src/hoarfrost/composite/toast.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/toast.rs)
- Specification: `UIF-COMP-002`

---

### `composite::breadcrumb` and `composite::overflow_toolbar` - responsive bars (`UIF-COMP-008`)

#### Overview

The `composite::breadcrumb` and `composite::overflow_toolbar` modules provide headless middle-elision and priority-overflow layout solvers, style resolvers, and menu-cascade hosts (`BreadcrumbLayout`, `BreadcrumbHost`, `OverflowLayout`, `OverflowToolbarHost`) and do not construct standalone `iced` widgets directly. Both modules wire their overflow menus (`[…]` and `[⋯]`) directly into `surface::menu::MenuCascade` so elided items never disappear silently (`UIF-COMP-008`).

#### Prerequisites & Requirements

- The caller **MUST** enable the `iced` cargo feature (`#[cfg(feature = "iced")]`).
- Item identifiers `Id` **MUST** implement `std::fmt::Display` when constructing overflow menus.
- The caller **MUST** pass a mutable `&mut FocusState` when opening, closing, or routing keys through `BreadcrumbHost` or `OverflowToolbarHost`.

#### Syntax / Method Signature

```rust
// composite::breadcrumb
#[derive(Clone, Debug, PartialEq)]
pub struct BreadcrumbItem<Id> {
    pub id: Id,
    pub label: String,
    pub natural_width: f32,
}

impl<Id> BreadcrumbItem<Id> {
    pub fn new(id: Id, label: impl Into<String>, natural_width: f32) -> Self;
    pub fn effective_width(&self) -> f32;
    pub fn is_truncated(&self) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CrumbRole {
    Ancestor,
    Terminal,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VisibleCrumb {
    pub index: usize,
    pub role: CrumbRole,
    pub bounds: Rectangle,
    pub truncated: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BreadcrumbLayout { /* private fields */ }

impl BreadcrumbLayout {
    pub fn solve<Id>(items: &[BreadcrumbItem<Id>], available_width: f32) -> Self;
    pub fn is_elided(&self) -> bool;
    pub fn hidden_range(&self) -> Option<Range<usize>>;
    pub fn hidden_indices(&self) -> Vec<usize>;
    pub fn visible_crumbs(&self) -> &[VisibleCrumb];
    pub fn trigger_bounds(&self) -> Option<Rectangle>;
    pub fn separator_bounds(&self) -> &[Rectangle];
    pub fn total_width(&self) -> f32;
}

#[derive(Clone)]
pub enum BreadcrumbEmission<Id> {
    MenuToggled(bool),
    CrumbActivated(Id),
}

#[derive(Clone, Debug)]
pub struct BreadcrumbOutcome<Id> {
    pub cascade: CascadeOutcome,
    pub emissions: Vec<BreadcrumbEmission<Id>>,
}

impl<Id> BreadcrumbOutcome<Id> {
    pub fn noop() -> Self;
    pub fn emissions(emissions: Vec<BreadcrumbEmission<Id>>) -> Self;
}

#[derive(Debug)]
pub struct BreadcrumbHost { /* private fields */ }

impl BreadcrumbHost {
    pub fn new(trigger: FocusId) -> Self;
    pub fn is_menu_open(&self) -> bool;
    pub const fn cascade(&self) -> &MenuCascade;
    pub const fn trigger(&self) -> &FocusId;
    pub fn item_for_row(&self, row: usize) -> Option<usize>;
    pub fn activate_crumb<Id: Clone>(
        items: &[BreadcrumbItem<Id>],
        index: usize,
    ) -> BreadcrumbOutcome<Id>;
    pub fn open_overflow<Id: fmt::Display>(
        &mut self,
        items: &[BreadcrumbItem<Id>],
        layout: &BreadcrumbLayout,
        focus: &mut FocusState,
    ) -> BreadcrumbOutcome<Id>;
    pub fn close_overflow<Id>(&mut self, focus: &mut FocusState) -> BreadcrumbOutcome<Id>;
    pub fn menu_key<Id: Clone + fmt::Display>(
        &mut self,
        key: MenuKey,
        items: &[BreadcrumbItem<Id>],
        focus: &mut FocusState,
    ) -> BreadcrumbOutcome<Id>;
    pub fn escape<Id>(&mut self, focus: &mut FocusState) -> BreadcrumbOutcome<Id>;
    pub fn close_menu<Id>(&mut self, focus: &mut FocusState) -> BreadcrumbOutcome<Id>;
}

// composite::overflow_toolbar
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OverflowRank(u8);

impl OverflowRank {
    pub const DISPOSABLE: Self = Self(0);
    pub const SECONDARY: Self = Self(100);
    pub const PRIMARY: Self = Self(200);
    pub const LAST: Self = Self(255);
    pub const fn new(value: u8) -> Self;
    pub const fn value(self) -> u8;
}

#[derive(Clone, Debug, PartialEq)]
pub struct OverflowItem<Id> {
    pub id: Id,
    pub label: String,
    pub width: f32,
    pub rank: OverflowRank,
    pub group: u16,
    pub enabled: bool,
    pub pending: ActionPending,
    pub shortcut: Option<String>,
}

impl<Id> OverflowItem<Id> {
    pub fn new(id: Id, label: impl Into<String>, width: f32, rank: OverflowRank) -> Self;
    pub const fn with_group(self, group: u16) -> Self;
    pub const fn with_enabled(self, enabled: bool) -> Self;
    pub const fn with_pending(self, pending: ActionPending) -> Self;
    pub fn with_shortcut(self, shortcut: impl Into<String>) -> Self;
}

#[derive(Clone, Debug, PartialEq)]
pub struct OverflowLayout { /* private fields */ }

impl OverflowLayout {
    pub fn solve<Id>(items: &[OverflowItem<Id>], available_width: f32) -> Self;
    pub fn is_overflowing(&self) -> bool;
    pub fn inline_indices(&self) -> Vec<usize>;
    pub fn overflow_indices(&self) -> Vec<usize>;
    pub fn trigger_bounds(&self) -> Option<Rectangle>;
    pub fn used_width(&self) -> f32;
}

#[derive(Clone, Debug, PartialEq)]
pub enum ToolbarEmission<Id> {
    MenuToggled(bool),
    Activated(Commit<RowSettlement>),
    StopRequested(CancelRequest),
    ActivatedId(Id),
}

#[derive(Clone, Debug)]
pub struct ToolbarOutcome<Id> {
    pub cascade: CascadeOutcome,
    pub emissions: Vec<ToolbarEmission<Id>>,
}

impl<Id> ToolbarOutcome<Id> {
    pub fn noop() -> Self;
    pub fn emissions(emissions: Vec<ToolbarEmission<Id>>) -> Self;
}

#[derive(Debug)]
pub struct OverflowToolbarHost { /* private fields */ }

impl OverflowToolbarHost {
    pub fn new(trigger: FocusId) -> Self;
    pub fn is_menu_open(&self) -> bool;
    pub const fn cascade(&self) -> &MenuCascade;
    pub const fn actions(&self) -> &RowActionEngine;
    pub const fn trigger(&self) -> &FocusId;
    pub fn trigger_state(&self, focused: bool) -> VisualState;
    pub fn item_for_row(&self, row: usize) -> Option<usize>;
    pub fn open_menu<Id: fmt::Display>(
        &mut self,
        items: &[OverflowItem<Id>],
        layout: &OverflowLayout,
        focus: &mut FocusState,
    ) -> ToolbarOutcome<Id>;
    pub fn close_menu<Id>(&mut self, focus: &mut FocusState) -> ToolbarOutcome<Id>;
    pub fn toggle_menu<Id: fmt::Display>(
        &mut self,
        items: &[OverflowItem<Id>],
        layout: &OverflowLayout,
        focus: &mut FocusState,
    ) -> ToolbarOutcome<Id>;
    pub fn menu_key<Id: fmt::Display + Clone>(
        &mut self,
        key: MenuKey,
        items: &[OverflowItem<Id>],
        focus: &mut FocusState,
    ) -> ToolbarOutcome<Id>;
    pub fn escape<Id: fmt::Display>(&mut self, focus: &mut FocusState) -> ToolbarOutcome<Id>;
}
```

#### Examples

```rust
use hoarfrost::composite::breadcrumb::{BreadcrumbHost, BreadcrumbItem, BreadcrumbLayout};
use hoarfrost::composite::overflow_toolbar::{OverflowItem, OverflowLayout, OverflowRank};
use hoarfrost::focus::{FocusId, FocusState};

let crumbs = vec![
    BreadcrumbItem::new("root", "Workspace", 80.0),
    BreadcrumbItem::new("mid", "src", 40.0),
    BreadcrumbItem::new("leaf", "lib.rs", 60.0),
];
let layout = BreadcrumbLayout::solve(&crumbs, 150.0);
assert!(layout.is_elided());

let mut focus = FocusState::new(vec![FocusId::new("bc.trigger")]);
let mut host = BreadcrumbHost::new(FocusId::new("bc.trigger"));
let outcome = host.open_overflow(&crumbs, &layout, &mut focus);
assert_eq!(outcome.emissions.len(), 1);

let tools = vec![
    OverflowItem::new("run", "Run", 56.0, OverflowRank::PRIMARY),
    OverflowItem::new("fmt", "Format", 68.0, OverflowRank::DISPOSABLE),
];
let bar = OverflowLayout::solve(&tools, 96.0);
assert_eq!(bar.overflow_indices(), vec![1]);
```

#### Type references

- `BreadcrumbItem` - Single path segment with `id`, `label`, and measured `natural_width` (capped at `180px`).
- `CrumbRole` - Segment role (`Ancestor` in `text.muted` vs `Terminal` in `text.strong` `SemiBold`).
- `BreadcrumbLayout` - Solved middle-elision layout preserving item `0` and the terminal item before collapsing middle segments into `[…]`.
- `BreadcrumbEmission` - Host output event (`MenuToggled(bool)` or `CrumbActivated(Id)`).
- `BreadcrumbOutcome` - Combined `CascadeOutcome` and `Vec<BreadcrumbEmission<Id>>` returned by `BreadcrumbHost`.
- `BreadcrumbHost` - Per-trail state wrapping `MenuCascade` for the `[…]` overflow popup.
- `OverflowRank` - Eviction priority rank (`DISPOSABLE` `0` demotes first; `LAST` `255` demotes last).
- `OverflowItem` - Toolbar control descriptor with width, `OverflowRank`, group id, and `ActionPending` state.
- `OverflowLayout` - Solved inline and overflow index sets plus optional `28x28px` `[⋯]` trigger bounds.
- `ToolbarEmission` - Output event from `OverflowToolbarHost` (`MenuToggled`, `Activated`, `StopRequested`, `ActivatedId`).
- `ToolbarOutcome` - Combined `CascadeOutcome` and `Vec<ToolbarEmission<Id>>`.
- `OverflowToolbarHost` - Per-toolbar state wrapping `MenuCascade` and `RowActionEngine`.

#### Return values

- `BreadcrumbLayout::solve` and `OverflowLayout::solve` partition every input index into either the inline set or the hidden/overflow set without dropping any item (`UIF-COMP-008`).
- `BreadcrumbHost::open_overflow` and `OverflowToolbarHost::open_menu` return `noop()` if nothing is elided/overflowed.

#### Side effects

- `BreadcrumbHost` and `OverflowToolbarHost` push and pop focus traps on the supplied `FocusState` via their internal `MenuCascade`.

#### References

- Implementation: [`src/hoarfrost/composite/breadcrumb.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/breadcrumb.rs), [`src/hoarfrost/composite/overflow_toolbar.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/overflow_toolbar.rs)
- Specification: `UIF-COMP-008`

---

### `composite::pagination`, `composite::pips_pager`, `composite::stepper` - paging and workflows (`UIF-COMP-011`, `UIF-COMP-014`)

#### Overview

The `composite::pagination`, `composite::pips_pager`, and `composite::stepper` modules provide live `iced` widgets (`PaginationBar`, `PipsPager`, `Stepper`, `Element<'a, Message>`) driven by controlled state models (`PaginationModel`, `PipsPagerModel`, `StepperModel`). All boundary buttons (`[⏮]`, `[◀]`, `[▶]`, `[⏭]`, and `[← Previous]`) render as `Disabled` on page/step `0` or the final page/step rather than disappearing, guaranteeing `0px` layout shift (`UIF-PRIN-003`, `UIF-PRIN-004`).

#### Prerequisites & Requirements

- The caller **MUST** enable the `iced` cargo feature (`#[cfg(feature = "iced")]`).
- `Message` **MUST** implement `Clone + 'a`.
- `StepId` **MUST** implement `Clone + PartialEq + 'a`.

#### Syntax / Method Signature

```rust
// composite::pagination
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaginationModel {
    pub page: usize,
    pub page_size: usize,
    pub total_items: usize,
    pub page_size_options: Vec<usize>,
}

impl PaginationModel {
    pub fn new(
        page: usize,
        page_size: usize,
        total_items: usize,
        page_size_options: Vec<usize>,
    ) -> Self;
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

pub struct PaginationBar<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> { /* private fields */ }

impl<'a, Message: 'a + Clone> PaginationBar<'a, Message, iced::Theme, iced::Renderer> {
    pub fn new(
        model: PaginationModel,
        on_page_change: impl Fn(Commit<usize>) -> Message + 'a,
        on_page_size_change: impl Fn(Commit<usize>) -> Message + 'a,
    ) -> Self;
    pub fn model(&self) -> &PaginationModel;
}

pub fn format_thousands(n: usize) -> String;
pub fn pagination_bar<'a, Message: 'a + Clone>(
    model: PaginationModel,
    on_page_change: impl Fn(Commit<usize>) -> Message + 'a,
    on_page_size_change: impl Fn(Commit<usize>) -> Message + 'a,
) -> Element<'a, Message>;

// composite::pips_pager
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PipsPagerModel {
    pub page: usize,
    pub total_pages: usize,
}

impl PipsPagerModel {
    pub const fn new(page: usize, total_pages: usize) -> Self;
    pub const fn can_prev(&self) -> bool;
    pub const fn can_next(&self) -> bool;
    pub const fn is_first_page(&self) -> bool;
    pub const fn is_last_page(&self) -> bool;
}

pub struct PipsPager<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> { /* private fields */ }

impl<'a, Message: 'a + Clone> PipsPager<'a, Message, iced::Theme, iced::Renderer> {
    pub fn new(
        model: PipsPagerModel,
        on_page_change: impl Fn(Commit<usize>) -> Message + 'a,
    ) -> Self;
    pub fn model(&self) -> &PipsPagerModel;
}

pub fn pips_pager<'a, Message: 'a + Clone>(
    model: PipsPagerModel,
    on_page_change: impl Fn(Commit<usize>) -> Message + 'a,
) -> Element<'a, Message>;

// composite::stepper
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StepStatus {
    Pending,
    Current,
    Completed,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum StepperOrientation {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepItem<StepId> {
    pub id: StepId,
    pub label: String,
    pub status: StepStatus,
    pub disabled: bool,
}

impl<StepId> StepItem<StepId> {
    pub fn new(id: StepId, label: impl Into<String>, status: StepStatus) -> Self;
    pub fn disabled(self, disabled: bool) -> Self;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepperModel<StepId> {
    pub steps: Vec<StepItem<StepId>>,
    pub current_step: StepId,
    pub linear: bool,
}

impl<StepId: PartialEq> StepperModel<StepId> {
    pub fn new(steps: Vec<StepItem<StepId>>, current_step: StepId, linear: bool) -> Self;
    pub fn current_index(&self) -> Option<usize>;
    pub fn can_select(&self, target_id: &StepId) -> bool;
    pub fn can_previous(&self) -> bool;
    pub fn can_next(&self) -> bool;
    pub fn is_last_step(&self) -> bool;
    pub fn previous_step_id(&self) -> Option<StepId> where StepId: Clone;
    pub fn next_step_id(&self) -> Option<StepId> where StepId: Clone;
}

pub struct WizardFooterConfig<'a, Message, StepId> {
    pub on_previous: Option<Box<dyn Fn(Commit<StepId>) -> Message + 'a>>,
    pub on_next: Option<Box<dyn Fn(Commit<StepId>) -> Message + 'a>>,
    pub on_finish: Option<Box<dyn Fn(Commit<()>) -> Message + 'a>>,
    pub on_cancel: Option<Box<dyn Fn(Cancel<()>) -> Message + 'a>>,
}

impl<'a, Message: 'a, StepId: 'a> WizardFooterConfig<'a, Message, StepId> {
    pub fn new() -> Self;
    pub fn on_previous(self, handler: impl Fn(Commit<StepId>) -> Message + 'a) -> Self;
    pub fn on_next(self, handler: impl Fn(Commit<StepId>) -> Message + 'a) -> Self;
    pub fn on_finish(self, handler: impl Fn(Commit<()>) -> Message + 'a) -> Self;
    pub fn on_cancel(self, handler: impl Fn(Cancel<()>) -> Message + 'a) -> Self;
}

pub fn wizard_footer<'a, Message: 'a + Clone, StepId: 'a + Clone + PartialEq>(
    model: &StepperModel<StepId>,
    config: WizardFooterConfig<'a, Message, StepId>,
) -> Element<'a, Message>;

pub struct Stepper<'a, Message, StepId, Theme = iced::Theme, Renderer = iced::Renderer> { /* private fields */ }

impl<'a, Message: 'a + Clone, StepId: 'a + Clone + PartialEq>
    Stepper<'a, Message, StepId, iced::Theme, iced::Renderer>
{
    pub fn new(model: StepperModel<StepId>) -> Self;
    pub fn orientation(self, orientation: StepperOrientation) -> Self;
    pub fn on_step_select(self, f: impl Fn(Commit<StepId>) -> Message + 'a) -> Self;
    pub fn on_next(self, f: impl Fn(Commit<StepId>) -> Message + 'a) -> Self;
    pub fn on_previous(self, f: impl Fn(Commit<StepId>) -> Message + 'a) -> Self;
    pub fn on_finish(self, f: impl Fn(Commit<()>) -> Message + 'a) -> Self;
    pub fn on_cancel(self, f: impl Fn(Cancel<()>) -> Message + 'a) -> Self;
    pub fn model(&self) -> &StepperModel<StepId>;
    pub fn orientation_value(&self) -> StepperOrientation;
}

pub fn stepper<'a, Message: 'a + Clone, StepId: 'a + Clone + PartialEq>(
    model: StepperModel<StepId>,
) -> Stepper<'a, Message, StepId>;
```

#### Examples

```rust
use hoarfrost::composite::pagination::{pagination_bar, PaginationModel};
use hoarfrost::composite::pips_pager::{pips_pager, PipsPagerModel};

#[derive(Clone)]
enum Message {
    PageChanged(usize),
    PageSizeChanged(usize),
}

let model = PaginationModel::new(0, 50, 1_240, vec![25, 50, 100]);
assert_eq!(model.range_summary(), "Showing 1 - 50 of 1,240");
let bar: iced::Element<'_, Message> = pagination_bar(
    model,
    |c| Message::PageChanged(c.into_inner()),
    |c| Message::PageSizeChanged(c.into_inner()),
);

let pips: iced::Element<'_, Message> = pips_pager(
    PipsPagerModel::new(0, 5),
    |c| Message::PageChanged(c.into_inner()),
);
```

#### Type references

- `PaginationModel` - Zero-based page index, page size, total item count, and page-size selector options.
- `PaginationBar` - Live `32px` footer bar widget displaying tabular range summary, page-size selector, and navigation buttons.
- `PipsPagerModel` - Zero-based page index and total page count for a compact dot strip.
- `PipsPager` - Live `24px` carousel/pager strip morphing the active `6x6px` dot into a `16x6px` pill.
- `StepStatus` - Step lifecycle state (`Pending` `1..N`, `Current` `1..N`, `Completed` `'✓'`, `Error` `'!'`).
- `StepperOrientation` - Layout axis for `Stepper` (`Horizontal` vs `Vertical`).
- `StepItem` - Single workflow step with `id`, `label`, `status`, and `disabled` flag.
- `StepperModel` - Controlled step sequence with `linear` sequential-completion gate.
- `WizardFooterConfig` - Callback bundle for `wizard_footer` (`on_previous`, `on_next`, `on_finish`, `on_cancel`).
- `Stepper` - Live workflow header/column widget.

#### Return values

- `PaginationModel::total_pages` returns `0` when `total_items == 0` or `page_size == 0`, and `total_items.div_ceil(page_size)` otherwise.
- `pagination_bar`, `pips_pager`, and `wizard_footer` return a live `Element<'a, Message>`.
- `stepper` returns a `Stepper<'a, Message, StepId>` builder that converts into `Element<'a, Message>` via `From`.

#### Side effects

- None. All models are pure value types owned by the host application.

#### References

- Implementation: [`src/hoarfrost/composite/pagination.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/pagination.rs), [`src/hoarfrost/composite/pips_pager.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/pips_pager.rs), [`src/hoarfrost/composite/stepper.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/stepper.rs)
- Specification: `UIF-COMP-011`, `UIF-COMP-014`

---

### `composite::grid_view`, `composite::side_sheet`, `composite::split_divider` - grids, drawers, and splitters (`UIF-COMP-012`, `UIF-COMP-013`, `UIF-COMP-016`, `UIF-PANE-001`)

#### Overview

The `composite::grid_view`, `composite::side_sheet`, and `composite::split_divider` modules provide live `iced` widgets (`GridView`, `SideSheet`, `SplitDivider`, `Element<'a, Message>`) alongside headless layout solvers and linear drag-gesture engines (`solve_columns`, `SideSheetResize`, `SplitDividerResize`).

#### Prerequisites & Requirements

- The caller **MUST** enable the `iced` cargo feature (`#[cfg(feature = "iced")]`).
- `GridView` tile identifiers `Id` **MUST** implement `Clone + Ord + 'static` when mounted as an `iced::Element`.
- When extractors read values from `Live<T>`, `Commit<T>`, or `Cancel<T>` callbacks, the caller **MUST** call `.into_inner()` / `.value()` (or `.into_restored()` / `.restored()` on `Cancel<T>`) because the tuple fields are private (`UIF-BIND-003`).

#### Syntax / Method Signature

```rust
// composite::grid_view
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridNavKey {
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridTile<Id> {
    pub id: Id,
    pub title: String,
    pub subtitle: Option<String>,
    pub preview_icon: Option<Icon>,
    pub disabled: bool,
}

impl<Id> GridTile<Id> {
    pub fn new(id: Id, title: impl Into<String>) -> Self;
    pub fn subtitle(self, subtitle: impl Into<String>) -> Self;
    pub fn preview_icon(self, icon: impl Into<Icon>) -> Self;
    pub const fn disabled(self, disabled: bool) -> Self;
}

#[derive(Debug, Clone, PartialEq)]
pub struct GridViewModel<Id> {
    pub items: Vec<GridTile<Id>>,
    pub selected_ids: BTreeSet<Id>,
    pub anchor_id: Option<Id>,
    pub focused_id: Option<Id>,
    pub min_tile_width: f32,
    pub tile_height: f32,
    pub selection_mode: TreeSelectionMode,
}

impl<Id: Clone + Ord> GridViewModel<Id> {
    pub fn new(items: Vec<GridTile<Id>>) -> Self;
    pub const fn selection_mode(self, mode: TreeSelectionMode) -> Self;
    pub const fn min_tile_width(self, width: f32) -> Self;
    pub const fn tile_height(self, height: f32) -> Self;
    pub fn index_of(&self, id: &Id) -> Option<usize>;
    pub fn id_at(&self, index: usize) -> Option<Id>;
    pub fn step_nav(&self, current_index: usize, cols: usize, key: GridNavKey) -> usize;
    pub fn select_single(&mut self, id: Id) -> bool;
    pub fn select_range_to(&mut self, id: Id) -> bool;
    pub fn toggle_selection(&mut self, id: Id) -> bool;
    pub fn select_all(&mut self) -> bool;
    pub fn clear_selection(&mut self) -> bool;
}

pub struct GridView<'a, Message, Id, Theme = iced::Theme, Renderer = iced::Renderer> { /* private fields */ }

impl<'a, Message, Id: Clone + Ord + 'a, Theme, Renderer>
    GridView<'a, Message, Id, Theme, Renderer>
{
    pub fn new(model: GridViewModel<Id>, available_width: f32) -> Self;
    pub fn model(&self) -> &GridViewModel<Id>;
    pub fn available_width(&self) -> f32;
    pub fn on_selection_change(self, f: impl Fn(Commit<BTreeSet<Id>>) -> Message + 'a) -> Self;
    pub fn on_activate(self, f: impl Fn(Commit<Id>) -> Message + 'a) -> Self;
    pub fn on_context_menu(
        self,
        f: impl Fn(Commit<(BTreeSet<Id>, Point)>) -> Message + 'a,
    ) -> Self;
}

pub fn solve_columns(
    available_width: f32,
    min_tile_width: f32,
    gap: f32,
    gutter: f32,
) -> (usize, f32);
pub fn tile_rect(
    origin: Point,
    index: usize,
    cols: usize,
    tile_width: f32,
    tile_height: f32,
    gap: f32,
) -> Rectangle;
pub fn grid_view<'a, Message, Id: Clone + Ord + 'a>(
    model: GridViewModel<Id>,
    available_width: f32,
) -> GridView<'a, Message, Id>;

// composite::side_sheet
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SheetMode {
    #[default]
    Docked,
    ModelessOverlay,
    ModalScrim,
}

impl SheetMode {
    pub const fn toggle(self) -> Self;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SheetPlacement {
    Left,
    #[default]
    Right,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SideSheetModel {
    pub title: String,
    pub mode: SheetMode,
    pub placement: SheetPlacement,
    pub width: f32,
    pub open: bool,
    pub dirty: bool,
    pub busy: bool,
}

impl SideSheetModel {
    pub fn new(title: impl Into<String>) -> Self;
    pub fn with_mode(self, mode: SheetMode) -> Self;
    pub fn with_placement(self, placement: SheetPlacement) -> Self;
    pub fn with_width(self, width: f32) -> Self;
    pub fn with_open(self, open: bool) -> Self;
    pub fn with_dirty(self, dirty: bool) -> Self;
    pub fn with_busy(self, busy: bool) -> Self;
    pub fn clamp_width(w: f32) -> f32;
    pub fn effective_mode(&self, viewport_width: f32) -> SheetMode;
    pub fn can_close(&self) -> bool;
    pub fn z_band(&self, viewport_width: f32) -> ZBand;
    pub fn escape_rank(&self, viewport_width: f32) -> u8;
}

pub struct SideSheet<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> { /* private fields */ }

impl<'a, Message, Theme, Renderer> SideSheet<'a, Message, Theme, Renderer> {
    pub fn new(
        model: SideSheetModel,
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self;
    pub fn footer(self, footer: impl Into<Element<'a, Message, Theme, Renderer>>) -> Self;
    pub fn on_resize_change(self, f: impl Fn(Live<f32>) -> Message + 'a) -> Self;
    pub fn on_resize_commit(self, f: impl Fn(Commit<f32>) -> Message + 'a) -> Self;
    pub fn on_resize_cancel(self, f: impl Fn(Cancel<f32>) -> Message + 'a) -> Self;
    pub fn on_mode_toggle(self, f: impl Fn(Commit<SheetMode>) -> Message + 'a) -> Self;
    pub fn on_close_request(self, f: impl Fn(Cancel<()>) -> Message + 'a) -> Self;
    pub fn model(&self) -> &SideSheetModel;
    pub fn width(&self) -> f32;
    pub fn is_open(&self) -> bool;
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

// composite::split_divider
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SplitAxis {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitSnapshot {
    pub ratio: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SplitDividerModel {
    pub axis: SplitAxis,
    pub ratio: f32,
    pub min_ratio: f32,
    pub max_ratio: f32,
    pub disabled: bool,
}

impl SplitDividerModel {
    pub fn new(axis: SplitAxis, ratio: f32) -> Self;
    pub fn with_min_ratio(self, min: f32) -> Self;
    pub fn with_max_ratio(self, max: f32) -> Self;
    pub fn with_disabled(self, disabled: bool) -> Self;
    pub fn clamp_ratio(raw: f32, min_ratio: f32, max_ratio: f32) -> f32;
    pub fn clamped_ratio(&self) -> f32;
}

pub struct SplitDivider<'a, Message> { /* private fields */ }

impl<'a, Message> SplitDivider<'a, Message> {
    pub fn new(model: SplitDividerModel) -> Self;
    pub fn with_total_length(self, length: f32) -> Self;
    pub fn on_resize_change<F: Fn(Live<f32>) -> Message + 'a>(self, f: F) -> Self;
    pub fn on_resize_commit<F: Fn(Commit<f32>) -> Message + 'a>(self, f: F) -> Self;
    pub fn on_resize_cancel<F: Fn(Cancel<SplitSnapshot>) -> Message + 'a>(self, f: F) -> Self;
}

pub fn compute_grab_rect(bounds: Rectangle, axis: SplitAxis) -> Rectangle;
pub fn split_divider_geometry(bounds: Rectangle, axis: SplitAxis) -> (Rectangle, Rectangle);
pub fn split_divider_layers(
    bounds: Rectangle,
    axis: SplitAxis,
    visual: VisualState,
    palette: &Palette,
) -> Vec<(&'static str, Layer)>;
pub fn split_divider<'a, Message: 'a>(model: SplitDividerModel) -> SplitDivider<'a, Message>;
```

#### Examples

```rust
use hoarfrost::composite::grid_view::solve_columns;
use hoarfrost::composite::side_sheet::{side_sheet, SheetMode, SideSheetModel};
use hoarfrost::composite::split_divider::{split_divider, SplitAxis, SplitDividerModel};

#[derive(Clone)]
enum Message {
    ResizeLive(f32),
    ResizeCommit(f32),
    ResizeCancel(f32),
    SplitCancel(f32),
}

let (cols, tile_w) = solve_columns(640.0, 160.0, 12.0, 10.0);
assert_eq!(cols, 3);
assert!(tile_w >= 160.0);

let sheet_model = SideSheetModel::new("Inspector").with_mode(SheetMode::Docked);
let _drawer: iced::Element<'_, Message> = side_sheet(sheet_model, iced::widget::text("Details"))
    .on_resize_change(|live| Message::ResizeLive(live.into_inner()))
    .on_resize_commit(|commit| Message::ResizeCommit(commit.into_inner()))
    .on_resize_cancel(|cancel| Message::ResizeCancel(cancel.into_restored()))
    .into();

let div_model = SplitDividerModel::new(SplitAxis::Horizontal, 0.5)
    .with_min_ratio(0.2)
    .with_max_ratio(0.8);
let _divider: iced::Element<'_, Message> = split_divider(div_model)
    .on_resize_change(|live| Message::ResizeLive(live.into_inner()))
    .on_resize_commit(|commit| Message::ResizeCommit(commit.into_inner()))
    .on_resize_cancel(|cancel| Message::SplitCancel(cancel.into_restored().ratio))
    .into();
```

#### Type references

- `GridNavKey` - 2D arrow/Home/End navigation key (`Left`, `Right`, `Up`, `Down`, `Home`, `End`).
- `GridTile` - Single grid card descriptor with `id`, `title`, optional `subtitle`, and optional `preview_icon`.
- `GridViewModel` - State model for a 2D virtualized card grid supporting `Single` and `ExtendedMulti` selection.
- `GridView` - Live 2D virtualized tile grid widget culling off-screen rows.
- `SheetMode` - Layering mode for `SideSheet` (`Docked` Band 0, `ModelessOverlay` Band 1, `ModalScrim` Band 3).
- `SheetPlacement` - Edge anchor (`Left` or `Right`).
- `SideSheetModel` - Controlled inspector drawer model clamping width to `[280px, 520px]` and promoting to `ModalScrim` below `800px` viewport width.
- `SideSheet` - Live inspector drawer widget with `[Pin/Unpin]`, `[Close ×]`, and a `10px` resize edge.
- `SplitAxis` - Pane split direction (`Horizontal` width split vs `Vertical` height split).
- `SplitSnapshot` - Pre-drag split ratio captured before an active resize drag.
- `SplitDividerModel` - Controlled split ratio (`0.0..=1.0`) with `min_ratio` and `max_ratio` clamps.
- `SplitDivider` - Live `10px`-grab pane splitter widget with a `1px` line and centered `2px x 24px` grip pill.

#### Return values

- `solve_columns` returns `(cols, tile_width)` with `cols >= 1` and `tile_width >= min_tile_width`.
- `SideSheetModel::effective_mode` returns `SheetMode::ModalScrim` whenever `viewport_width < 800.0`.
- `grid_view`, `side_sheet`, and `split_divider` return widget builders that convert into `Element<'a, Message>` via `From`.

#### Side effects

- `GridView`, `SideSheet`, and `SplitDivider` maintain ephemeral hover and drag state inside the `iced` widget tree (`GridViewState`, `SideSheetState`, `SplitDividerState`).

#### References

- Implementation: [`src/hoarfrost/composite/grid_view.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/grid_view.rs), [`src/hoarfrost/composite/side_sheet.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/side_sheet.rs), [`src/hoarfrost/composite/split_divider.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/split_divider.rs)
- Specification: `UIF-COMP-012`, `UIF-COMP-013`, `UIF-COMP-016`, `UIF-PANE-001`

---

### `composite::code_block` and `composite::diff_viewer` - code and diff viewers (`UIF-COMP-015`)

#### Overview

The `composite::code_block` and `composite::diff_viewer` modules provide live `iced` widgets (`CodeBlock`, `DiffViewer`, `Element<'a, Message>`) driven by declarative state models (`CodeBlockModel`, `DiffViewerModel`). `CodeBlock` renders a fenced `surface.code` container with a `40px` tabular line-number gutter and a zero-shift `1500ms` copy confirmation button (`📋` -> `✓`), while `DiffViewer` renders unified or 50%/50% side-by-side hunks with redundant `+`/`-` glyphs and `2px` status bars (`UIF-PRIN-005`).

#### Prerequisites & Requirements

- The caller **MUST** enable the `iced` cargo feature (`#[cfg(feature = "iced")]`).
- `Message` **MUST** implement `Clone + 'a`.
- `HunkId` **MUST** implement `Clone + PartialEq + 'a`.
- To drive the `1500ms` copy-confirmation checkmark on `CodeBlockModel`, the application **MUST** call `mark_copied()` when handling `on_copy` and advance `tick_copied(delta_ms)` on timer ticks.

#### Syntax / Method Signature

```rust
// composite::code_block
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeBlockModel {
    pub language: Option<String>,
    pub code: String,
    pub show_line_numbers: bool,
    pub copied_hold_remaining_ms: u32,
}

impl CodeBlockModel {
    pub fn new(code: impl Into<String>) -> Self;
    pub fn with_language(self, language: impl Into<String>) -> Self;
    pub fn with_line_numbers(self, show: bool) -> Self;
    pub fn mark_copied(&mut self);
    pub fn tick_copied(&mut self, delta_ms: u32);
    pub fn is_copied(&self) -> bool;
    pub fn line_count(&self) -> usize;
}

pub struct CodeBlock<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> { /* private fields */ }

impl<'a, Message: 'a + Clone> CodeBlock<'a, Message, iced::Theme, iced::Renderer> {
    pub fn new(model: CodeBlockModel) -> Self;
    pub fn on_copy(self, f: impl Fn(Commit<String>) -> Message + 'a) -> Self;
    pub fn model(&self) -> &CodeBlockModel;
}

pub fn code_block<'a, Message: 'a + Clone>(model: CodeBlockModel) -> CodeBlock<'a, Message>;

// composite::diff_viewer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiffLineKind {
    Context,
    Added,
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
    pub text: String,
}

impl DiffLine {
    pub fn context(old_line: u32, new_line: u32, text: impl Into<String>) -> Self;
    pub fn added(new_line: u32, text: impl Into<String>) -> Self;
    pub fn deleted(old_line: u32, text: impl Into<String>) -> Self;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunk<HunkId> {
    pub id: HunkId,
    pub header: String,
    pub lines: Vec<DiffLine>,
}

impl<HunkId> DiffHunk<HunkId> {
    pub fn new(id: HunkId, header: impl Into<String>, lines: Vec<DiffLine>) -> Self;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DiffDisplayMode {
    #[default]
    Unified,
    SideBySide,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiffViewerModel<HunkId> {
    pub hunks: Vec<DiffHunk<HunkId>>,
    pub mode: DiffDisplayMode,
}

impl<HunkId> DiffViewerModel<HunkId> {
    pub fn new(hunks: Vec<DiffHunk<HunkId>>) -> Self;
    pub fn with_mode(self, mode: DiffDisplayMode) -> Self;
}

pub struct DiffViewer<'a, Message, HunkId, Theme = iced::Theme, Renderer = iced::Renderer> { /* private fields */ }

impl<'a, Message: 'a + Clone, HunkId: 'a + Clone + PartialEq>
    DiffViewer<'a, Message, HunkId, iced::Theme, iced::Renderer>
{
    pub fn new(model: DiffViewerModel<HunkId>) -> Self;
    pub fn on_accept_hunk(self, f: impl Fn(Commit<HunkId>) -> Message + 'a) -> Self;
    pub fn on_reject_hunk(self, f: impl Fn(Commit<HunkId>) -> Message + 'a) -> Self;
    pub fn on_mode_change(self, f: impl Fn(Commit<DiffDisplayMode>) -> Message + 'a) -> Self;
    pub fn model(&self) -> &DiffViewerModel<HunkId>;
}

pub fn diff_viewer<'a, Message: 'a + Clone, HunkId: 'a + Clone + PartialEq>(
    model: DiffViewerModel<HunkId>,
) -> DiffViewer<'a, Message, HunkId>;
```

#### Examples

```rust
use hoarfrost::composite::code_block::{code_block, CodeBlockModel};
use hoarfrost::composite::diff_viewer::{
    diff_viewer, DiffDisplayMode, DiffHunk, DiffLine, DiffViewerModel,
};

#[derive(Clone)]
enum Message {
    CopyCode(String),
    AcceptHunk(u32),
    RejectHunk(u32),
    ModeChanged(DiffDisplayMode),
}

let mut cb_model = CodeBlockModel::new("fn main() {}\n").with_language("rust");
cb_model.mark_copied();
assert!(cb_model.is_copied());
let _cb: iced::Element<'_, Message> = code_block(cb_model)
    .on_copy(|c| Message::CopyCode(c.into_inner()))
    .into();

let hunk = DiffHunk::new(
    1u32,
    "@@ -1,2 +1,2 @@",
    vec![
        DiffLine::deleted(1, "let x = 1;"),
        DiffLine::added(1, "let x = 2;"),
    ],
);
let dv_model = DiffViewerModel::new(vec![hunk]).with_mode(DiffDisplayMode::SideBySide);
let _dv: iced::Element<'_, Message> = diff_viewer(dv_model)
    .on_accept_hunk(|c| Message::AcceptHunk(c.into_inner()))
    .on_reject_hunk(|c| Message::RejectHunk(c.into_inner()))
    .on_mode_change(|c| Message::ModeChanged(c.into_inner()))
    .into();
```

#### Type references

- `CodeBlockModel` - Language badge, source code text, line-number toggle, and `copied_hold_remaining_ms` timer.
- `CodeBlock` - Live fenced code viewer widget (`surface.code` fill, `28px` header, `24x24px` copy button).
- `DiffLineKind` - Line classification (`Context`, `Added`, or `Deleted`).
- `DiffLine` - Single diff row carrying `DiffLineKind`, optional `old_line`/`new_line` numbers, and `text`.
- `DiffHunk` - Cohesive hunk of `DiffLine` rows with `id` and `header`.
- `DiffDisplayMode` - Diff layout mode (`Unified` single pane vs `SideBySide` 50%/50% split).
- `DiffViewerModel` - State model holding `Vec<DiffHunk<HunkId>>` and `DiffDisplayMode`.
- `DiffViewer` - Live diff inspection widget with hunk-level accept/reject actions and mode toggle.

#### Return values

- `CodeBlockModel::line_count` returns at least `1` (even for an empty string).
- `code_block` and `diff_viewer` return widget builders (`CodeBlock`, `DiffViewer`) that convert into `Element<'a, Message>` via `From`.

#### Side effects

- `CodeBlockModel::mark_copied` sets `copied_hold_remaining_ms` to `HOLD_COPIED_MS` (`1500`); `tick_copied` saturating-subtracts `delta_ms`.

#### References

- Implementation: [`src/hoarfrost/composite/code_block.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/code_block.rs), [`src/hoarfrost/composite/diff_viewer.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/diff_viewer.rs)
- Specification: `UIF-COMP-015`, `UIF-PRIN-003`, `UIF-PRIN-005`, `UIF-PRIN-006`

---

### `composite::empty_state` - zero-data placeholder (`UIF-COMP-014`)

#### Overview

The `composite::empty_state` module provides a live `iced` widget (`EmptyState`, `Element<'a, Message>`) and a headless token style resolver (`EmptyStateStyle`, `resolve_style`) for zero-item, zero-match, and load-error placeholders. It centers a `360px`-max-width card with a `40x40px` icon badge, `16px SemiBold` headline, `13px Regular` description, and an optional recovery CTA button so empty containers never collapse to zero dimensions (`UIF-PRIN-004`).

#### Prerequisites & Requirements

- The caller **MUST** enable the `iced` cargo feature (`#[cfg(feature = "iced")]`).
- `Message` and `ActionId` **MUST** implement `Clone + 'a`.
- The application **MUST** use `EmptyStateKind::LoadError { code }` for runtime or network failures so the badge icon is promoted to `status.danger.text` (`UIF-COMP-014`).

#### Syntax / Method Signature

```rust
pub use crate::composite::icon::Icon;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum EmptyStateKind {
    NoItems,
    NoMatches { query: String },
    LoadError { code: String },
}

impl EmptyStateKind {
    pub fn is_error(&self) -> bool;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmptyStateModel<ActionId> {
    pub kind: EmptyStateKind,
    pub icon: Icon,
    pub title: String,
    pub description: String,
    pub primary_action: Option<(String, ActionId)>,
}

impl<ActionId> EmptyStateModel<ActionId> {
    pub fn new(
        kind: EmptyStateKind,
        icon: impl Into<Icon>,
        title: impl Into<String>,
        description: impl Into<String>,
    ) -> Self;
    pub fn with_action(self, label: impl Into<String>, action: ActionId) -> Self;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EmptyStateStyle {
    pub badge_background: Color,
    pub badge_border_color: Color,
    pub icon_color: Color,
    pub title_color: Color,
    pub description_color: Color,
}

pub fn resolve_style(kind: &EmptyStateKind, palette: &Palette) -> EmptyStateStyle;

pub struct EmptyState<'a, Message, ActionId, Theme = iced::Theme, Renderer = iced::Renderer> { /* private fields */ }

impl<'a, Message: 'a + Clone, ActionId: Clone + 'a>
    EmptyState<'a, Message, ActionId, iced::Theme, iced::Renderer>
{
    pub fn new(
        model: EmptyStateModel<ActionId>,
        on_action: impl Fn(Commit<ActionId>) -> Message + 'a,
    ) -> Self;
    pub fn model(&self) -> &EmptyStateModel<ActionId>;
}

pub fn empty_state<'a, Message: 'a + Clone, ActionId: Clone + 'a>(
    model: EmptyStateModel<ActionId>,
    on_action: impl Fn(Commit<ActionId>) -> Message + 'a,
) -> Element<'a, Message>;
```

#### Examples

```rust
use hoarfrost::composite::empty_state::{
    empty_state, resolve_style, EmptyStateKind, EmptyStateModel,
};
use hoarfrost::token::color::Palette;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Action {
    ClearFilter,
}

#[derive(Clone)]
enum Message {
    Recover(Action),
}

let kind = EmptyStateKind::NoMatches {
    query: "telemetry".into(),
};
assert!(!kind.is_error());

let palette = Palette::dark();
let style = resolve_style(&kind, &palette);
assert_eq!(style.icon_color, palette.text_muted);

let model = EmptyStateModel::new(
    kind,
    "🔍",
    "No matching items",
    "No items matched \"telemetry\".",
)
.with_action("Clear filter", Action::ClearFilter);

let _element: iced::Element<'_, Message> =
    empty_state(model, |c| Message::Recover(c.into_inner()));
```

#### Type references

- `EmptyStateKind` - Semantic reason for the placeholder (`NoItems`, `NoMatches { query }`, or `LoadError { code }`).
- `EmptyStateModel` - Declarative model holding `EmptyStateKind`, `Icon`, `title`, `description`, and optional `(String, ActionId)` CTA button.
- `EmptyStateStyle` - Resolved badge background, badge border, icon foreground, title color, and description color.
- `EmptyState` - Live centered placeholder widget (`Length::Fill` width/height, `360px` max card width).

#### Return values

- `resolve_style` returns `EmptyStateStyle` with `icon_color = palette.status_danger_text` when `kind.is_error()` is `true` and `palette.text_muted` otherwise.
- `empty_state` returns a live `Element<'a, Message>`.

#### Side effects

- None. `EmptyState` is a stateless wrapper around its `EmptyStateModel`.

#### References

- Implementation: [`src/hoarfrost/composite/empty_state.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/empty_state.rs)
- Specification: `UIF-COMP-014`, `UIF-PRIN-004`

---

### `composite::command_palette` - modal command launcher (`UIF-COMP-010`)

#### Overview

The `composite::command_palette` module provides a live `iced` modal widget (`CommandPalette`, `Element<'a, Message>`) and a headless state model (`CommandPaletteModel`) for quick-open and command execution. It mounts a `560x380px` overlay card in Band 3 (`ZBand::Modal`, Escape Rank 2) at a `64px` top offset with removable mode prefix chips (`>`, `#`, `@`), matched-span highlighting, a monotonic async `generation` counter, and a two-stage `Escape` dismiss contract (`UIF-COMP-010`).

#### Prerequisites & Requirements

- The caller **MUST** enable the `iced` cargo feature (`#[cfg(feature = "iced")]`).
- `ItemId` **MUST** implement `'static + Clone + PartialEq` when `CommandPalette` is mounted as an `iced::Element`.
- When servicing asynchronous search results, the application **MUST** compare the response's generation against `CommandPaletteModel::generation` and discard stale responses (`UIF-COMP-010`).

#### Syntax / Method Signature

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PaletteMode {
    #[default]
    Files,
    Commands,
    Symbols,
    Tags,
}

impl PaletteMode {
    pub const fn prefix(self) -> Option<char>;
    pub const fn from_prefix(ch: char) -> Option<Self>;
    pub const fn label(self) -> &'static str;
    pub const fn placeholder(self) -> &'static str;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteItem<ItemId> {
    pub id: ItemId,
    pub title: String,
    pub subtitle: Option<String>,
    pub match_spans: Vec<TextRange>,
    pub shortcut: Option<String>,
}

impl<ItemId> PaletteItem<ItemId> {
    pub fn new(id: ItemId, title: impl Into<String>) -> Self;
    pub fn with_subtitle(self, subtitle: impl Into<String>) -> Self;
    pub fn with_match_spans(self, match_spans: Vec<TextRange>) -> Self;
    pub fn with_shortcut(self, shortcut: impl Into<String>) -> Self;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteGroup<ItemId> {
    pub heading: String,
    pub items: Vec<PaletteItem<ItemId>>,
}

impl<ItemId> PaletteGroup<ItemId> {
    pub fn new(heading: impl Into<String>, items: Vec<PaletteItem<ItemId>>) -> Self;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscapeDisposition {
    ClearedQuery,
    Dismiss,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommandPaletteModel<ItemId> {
    pub query: String,
    pub mode: PaletteMode,
    pub groups: Vec<PaletteGroup<ItemId>>,
    pub highlighted_id: Option<ItemId>,
    pub generation: u64,
    pub busy: bool,
    pub open: bool,
}

impl<ItemId: Clone + PartialEq> CommandPaletteModel<ItemId> {
    pub fn new(groups: Vec<PaletteGroup<ItemId>>) -> Self;
    pub fn with_query(self, q: impl Into<String>) -> Self;
    pub fn with_mode(self, m: PaletteMode) -> Self;
    pub fn with_open(self, open: bool) -> Self;
    pub fn with_busy(self, busy: bool) -> Self;
    pub fn with_highlighted_id(self, id: Option<ItemId>) -> Self;
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

#[derive(Debug, Clone, PartialEq)]
pub struct CommandPaletteState<ItemId> {
    pub scroll_offset: f32,
    pub hovered_id: Option<ItemId>,
    pub esc_hovered: bool,
    pub chip_hovered: bool,
}

pub type QueryChangeHandler<'a, Message> =
    Box<dyn Fn(Live<(PaletteMode, String, u64)>) -> Message + 'a>;

pub struct CommandPalette<'a, Message, ItemId, Theme = iced::Theme, Renderer = iced::Renderer> { /* private fields */ }

impl<'a, Message, ItemId: 'a + Clone + PartialEq, Theme, Renderer>
    CommandPalette<'a, Message, ItemId, Theme, Renderer>
{
    pub fn new(model: CommandPaletteModel<ItemId>) -> Self;
    pub fn on_query_change(
        self,
        f: impl Fn(Live<(PaletteMode, String, u64)>) -> Message + 'a,
    ) -> Self;
    pub fn on_mode_change(self, f: impl Fn(Commit<PaletteMode>) -> Message + 'a) -> Self;
    pub fn on_highlight_change(self, f: impl Fn(Live<ItemId>) -> Message + 'a) -> Self;
    pub fn on_execute(self, f: impl Fn(Commit<ItemId>) -> Message + 'a) -> Self;
    pub fn on_dismiss(self, f: impl Fn(Cancel<()>) -> Message + 'a) -> Self;
    pub fn palette(self, palette: Palette) -> Self;
    pub fn model(&self) -> &CommandPaletteModel<ItemId>;
    pub fn is_open(&self) -> bool;
}

pub fn card_bounds(viewport: Rectangle) -> Rectangle;
pub fn highlight_spans<'s>(text: &'s str, spans: &[TextRange]) -> Vec<(&'s str, bool)>;
pub fn total_content_height<ItemId>(groups: &[PaletteGroup<ItemId>]) -> f32;
pub fn command_palette<'a, Message, ItemId: 'static + Clone + PartialEq, Theme, Renderer>(
    model: CommandPaletteModel<ItemId>,
) -> CommandPalette<'a, Message, ItemId, Theme, Renderer>;
```

#### Examples

```rust
use hoarfrost::composite::command_palette::{
    command_palette, CommandPaletteModel, EscapeDisposition, PaletteGroup, PaletteItem,
    PaletteMode,
};

#[derive(Clone)]
enum Message {
    QueryChanged(PaletteMode, String, u64),
    ModeChanged(PaletteMode),
    Execute(&'static str),
    Dismiss,
}

let group = PaletteGroup::new(
    "Recent",
    vec![PaletteItem::new("file.open", "Open File").with_shortcut("Ctrl+O")],
);
let mut model = CommandPaletteModel::new(vec![group]);
assert_eq!(model.set_query_from_input(">open"), Some(PaletteMode::Commands));
assert_eq!(model.query, "open");
assert_eq!(model.handle_escape(), EscapeDisposition::ClearedQuery);
assert_eq!(model.handle_escape(), EscapeDisposition::Dismiss);

let _palette: iced::Element<'_, Message> = command_palette(model)
    .on_query_change(|live| {
        let (mode, q, gen_id) = live.into_inner();
        Message::QueryChanged(mode, q, gen_id)
    })
    .on_mode_change(|c| Message::ModeChanged(c.into_inner()))
    .on_execute(|c| Message::Execute(c.into_inner()))
    .on_dismiss(|_| Message::Dismiss)
    .into();
```

#### Type references

- `PaletteMode` - Operational search mode (`Files` default, `Commands` `'>'`, `Symbols` `'@'`, `Tags` `'#'`).
- `PaletteItem` - Single result row (`32px` single-line or `44px` with `subtitle`) carrying `match_spans` and optional `shortcut`.
- `PaletteGroup` - Titled group of `PaletteItem` rows with a `24px` section heading.
- `EscapeDisposition` - Two-stage `Escape` outcome (`ClearedQuery` on first press when query is non-empty; `Dismiss` when query is empty).
- `CommandPaletteModel` - Controlled state model tracking `query`, `mode`, `groups`, `highlighted_id`, monotonic `generation`, `busy`, and `open`.
- `CommandPaletteState` - Widget-tree state tracking scroll offset and hover targets.
- `CommandPalette` - Live modal command launcher widget.

#### Return values

- `CommandPaletteModel::handle_escape` clears `self.query`, increments `self.generation`, and returns `EscapeDisposition::ClearedQuery` when `query` is non-empty; otherwise it returns `EscapeDisposition::Dismiss`.
- `CommandPaletteModel::set_query_from_input` strips leading `>`, `#`, or `@` prefixes and returns `Some(new_mode)` whenever `self.mode` changes.
- `command_palette` returns a `CommandPalette` builder that converts into `Element<'a, Message, Theme, Renderer>` via `From`.

#### Side effects

- `CommandPaletteModel` methods mutate `query`, `mode`, `highlighted_id`, `generation`, and `open` in place.

#### References

- Implementation: [`src/hoarfrost/composite/command_palette.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/command_palette.rs)
- Specification: `UIF-COMP-010`, `UIF-PRIN-003`, `UIF-TOK-006`

---

### `composite::keybind_recorder` - shortcut capture control (`UIF-COMP-015`)

#### Overview

The `composite::keybind_recorder` module provides a live `iced` widget (`KeybindRecorder`, `Element<'a, Message>`) and a headless chord model (`KeyChord`, `KeybindRecorderModel`) for capturing keyboard shortcuts. It renders a `32px` `surface.code` control with `24px` monospace keycap pills (`[Ctrl] + [Shift] + [P]`), a `24x24px` `[×]` clear button, live modifier previews (`Live<KeyChord>`), settled commits (`Commit<Option<KeyChord>>`), `Escape` cancellation (`Cancel<()>`), and inline conflict warnings in `status.danger.text`.

#### Prerequisites & Requirements

- The caller **MUST** enable the `iced` cargo feature (`#[cfg(feature = "iced")]`).
- `Message` **MUST** implement `Clone + 'a`.
- When handling `on_chord_preview`, `on_chord_commit`, or `on_cancel`, the caller **MUST** extract payloads via `.into_inner()` or `.into_restored()` rather than destructuring private tuple fields (`UIF-BIND-003`).

#### Syntax / Method Signature

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct KeyChord {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub super_key: bool,
    pub key: String,
}

impl KeyChord {
    pub fn new(
        ctrl: bool,
        alt: bool,
        shift: bool,
        super_key: bool,
        key: impl Into<String>,
    ) -> Self;
    pub fn parts(&self) -> Vec<String>;
    pub fn format(&self) -> String;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeybindRecorderModel {
    pub chord: Option<KeyChord>,
    pub listening: bool,
    pub conflict_with: Option<String>,
    pub disabled: bool,
}

impl KeybindRecorderModel {
    pub fn new() -> Self;
    pub fn with_chord(self, chord: Option<KeyChord>) -> Self;
    pub fn with_listening(self, listening: bool) -> Self;
    pub fn with_conflict(self, conflict: Option<String>) -> Self;
    pub fn with_disabled(self, disabled: bool) -> Self;
}

pub type ChordCommitCallback<'a, Message> = Box<dyn Fn(Commit<Option<KeyChord>>) -> Message + 'a>;

pub struct KeybindRecorder<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> { /* private fields */ }

impl<'a, Message: 'a + Clone> KeybindRecorder<'a, Message, iced::Theme, iced::Renderer> {
    pub fn new(model: KeybindRecorderModel) -> Self;
    pub fn on_chord_preview(self, f: impl Fn(Live<KeyChord>) -> Message + 'a) -> Self;
    pub fn on_chord_commit(self, f: impl Fn(Commit<Option<KeyChord>>) -> Message + 'a) -> Self;
    pub fn on_cancel(self, f: impl Fn(Cancel<()>) -> Message + 'a) -> Self;
    pub fn on_start_listen(self, f: impl Fn() -> Message + 'a) -> Self;
    pub fn model(&self) -> &KeybindRecorderModel;
}

pub fn keybind_recorder<'a, Message: 'a + Clone>(
    model: KeybindRecorderModel,
) -> KeybindRecorder<'a, Message>;
```

#### Examples

```rust
use hoarfrost::composite::keybind_recorder::{
    keybind_recorder, KeyChord, KeybindRecorderModel,
};

#[derive(Clone)]
enum Message {
    StartListen,
    Preview(KeyChord),
    CommitChord(Option<KeyChord>),
    CancelListen,
}

let chord = KeyChord::new(true, false, true, false, "P");
assert_eq!(chord.format(), "Ctrl + Shift + P");

let model = KeybindRecorderModel::new()
    .with_chord(Some(chord))
    .with_conflict(Some("workbench.action.showCommands".into()));

let _recorder: iced::Element<'_, Message> = keybind_recorder(model)
    .on_start_listen(|| Message::StartListen)
    .on_chord_preview(|live| Message::Preview(live.into_inner()))
    .on_chord_commit(|commit| Message::CommitChord(commit.into_inner()))
    .on_cancel(|cancel| {
        let () = cancel.into_restored();
        Message::CancelListen
    })
    .into();
```

#### Type references

- `KeyChord` - Normalized shortcut combination (`ctrl`, `alt`, `shift`, `super_key`, and base `key`).
- `KeybindRecorderModel` - State model tracking `chord`, `listening` capture mode, `conflict_with` warning string, and `disabled` flag.
- `ChordCommitCallback` - Boxed callback alias for `Commit<Option<KeyChord>>`.
- `KeybindRecorder` - Live shortcut capture widget intercepting key presses while `listening` is `true`.

#### Return values

- `KeyChord::parts` returns modifier and key strings in canonical order (`Ctrl`, `Alt`, `Shift`, `Super`, `<key>`).
- `keybind_recorder` returns a `KeybindRecorder<'a, Message>` builder that converts into `Element<'a, Message>` via `From`.

#### Side effects

- While `model.listening && !model.disabled`, `KeybindRecorder::update` captures keyboard events (`shell.capture_event()`): `Escape` emits `on_cancel(Cancel::new(()))`, `Backspace`/`Delete` emits `on_chord_commit(Commit::new(None))`, modifier-only presses emit `on_chord_preview(Live::new(chord))`, and non-modifier key presses emit both `on_chord_preview` and `on_chord_commit(Commit::new(Some(chord)))`.

#### References

- Implementation: [`src/hoarfrost/composite/keybind_recorder.rs`](file:///D:/hoarfrost/src/hoarfrost/composite/keybind_recorder.rs)
- Specification: `UIF-COMP-015`, `UIF-PRIN-005`, `UIF-PRIN-006`

---
### `surface::zorder` - z-ordering, surface registry, and escape unwind (`UIF-ZORD-001`, `UIF-WIN-003`)

#### Overview

`surface::zorder` is the single authority for layered window stacking order, input routing, and Escape unwinding across the four UI bands (`UIF-ZORD-001`, `UIF-WIN-002`, `UIF-WIN-003`). It holds no widgets and renders nothing, operating purely as a structural coordinator that answers keyboard ownership, pointer hit testing, and reverse-order dismissal.

#### Prerequisites & Requirements

- Cargo feature `zorder` **MUST** be enabled (which implies `menu`).
- The application **MUST** maintain exactly one `SurfaceRegistry` instance per window.
- `SurfaceId` values **MUST** be unique within a single `SurfaceRegistry`.
- A surface with no bounding regions is never hit by pointer routing; hosts **MUST** register real bounding rectangles in root coordinates for pointer hits.
- Hit testing follows iced conventions: right and bottom bounds of each rectangle are exclusive.
- When unwinding `Escape`, `SurfaceRegistry::escape` automatically removes the dismissed entry from its stack; the host's unwinding closure **MUST** update host state and **MUST NOT** call `SurfaceRegistry::remove` or unregister the surface during the walk.
- Hosts managing drag-capable controls **MUST** invoke `escape_with_pre_rank` to evaluate active drag captures before descending the surface rank walk.

#### Syntax / Method Signature

```rust
// Bands and band policy
pub enum ZBand {
    Workspace = 0,
    UtilityOverlay = 1,
    Menu = 2,
    Modal = 3,
}

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

pub enum Backdrop {
    None,
    TransparentCatcher,
    DimmedScrim,
}

pub enum PointerRule {
    Direct,
    CaptureInsidePassOutside,
    CaptureInsideDismissOutside,
    CaptureInsideAbsorbOutside,
}

pub enum KeyClass {
    Tab,
    ShiftTab,
    ArrowNavigation,
    Enter,
    Space,
    Escape,
}

pub enum KeyDisposition {
    Swallow,
    SwallowWhenFocused,
    PassThrough,
}

// Identity and registration
pub struct SurfaceId(String);

impl SurfaceId {
    pub fn new(id: impl Into<String>) -> Self;
    pub fn as_str(&self) -> &str;
}

impl From<&str> for SurfaceId;
impl From<String> for SurfaceId;

pub enum SurfaceKind {
    Workspace,
    SearchFilter,
    MovableCard,
    ToastStack,
    MenuCard,
    TeachingPopover,
    Modal,
    InputDialog,
}

impl SurfaceKind {
    pub const fn band(self) -> ZBand;
}

pub struct Surface;

impl Surface {
    pub fn new(id: SurfaceId, kind: SurfaceKind, regions: Vec<Rectangle>) -> Self;
    pub fn id(&self) -> &SurfaceId;
    pub const fn kind(&self) -> SurfaceKind;
    pub const fn band(&self) -> ZBand;
    pub fn regions(&self) -> &[Rectangle];
    pub fn has_regions(&self) -> bool;
    pub fn contains(&self, point: Point) -> bool;
}

pub struct SurfaceRegistry;

impl SurfaceRegistry {
    pub fn new() -> Self;
    pub fn register(&mut self, id: SurfaceId, kind: SurfaceKind, regions: Vec<Rectangle>) -> &Surface;
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
    where
        F: FnMut(&Surface) -> UnwindAnswer;
}

// Pointer routing answers
pub enum PointerTarget {
    Surface(SurfaceId),
    DismissBand(ZBand),
    Scrim(SurfaceId),
    Workspace,
}

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
pub enum UnwindAnswer {
    Unwound,
    Dismissed,
    Declined(DeclineReason),
}

impl UnwindAnswer {
    pub fn from_cascade_close(outcome: &CascadeOutcome) -> Self;
}

pub enum DeclineReason {
    NotOpen,
    NotFocused,
    CannotUnwind,
}

pub struct Declination {
    pub surface: SurfaceId,
    pub reason: DeclineReason,
}

pub enum EscapeRefusal {
    EmptyStack,
    AllDeclined { declined: Vec<Declination> },
}

impl EscapeRefusal {
    pub fn is_empty_stack(&self) -> bool;
}

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

##### Band Policy Reference

| Band | `index` | `escape_rank` | `backdrop()` | `pointer_rule()` | `swallowed_keys()` | blocks lower | promotes on click |
|---|---|---|---|---|---|---|---|
| `Workspace` | 0 | 0 | `None` | `Direct` | `Tab`, `ShiftTab` | no | no |
| `UtilityOverlay` | 1 | 1 | `None` | `CaptureInsidePassOutside` | *(empty)* | no | yes |
| `Menu` | 2 | **3** | `TransparentCatcher` | `CaptureInsideDismissOutside` | all six classes | no | no |
| `Modal` | 3 | **2** | `DimmedScrim` | `CaptureInsideAbsorbOutside` | `Tab`, `ShiftTab`, `Enter`, `Escape` | yes | no |

#### Examples

```rust
use hoarfrost::surface::zorder::{
    DeclineReason, EscapeOutcome, SurfaceKind, SurfaceRegistry, UnwindAnswer,
};
use iced::Rectangle;

let mut registry = SurfaceRegistry::new();
registry.register("filter".into(), SurfaceKind::SearchFilter, vec![Rectangle::default()]);
registry.register("menu".into(), SurfaceKind::MenuCard, vec![Rectangle::default()]);

// Drive UIF-WIN-003 from your key handler.
match registry.escape(|surface| {
    if surface.kind() == SurfaceKind::MenuCard {
        // Dismiss the menu card
        UnwindAnswer::Dismissed
    } else {
        UnwindAnswer::Declined(DeclineReason::CannotUnwind)
    }
}) {
    EscapeOutcome::Consumed { dismissed, surface, .. } => {
        // When dismissed is true, registry has already popped the surface.
    }
    EscapeOutcome::Ignored(refusal) => {
        // Defined no-op; key remains unconsumed.
    }
}
```

#### Type references

- `ZBand` - One of four stacking layers with fixed backdrop, pointer, key, and escape rankings.
- `SurfaceKind` - High-level classification linking a surface to its designated `ZBand`.
- `Surface` - Unique identity, kind, and slice of root-space rectangles defining a clickable region.
- `SurfaceRegistry` - Sorted surface stack maintained by band hierarchy and insertion order.
- `PointerRoute` - Resolved target, passed-through bands, and workspace reachability flag.
- `UnwindAnswer` - Surface response (`Unwound`, `Dismissed`, or `Declined`) to an Escape walk inquiry.
- `Declination` - Refusal record pairing a `SurfaceId` with a specific `DeclineReason`.
- `PreRankedEscape` - Composite outcome reporting whether drag capture or rank walk handled Escape.

#### Return values

- `register` returns a reference to the registered `Surface`.
- `unregister` returns `Some(Surface)` if present, or `None`.
- `promote` returns `true` if stacking order changed; `false` if unknown or already topmost in its band.
- `route_pointer` returns a pure `PointerRoute` without state mutation.
- `pointer_down` routes pointer events and automatically raises surfaces in bands where `promotes_on_click` is true.
- `escape` returns `EscapeOutcome::Consumed` on acceptance or `EscapeOutcome::Ignored` if empty or all declined.
- `escape_with_pre_rank` returns `PreRankedEscape`, whose `outcome()` is `None` when a captured drag consumed Escape.

#### Side effects

- `register`, `unregister`, `clear`, and `promote` mutate internal registry storage.
- `escape` sequentially polls surfaces and **automatically removes** any entry returning `UnwindAnswer::Dismissed` from the registry.
- All routing and unwinding functions are pure state transitions reading no system clocks or filesystem paths.

#### References

[`surface/zorder.rs`](file:///D:/hoarfrost/src/hoarfrost/surface/zorder.rs)

---

### `surface::modal` - modal dialogs with backdrop scrim (`UIF-SURF-001`)

#### Overview

`surface::modal` provides the frame, focus trap, sizing solvers, and error management for blocking Band 3 modal cards (`UIF-MOD-*`, `UIF-SURF-001`). Paired with `surface::modal_tier`, it guards against accidental discard of unsaved edits and suppresses duplicate activation while actions are busy.

#### Prerequisites & Requirements

- Cargo feature `modal` (which implies `zorder`) **MUST** be enabled.
- A card **MUST** define an `AxisPolicy` per dimension via `ModalSizing`.
- The card's controls **MUST** be registered as `Vec<FocusMember>` restricted to the derived group `modal::group_for(&id)`.
- Hosts **MUST** record pending action status each frame via `set_pending`.
- A dirty card **MUST NOT** be dismissed via `Escape`, scrim clicks, or close buttons without confirmation.

#### Syntax / Method Signature

```rust
// Sizing policy (pure)
pub enum Desired {
    Fixed(f32),
    Viewport(f32),
    Content,
}

pub enum Cap {
    Pixels(f32),
    Viewport(f32),
    None,
}

impl Cap {
    pub fn resolve(self, available: f32) -> f32;
}

pub struct AxisPolicy {
    pub desired: Desired,
    pub min: f32,
    pub max: Cap,
}

impl AxisPolicy {
    pub fn resolve(&self, available: f32, content: f32) -> f32;
}

pub struct ModalSizing {
    pub width: AxisPolicy,
    pub height: AxisPolicy,
}

impl ModalSizing {
    pub fn solve(&self, viewport: Rectangle, content: Size) -> Rectangle;
}

pub struct ResizeFloors {
    pub width: f32,
    pub height: f32,
}

impl ResizeFloors {
    pub fn resolve(self, wanted: Size, viewport: Size) -> Size;
}

pub struct SubRegionBands {
    pub header: f32,
    pub footer: f32,
    pub body_padding: f32,
}

pub fn subregion_max_height(card_height: f32, bands: SubRegionBands) -> f32;
pub fn subregion_height(card_height: f32, bands: SubRegionBands, cap: f32) -> f32;

// Errors
pub enum ErrorLocation {
    Position { line: u32, column: u32 },
    Region(String),
}

pub enum Recovery {
    Retry,
    FocusControl(FocusId),
    Acknowledge,
}

pub struct ModalError;

impl ModalError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self;
    pub fn at(mut self, location: ErrorLocation) -> Self;
    pub fn with_recovery(mut self, recovery: Recovery) -> Self;
    pub fn code(&self) -> &str;
    pub fn message(&self) -> &str;
    pub fn location(&self) -> Option<&ErrorLocation>;
    pub const fn recovery(&self) -> &Recovery;
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
    pub fn open(
        spec: ModalSpec,
        viewport: Rectangle,
        content: Size,
        focus: &mut FocusState,
        registry: &mut SurfaceRegistry,
    ) -> Self;
    pub fn id(&self) -> &SurfaceId;
    pub fn group_id(&self) -> &GroupId;
    pub fn title(&self) -> &str;
    pub fn is_inside_card(&self, at: Point) -> bool;
    pub fn hit_test(&self, at: Point) -> ModalHit;
    pub fn resize(&mut self, viewport: Rectangle, content: Size, registry: &mut SurfaceRegistry);
    pub const fn is_dirty(&self) -> bool;
    pub fn set_dirty(&mut self, dirty: bool) -> bool;
    pub fn error(&self) -> Option<&ModalError>;
    pub fn has_error(&self) -> bool;
    pub fn set_error(&mut self, error: ModalError) -> Option<ModalError>;
    pub fn clear_error(&mut self) -> Option<ModalError>;
    pub const fn retry_pending(&self) -> bool;
    pub fn take_retry(&mut self) -> Option<ModalError>;
    pub fn set_pending(&mut self, pending: ActionPending);
    pub fn pending(&self) -> ActionPending;
    pub fn is_busy(&self) -> bool;
    pub fn try_activate_primary(&mut self) -> Activation;
    pub fn request_cancel(&mut self) -> Option<CancelRequest>;
    pub fn controlled(&self) -> Controlled<'_, Self>;
    pub fn ephemeral(&self) -> Ephemeral<&ModalAction>;
    pub fn sync_members(&mut self, members: Vec<FocusMember>, focus: &mut FocusState) -> Option<FocusId>;
    pub fn on_key(
        &mut self,
        class: KeyClass,
        focus: &mut FocusState,
        registry: &mut SurfaceRegistry,
    ) -> ModalOutcome;
    pub fn on_pointer(
        &mut self,
        at: Point,
        target: CardTarget,
        focus: &mut FocusState,
        registry: &mut SurfaceRegistry,
    ) -> ModalOutcome;
    pub fn close(&mut self, focus: &mut FocusState, registry: &mut SurfaceRegistry) -> ModalOutcome;
    pub fn unwind_answer(&mut self, focus: &mut FocusState) -> UnwindAnswer;
    pub fn escape_via(&mut self, registry: &mut SurfaceRegistry, focus: &mut FocusState) -> EscapeOutcome;
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

pub enum ModalHit {
    Card,
    Scrim,
}

pub enum CardTarget {
    Padding,
    Disabled,
    CloseButton,
    ErrorRecovery,
    Control(FocusId),
}

pub enum DismissCause {
    Escape,
    Scrim,
    CloseButton,
    Programmatic,
}

pub enum ModalNoOp {
    NotOpen,
    KeyNotTrapped,
    KeyOwnedByControl,
    NothingFocusable,
    KeyConsumedElsewhere,
    NotRegistered,
}

pub enum ModalOutcome {
    Dismissed { cause: DismissCause },
    DismissRefused { cause: DismissCause },
    Absorbed { target: CardTarget },
    FocusMoved { focused: FocusId },
    NoOp { reason: ModalNoOp },
}

pub struct ModalFrame {
    pub surface: Color,
    pub border: Color,
    pub border_width: f32,
    pub radius: f32,
    pub scrim: Color,
    pub divider: Color,
    pub content_padding: f32,
    pub footer_padding_horizontal: f32,
    pub footer_padding_vertical: f32,
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

#### Examples

```rust
use hoarfrost::focus::{FocusMember, FocusState};
use hoarfrost::surface::modal::{
    AxisPolicy, Cap, Desired, ModalHost, ModalOutcome, ModalSizing, ModalSpec,
};
use hoarfrost::surface::zorder::{KeyClass, SurfaceRegistry};
use iced::{Rectangle, Size};

let sizing = ModalSizing {
    width: AxisPolicy { desired: Desired::Fixed(720.0), min: 480.0, max: Cap::Viewport(0.8) },
    height: AxisPolicy { desired: Desired::Content, min: 240.0, max: Cap::Pixels(480.0) },
};
let spec = ModalSpec::new("about", "About", sizing)
    .with_controls(vec![FocusMember::new("close")])
    .with_primary("close");

let mut host = ModalHost::open(spec, viewport, content, &mut focus, &mut registry);

match host.on_key(KeyClass::Escape, &mut focus, &mut registry) {
    ModalOutcome::DismissRefused { .. } => {
        // Unsaved edits present; prompt for user confirmation.
    }
    ModalOutcome::Dismissed { .. } => {
        // Modal is closed and teardown is complete.
    }
    _ => {}
}
```

#### Type references

- `ModalSizing` - Axis sizing configuration defining desired size, minimum floor, and cap.
- `ModalError` - Error code, human-readable message, optional location, and recovery action.
- `Recovery` - Recovery affordance: `Retry`, `FocusControl(FocusId)`, or `Acknowledge`.
- `ModalHost` - Frame host managing trap, geometry bounds, and event absorption.
- `ModalAction` - Ephemeral action state controller suppressing double activation while busy.
- `CardTarget` - Hit target reporting what was clicked inside the modal boundary.
- `ModalOutcome` - Outcome of pointer or key events (`Dismissed`, `DismissRefused`, `Absorbed`, etc.).
- `ModalFrame` - Resolved visual styling properties derived from the design palette.

#### Return values

- `open` initializes geometry, registers into Band 3, pushes a focus trap, and returns `ModalHost`.
- `set_error` and `clear_error` return the replaced `Option<ModalError>`.
- `take_retry` returns the pending retry `Option<ModalError>` and resets the flag.
- `try_activate_primary` returns `Activation` indicating if execution was permitted or suppressed.
- `on_key`, `on_pointer`, and `close` return `ModalOutcome`.
- `unwind_answer` returns `UnwindAnswer::Dismissed` when open and clean (not busy), `UnwindAnswer::Declined(DeclineReason::CannotUnwind)` when dirty or busy, and `UnwindAnswer::Declined(DeclineReason::NotOpen)` when closed; it never returns `NotFocused`.

#### Side effects

- `open`, `close`, `on_key`, `on_pointer`, and `sync_members` mutate `FocusState` and `SurfaceRegistry`.
- `teardown` pops the focus trap and unregisters the modal's focus group.
- `set_pending` updates the clock-free busy state.
- No network, file, or wall-clock operations occur.

#### References

[`surface/modal.rs`](file:///D:/hoarfrost/src/hoarfrost/surface/modal.rs)
[`surface/modal_tier.rs`](file:///D:/hoarfrost/src/hoarfrost/surface/modal_tier.rs)

---

### `surface::input_dialog` - prompts and file/folder pickers (`UIF-SURF-002`)

#### Overview

`surface::input_dialog` implements the compact Band 3 card for prompts, renames, destinations, and destructive confirmations (`UIF-INP-*`, `UIF-SURF-002`). It provides a zero-shift box model with a permanently reserved validation slot and pairs with `surface::dialog_button` to prevent double-dispatch.

#### Prerequisites & Requirements

- Cargo feature `dialog` (which implies `button` and `zorder`) **MUST** be enabled.
- Input validation **MUST** use a pure `ValidationRule` producing a `Verdict`.
- The rename stem selection rule **MUST** conform to `UIF-INP-004`: selecting only the basename before the final dot (or all characters for dotfiles or extension-free names).
- Validation messages **MUST NOT** resize or shift the card layout; the validation row **MUST** remain permanently reserved.
- `InputDialogHost::open` **MUST** place the card via `Anchor::CursorProximal` and **MUST** always assign initial focus to `primary_focus()`.

#### Syntax / Method Signature

```rust
// Selection and validation (pure)
pub struct Selection {
    pub start: usize,
    pub end: usize,
}

impl Selection {
    pub const EMPTY: Self;
    pub const fn len(&self) -> usize;
    pub fn slice<'a>(&self, value: &'a str) -> &'a str;
    pub fn all(value: &str) -> Self;
    pub fn rename_stem(value: &str) -> Self;
}

pub struct ValidationMessage {
    pub code: String,
    pub text: String,
}

impl ValidationMessage {
    pub fn new(code: impl Into<String>, text: impl Into<String>) -> Self;
}

pub enum Verdict {
    Accept,
    Reject(ValidationMessage),
}

impl Verdict {
    pub const fn is_accept(&self) -> bool;
    pub fn message(&self) -> Option<&ValidationMessage>;
}

pub enum ValidationRule {
    Name,
    Rename { current: String },
    Destination,
    Confirm { count: usize },
    Always,
}

impl ValidationRule {
    pub fn check(&self, value: &str) -> Verdict;
}

// Box model (pure)
pub enum InputControl {
    TextEntry { initial: String, selection: Selection },
    FolderList { folders: Vec<String>, selected: usize },
    Confirmation { summary: String },
}

impl InputControl {
    pub fn row_height(&self) -> f32;
    pub const fn is_text_entry(&self) -> bool;
    pub fn value_at(&self, selected: usize) -> String;
}

pub fn folder_list_height(rows: usize) -> f32;

pub struct FooterButtons {
    pub cancel: f32,
    pub ok: f32,
}

pub struct InputLayout {
    pub card: Rectangle,
    pub title: Rectangle,
    pub context: Rectangle,
    pub control: Rectangle,
    pub validation: Rectangle,
    pub footer: Rectangle,
    pub cancel: Rectangle,
    pub ok: Rectangle,
}

impl InputLayout {
    pub fn card_height(control: &InputControl) -> f32;
    pub fn card_size(control: &InputControl) -> Size;
    pub fn solve(card: Rectangle, control: &InputControl, buttons: FooterButtons) -> Self;
}

pub enum ButtonIntent {
    Standard,
    Danger,
}

pub enum DeleteTargets {
    Single(String),
    Many(usize),
}

impl DeleteTargets {
    pub fn count(&self) -> usize;
    pub fn summary(&self) -> String;
}

// Spec and host
pub struct InputDialogSpec;

impl InputDialogSpec {
    pub fn new(
        id: impl Into<SurfaceId>,
        title: impl Into<String>,
        context: impl Into<String>,
        control: InputControl,
        rule: ValidationRule,
    ) -> Self;
    pub fn rename(
        id: impl Into<SurfaceId>,
        container: impl Into<String>,
        current: impl Into<String>,
    ) -> Self;
    pub fn create(id: impl Into<SurfaceId>, title: &str, container: impl Into<String>) -> Self;
    pub fn move_item(
        id: impl Into<SurfaceId>,
        name: impl Into<String>,
        folders: Vec<String>,
    ) -> Self;
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
    pub fn open(
        spec: InputDialogSpec,
        point: Point,
        keyboard_invoked: bool,
        viewport: Rectangle,
        focus: &mut FocusState,
        registry: &mut SurfaceRegistry,
    ) -> Self;
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
    pub fn commit_answer(
        &mut self,
        focus: &mut FocusState,
        registry: &mut SurfaceRegistry,
    ) -> Option<Commit<InputAnswer>>;
    pub fn cancel_answer(
        &mut self,
        focus: &mut FocusState,
        registry: &mut SurfaceRegistry,
    ) -> Option<Cancel<InputAnswer>>;
    pub fn submit(&mut self, focus: &mut FocusState, registry: &mut SurfaceRegistry) -> Option<InputAnswer>;
    pub fn cancel(&mut self, focus: &mut FocusState, registry: &mut SurfaceRegistry) -> Option<InputAnswer>;
    pub fn set_pending(&mut self, pending: ActionPending);
    pub fn pending(&self) -> ActionPending;
    pub fn is_busy(&self) -> bool;
    pub fn request_cancel(&mut self) -> Option<CancelRequest>;
    pub fn ephemeral(&self) -> Ephemeral<&DialogButtonController>;
    pub fn on_key(
        &mut self,
        class: KeyClass,
        focus: &mut FocusState,
        registry: &mut SurfaceRegistry,
    ) -> InputOutcome;
    pub fn on_pointer(
        &mut self,
        at: Point,
        target: InputTarget,
        focus: &mut FocusState,
        registry: &mut SurfaceRegistry,
    ) -> InputOutcome;
    pub fn close(&mut self, focus: &mut FocusState, registry: &mut SurfaceRegistry) -> InputOutcome;
    pub fn unwind_answer(&mut self, focus: &mut FocusState) -> UnwindAnswer;
    pub fn escape_via(&mut self, registry: &mut SurfaceRegistry, focus: &mut FocusState) -> EscapeOutcome;
}

pub enum InputTarget {
    Padding,
    Disabled,
    Entry,
    FolderRow(usize),
    Cancel,
    Ok,
}

pub enum InputNoOp {
    NotOpen,
    KeyNotTrapped,
    KeyOwnedByControl,
    NothingFocusable,
    KeyConsumedElsewhere,
    NotRegistered,
}

pub enum DismissCause {
    Escape,
    CancelButton,
    Scrim,
    Programmatic,
}

pub enum InputAnswer {
    Accepted { value: String },
    Cancelled,
}

pub enum InputOutcome {
    Accepted { answer: InputAnswer },
    Cancelled { cause: DismissCause },
    ErrorRevealed { message: ValidationMessage },
    Absorbed { target: InputTarget },
    FocusMoved { focused: FocusId },
    NoOp { reason: InputNoOp },
}

pub const fn card_border() -> f32;

// Footer button engine
pub enum DialogButtonRole {
    Primary,
    Secondary,
}

pub struct DialogButtonSettlement {
    pub role: DialogButtonRole,
    pub source: ActivationSource,
    pub pending: ActionPending,
    pub dispatched: bool,
}

pub enum DialogButtonEmission {
    Committed(Commit<DialogButtonSettlement>),
    Cancelled(Cancel<DialogButtonSettlement>),
    CancelRequested(CancelRequest),
}

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

#### Examples

```rust
use hoarfrost::surface::input_dialog::{InputDialogHost, InputDialogSpec, InputOutcome};
use hoarfrost::surface::zorder::KeyClass;
use iced::Point;

let spec = InputDialogSpec::rename("rename-1", "/notes", "draft.md");
let mut host = InputDialogHost::open(spec, Point::ORIGIN, false, viewport, &mut focus, &mut registry);

match host.on_key(KeyClass::Enter, &mut focus, &mut registry) {
    InputOutcome::Accepted { answer } => {
        // Renamed value accepted
    }
    InputOutcome::ErrorRevealed { message } => {
        // Message displayed in permanent validation slot
    }
    _ => {}
}
```

#### Type references

- `ValidationRule` - Injected data defining naming, rename, destination, and confirmation constraints.
- `Verdict` - `Accept` or `Reject(ValidationMessage)` carrying a stable diagnostic error code.
- `Selection` - Unicode character range supporting `rename_stem` basename extraction.
- `InputControl` - Content variant (`TextEntry`, `FolderList`, or `Confirmation`).
- `folder_list_height` - Module-level helper calculating folder list height clamped to `FOLDER_LIST_MAX_HEIGHT`.
- `InputLayout` - Eight computed row and button bounding rectangles for the dialog frame.
- `InputDialogHost` - Lifecycle host managing placement, focus traps, and input evaluation.
- `DialogButtonController` - Action engine managing footer button pendency and duplicate suppression.

#### Return values

- `open` positions the dialog (centred if `keyboard_invoked: true`, cursor-proximal if `false`), registers into Band 3, pushes a trap, and focuses `primary_focus()`.
- `set_value` validates the input and returns the new `Verdict`.
- `ok_is_enabled` returns `true` only when the active verdict is `Accept`.
- `submit` and `commit_answer` return `None` when input validation fails, without closing the dialog.
- `unwind_answer` returns `UnwindAnswer::Dismissed` when open, or `Declined(DeclineReason::NotOpen)` when closed.

#### Side effects

- `open`, `close`, `on_key`, `on_pointer`, `set_value`, and `set_selected` mutate `FocusState` and `SurfaceRegistry`.
- Revealing validation errors updates `revealed` without modifying the fixed `InputLayout`.
- No wall-clock reads or filesystem access are performed.

#### References

[`surface/input_dialog.rs`](file:///D:/hoarfrost/src/hoarfrost/surface/input_dialog.rs)
[`surface/dialog_button.rs`](file:///D:/hoarfrost/src/hoarfrost/surface/dialog_button.rs)

---

### `surface::movable` - movable dialogs with drag bounds (`UIF-SURF-003`)

#### Overview

`surface::movable` provides floating Band 1 cards for background operations logs and agent debug inspectors (`UIF-MOV-*`, `UIF-SURF-003`). It coordinates dragging, edge clamping, title-bar reachability, auto-scroll follow-streams, and tier-based gesture cancellation without blocking workspace interactions.

#### Prerequisites & Requirements

- Cargo feature `movable` (which implies `zorder`) **MUST** be enabled.
- Geometry persistence **MUST** use the `GeometryStorage` seam; tests **MUST** use `NoopGeometryStorage` or `MemoryGeometryStorage` and **MUST NOT** touch live configurations or disk paths.
- Title-bar trailing controls are an exclusion zone; drag initiation **MUST** test `is_draggable_at`.
- Active drag cancellations via `Escape` **MUST** cancel the gesture directly and restore `drag_origin`; `MovableHost` **MUST NOT** route drag cancellation through `PreRankCoordinator`.
- Outside clicks pass through Band 1; hosts **MUST** check `MovableResponse::passed_through`.

#### Syntax / Method Signature

```rust
// The serialisable geometry value
pub struct MovableGeometry {
    pub position: Point,
    pub size: Size,
}

impl MovableGeometry {
    pub const DEFAULT: Self;
    pub const fn new(position: Point, size: Size) -> Self;
    pub fn sanitized(self) -> Self;
    pub fn encode(self) -> String;
    pub fn decode(text: &str) -> Option<Self>;
}

// Persistence seam
pub trait GeometryStorage {
    fn load(&mut self, key: &str) -> Option<MovableGeometry>;
    fn save(&mut self, key: &str, geometry: MovableGeometry);
    fn clear(&mut self, key: &str);
}

pub struct NoopGeometryStorage;
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
pub struct ScrollGeometry {
    pub offset: f32,
    pub content: f32,
    pub viewport: f32,
}

impl ScrollGeometry {
    pub fn max_offset(self) -> f32;
    pub fn is_empty(self) -> bool;
    pub fn at_bottom(self) -> bool;
    pub fn clamped(self) -> Self;
    pub fn scrolled_by(self, delta: f32) -> Self;
    pub fn at_bottom_now(self) -> Self;
}

pub enum AutoScrollSetting {
    Enabled,
    Disabled,
}

pub enum FollowState {
    Following,
    Paused,
}

pub enum HoldReason {
    Disabled,
    Inspecting,
}

pub enum AutoScrollDecision {
    Follow { entries: usize },
    Hold { entries: usize, reason: HoldReason, behind: usize },
}

pub const AT_BOTTOM_TOLERANCE: f32 = space::XXS;

// Input vocabulary
pub enum PointerPhase {
    Pressed,
    Moved,
    Released,
}

pub enum SurfaceTarget {
    Outside,
    TitleBar,
    TitleGrip,
    Title,
    Summary,
    Dock,
    Close,
    FilterBar,
    AutoScrollToggle,
    Control(FocusId),
    Content,
    Footer,
    Resize(ResizeHandle),
}

pub enum Zone {
    Outside,
    TitleDrag,
    TitleActions,
    FilterBar,
    Content,
    Footer,
    Grip(ResizeHandle),
}

pub enum CardControl {
    Dock,
    AutoScrollToggle,
    Custom(FocusId),
}

pub enum MovableTask {
    ScrollToBottom,
    SetCursor(mouse::Interaction),
    Activate(FocusId),
}

pub enum MovableNoOp {
    NotOpen,
    NotRegistered,
    KeyNotTrapped,
    NotFocused,
    AlreadyCapturing,
    GestureCancelled,
    AlreadyOpen,
    Idle,
    OwnedByOtherSurface,
}

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
    pub fn open(
        spec: MovableSpec,
        viewport: Rectangle,
        focus: &mut FocusState,
        registry: &mut SurfaceRegistry,
        storage: &mut dyn GeometryStorage,
    ) -> Self;
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
    pub fn on_pointer(
        &mut self,
        phase: PointerPhase,
        at: Point,
        target: SurfaceTarget,
        focus: &mut FocusState,
        registry: &mut SurfaceRegistry,
        storage: &mut dyn GeometryStorage,
    ) -> MovableResponse;
    pub fn drag_by(&mut self, delta: Vector) -> MovableAction;
    pub fn resize_by(&mut self, handle: ResizeHandle, delta: Vector) -> MovableAction;
    pub fn end_drag(&mut self, registry: &mut SurfaceRegistry, storage: &mut dyn GeometryStorage) -> MovableAction;
    pub fn end_resize(&mut self, registry: &mut SurfaceRegistry, storage: &mut dyn GeometryStorage) -> MovableAction;
    pub fn set_geometry(
        &mut self,
        geometry: MovableGeometry,
        registry: &mut SurfaceRegistry,
        storage: &mut dyn GeometryStorage,
    ) -> MovableAction;
    pub fn set_viewport(
        &mut self,
        viewport: Rectangle,
        registry: &mut SurfaceRegistry,
        storage: &mut dyn GeometryStorage,
    ) -> MovableAction;
    pub fn at_bottom(&self) -> bool;
    pub fn set_auto_scroll(&mut self, setting: AutoScrollSetting) -> MovableAction;
    pub fn sync_scroll(&mut self, scroll: ScrollGeometry) -> MovableAction;
    pub fn scroll_by(&mut self, delta: f32) -> MovableAction;
    pub fn scroll_to_bottom(&mut self) -> MovableAction;
    pub fn on_append(&mut self, entries: usize) -> AutoScrollDecision;
    pub fn on_scroll(&mut self, at: Point, delta: f32, registry: &mut SurfaceRegistry) -> MovableResponse;
    pub fn holds_focus(&self, focus: &FocusState) -> bool;
    pub fn on_key(
        &mut self,
        class: KeyClass,
        focus: &mut FocusState,
        registry: &mut SurfaceRegistry,
        storage: &mut dyn GeometryStorage,
    ) -> MovableResponse;
    pub fn unwind_answer(&mut self, focus: &mut FocusState, storage: &mut dyn GeometryStorage) -> UnwindAnswer;
    pub fn escape_via(
        &mut self,
        registry: &mut SurfaceRegistry,
        focus: &mut FocusState,
        storage: &mut dyn GeometryStorage,
    ) -> EscapeOutcome;
    pub fn close(
        &mut self,
        focus: &mut FocusState,
        registry: &mut SurfaceRegistry,
        storage: &mut dyn GeometryStorage,
    ) -> MovableResponse;
    pub fn reopen(
        &mut self,
        viewport: Rectangle,
        focus: &mut FocusState,
        registry: &mut SurfaceRegistry,
        storage: &mut dyn GeometryStorage,
    ) -> MovableResponse;
    pub fn persist(&mut self, storage: &mut dyn GeometryStorage);
    pub fn sync_members(&mut self, members: Vec<FocusMember>, focus: &mut FocusState) -> Option<FocusId>;
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
    pub card_surface: Color,
    pub card_border: Color,
    pub card_border_width: f32,
    pub card_radius: f32,
    pub title_bar_surface: Color,
    pub bar_surface: Color,
    pub content_surface: Color,
    pub title_text: Color,
    pub meta_text: Color,
    pub tag_text: Color,
    pub divider: Color,
    pub title_bar_height: f32,
    pub title_bar_padding_horizontal: f32,
    pub filter_bar_height: f32,
    pub filter_bar_padding_vertical: f32,
    pub filter_bar_gap: f32,
    pub footer_height: f32,
    pub footer_padding_horizontal: f32,
    pub resize_handle_size: f32,
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

#### Examples

```rust
use hoarfrost::surface::movable::{
    MemoryGeometryStorage, MovableHost, MovableSpec, PointerPhase, SurfaceTarget,
};
use iced::Point;

let mut storage = MemoryGeometryStorage::new();
let mut host = MovableHost::open(
    MovableSpec::new("log", "Session log"),
    viewport,
    &mut focus,
    &mut registry,
    &mut storage,
);

// Pointer drag sequence
host.on_pointer(
    PointerPhase::Pressed,
    Point::new(100.0, 100.0),
    SurfaceTarget::TitleBar,
    &mut focus,
    &mut registry,
    &mut storage,
);
host.on_pointer(
    PointerPhase::Moved,
    Point::new(120.0, 110.0),
    SurfaceTarget::TitleBar,
    &mut focus,
    &mut registry,
    &mut storage,
);
let response = host.on_pointer(
    PointerPhase::Released,
    Point::new(120.0, 110.0),
    SurfaceTarget::TitleBar,
    &mut focus,
    &mut registry,
    &mut storage,
);
```

#### Type references

- `MovableGeometry` - Position and dimension data supporting lossless string encoding and decoding.
- `GeometryStorage` - Persistence trait with `NoopGeometryStorage` and `MemoryGeometryStorage` implementations.
- `ScrollGeometry` - Viewport scroll extent, content length, and offset math.
- `AutoScrollDecision` - Result of stream appends: `Follow` or `Hold` (with reason and backlog count).
- `Zone` - Spatial zone at cursor coordinates (e.g., `TitleDrag`, `TitleActions`, `Grip`).
- `MovableResponse` - Action result carrying side-effect tasks, promotion flags, and pass-through status.
- `DragGesture` - Tier transcript recorder managing `Live`, `Commit`, and `Cancel` tiers.

#### Return values

- `open` loads saved geometry or falls back to `DEFAULT`, clamps into viewport, and registers into Band 1.
- `unwind_answer` cancels active drag directly via `self.drag.cancel()`, restores `drag_origin`, and returns `UnwindAnswer::Unwound`. If not dragging, it returns `UnwindAnswer::Dismissed` when focused, or `Declined` otherwise.
- `on_pointer` returns `MovableResponse`. Outside clicks set `passed_through: true`.

#### Side effects

- Drag and resize commits persist geometry to `GeometryStorage::save`.
- `SurfaceRegistry` and `FocusState` are mutated on open, close, and reorder.
- No background threads, network calls, or clock reads occur within the crate.

#### References

[`surface/movable.rs`](file:///D:/hoarfrost/src/hoarfrost/surface/movable.rs)
[`surface/drag_tier.rs`](file:///D:/hoarfrost/src/hoarfrost/surface/drag_tier.rs)

---

### `surface::menu` - contextual, dropdown, and cascade menus (`UIF-SURF-004`)

#### Overview

`surface::menu` provides the unified engine for fixed dropdown menus and floating context menus (`UIF-FMENU-*`, `UIF-CMENU-*`, `UIF-SURF-004`). It decouples tree models from cascade openness, row widgets, action lifecycle tracking, and diagonal aiming corridors.

#### Prerequisites & Requirements

- Cargo feature `menu` (which implies `button` and `tooltip`) **MUST** be enabled.
- Interactive rows **MUST** have unique keys within each menu (`Menu::validate` **MUST** pass with zero violations).
- Destructive rows **MUST** be positioned last in their row group and **MUST NOT** share a group with non-destructive items.
- Radio groups **MUST** have at most one selected member.
- Hosts **MUST** navigate menu levels with `MenuKey` and execute `CascadeOutcome::focus_task` to reposition widget focus.
- The aiming corridor **MUST** receive caller-provided `TimeMs` timestamps rather than accessing a clock directly.

#### Syntax / Method Signature

```rust
// model - pure data
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

pub enum RowKind {
    Action,
    Check { checked: bool },
    Radio { group: RadioGroupId, selected: bool },
    Submenu { child: Box<Menu> },
    Heading,
    Separator,
}

pub enum ClosePolicy {
    Close,
    StayOpen,
}

pub enum LeadingSlot {
    Check,
    Radio,
    Icon,
    None,
}

pub enum TrailingSlot {
    Shortcut,
    Chevron,
    None,
}

pub struct Row;

impl Row {
    pub fn action(key: impl Into<String>, label: impl Into<String>) -> Self;
    pub fn checkable(key: impl Into<String>, label: impl Into<String>, checked: bool) -> Self;
    pub fn radio(
        key: impl Into<String>,
        label: impl Into<String>,
        group: impl Into<RadioGroupId>,
        selected: bool,
    ) -> Self;
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

impl RadioGroupInfo {
    pub fn is_selected(&self, row: usize) -> bool;
}

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

impl Activation {
    pub fn is_absorbed(&self) -> bool;
}

pub enum MenuViolation {
    MissingKey { row: usize, label: String },
    DuplicateKey { row: usize, key: String, first: usize },
    DestructiveNotLast { row: usize, group: usize },
    MixedGroup { group: usize, row: usize },
    RadioOverSelected { group: RadioGroupId, rows: Vec<usize> },
}

// cascade - openness
pub enum MenuTreeKind {
    Context,
    Fixed,
}

pub struct SubmenuTrigger {
    pub parent: MenuId,
    pub row: usize,
}

pub struct OpenCard;

impl OpenCard {
    pub fn id(&self) -> &MenuId;
    pub fn group_id(&self) -> &GroupId;
    pub fn menu(&self) -> &Menu;
    pub fn trigger(&self) -> Option<&SubmenuTrigger>;
    pub fn is_root(&self) -> bool;
    pub fn focused_row(&self) -> Option<usize>;
}

pub enum MenuKey {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Space,
    Escape,
}

pub enum IgnoredReason {
    NotOpen,
    AlreadyOpen { card: MenuId },
    UnknownCard { card: MenuId },
    NotSubmenu { card: MenuId, row: usize },
    NoFocusableRows { card: MenuId },
    RowOutOfRange { card: MenuId, row: usize },
    NothingFocused { card: MenuId },
}

pub enum FocusTransition {
    Unchanged,
    Moved { from: Option<FocusId>, to: FocusId },
    Cleared { from: FocusId },
}

pub struct CascadeOutcome;

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
    pub fn open(&mut self, source: &Menu, kind: MenuTreeKind, focus: &mut FocusState) -> CascadeOutcome;
    pub fn toggle(&mut self, source: &Menu, kind: MenuTreeKind, focus: &mut FocusState) -> CascadeOutcome;
    pub fn open_submenu(&mut self, card: &MenuId, row: usize, focus: &mut FocusState) -> CascadeOutcome;
    pub fn hover_submenu_row(
        &mut self,
        aim: &mut AimCorridor,
        card: &MenuId,
        row: usize,
        now: TimeMs,
        focus: &mut FocusState,
    ) -> CascadeOutcome;
    pub fn pointer_left_card(aim: &mut AimCorridor, now: TimeMs) -> bool;
    pub fn activate_row(&mut self, card: &MenuId, row: usize, focus: &mut FocusState) -> CascadeOutcome;
    pub fn on_key(&mut self, key: MenuKey, focus: &mut FocusState) -> CascadeOutcome;
    pub fn move_focus(&mut self, delta: i32, focus: &mut FocusState) -> CascadeOutcome;
    pub fn open_focused_submenu(&mut self, focus: &mut FocusState) -> CascadeOutcome;
    pub fn activate_focused(&mut self, focus: &mut FocusState) -> CascadeOutcome;
    pub fn close_deepest(&mut self, focus: &mut FocusState) -> CascadeOutcome;
    pub fn close_tree(&mut self, focus: &mut FocusState) -> CascadeOutcome;
    pub fn sync(&mut self, source: &Menu, focus: &mut FocusState) -> CascadeOutcome;
}

// aim - the corridor
pub fn grace_ms() -> u64;
pub enum AimTarget {
    ParentRow { row: usize },
    ChildBranch,
    Outside,
}

pub enum AimDecision {
    Idle,
    Preserve,
    Hold { row: usize, remaining_ms: u64 },
    Switch { row: usize },
    Collapse,
}

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

// row_action - one row's action engine
pub enum RevertKind {
    Value,
    FireAndSettle,
    Navigation,
    Nothing,
}

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

pub enum RowEmission {
    Committed(Commit<RowSettlement>),
    Cancelled(Cancel<RowSettlement>),
    CancelRequested(CancelRequest),
}

pub struct RowActionEngine;

impl RowActionEngine {
    pub fn new() -> Self;
    pub fn observe(&mut self, pending: ActionPending);
    pub fn pending(&self) -> ActionPending;
    pub fn begin(&mut self, row: &Row, source: ActivationSource) -> Option<RowEmission>;
    pub fn commit(
        &mut self,
        row: &Row,
        effect: &ActivationEffect,
        source: ActivationSource,
    ) -> Option<RowEmission>;
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

// view - the row matrix and the row widget
pub struct RowState {
    pub base: RowBase,
    pub focused: bool,
}

pub enum RowBase {
    Rest,
    Hovered,
    Pressed,
    ActiveSubmenuParent,
    DestructiveHovered,
    DestructivePressed,
    Disabled,
    SectionHeading,
}

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

pub struct MenuRow<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>;

impl<'a, Message, Theme, Renderer> MenuRow<'a, Message, Theme, Renderer> {
    pub fn new(
        row: Row,
        focus_id: impl Into<FocusId>,
        on_event: impl Fn(RowEvent) -> Message + 'a,
    ) -> Self;
    pub fn with_body(
        row: Row,
        body: impl Into<Element<'a, Message, Theme, Renderer>>,
        focus_id: impl Into<FocusId>,
        on_event: impl Fn(RowEvent) -> Message + 'a,
    ) -> Self;
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

#### Examples

```rust
use hoarfrost::focus::FocusState;
use hoarfrost::surface::menu::cascade::MenuKey;
use hoarfrost::surface::menu::model::{Menu, Row};
use hoarfrost::surface::menu::{MenuCascade, MenuTreeKind};

let menu = Menu::new("file", vec![
    Row::action("new", "New"),
    Row::submenu("open_recent", "Open Recent", Menu::new("recent", vec![Row::action("f1", "file.md")])),
    Row::separator(),
    Row::action("exit", "Exit"),
]);
assert!(menu.is_valid());

let mut cascade = MenuCascade::new();
cascade.open(&menu, MenuTreeKind::Fixed, &mut focus);
let outcome = cascade.on_key(MenuKey::Down, &mut focus);
let _task = outcome.focus_task::<()>(&focus);
```

#### Type references

##### Module Responsibilities

| Module | Responsibility |
|---|---|
| `menu::model` | Pure data definitions: rows, groups, radio selections, and validation |
| `menu::view` | Visual state matrix and the focusable `MenuRow` widget |
| `menu::cascade` | Open cards stack, mutual exclusion, and keyboard rove navigation |
| `menu::row_action` | Action engine tracking row pendency, suppression, and revertibility |
| `menu::aim` | Diagonal aiming corridor keeping submenus open while moving diagonally |

- `Menu` / `Row` - Cloneable, heap-allocated representation of hierarchical menu cards and rows.
- `MenuViolation` - Diagnostic errors detected by `Menu::validate` (e.g. duplicate keys, misplaced destructive items).
- `MenuCascade` - Stack tracking currently open menu levels and keyboard focus progression.
- `CascadeOutcome` - Transition report detailing opened or closed cards, activation records, and focus tasks.
- `AimCorridor` - State machine preventing premature submenu closure during diagonal pointer transitions.
- `RowActionEngine` - Controller managing single-row activation pendency and duplicate filtering.
- `MenuRow` - Iced widget rendering individual menu items with focus, check, and shortcut slots.

#### Return values

- `open`, `toggle`, and `on_key` return `CascadeOutcome`.
- `CascadeOutcome::focus_task` returns an iced `Task` applying focus changes in the widget tree.
- `activate` returns `Some(Activation)` or `None` if the row is non-interactive.
- `select_radio` returns `Option<Vec<usize>>` containing updated row indices.
- `validate` returns a vector of `MenuViolation`s (empty if compliant).

#### Side effects

- `MenuCascade` transitions mutate open cards and modify `FocusState`.
- `AimCorridor` mutates internal timers against supplied `TimeMs` without reading system clocks.
- `sync` overwrites cascade cards with models from source definitions.

#### References

[`surface/menu/mod.rs`](file:///D:/hoarfrost/src/hoarfrost/surface/menu/mod.rs)
[`model.rs`](file:///D:/hoarfrost/src/hoarfrost/surface/menu/model.rs)
[`cascade.rs`](file:///D:/hoarfrost/src/hoarfrost/surface/menu/cascade.rs)
[`aim.rs`](file:///D:/hoarfrost/src/hoarfrost/surface/menu/aim.rs)
[`row_action.rs`](file:///D:/hoarfrost/src/hoarfrost/surface/menu/row_action.rs)
[`view.rs`](file:///D:/hoarfrost/src/hoarfrost/surface/menu/view.rs)

---

### `surface::popover` - anchored teaching cards (`UIF-SURF-005`)

#### Overview

`surface::popover` implements the interactive teaching callout card anchored to trigger elements in Band 2 (`UIF-ATOM-007`, `UIF-SURF-005`). It pairs a focus-trapped model with placement resolution and strict dismissal precedence, consuming Escape at Rank 3 without destroying underlying surfaces.

#### Prerequisites & Requirements

- Cargo feature `popover` (which implies `zorder`) **MUST** be enabled.
- Popover surfaces **MUST** be registered in Band 2 (`SurfaceKind::TeachingPopover`), inheriting click-away catchers.
- A callout model **MUST** provide at least one focusable control; the framework-supplied dismiss button guarantees non-empty focus groups.
- Dismissal precedence is fixed: footer buttons outrank Escape, which outranks click-away, which outranks programmatic closure.
- Popovers **MUST NOT** dismiss on pointer exit, allowing cursor travel across gaps to reach action buttons.

#### Syntax / Method Signature

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
    pub const fn placement(&self) -> PreferredSide;
    pub const fn is_open_wanted(&self) -> bool;
    pub fn has_primary_action(&self) -> bool;
    pub fn footer_members(&self) -> Vec<FocusMember>;
    pub fn initial_focus(&self) -> FocusId;
}

pub const DISMISS_LABEL: &str = "Dismiss";
pub const POPOVER_GROUP: &str = "popover";
pub fn group_for() -> GroupId;
pub fn primary_focus_id() -> FocusId;
pub fn dismiss_focus_id() -> FocusId;

// place
pub enum PreferredSide {
    #[default]
    Below,
    Above,
    Leading,
    Trailing,
}

pub const POPOVER_GAP: f32 = space::XS;
pub type Placement = anchor::Placement;
pub fn resolve(trigger: Rectangle, side: PreferredSide, card_size: Size, viewport: Rectangle) -> Placement;

// host
pub enum DismissCause {
    PrimaryAction,
    DismissButton,
    Escape,
    ClickOutside,
    Programmatic,
}

pub enum IgnoredReason {
    NotOpen,
    NoPrimaryAction,
    InsideCard,
}

pub enum PopoverEvent<ActionId> {
    Action(ActionId),
    Dismissed(DismissCause),
    Ignored(IgnoredReason),
}

pub enum OpenRefusal {
    NotRequested,
    NoFocusableControl,
}

pub struct PopoverHost<ActionId: Clone + PartialEq>;

impl<ActionId: Clone + PartialEq> PopoverHost<ActionId> {
    pub fn open(
        id: SurfaceId,
        model: PopoverModel<ActionId>,
        trigger: Rectangle,
        card_size: Size,
        viewport: Rectangle,
        focus: &mut FocusState,
        registry: &mut SurfaceRegistry,
    ) -> Result<Self, OpenRefusal>;
    pub fn id(&self) -> &SurfaceId;
    pub fn group_id(&self) -> &GroupId;
    pub fn model(&self) -> &PopoverModel<ActionId>;
    pub fn is_inside_card(&self, at: Point) -> bool;
    pub fn footer_ids(&self) -> Vec<FocusId>;
    pub fn activate_primary(&mut self, focus: &mut FocusState) -> PopoverEvent<ActionId>;
    pub fn dismiss_dismiss_button(&mut self, focus: &mut FocusState) -> PopoverEvent<ActionId>;
    pub fn dismiss(&mut self, cause: DismissCause, focus: &mut FocusState) -> PopoverEvent<ActionId>;
    pub fn on_pointer(&mut self, at: Point, focus: &mut FocusState) -> PopoverEvent<ActionId>;
    pub fn on_escape(&mut self, focus: &mut FocusState) -> PopoverEvent<ActionId>;
    pub fn close(&mut self, focus: &mut FocusState, registry: &mut SurfaceRegistry) -> PopoverEvent<ActionId>;
    pub fn unwind_answer(&mut self, focus: &mut FocusState) -> UnwindAnswer;
    pub fn escape_via(&mut self, registry: &mut SurfaceRegistry, focus: &mut FocusState) -> EscapeOutcome;
}

// style and box limits
pub struct PopoverStyle {
    pub background: Color,
    pub border_color: Color,
    pub border_width: f32,
    pub radius: f32,
    pub padding: f32,
    pub title_color: Color,
    pub title_size: f32,
    pub title_weight: Weight,
    pub body_color: Color,
    pub body_size: f32,
    pub body_weight: Weight,
    pub width_min: f32,
    pub width_max: f32,
}

impl PopoverStyle {
    pub fn resolve(palette: &Palette) -> Self;
}

pub fn clamp_width(measured: f32) -> f32;
pub fn card_chrome_class(style: &PopoverStyle, palette: &Palette) -> ChromeClass;

pub const WIDTH_MIN: f32 = 280.0;
pub const WIDTH_MAX: f32 = 360.0;
pub const PADDING: f32 = space::MD;
pub const BORDER_WIDTH: f32 = shape::BORDER_HAIRLINE;
pub const RADIUS: f32 = shape::RADIUS_CARD;
pub const TYPE_RUNG: typography::TypeRung = typography::UI_COMPACT;
pub const TITLE_WEIGHT: Weight = Weight::SemiBold;
pub const BODY_WEIGHT: Weight = Weight::Regular;
```

#### Examples

```rust
use hoarfrost::surface::popover::{OpenRefusal, PopoverHost, PopoverModel};
use iced::{Rectangle, Size};

let model = PopoverModel::new("Exporting", "Select a target format before exporting.")
    .with_action("Understood", "ack")
    .with_open(true);

match PopoverHost::open(
    "popover-1".into(),
    model,
    Rectangle::default(),
    Size::new(300.0, 140.0),
    viewport,
    &mut focus,
    &mut registry,
) {
    Ok(mut host) => {
        // Active popover card mounted in Band 2
    }
    Err(OpenRefusal::NotRequested) => {}
    Err(OpenRefusal::NoFocusableControl) => {}
}
```

#### Type references

- `PopoverModel` - Declarative card model with title, body, optional primary action, and placement side.
- `PopoverHost` - Lifecycle host managing placement resolution, focus traps, and Escape routing.
- `PreferredSide` - Desired placement orientation relative to trigger (`Below`, `Above`, `Leading`, `Trailing`).
- `PopoverEvent` - Interaction event produced by buttons or dismiss triggers.
- `OpenRefusal` - Reason preventing card opening (`NotRequested` or `NoFocusableControl`).
- `PopoverStyle` - Resolved appearance properties conforming to contrast locks.
- `clamp_width` - Module-level width clamper enforcing the 280px - 360px range.
- `card_chrome_class` - Module-level classifier for contrast audit rules.

#### Return values

- `open` returns `Ok(PopoverHost)` on success or `Err(OpenRefusal)` if opening was refused.
- `activate_primary` returns `PopoverEvent::Action` or `PopoverEvent::Ignored(IgnoredReason::NoPrimaryAction)`.
- `dismiss` returns `PopoverEvent::Dismissed(cause)`.
- `unwind_answer` returns `UnwindAnswer::Dismissed` when open, or `Declined(DeclineReason::NotOpen)` when closed.
- `clamp_width` clamps inputs into the `280.0..=360.0` band, falling back to `WIDTH_MIN` for non-finite values.

#### Side effects

- `open` registers into `SurfaceRegistry` (Band 2) and acquires the popover focus trap in `FocusState`.
- `close` and dismissals pop the trap and remove the surface registration.
- `PopoverStyle::resolve` verifies foreground-to-fill contrast locks.

#### References

[`surface/popover/mod.rs`](file:///D:/hoarfrost/src/hoarfrost/surface/popover/mod.rs)
[`model.rs`](file:///D:/hoarfrost/src/hoarfrost/surface/popover/model.rs)
[`place.rs`](file:///D:/hoarfrost/src/hoarfrost/surface/popover/place.rs)
[`host.rs`](file:///D:/hoarfrost/src/hoarfrost/surface/popover/host.rs)

---

### `surface::tooltip` - hover and focus tooltips (`UIF-SURF-008`)

#### Overview

`surface::tooltip` implements the framework-owned hover and keyboard-focus tooltip subsystem (`UIF-ATOM-007`, `UIF-SURF-008`). It operates as an elevated presentation layer outside the z-band registry, timing visibility through pure time values and supporting zero-latency warm handoffs.

#### Prerequisites & Requirements

- Cargo feature `tooltip` **MUST** be enabled.
- Timing calls **MUST** receive `TimeMs` values supplied by the caller; host controllers **MUST NOT** query system clocks directly.
- Visibility **MUST** be determined via `HoverController::tick(now)`.
- Keyboard-focus arrivals **MUST** open immediately at 0 ms delay (`KEYBOARD_FOCUS_DELAY`).
- Tooltip dismissals **MUST NOT** consume pointer or keyboard input; `Dismissal::consumed_input` is **ALWAYS** `false`.
- Measured label widths **MUST** be clamped with `clamp_width` against `MAX_WIDTH` (280px).

#### Syntax / Method Signature

```rust
pub struct TimeMs(pub u64);

impl TimeMs {
    pub fn now() -> Self;
    pub const fn millis(ms: u64) -> Self;
    pub const fn as_millis(self) -> u64;
    pub const fn elapsed_since(self, earlier: Self) -> u64;
}

pub enum Visibility {
    Hidden,
    Pending,
    Shown,
}

pub enum ArrivalSource {
    Pointer,
    Keyboard,
}

pub enum DismissReason {
    PointerExit,
    MousePress,
    AnchorGone,
    FocusMoved,
    Escape,
    LeftWindow,
}

pub struct Dismissal {
    pub reason: DismissReason,
    pub was_active: bool,
    pub consumed_input: bool,
}

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
pub enum ArrivalWarmth {
    Cold,
    Warm,
}

pub struct WarmSession;

impl WarmSession {
    pub fn note_visible(&mut self, at: TimeMs);
    pub fn warmth_for(&self, arrival: TimeMs) -> ArrivalWarmth;
    pub fn is_warm(&self, now: TimeMs) -> bool;
    pub fn warmth_remaining(&self, now: TimeMs) -> u64;
    pub fn reset(&mut self);
}

pub const fn handoff_window_ms() -> u64;
pub const fn cold_wait_ms() -> u64;
pub const fn warm_wait_ms() -> u64;

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
pub struct TooltipContent {
    pub label: String,
    pub shortcut: Option<String>,
}

impl TooltipContent {
    pub fn new(label: impl Into<String>) -> Self;
    pub fn with_shortcut(label: impl Into<String>, shortcut: impl Into<String>) -> Self;
}

pub const SHORTCUT_GAP: f32 = space::SM;
pub const HOVER_DELAY: u64 = time::DELAY_TOOLTIP_OPEN.ms;
pub const KEYBOARD_FOCUS_DELAY: u64 = time::INSTANT.ms;
pub const MAX_WIDTH: f32 = 280.0;
pub const PADDING_VERTICAL: f32 = space::XS;
pub const PADDING_HORIZONTAL: f32 = space::SM;
pub fn clamp_width(measured: f32) -> f32;

pub struct TooltipStyle {
    pub background: Color,
    pub foreground: Color,
    pub border_color: Color,
    pub border_width: f32,
    pub radius: f32,
    pub font: typography::TypeRung,
    pub max_width: f32,
    pub padding_vertical: f32,
    pub padding_horizontal: f32,
    pub shortcut_foreground: Color,
    pub shortcut_font: typography::FontFamily,
}

impl TooltipStyle {
    pub fn resolve(palette: &Palette) -> Self;
}

pub fn card_size(label_width: f32, shortcut_width: Option<f32>, style: &TooltipStyle) -> Size;
pub fn card_quad(bounds: Rectangle, style: &TooltipStyle) -> renderer::Quad;

pub struct TooltipState;

impl TooltipState {
    pub fn new() -> Self;
    pub fn is_shown(&self, now: TimeMs) -> bool;
}

pub struct Tooltip<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer>;

impl<'a, Message, Theme, Renderer> Tooltip<'a, Message, Theme, Renderer> {
    pub fn new(
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
        tooltip: impl Into<TooltipContent>,
    ) -> Self;
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

#### Examples

```rust
use hoarfrost::surface::tooltip::{tooltip, HoverController, TimeMs, TooltipContent};
use iced::widget::button;

let content = TooltipContent::with_shortcut("Save", "Ctrl+S");
let wrapped_btn = tooltip(button("Save"), content);

let mut controller = HoverController::new();
controller.pointer_enter(TimeMs(0));
let state = controller.tick(TimeMs(400));
```

#### Type references

- `TimeMs` - Host-injected monotonic time value in milliseconds.
- `HoverController` - State machine governing pointer and keyboard delay transitions.
- `WarmSession` - Cross-control cache coordinating instant warm handoffs across adjacent tooltips.
- `TooltipAnchor` - Placement anchor mapping to control bounds, composite bands, or cursor points.
- `TooltipContent` - Tooltip payload consisting of primary explanation text and optional shortcut.
- `TooltipStyle` - Resolved theme properties matching token contrast standards.
- `Tooltip` - Iced element wrapper displaying anchored tooltip cards.

#### Return values

- `pointer_enter` and `keyboard_focus_enter` return `true` if the controller became active.
- `tick` returns `Visibility::Hidden`, `Visibility::Pending`, or `Visibility::Shown`.
- `dismiss` returns a `Dismissal` struct where `consumed_input` is **always `false`**.
- `clamp_width` bounds measurements to `0.0..=MAX_WIDTH` (280px).

#### Side effects

- `HoverController` and `WarmSession` update internal timestamps and suppression records.
- Tooltips render as non-interactive overlay layers outside `SurfaceRegistry` and do not consume input events.

#### References

[`surface/tooltip/mod.rs`](file:///D:/hoarfrost/src/hoarfrost/surface/tooltip/mod.rs)
[`hover.rs`](file:///D:/hoarfrost/src/hoarfrost/surface/tooltip/hover.rs)
[`warm.rs`](file:///D:/hoarfrost/src/hoarfrost/surface/tooltip/warm.rs)
[`place.rs`](file:///D:/hoarfrost/src/hoarfrost/surface/tooltip/place.rs)
[`widget.rs`](file:///D:/hoarfrost/src/hoarfrost/surface/tooltip/widget.rs)

---

### `geometry::anchor` and `geometry::clamp` - placement and clamping

#### Overview

`geometry::anchor` and `geometry::clamp` provide pure boundary arithmetic for placing, flipping, and clamping floating surfaces against triggers or pointers (`UIF-CMENU-001`, `UIF-MOV-002`). They guarantee title-bar reachability and detect viewport collisions without widget dependencies or clock access.

#### Prerequisites & Requirements

- Viewport bounds **MUST** be supplied as a `Rectangle` in root coordinates.
- Card dimensions **MUST** be measured before solving placement.
- When using `Anchor::CursorProximal { keyboard_invoked: true, .. }`, the card **MUST** be centred in the viewport inset.
- Clamping of movable cards **MUST** reuse `ClampRule::MOVABLE` to prevent reachability divergence.
- Hosts **MUST** read `Placement::was_flipped()` and `was_clamped()` flags instead of recalculating edge bounds.

#### Syntax / Method Signature

```rust
pub enum Anchor {
    Pointer { point: Point },
    Control { rect: Rectangle, alignment: ControlAlignment },
    CursorProximal { point: Point, keyboard_invoked: bool },
}

pub enum VerticalAlign {
    Below,
    Above,
    Top,
    Bottom,
    Middle,
}

pub enum HorizontalAlign {
    Start,
    End,
    Center,
    Trailing,
    Leading,
}

pub struct ControlAlignment {
    pub vertical: VerticalAlign,
    pub horizontal: HorizontalAlign,
}

impl ControlAlignment {
    pub const fn new(vertical: VerticalAlign, horizontal: HorizontalAlign) -> Self;
    pub const fn below_start() -> Self;
    pub const fn below_end() -> Self;
    pub const fn submenu() -> Self;
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

pub struct SizePolicy {
    pub min: Size,
    pub max: Option<Size>,
}

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

impl ClampRule {
    pub const MOVABLE: Self;
}

pub enum ResizeHandle {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
}

impl ResizeHandle {
    pub const ALL: [Self; 8];
    pub const fn drags_start_x(self) -> bool;
    pub const fn drags_end_x(self) -> bool;
    pub const fn drags_start_y(self) -> bool;
    pub const fn drags_end_y(self) -> bool;
}

pub fn clamp_position(position: Point, card_size: Size, viewport: Rectangle, rule: ClampRule) -> Point;
pub fn drag_position(current: Point, delta: Vector, card_size: Size, viewport: Rectangle, rule: ClampRule) -> Point;
pub fn clamp_rect(rect: Rectangle, viewport: Rectangle, rule: ClampRule) -> Rectangle;
pub fn clamp_size(rect: Rectangle, viewport: Rectangle, rule: ClampRule) -> Rectangle;
pub fn resize(
    rect: Rectangle,
    handle: ResizeHandle,
    delta: Vector,
    viewport: Rectangle,
    rule: ClampRule,
) -> Rectangle;
pub fn is_reachable(rect: Rectangle, viewport: Rectangle, rule: ClampRule) -> bool;
pub fn title_bar_rect(rect: Rectangle, rule: ClampRule) -> Rectangle;
pub fn visible_title_width(rect: Rectangle, viewport: Rectangle) -> f32;
```

#### Examples

```rust
use hoarfrost::geometry::anchor::{solve, Anchor, ControlAlignment};
use iced::{Point, Rectangle, Size};

let placement = solve(
    Anchor::CursorProximal {
        point: Point::ORIGIN,
        keyboard_invoked: true,
    },
    Size::new(320.0, 192.0),
    Rectangle::with_size(Size::new(1024.0, 768.0)),
);
// Centred in viewport when keyboard_invoked is true
assert!(!placement.was_flipped());
```

#### Type references

- `Anchor` - Placement reference: `Pointer`, `Control` (with alignment), or `CursorProximal`.
- `ControlAlignment` - Cross-axis alignment specification for control-anchored cards.
- `Placement` - Solved card origin and size alongside flip and clamp status indicators.
- `ClampRule` - Constraints specifying minimum visible title bar, top bounds, and size limits.
- `ResizeHandle` - Eight edge and corner handles driving directional resizing math.

#### Return values

- `solve` calculates a non-failing `Placement` containing origin, dimensions, and diagnostic flags.
- `visible_height` returns the unobstructed vertical extent remaining within viewport margins.
- `clamp_position` and `drag_position` return the validated top-left `Point`.
- `is_reachable` returns `true` if the card's title bar preserves `min_visible_title` within bounds.

#### Side effects

None. All functions in `geometry::anchor` and `geometry::clamp` are pure calculations.

#### References

[`geometry/anchor.rs`](file:///D:/hoarfrost/src/hoarfrost/geometry/anchor.rs)
[`geometry/clamp.rs`](file:///D:/hoarfrost/src/hoarfrost/geometry/clamp.rs)

---

### `geometry::window_geometry` and `geometry::pane_solver` - window and pane maths

#### Overview

`geometry::window_geometry` and `geometry::pane_solver` govern the window sizing schema, UI scale ladder, and pane division geometry (`UIF-WIN-001`, `UIF-PANE-002`). They preserve central workspace width floors and sanitize pane adjustments across varied display scales.

#### Prerequisites & Requirements

- Window geometry persistence **MUST** use the `WindowStorage` trait interface; tests **MUST** use `NoopWindowStorage` or `MemoryWindowStorage` without touching live configuration files.
- UI scaling **MUST** be instantiated via `UiScale::new` to ensure strict `0.75x..=2.00x` clamping with `0.10x` step quantization.
- Window layouts **MUST** reserve at least `CENTER_PANE_MIN_W` (240px) for the central workspace pane.

#### Syntax / Method Signature

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
    pub fn new(val: f32) -> Self;
    pub fn step_up(self) -> Self;
    pub fn step_down(self) -> Self;
    pub fn factor(self) -> f32;
    pub fn compose_with_dpi(self, os_dpi_scale: f32) -> f32;
}

pub struct WindowGeometry {
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub width: f32,
    pub height: f32,
    pub left_split_ratio: f32,
    pub right_split_ratio: f32,
    pub ui_scale: UiScale,
}

pub struct SolvedWindowLayout {
    pub window_width: f32,
    pub window_height: f32,
    pub left_pane_width: f32,
    pub center_pane_width: f32,
    pub right_pane_width: f32,
}

pub fn solve_window_layout(
    requested_w: f32,
    requested_h: f32,
    left_ratio: f32,
    right_ratio: f32,
    right_pane_mounted: bool,
) -> SolvedWindowLayout;

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

pub fn solve_clamped_pane_width(viewport_w: f32, requested_w: f32, min_px: f32, max_px: f32) -> f32;
pub fn auto_fit_pane_width(
    labels: &[&str],
    font_size: f32,
    icon_w: f32,
    padding: f32,
    min_w: f32,
    max_w: f32,
) -> f32;
```

#### Examples

```rust
use hoarfrost::geometry::window_geometry::{
    solve_window_layout, MemoryWindowStorage, UiScale, WindowGeometry, WindowStorage,
};

let store = MemoryWindowStorage::new();
let scale = UiScale::new(1.25);
let geom = WindowGeometry {
    x: Some(100.0),
    y: Some(80.0),
    width: 1200.0,
    height: 800.0,
    left_split_ratio: 0.2,
    right_split_ratio: 0.25,
    ui_scale: scale,
};
store.save_geometry(&geom);

let layout = solve_window_layout(geom.width, geom.height, geom.left_split_ratio, geom.right_split_ratio, true);
assert!(layout.center_pane_width >= 240.0);
```

#### Type references

- `UiScale` - Quantized UI scale factor clamped to the 0.75x - 2.00x scale ladder in 0.10x increments.
- `WindowGeometry` - Complete serializable window position, dimension, pane split, and scale data.
- `SolvedWindowLayout` - Solved pixel dimensions for left, center, and right window panes.
- `WindowStorage` - Thread-safe persistence seam with in-memory test mocks.

#### Return values

- `solve_window_layout` returns `SolvedWindowLayout`, ensuring `center_pane_width >= CENTER_PANE_MIN_W`.
- `solve_clamped_pane_width` returns the clamped width respecting `min_px` and viewport limits.
- `auto_fit_pane_width` returns the calculated width fitting the longest label without wrapping.

#### Side effects

- `MemoryWindowStorage` saves window state in an in-memory lock without disk I/O.
- Solvers are pure arithmetic operations.

#### References

[`geometry/window_geometry.rs`](file:///D:/hoarfrost/src/hoarfrost/geometry/window_geometry.rs)
[`geometry/pane_solver.rs`](file:///D:/hoarfrost/src/hoarfrost/geometry/pane_solver.rs)

---

### `layout::density`, `layout::emphasis`, `layout::footer`, `layout::optical` - typography and layout rules

#### Overview

`layout` translates spatial design principles into machine-checkable layout validators covering row density homogeneity, single focal points, ordered footer actions, and optical glyph centring (`UIF-PRIN-006`). Each checker is a pure diagnostic tool verifying UI structure without rendering.

#### Prerequisites & Requirements

- All controls in a single horizontal row **MUST** share the same declared `ControlDensity` (`Compact = 28px` or `Standard = 32px`); mixing rungs within a single band is strictly forbidden (`UIF-PRIN-006`).
- Containers **MUST NOT** include more than one `Emphasis::High` control.
- Footer action layouts **MUST** follow canonical slot order: `Utility`, then `Cancel`, then `Confirm`.
- Destructive footers **MUST** include an explicit `Cancel` action and **MUST** assign initial focus to `Cancel`.
- Checkers **MUST** receive accurate measured heights rather than assumed token defaults.

#### Syntax / Method Signature

```rust
// density
pub enum ControlDensity {
    Compact,
    Standard,
}

impl ControlDensity {
    pub const ALL: [Self; 2];
    pub const fn height(self) -> f32;
    pub fn from_height(height: f32) -> Option<Self>;
    pub const fn slug(self) -> &'static str;
}

pub enum DensityBand {
    Navbar,
    FormRow,
    OverflowToolbar,
    FilterBar,
    DialogFooter,
}

pub struct RowSlot<'a> {
    pub name: &'a str,
    pub height: f32,
}

impl RowSlot<'_> {
    pub fn rung(&self) -> Option<ControlDensity>;
}

pub struct DensityViolation<'a> {
    pub band: DensityBand,
    pub declared: ControlDensity,
    pub control: &'a str,
    pub height: f32,
    pub rung: Option<ControlDensity>,
}

pub struct DensityReport<'a> {
    pub band: DensityBand,
    pub declared: ControlDensity,
    pub violations: Vec<DensityViolation<'a>>,
    pub rung_spread: Vec<ControlDensity>,
}

impl DensityReport<'_> {
    pub fn is_homogeneous(&self) -> bool;
    pub fn mixes_rungs(&self) -> bool;
    pub fn off_rung_count(&self) -> usize;
}

pub fn check_row_density<'a>(
    band: DensityBand,
    declared: ControlDensity,
    slots: &[RowSlot<'a>],
) -> DensityReport<'a>;
pub fn overflow_toolbar_declared_density() -> Option<ControlDensity>;
pub fn form_field_declared_density() -> Option<ControlDensity>;
pub fn check_overflow_toolbar<'a>(controls: &[RowSlot<'a>]) -> Option<DensityReport<'a>>;
pub fn check_form_field_row<'a>(controls: &[RowSlot<'a>]) -> Option<DensityReport<'a>>;

// emphasis
pub enum Emphasis {
    High,
    Medium,
    Quiet,
}

pub enum ContainerScope {
    ModalFooter,
    ToolbarCluster,
    Card,
    Form,
}

pub struct EmphasisSlot<'a> {
    pub name: &'a str,
    pub emphasis: Emphasis,
}

impl EmphasisSlot<'_> {
    pub fn of_button_variant(name: &'a str, variant: ButtonVariant) -> Self;
    pub fn of_dialog_role(name: &'a str, role: DialogButtonRole) -> Self;
}

impl From<ButtonVariant> for Emphasis;
impl From<DialogButtonRole> for Emphasis;

pub struct FocalPointReport<'a> {
    pub scope: ContainerScope,
    pub high: Vec<EmphasisSlot<'a>>,
    pub total: usize,
}

impl FocalPointReport<'_> {
    pub fn is_single_focal_point(&self) -> bool;
    pub fn excess(&self) -> usize;
    pub fn high_names(&self) -> Vec<&'a str>;
}

pub fn check_focal_point<'a>(scope: ContainerScope, slots: &[EmphasisSlot<'a>]) -> FocalPointReport<'a>;

// footer
pub enum FooterSlot {
    Utility,
    Cancel,
    Confirm,
}

impl FooterSlot {
    pub const ORDER: [Self; 3];
    pub const ALL: [Self; 3];
    pub const fn position(self) -> u8;
}

pub const GROUP_GAP: f32 = space::SM;
pub enum FooterAlignment {
    FarLeft,
    Right,
}
pub enum FooterKind {
    Plain,
    Destructive,
}

pub struct FooterAction<'a> {
    pub name: &'a str,
    pub slot: FooterSlot,
    pub emphasis: Emphasis,
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
    pub kind: FooterKind,
    pub density: ControlDensity,
    pub scope: ContainerScope,
    pub actions: Vec<FooterAction<'a>>,
    pub initial_focus: Option<&'a str>,
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

pub struct OpticalInset {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl OpticalInset {
    pub const ZERO: Self;
    pub const fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self;
    pub fn exceeds(&self, slot_width: f32, slot_height: f32) -> bool;
    pub fn is_symmetric(&self) -> bool;
}

pub struct OpticalShift {
    pub x: f32,
    pub y: f32,
}

pub fn optical_box(slot_width: f32, slot_height: f32, inset: OpticalInset) -> Rectangle;
pub fn optical_shift(inset: OpticalInset) -> OpticalShift;
```

#### Examples

```rust
use hoarfrost::layout::density::{check_row_density, ControlDensity, DensityBand, RowSlot};

let slots = [
    RowSlot { name: "search", height: 32.0 },
    RowSlot { name: "filter", height: 32.0 },
];
let report = check_row_density(DensityBand::FilterBar, ControlDensity::Standard, &slots);
assert!(report.is_homogeneous());
```

#### Type references

- `ControlDensity` - Closed enum (`Compact = 28px` or `Standard = 32px`) representing row control density rungs.
- `DensityReport` - Diagnostic output reporting row violations, rung spread, and homogeneity.
- `Emphasis` - Relative importance level (`High`, `Medium`, `Quiet`) assigned to interactive controls.
- `FocalPointReport` - Verification outcome ensuring no container has more than one `High` emphasis item.
- `FooterPlan` - Complete data model for dialog footers defining slots, actions, and initial focus.
- `OpticalInset` - Per-edge margins separating a glyph's em box from its visible ink.

#### Return values

- `check_row_density` returns `DensityReport` containing violations and rung spread.
- `check_focal_point` returns `FocalPointReport` indicating if `excess() == 0`.
- `FooterPlan::validate` returns `Vec<FooterViolation>` (empty if valid).
- `optical_shift` returns the translation `OpticalShift` required to centre glyph ink geometrically.

#### Side effects

None. All checking routines and calculation functions are pure operations.

#### References

[`layout/density.rs`](file:///D:/hoarfrost/src/hoarfrost/layout/density.rs)
[`layout/emphasis.rs`](file:///D:/hoarfrost/src/hoarfrost/layout/emphasis.rs)
[`layout/footer.rs`](file:///D:/hoarfrost/src/hoarfrost/layout/footer.rs)
[`layout/optical.rs`](file:///D:/hoarfrost/src/hoarfrost/layout/optical.rs)

---

### `a11y` - accessibility contracts and tab-stop rules

#### Overview

`a11y` defines pure semantic accessibility descriptors and single tab-stop validation for composite controls (`UIF-PRIN-008`). It derives screen-reader contracts directly from widget models to eliminate semantic drift without layout dependencies.

#### Prerequisites & Requirements

- Descriptors **MUST** be derived from widget models using `project_*` functions rather than being manually created.
- Fields in `A11yContract` (except `role`) **MUST** be `Some` only when supported by the underlying widget model; bare contracts are invalid.
- Bounded composite groups **MUST** use roving tab stops that wrap at edges, whereas spatial collections **MUST** clamp at edges.
- Each composite control **MUST** register as exactly one tab stop (`UIF-PRIN-008`).

#### Syntax / Method Signature

```rust
pub enum A11yRole {
    Button,
    Checkbox,
    Radio,
    RadioGroup,
    Group,
    Menu,
    MenuItem,
    MenuItemCheckBox,
    MenuItemRadio,
    Separator,
    Heading,
    TreeItem,
    TextBox,
    ProgressBar,
}

pub enum Checked {
    Unchecked,
    Indeterminate,
    Checked,
}

pub enum A11yValue<'a> {
    Text(&'a str),
    Number(i64),
    Fraction(f32),
}

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

pub enum EdgeBehaviour {
    Wraps,
    Clamps,
}

pub enum CompositeCategory {
    BoundedGroup,
    SpatialCollection,
}

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
    WrongEdgeBehaviour {
        id: &'a str,
        category: CompositeCategory,
        declared: EdgeBehaviour,
        required: EdgeBehaviour,
    },
    EnabledExceedsMembers {
        id: &'a str,
        member_count: usize,
        enabled_count: usize,
    },
    ActiveMemberNotEnabled {
        id: &'a str,
        active: usize,
    },
}

pub fn check_composite<'a>(boundary: &CompositeBoundary<'a>) -> Vec<TabStopViolation<'a>>;
pub fn check_roster<'a>(boundaries: &[CompositeBoundary<'a>]) -> Vec<TabStopViolation<'a>>;
pub fn total_tab_stops(boundaries: &[CompositeBoundary<'_>]) -> usize;
pub fn total_members(boundaries: &[CompositeBoundary<'_>]) -> usize;

// Semantic projection functions
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
pub fn project_nav_rail_destination<DestId>(
    destination: &NavDestination<DestId>,
    is_selected: bool,
) -> A11yContract<'_>;
pub fn project_stepper_step<StepId>(step: &StepItem<StepId>, is_active: bool) -> A11yContract<'_>;
pub fn project_grid_tile<Id>(tile: &GridTile<Id>, is_selected: bool) -> A11yContract<'_>;
pub fn project_command_palette_item<ItemId>(
    item: &PaletteItem<ItemId>,
    is_selected: bool,
) -> A11yContract<'_>;
```

#### Examples

```rust
use hoarfrost::a11y::{check_composite, CompositeBoundary, CompositeCategory, EdgeBehaviour};

let boundary = CompositeBoundary {
    id: "nav-tabs",
    category: CompositeCategory::BoundedGroup,
    edge: EdgeBehaviour::Wraps,
    member_count: 3,
    enabled: &[0, 1, 2],
    active: Some(0),
};
assert!(check_composite(&boundary).is_empty());
```

#### Type references

- `A11yRole` - Closed set of 14 ARIA roles directly supported by hoarfrost widgets.
- `A11yContract` - Semantic descriptor exposing accessibility metadata for assistive tools and tests.
- `CompositeCategory` - Classification: `BoundedGroup` (wraps) or `SpatialCollection` (clamps).
- `CompositeBoundary` - Specification of roving focus membership, edge rules, and active states.
- `TabStopViolation` - Diagnostic errors flagging invalid boundary configurations.

#### Return values

- `check_composite` and `check_roster` return `Vec<TabStopViolation>` (empty if compliant).
- `total_tab_stops` returns the count of composite boundaries (one stop per composite).
- `A11yContract::is_bare` returns `true` if all optional metadata fields are `None`.

#### Side effects

None. All contract projections and boundary audits are purely functional.

#### References

[`a11y/contract.rs`](file:///D:/hoarfrost/src/hoarfrost/a11y/contract.rs)
[`a11y/project.rs`](file:///D:/hoarfrost/src/hoarfrost/a11y/project.rs)
[`a11y/tab_stop.rs`](file:///D:/hoarfrost/src/hoarfrost/a11y/tab_stop.rs)

---

<a id="utility-functions"></a>
### `microcopy` - label rules as checkers (`UIF-PRIN-007`)

#### Overview

`microcopy` transforms specification microcopy requirements into machine-checkable verifiers for action ellipsis biconditionals, verb-first CTAs, sentence casing, literal pins, tabular numerics, and truncation tooltips (`UIF-PRIN-007`). It operates as test-time auditing machinery to prevent copy drift.

#### Prerequisites & Requirements

- Action labels opening secondary dialogs or prompts **MUST** end with unspaced ASCII `"..."` (`ASCII_ACTION_ELLIPSIS`).
- Navbar menu triggers that open dialogs **MUST** end with unspaced `"..."` (`check_action_ellipsis`).
- Immediate actions **MUST NOT** carry an ellipsis.
- Unicode `…` (U+2026) **MUST NOT** be used as an action suffix; it is strictly reserved for layout truncation, placeholders, and breadcrumb elision.
- Generic CTA labels (`OK`, `Yes`, `No`) **MUST NOT** be used where specific imperative verbs apply.
- Pinned literals in `HOST_PIN_TABLE` are matched by exact, case-sensitive equality.
- Truncated text slots **MUST** provide an accessible tooltip containing the full untruncated string.

#### Syntax / Method Signature

```rust
// action_label
pub const ASCII_ACTION_ELLIPSIS: &str = "...";
pub const UNICODE_ELLIPSIS: char = '\u{2026}';

pub enum PromptBehaviour {
    Immediate,
    LaunchesPrompt,
}

impl PromptBehaviour {
    pub const fn requires_action_ellipsis(self) -> bool;
    pub const fn slug(self) -> &'static str;
}

pub struct ActionLabel<'a> {
    pub label: &'a str,
    pub behaviour: PromptBehaviour,
}

pub enum EllipsisViolation<'a> {
    MissingActionEllipsis { label: &'a str },
    SpuriousActionEllipsis { label: &'a str },
    SpacedActionEllipsis { label: &'a str },
    UnicodeEllipsisAsActionSuffix { label: &'a str },
}

pub struct EllipsisVerdict<'a> {
    pub label: &'a str,
    pub behaviour: PromptBehaviour,
    pub violation: Option<EllipsisViolation<'a>>,
    pub exempt: bool,
}

impl EllipsisVerdict<'_> {
    pub fn is_compliant(&self) -> bool;
}

pub fn ends_with_ascii_ellipsis(label: &str) -> bool;
pub fn ends_with_unicode_ellipsis(label: &str) -> bool;
pub fn is_spaced_ellipsis(label: &str) -> bool;
pub fn check_action_ellipsis<'a>(action: &ActionLabel<'a>) -> EllipsisVerdict<'a>;

pub const GENERIC_CTA_LABELS: [&str; 4] = ["ok", "okay", "yes", "no"];
pub fn is_generic_cta(label: &str) -> bool;

pub struct VerbFirstVerdict<'a> {
    pub label: &'a str,
    pub generic: bool,
    pub exempt: bool,
}

impl VerbFirstVerdict<'_> {
    pub fn is_compliant(&self) -> bool;
}

pub fn check_verb_first(label: &str) -> VerbFirstVerdict<'_>;
pub fn check_action<'a>(action: &ActionLabel<'a>) -> (EllipsisVerdict<'a>, VerbFirstVerdict<'a>);

// case_rule
pub enum WordCase {
    Uncased,
    Lower,
    Title,
    Upper,
    Mixed,
}

impl WordCase {
    pub fn of(word: &str) -> Self;
}

pub enum Capitalisation {
    Uncasable,
    Sentence,
    AllCaps,
    MidWordCapital,
    FirstWordLowercase,
    TitleCase,
}

pub const SPEC_NAMED_LICENSED: [&str; 4] = ["Markdown", "PDF", "MCP", "JSON"];
pub const DEFAULT_LICENSED: &[&str] = &[
    "AI", "API", "CSV", "DAV", "Git", "GitHub", "HTML", "ID", "JMAP", "JSON",
    "MCP", "Markdown", "OAuth", "PDF", "TOML", "UI", "URI", "URL", "Windows", "XML", "YAML",
];

pub struct CaseReport<'a> {
    pub label: &'a str,
    pub capitalisation: Capitalisation,
    pub exempt: bool,
    pub pinned_by: Option<&'static str>,
}

impl CaseReport<'_> {
    pub fn is_compliant(&self) -> bool;
}

pub fn classify_with(label: &str, licensed: &[&str]) -> Capitalisation;
pub fn classify(label: &str) -> Capitalisation;
pub fn check_label(label: &str) -> CaseReport<'_>;
pub fn check_label_with<'a>(label: &'a str, licensed: &[&str]) -> CaseReport<'a>;

// pins
pub enum PinExemption {
    SentenceCase,
    VerbFirst,
    Ellipsis,
}

pub struct PinnedLabel {
    pub literal: &'static str,
    pub pinned_by: &'static str,
    pub exempts: &'static [PinExemption],
}

pub const HOST_PIN_TABLE: &[PinnedLabel];
pub fn pin_for(label: &str) -> Option<&'static PinnedLabel>;
pub fn is_pinned(label: &str) -> bool;
pub fn is_exempt(label: &str, exemption: PinExemption) -> bool;
pub const HOST_PIN_REQUIREMENTS: &str = "UI-009..UI-034, UI-062";

// tabular
pub enum TabularNums {
    Required,
    NotRequired,
}

pub enum ReadoutKind {
    Counter,
    Percentage,
    Timestamp,
    PaginationRange,
    NumericColumn,
}

pub struct NumericReadout<'a> {
    pub id: &'a str,
    pub kind: ReadoutKind,
    pub tabular: TabularNums,
    pub header_right_aligned: bool,
    pub cell_right_aligned: bool,
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

pub enum TruncationMode {
    End,
    Middle,
}

impl TruncationMode {
    pub const fn slug(self) -> &'static str;
}

pub struct Truncated {
    pub text: String,
    pub mode: TruncationMode,
    pub was_truncated: bool,
    pub head_kept: bool,
    pub tail_preserved: bool,
}

pub fn truncate_end(text: &str, budget: usize) -> Truncated;
pub fn truncate_middle(text: &str, budget: usize) -> Truncated;
pub fn truncate(text: &str, budget: usize, mode: TruncationMode) -> Truncated;

pub struct LabelSlot<'a> {
    pub id: &'a str,
    pub text: &'a str,
    pub tooltip: Option<&'a str>,
}

pub enum TruncationTooltipViolation<'a> {
    MissingTooltip { id: &'a str, full: &'a str },
    TooltipNotFullText { id: &'a str, full: &'a str, tooltip: &'a str },
}

pub fn check_truncation_tooltip<'a>(
    slot: &LabelSlot<'a>,
    mode: TruncationMode,
    budget: usize,
) -> Vec<TruncationTooltipViolation<'a>>;
```

#### Examples

```rust
use hoarfrost::microcopy::action_label::{check_action_ellipsis, ActionLabel, PromptBehaviour};

let label = ActionLabel {
    label: "Rename...",
    behaviour: PromptBehaviour::LaunchesPrompt,
};
let verdict = check_action_ellipsis(&label);
assert!(verdict.is_compliant());
```

#### Type references

- `ActionLabel` - Pair combining an action's user-facing label string with its `PromptBehaviour`.
- `EllipsisVerdict` - Audit result reporting compliance or violations with prompt ellipsis rules.
- `Capitalisation` - Structural classification (e.g., `Sentence`, `TitleCase`, `AllCaps`).
- `CaseReport` - Diagnostic report determining sentence-case compliance and checking pin exemptions.
- `PinnedLabel` - Literal string pinned by specification requirements and exempt from specific rules.
- `NumericReadout` - Descriptor specifying numeric readout alignment and tabular figure requirements.
- `Truncated` - Output of text truncation indicating whether head and tail characters were preserved.

#### Return values

- `check_action_ellipsis` returns `EllipsisVerdict` with `is_compliant()`.
- `check_verb_first` returns `VerbFirstVerdict` flagging generic terms.
- `check_label` returns `CaseReport` with `is_compliant()`.
- `truncate_middle` and `truncate_end` return `Truncated`.
- `check_truncation_tooltip` returns `Vec<TruncationTooltipViolation>` (empty when compliant).

#### Side effects

None. All checkers and string manipulation functions are pure.

#### References

[`microcopy/action_label.rs`](file:///D:/hoarfrost/src/hoarfrost/microcopy/action_label.rs)
[`microcopy/case_rule.rs`](file:///D:/hoarfrost/src/hoarfrost/microcopy/case_rule.rs)
[`microcopy/pins.rs`](file:///D:/hoarfrost/src/hoarfrost/microcopy/pins.rs)
[`microcopy/tabular.rs`](file:///D:/hoarfrost/src/hoarfrost/microcopy/tabular.rs)
[`microcopy/truncate.rs`](file:///D:/hoarfrost/src/hoarfrost/microcopy/truncate.rs)

---

### `validation` - reward early, punish late (`UIF-PRIN-009`, `UIF-PRIN-010`)

#### Overview

`validation` coordinates user edit stances, disallowed character filters, and error punishment timing under the "Reward Early, Punish Late" discipline (`UIF-PRIN-009`, `UIF-PRIN-010`). It prevents premature error painting on pristine fields while immediately rejecting invalid keystrokes and supporting snapshot recovery.

#### Prerequisites & Requirements

- Edit controls **MUST** track `EditStance`, beginning in `Pristine` and advancing to `Touched` upon commit, blur, or submit.
- Error styling **MUST NOT** be painted while `Punishment::painted()` is `false`, ensuring pristine invalid fields do not display error borders.
- Submission **MUST** be blocked whenever `Punishment::blocks_submit()` is `true`.
- Disallowed characters (`FILENAME_ILLEGAL_CHARS`) **MUST** be rejected immediately, even on pristine fields.
- Snapshots of complex inputs **MUST** be captured prior to commits that can fail, restoring state via `RecoverySession`.

#### Syntax / Method Signature

```rust
pub enum EditStance {
    Pristine,
    Touched,
}

impl EditStance {
    pub const ALL: [Self; 2];
    pub const fn is_pristine(self) -> bool;
    pub const fn is_touched(self) -> bool;
}

pub const FILENAME_ILLEGAL_CHARS: &str = "/\\:*?\"<>|";

pub enum DisallowedClass {
    FilenameBasename,
}

impl DisallowedClass {
    pub const ALL: [Self; 1];
    pub const fn classify(self, character: char) -> Option<DisallowedKind>;
    pub fn first_offence(self, value: &str) -> Option<DisallowedChar>;
    pub fn offences(self, value: &str) -> impl Iterator<Item = DisallowedChar> + '_;
    pub fn contains_any(self, value: &str) -> bool;
}

pub enum DisallowedKind {
    IllegalPathCharacter,
    ControlCharacter,
}

impl DisallowedKind {
    pub const ALL: [Self; 2];
}

pub struct DisallowedChar {
    pub character: char,
    pub offset: usize,
    pub kind: DisallowedKind,
}

pub enum PunishReason {
    Valid,
    WithheldPristine,
    DisallowedImmediate,
    TouchedInvalid,
}

pub struct Punishment;

impl Punishment {
    pub const fn invalid(&self) -> bool;
    pub const fn painted(&self) -> bool;
    pub const fn reason(&self) -> PunishReason;
    pub const fn blocks_submit(&self) -> bool;
}

pub const fn punish(stance: EditStance, invalid: bool, disallowed: bool) -> Punishment;

pub struct ScrollOffset(f32);

impl ScrollOffset {
    pub fn new(pixels: f32) -> Self;
    pub fn is_zero(self) -> bool;
}

pub struct RestoredFields;

impl RestoredFields {
    pub fn count(self) -> usize;
}

pub struct EditSnapshot;

impl EditSnapshot {
    pub fn new(
        buffer: impl Into<String>,
        selection: Option<TextRange>,
        scroll: Option<ScrollOffset>,
    ) -> Self;
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

#### Examples

```rust
use hoarfrost::validation::{
    disallowed::DisallowedClass,
    punish::{punish, PunishReason},
    EditStance,
};

let input = "notes/chapter1.md";
let has_disallowed = DisallowedClass::FilenameBasename.contains_any(input);
let decision = punish(EditStance::Pristine, true, has_disallowed);

assert!(decision.painted());
assert_eq!(decision.reason(), PunishReason::DisallowedImmediate);
```

#### Type references

- `EditStance` - State indicating whether the user has committed to an input (`Pristine` or `Touched`).
- `DisallowedClass` - Classification of characters that are forbidden regardless of stance.
- `DisallowedChar` - Record of an illegal character, its byte offset, and its violation kind.
- `Punishment` - Evaluation outcome determining error display, submission blocking, and reason.
- `EditSnapshot` - Complete text, selection, and scroll position backup for recovery sessions.
- `RecoverySession` - Gesture coordinator restoring edit snapshots upon cancellation.

#### Return values

- `punish` returns a `Punishment` struct reporting `invalid()`, `painted()`, and `blocks_submit()`.
- `DisallowedClass::first_offence` returns `Option<DisallowedChar>`.
- `RecoverySession::cancel` returns `Option<Cancel<EditSnapshot>>`.
- `settle` returns `true` if an active recovery gesture was closed.

#### Side effects

- `RecoverySession` stores in-flight snapshots and applies them back into models during cancellation.
- Core validation evaluation functions are purely functional.

#### References

[`validation/mod.rs`](file:///D:/hoarfrost/src/hoarfrost/validation/mod.rs)
[`validation/stance.rs`](file:///D:/hoarfrost/src/hoarfrost/validation/stance.rs)
[`validation/disallowed.rs`](file:///D:/hoarfrost/src/hoarfrost/validation/disallowed.rs)
[`validation/punish.rs`](file:///D:/hoarfrost/src/hoarfrost/validation/punish.rs)
[`validation/recovery.rs`](file:///D:/hoarfrost/src/hoarfrost/validation/recovery.rs)

---

### `invariant_gate` - verification gates (`UIF-VER-001..005`)

#### Overview

`invariant_gate` aggregates the framework's machine-checkable styling, density, focal-point, and microcopy rules into a unified verification gate (`UIF-VER-001` through `UIF-VER-005`). It provides diagnostic accumulators so test suites can gather all rule violations rather than panicking on first failure.

#### Prerequisites & Requirements

- Invariant checks **MUST** be run within testing harness suites rather than UI render paths.
- Checkers **MUST** report all findings to an `InvariantReport` accumulator.
- CI quality gates **MUST** assert `InvariantReport::is_clean()`.
- Any discovered invariant violation **MUST** be treated as a build-breaking failure.

#### Syntax / Method Signature

```rust
pub enum InvariantCategory {
    SentenceCase,
    ActionEllipsis,
    EmphasisFocalPoint,
    RowDensity,
    TooltipObligation,
    SecretRedaction,
    ValidationPunishment,
}

impl InvariantCategory {
    pub const ALL: [Self; 7];
    pub const fn slug(self) -> &'static str;
}

pub struct InvariantViolation {
    pub category: InvariantCategory,
    pub target: String,
    pub message: String,
}

impl InvariantViolation {
    pub fn new(
        category: InvariantCategory,
        target: impl Into<String>,
        message: impl Into<String>,
    ) -> Self;
}

pub struct InvariantReport {
    pub total_checked: usize,
    pub violations: Vec<InvariantViolation>,
}

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
pub fn check_emphasis_focal_point<'a>(
    scope: ContainerScope,
    slots: &[EmphasisSlot<'a>],
) -> Result<(), InvariantViolation>;
pub fn check_row_density<'a>(
    declared: Option<ControlDensity>,
    slots: &[RowSlot<'a>],
) -> Result<(), InvariantViolation>;
pub fn check_label_truncation_tooltip<'a>(slot: &LabelSlot<'a>) -> Result<(), InvariantViolation>;
pub fn check_icon_only_button_tooltip(
    is_icon_only: bool,
    tooltip_label: Option<&str>,
) -> Result<(), InvariantViolation>;
pub fn check_secret_redaction<T: fmt::Debug + fmt::Display>(
    secret: &T,
    plaintext: &str,
) -> Result<(), InvariantViolation>;
pub fn check_validation_transition(stance: EditStance, invalid: bool, disallowed: bool) -> Punishment;
pub fn audit_invariants() -> InvariantReport;
```

#### Examples

```rust
use hoarfrost::invariant_gate::{check_sentence_case, InvariantReport};

let mut report = InvariantReport::new();
for label in ["Save", "Open...", "Close"] {
    report.record_result(check_sentence_case(label, true));
}
assert!(report.is_clean(), "Invariant gate failed:\n{}", report.describe());
```

#### Type references

- `InvariantCategory` - Seven closed pillars verified by the invariant gate (`UIF-VER-005`).
- `InvariantViolation` - Violation record specifying category, offending target name, and explanation.
- `InvariantReport` - Diagnostic accumulator tracking total checks performed and list of violations.

#### Return values

- Verification functions return `Ok(())` on compliance or `Err(InvariantViolation)` on failure.
- `check_validation_transition` returns the resolved `Punishment`.
- `InvariantReport::is_clean` returns `true` only when zero violations have been recorded.
- `describe` formats all violations into a multi-line report.

#### Side effects

- `InvariantReport::record_*` mutates internal report statistics and violation buffers.
- Functions execute without clock reads, file writes, or thread synchronization.

#### References

[`invariant_gate.rs`](file:///D:/hoarfrost/src/hoarfrost/invariant_gate.rs)

---

### `token_lint` - tokenization and contrast audit (`UIF-PRIN-001`)

#### Overview

`token_lint` statically inspects the codebase to enforce tokenization purity and contrast invariants (`UIF-PRIN-001`, `UIF-VER-001`). It verifies that color constructors appear exclusively in `token/color.rs` and that visually-named constants bind to design tokens.

#### Prerequisites & Requirements

- Direct color constructors (`Color::from_rgb`, etc.) **MUST NOT** be invoked outside `token/color.rs`.
- Visually-named scalar constants in `widget/` or `surface/` **MUST** bind to tokens, registered surface geometry constants, ratios, or zero.
- Allow-list entries **MUST** include documented rationales; stale allow entries **MUST NOT** remain in the registry.
- CI test suites **MUST** assert `Audit::is_clean()`.

#### Syntax / Method Signature

```rust
pub enum Rule {
    ColourLiteral,
    UnboundVisualScalar,
}

pub enum ScalarClass {
    TokenBound,
    Zero,
    Ratio,
    SurfaceBound,
    Violation,
    Unreadable,
}

pub struct Violation {
    pub rule: Rule,
    pub file: String,
    pub line: usize,
    pub code: String,
    pub detail: String,
}

impl Violation {
    pub fn key(&self) -> String;
    pub fn constant_name(&self) -> Option<&str>;
}

pub struct SourceScan {
    pub findings: Vec<Violation>,
    pub token_layer_colour_definitions: usize,
    pub token_bound: usize,
    pub zero: usize,
    pub ratio: usize,
    pub surface_bound: usize,
    pub unreadable: usize,
}

pub struct Scope<'a> {
    pub colour_definition_file: &'static str,
    pub visual_scope_prefixes: &'a [&'static str],
    pub surface_geometry: &'a [SurfaceGeometryConstant],
}

pub fn crate_scope<'a>() -> Scope<'a>;

pub const COLOUR_DEFINITION_FILE: &str = "token/color.rs";
pub const VISUAL_SCOPE_DIRS: [&str; 3] = ["widget/", "surface/", "composite/"];
pub const VISUAL_NAME_MARKERS: [&str; 10] = [
    "HEIGHT", "WIDTH", "PADDING", "RADIUS", "STROKE", "GAP", "SIZE", "INSET", "MARGIN", "SLOT",
];
pub const COLOUR_CONSTRUCTORS: [&str; 4] = [
    "Color::from_rgb", "Color::from_rgba", "Color::from_rgb8", "Color::from_rgba8",
];

pub struct SurfaceGeometryConstant {
    pub file: &'static str,
    pub name: &'static str,
    pub spec: &'static str,
}

pub const SURFACE_GEOMETRY_CONSTANTS: &[SurfaceGeometryConstant];
pub fn surface_geometry(file: &str, name: &str) -> Option<&'static SurfaceGeometryConstant>;
pub fn lookup_geometry<'s>(
    registry: &'s [SurfaceGeometryConstant],
    file: &str,
    name: &str,
) -> Option<&'s SurfaceGeometryConstant>;

pub struct AllowEntry {
    pub key: &'static str,
    pub reason: &'static str,
}

pub const ALLOW_LIST: &[AllowEntry];

pub struct Audit {
    pub violations: Vec<Violation>,
    pub suppressed: Vec<(Violation, &'static str)>,
    pub stale_allow_entries: Vec<&'static str>,
    pub stats: ScanStats,
}

impl Audit {
    pub fn is_clean(&self) -> bool;
    pub fn describe(&self) -> String;
}

pub struct ScanStats {
    pub files_scanned: usize,
    pub rule_a_files: usize,
    pub rule_b_files: usize,
    pub token_layer_colour_definitions: usize,
    pub token_bound: usize,
    pub zero: usize,
    pub ratio: usize,
    pub surface_bound: usize,
    pub unreadable: usize,
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

#### Examples

```rust
use hoarfrost::token_lint::audit_crate;

let audit = audit_crate();
assert!(audit.is_clean(), "Token lint audit failed:\n{}", audit.describe());
```

#### Type references

- `Rule` - Lint rule violated: `ColourLiteral` (Rule A) or `UnboundVisualScalar` (Rule B).
- `ScalarClass` - Classification of visual constant expression (e.g., `TokenBound`, `Zero`, `Ratio`).
- `Violation` - Offence record indicating rule, file, line number, source snippet, and details.
- `SurfaceGeometryConstant` - Registered layout boundary constant with specification citation.
- `AllowEntry` - Explicitly approved lint exemption with documented reason.
- `Audit` - Full audit result containing violations, suppressions, stale allowances, and statistics.

#### Return values

- `audit_crate` returns an `Audit` across all crate sources.
- `Audit::is_clean` returns `true` only if no unsuppressed violations and no stale allow entries exist.
- `scan_source` parses source text and returns a `SourceScan`.

#### Side effects

- `scan_tree` and `audit_crate` read source code files from disk. No filesystem mutations occur.

#### References

[`token_lint.rs`](file:///D:/hoarfrost/src/hoarfrost/token_lint.rs)

---

### `test_helpers` - headless testing harness

#### Overview

`test_helpers` provides a headless testing harness and frame capture engine for driving iced widget trees without a window (`INC-10`, `INC-11`). It simulates pointer and keyboard interactions, tracks widget focus across rebuilds, and verifies frame pixel changes.

#### Prerequisites & Requirements

- Tests **MUST** enable the `test-helpers` feature or execute under cargo test configuration.
- Tests involving persistence **MUST** use `NoopWindowStorage`, `MemoryWindowStorage`, `MemoryGeometryStorage`, or isolated `tempfile::TempDir` paths, and **MUST NOT** touch live user paths (`%APPDATA%`, `USERPROFILE`).
- Visual assertion suites **MUST** instantiate a single `Capture` session per test to avoid re-allocating headless graphics pipelines.
- Widget view callbacks **MUST** be closures rebuilding the UI tree per frame.

#### Syntax / Method Signature

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

pub struct RecordFocusables;
pub struct RecordCandidates;

impl RecordCandidates {
    pub fn candidates(&self) -> Vec<Candidate>;
}

pub struct Candidate;
pub const ALL_KINDS: [&str; 6] = [
    KIND_CONTAINER, KIND_SCROLLABLE, KIND_FOCUSABLE, KIND_TEXT_INPUT, KIND_TEXT, KIND_CUSTOM,
];
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
    harness: &mut Harness<'_, Message, Renderer>,
    model: &FocusState,
    step: &str,
);
pub fn focus_one<Message: Send + 'static, Renderer: renderer::Renderer>(
    harness: &mut Harness<'_, Message, Renderer>,
    label: &str,
);
pub fn moved(to: Point) -> Event;
pub fn pressed() -> Event;
pub fn released() -> Event;
pub fn centre(bounds: Rectangle) -> Point;

pub const PIXEL_SCALE: f32 = 2.0;
pub const INK_THRESHOLD: u32 = 90;
pub const SURFACE_TOLERANCE: u8 = 4;
pub const MIN_CHANGED_PIXELS: usize = 8;
pub fn headless_renderer() -> HeadlessRenderer;
pub fn screenshot<Message: 'static>(
    size: Size,
    scale: f32,
    background: Color,
    root: Element<'_, Message>,
) -> Vec<u8>;
pub fn ink_band(rgba: &[u8], physical_width: u32, scale: f32) -> Option<(f32, f32)>;
pub fn ink_band_between(
    rgba: &[u8],
    width: u32,
    scale: f32,
    y_min: f32,
    y_max: f32,
) -> Option<(f32, f32)>;
pub fn physical_width(size: Size, scale: f32) -> u32;
pub fn to_rgb(colour: Color) -> [u8; 3];

pub struct Capture;

impl Capture {
    pub fn new() -> Self;
    pub fn frame<Message: 'static>(&mut self, viewport: Size, root: Element<'_, Message>) -> Frame;
    pub fn frame_at<Message: 'static>(
        &mut self,
        viewport: Size,
        scale: f32,
        background: Color,
        root: Element<'_, Message>,
    ) -> Frame;
    pub fn frame_after<Message: 'static>(
        &mut self,
        viewport: Size,
        root: Element<'_, Message>,
        events: &[Event],
    ) -> Frame;
    pub fn frame_after_at<Message: 'static>(
        &mut self,
        viewport: Size,
        root: Element<'_, Message>,
        events: &[Event],
        cursor: mouse::Cursor,
    ) -> Frame;
    pub fn click<Message: 'static>(&mut self, root: Element<'_, Message>, at: Point) -> Vec<Message>;
    pub fn click_and_frame<Message: 'static>(
        &mut self,
        viewport: Size,
        root: Element<'_, Message>,
        at: Point,
    ) -> (Frame, Vec<Message>);
    pub fn frame_and_messages_after<Message: 'static>(
        &mut self,
        viewport: Size,
        root: Element<'_, Message>,
        events: &[Event],
        cursor: mouse::Cursor,
    ) -> (Frame, Vec<Message>);
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

#### Examples

```rust
use hoarfrost::bridge::Direction;
use hoarfrost::test_helpers::{focusable_tree, model_of, Harness};

let (_group, mut model) = model_of(&["a", "b", "c"]);
let mut harness = Harness::headless(|| focusable_tree(&["a", "b", "c"]));
assert_eq!(
    harness
        .tab(&mut model, Direction::Forward)
        .map(|id| id.as_str().to_owned()),
    Some("a".to_owned()),
);
```

#### Type references

- `Harness` - Stateful headless testing interface executing widget tree builds and operations.
- `RecordFocusables` - Operation recording all focusable bounds and widget identities visited.
- `RecordCandidates` - Operation recording published bounds and target candidate classifications.
- `Capture` - Multi-frame headless renderer session for pixel comparisons.
- `Frame` - Captured RGBA frame buffer supporting ink bounds, mask, and pixel delta calculations.

#### Return values

- `tab` advances focus through `bridge::tab_operation` and returns `Option<FocusId>`.
- `focused` returns the `Option<Id>` currently holding focus in the widget tree.
- `Capture::frame` returns a rasterized `Frame`.
- `Frame::differing_pixels` returns the number of pixels that differ between two frames.

#### Side effects

- `Harness::headless` and `Capture::new` instantiate headless GPU or software rendering pipelines.
- Event dispatch rebuilds the widget tree and updates internal caches.
- No live user filesystem paths or registry entries are modified.

#### References

[`test_helpers.rs`](file:///D:/hoarfrost/src/hoarfrost/test_helpers.rs)

---
## Configuration Reference

Hoarfrost has no runtime configuration object, no global registry, and no environment variable overrides. Applications configure the framework at three distinct boundaries: the Cargo features selected at build time, the explicit model and style arguments passed into widget constructors during view evaluation, and the host-implemented persistence and platform seams. Every visual scalar, spacing interval, duration token, and layout bound is a compiled constant.

---

### Cargo features

The root crate feature configuration defaults to empty (`default = []`). The core subsystem is unconditionally compiled into every downstream binary:
- Design tokens: [`token`](file:///D:/hoarfrost/src/hoarfrost/token/mod.rs)
- Interaction state machine: [`state`](file:///D:/hoarfrost/src/hoarfrost/state.rs)
- Focus policy and hierarchy: [`focus`](file:///D:/hoarfrost/src/hoarfrost/focus.rs)
- Iced runtime focus bridge: [`bridge`](file:///D:/hoarfrost/src/hoarfrost/bridge.rs)
- Ownership, activation, and gesture contracts: [`binding`](file:///D:/hoarfrost/src/hoarfrost/binding/mod.rs)
- Stable Part IDs: [`identity`](file:///D:/hoarfrost/src/hoarfrost/identity.rs)
- Verbatim string catalogue: [`strings`](file:///D:/hoarfrost/src/hoarfrost/strings.rs)
- Pure coordinate math and layout solvers: [`geometry`](file:///D:/hoarfrost/src/hoarfrost/geometry/mod.rs), [`layout`](file:///D:/hoarfrost/src/hoarfrost/layout.rs)
- Accessibility semantics: [`a11y`](file:///D:/hoarfrost/src/hoarfrost/a11y.rs)
- Stance-driven validation and recovery: [`validation`](file:///D:/hoarfrost/src/hoarfrost/validation/mod.rs)
- Linguistic and microcopy rules: [`microcopy`](file:///D:/hoarfrost/src/hoarfrost/microcopy/mod.rs)
- Structural invariant gates: [`invariant_gate`](file:///D:/hoarfrost/src/hoarfrost/invariant_gate.rs)
- Compile-time token linting: [`token_lint`](file:///D:/hoarfrost/src/hoarfrost/token_lint.rs)

Every interactive widget, composite container, and floating surface is opt-in, gated behind its own feature flag. Consumers **SHOULD** enable only the specific features required by their UI components. Consumers **MUST NOT** enable the meta-feature `all` merely to resolve import errors in production code; `all` is intended exclusively for test harnesses, documentation sweeps, and showcase galleries.

#### Atomic Widgets (`widget::*`)

Atomic controls encapsulate a single cohesive interaction state machine across rest, hover, press, focus, disabled, and invalid states.

| Feature | Implies | Public Modules & Re-exports | Normative Requirement |
|---|---|---|---|
| `avatar` | `badge` | [`widget::avatar`](file:///D:/hoarfrost/src/hoarfrost/widget/avatar.rs) (`Avatar`, `AvatarModel`, `AvatarSize`, `avatar`) | `UIF-ATOM-013`, spec §3.13 |
| `badge` | - | [`widget::badge`](file:///D:/hoarfrost/src/hoarfrost/widget/badge.rs), [`widget::badge_model`](file:///D:/hoarfrost/src/hoarfrost/widget/badge_model.rs) (`BadgeModel`, `BadgeStyle`, `resolve_badge_style`) | `UIF-ATOM-008`, spec §3.8 |
| `button` | `tooltip` | [`widget::button`](file:///D:/hoarfrost/src/hoarfrost/widget/button.rs), [`widget::button_widget`](file:///D:/hoarfrost/src/hoarfrost/widget/button_widget.rs), [`widget::button_model`](file:///D:/hoarfrost/src/hoarfrost/widget/button_model.rs), [`widget::button_controller`](file:///D:/hoarfrost/src/hoarfrost/widget/button_controller.rs), [`widget::progress`](file:///D:/hoarfrost/src/hoarfrost/widget/progress.rs), [`widget::progress_model`](file:///D:/hoarfrost/src/hoarfrost/widget/progress_model.rs) (`Button`, `ButtonModel`, `ButtonController`) | `UIF-ATOM-001`, `UIF-ATOM-001a`, `UIF-ATOM-009`, spec §3.1, §3.9 |
| `chip` | - | [`widget::chip`](file:///D:/hoarfrost/src/hoarfrost/widget/chip.rs), [`widget::chip_model`](file:///D:/hoarfrost/src/hoarfrost/widget/chip_model.rs) (`ChipModel`, `ChipStyle`, `resolve_chip_style`) | `UIF-ATOM-006`, spec §3.6 |
| `color-picker` | - | [`widget::color_picker`](file:///D:/hoarfrost/src/hoarfrost/widget/color_picker.rs) (`ColorPicker`, `ColorSwatchButton`, `Hsva`, `Rgba8`, `color_picker`) | `UIF-ATOM-012`, spec §3.12 |
| `combo-box` | `menu`, `text-input` | [`widget::combo_box`](file:///D:/hoarfrost/src/hoarfrost/widget/combo_box.rs), [`widget::combo_model`](file:///D:/hoarfrost/src/hoarfrost/widget/combo_model.rs), [`widget::combo_host`](file:///D:/hoarfrost/src/hoarfrost/widget/combo_host.rs) (`Select`, `ComboBox`, `ComboBoxModel`, `select`, `combo_box`) | `UIF-ATOM-004`, spec §3.4 |
| `date-picker` | - | [`widget::date_picker`](file:///D:/hoarfrost/src/hoarfrost/widget/date_picker.rs) (`DatePicker`, `DateRangePicker`, `CalendarView`, `CivilDate`, `date_picker`) | `UIF-ATOM-010`, spec §3.10 |
| `hyperlink` | `button` | [`widget::hyperlink`](file:///D:/hoarfrost/src/hoarfrost/widget/hyperlink.rs), [`widget::hyperlink_model`](file:///D:/hoarfrost/src/hoarfrost/widget/hyperlink_model.rs) (`HyperlinkModel`, `resolve_hyperlink_style`) | `UIF-ATOM-001d`, spec §3.1d |
| `number-box` | `button`, `range-model`, `text-input` | [`widget::number_box`](file:///D:/hoarfrost/src/hoarfrost/widget/number_box.rs), [`widget::number_box_host`](file:///D:/hoarfrost/src/hoarfrost/widget/number_box_host.rs) (`NumberBoxStyle`, `NumberBoxHost`) | `UIF-ATOM-005`, spec §3.5 |
| `password` | `text-input` | [`widget::password_model`](file:///D:/hoarfrost/src/hoarfrost/widget/password_model.rs), [`widget::password_host`](file:///D:/hoarfrost/src/hoarfrost/widget/password_host.rs) (`SecretInput`, `PasswordHost`, `password_host::mirror`) | `UIF-ATOM-002c`, spec §3.2c |
| `progress` | `button` | [`widget::progress`](file:///D:/hoarfrost/src/hoarfrost/widget/progress.rs), [`widget::progress_model`](file:///D:/hoarfrost/src/hoarfrost/widget/progress_model.rs) (`ProgressModel`, `ProgressStyle`, `resolve_progress_style`) | `UIF-ATOM-009`, spec §3.9 |
| `range-slider` | `range-model` | [`widget::range_slider`](file:///D:/hoarfrost/src/hoarfrost/widget/range_slider.rs), [`widget::range_slider_model`](file:///D:/hoarfrost/src/hoarfrost/widget/range_slider_model.rs) (`RangeSlider`, `RangeSliderModel`, `RangeThumb`, `range_slider`) | `UIF-ATOM-005b`, `UIF-ATOM-005`, spec §3.5b |
| `rating` | - | [`widget::rating`](file:///D:/hoarfrost/src/hoarfrost/widget/rating.rs) (`RatingControl`, `RatingModel`, `rating_control`) | `UIF-ATOM-013`, spec §3.13 |
| `search-box` | - | [`widget::search_box`](file:///D:/hoarfrost/src/hoarfrost/widget/search_box.rs) (`SearchBox`, `SearchBoxModel`) | `UIF-PANE-003`, `UIF-PRIN-006`, spec §4.3.2 |
| `segmented` | `button` | [`widget::segmented`](file:///D:/hoarfrost/src/hoarfrost/widget/segmented.rs), [`widget::segmented_model`](file:///D:/hoarfrost/src/hoarfrost/widget/segmented_model.rs) (`SegmentedModel`, `resolve_segmented_style`) | `UIF-ATOM-001b`, spec §3.1b |
| `slider` | `button`, `range-model` | [`widget::slider`](file:///D:/hoarfrost/src/hoarfrost/widget/slider.rs), [`widget::slider_host`](file:///D:/hoarfrost/src/hoarfrost/widget/slider_host.rs) (`SliderStyle`, `SliderHost`) | `UIF-ATOM-005`, spec §3.5 |
| `split-button` | `button`, `menu` | [`widget::split_button`](file:///D:/hoarfrost/src/hoarfrost/widget/split_button.rs), [`widget::split_button_model`](file:///D:/hoarfrost/src/hoarfrost/widget/split_button_model.rs), [`widget::split_button_host`](file:///D:/hoarfrost/src/hoarfrost/widget/split_button_host.rs) (`SplitButton`, `SplitButtonModel`) | `UIF-ATOM-001c`, spec §3.1c |
| `text-area` | `text-input` | [`widget::text_area_model`](file:///D:/hoarfrost/src/hoarfrost/widget/text_area_model.rs), [`widget::text_area_host`](file:///D:/hoarfrost/src/hoarfrost/widget/text_area_host.rs) (`TextAreaModel`, `TextAreaHost`) | `UIF-ATOM-002e`, spec §3.2e |
| `text-input` | `button` | [`widget::text_input`](file:///D:/hoarfrost/src/hoarfrost/widget/text_input.rs), [`widget::text_input_model`](file:///D:/hoarfrost/src/hoarfrost/widget/text_input_model.rs), [`widget::text_input_controller`](file:///D:/hoarfrost/src/hoarfrost/widget/text_input_controller.rs) (`TextInput`, `TextInputModel`, `TextInputController`, `text_input`) | `UIF-ATOM-002`, `UIF-ATOM-002a`, spec §3.2 |
| `time-picker` | - | [`widget::time_picker`](file:///D:/hoarfrost/src/hoarfrost/widget/time_picker.rs) (`TimePicker`, `TimePickerModel`, `CivilTime`, `time_picker`) | `UIF-ATOM-011`, spec §3.11 |
| `toggle` | `button` | [`widget::toggle`](file:///D:/hoarfrost/src/hoarfrost/widget/toggle.rs), [`widget::toggle_model`](file:///D:/hoarfrost/src/hoarfrost/widget/toggle_model.rs), [`widget::toggle_widget`](file:///D:/hoarfrost/src/hoarfrost/widget/toggle_widget.rs) (`Checkbox`, `RadioGroup`, `ToggleSwitch`) | `UIF-ATOM-003`, spec §3.3 |
| `auto-suggest` | `menu`, `text-input`, `tooltip` | [`widget::auto_suggest_model`](file:///D:/hoarfrost/src/hoarfrost/widget/auto_suggest_model.rs), [`widget::auto_suggest_host`](file:///D:/hoarfrost/src/hoarfrost/widget/auto_suggest_host.rs) (`AutoSuggestModel`, `AutoSuggestHost`) | `UIF-ATOM-002d`, spec §3.2d |

*(Note: Always-compiled core atom modules without feature gates include [`widget::focus`](file:///D:/hoarfrost/src/hoarfrost/widget/focus.rs) and [`widget::focusable`](file:///D:/hoarfrost/src/hoarfrost/widget/focusable.rs) under `UIF-ATOM-000` / `UIF-TOK-006`, and [`widget::rove`](file:///D:/hoarfrost/src/hoarfrost/widget/rove.rs) under `UIF-ATOM-001b` / `UIF-ATOM-003`.)*

#### Composite Containers (`composite::*`)

Composites manage relational layout, hierarchical elision, spatial grouping, and multi-control workflows.

| Feature | Implies | Public Modules & Re-exports | Normative Requirement |
|---|---|---|---|
| `breadcrumb` | `button`, `menu` | [`composite::breadcrumb`](file:///D:/hoarfrost/src/hoarfrost/composite/breadcrumb.rs) (`solve_breadcrumb_elision`, `BreadcrumbStyle`) | `UIF-COMP-008` |
| `card` | `button` | [`composite::card`](file:///D:/hoarfrost/src/hoarfrost/composite/card.rs) (`CardStyle`, `resolve_style`, `content_bounds`, `card_layer`) | `UIF-COMP-005` |
| `code-block` | - | [`composite::code_block`](file:///D:/hoarfrost/src/hoarfrost/composite/code_block.rs) (`CodeBlock`, `CodeBlockModel`, `code_block`) | `UIF-COMP-015`, `UIF-PRIN-003` |
| `command-palette` | `text-input` | [`composite::command_palette`](file:///D:/hoarfrost/src/hoarfrost/composite/command_palette.rs) (`CommandPalette`, `CommandPaletteModel`, `command_palette`) | `UIF-COMP-010`, `UIF-PRIN-003`, `UIF-TOK-006` |
| `diff-viewer` | - | [`composite::diff_viewer`](file:///D:/hoarfrost/src/hoarfrost/composite/diff_viewer.rs) (`DiffViewer`, `DiffViewerModel`, `diff_viewer`) | `UIF-COMP-015`, `UIF-PRIN-005`, `UIF-PRIN-006` |
| `empty-state` | `toast` | [`composite::empty_state`](file:///D:/hoarfrost/src/hoarfrost/composite/empty_state.rs) (`EmptyState`, `EmptyStateModel`, `empty_state`) | `UIF-COMP-014`, `UIF-PRIN-004` |
| `expander` | `button` | [`composite::expander`](file:///D:/hoarfrost/src/hoarfrost/composite/expander.rs) (`ExpanderModel`, `resolve_expander_style`) | `UIF-COMP-005` |
| `form-field` | `button` | [`composite::form_field`](file:///D:/hoarfrost/src/hoarfrost/composite/form_field.rs) (`solve_form_field_layout`, `FormFieldStyle`) | `UIF-COMP-001`, `UIF-PRIN-003` |
| `grid-view` | `tree-view` | [`composite::grid_view`](file:///D:/hoarfrost/src/hoarfrost/composite/grid_view.rs) (`GridView`, `GridViewModel`, `grid_view`) | `UIF-COMP-013`, `UIF-VER-002`, `UIF-PRIN-006` |
| `info-bar` | `button`, `notification` | [`composite::info_bar`](file:///D:/hoarfrost/src/hoarfrost/composite/info_bar.rs) (`InfoBarStyle`, `resolve_info_bar_style`) | `UIF-COMP-002` |
| `keybind-recorder` | - | [`composite::keybind_recorder`](file:///D:/hoarfrost/src/hoarfrost/composite/keybind_recorder.rs) (`KeybindRecorder`, `KeyChord`, `keybind_recorder`) | `UIF-COMP-015` |
| `list-tile` | `button` | [`composite::list_tile`](file:///D:/hoarfrost/src/hoarfrost/composite/list_tile.rs) (`solve_list_tile_layout`, `ListTileStyle`) | `UIF-COMP-005` |
| `navbar` | `tooltip` | [`composite::navbar`](file:///D:/hoarfrost/src/hoarfrost/composite/navbar.rs) (`NavBar`, `NavBarModel`, `NavBarMenuTrigger`) | `UIF-NAV-001`, `UIF-NAV-002`, `UIF-NAV-003` |
| `navigation-rail` | `badge`, `range-model`, `tooltip` | [`composite::navigation_rail`](file:///D:/hoarfrost/src/hoarfrost/composite/navigation_rail.rs) (`NavigationRail`, `NavigationRailModel`, `navigation_rail`) | `UIF-COMP-009`, `UIF-PRIN-005`, `UIF-PRIN-006` |
| `notification` | - | [`composite::notification`](file:///D:/hoarfrost/src/hoarfrost/composite/notification.rs) (`NotificationModel`, `NotificationStyle`) | `UIF-COMP-002` |
| `overflow-toolbar` | `button`, `menu` | [`composite::overflow_toolbar`](file:///D:/hoarfrost/src/hoarfrost/composite/overflow_toolbar.rs) (`solve_overflow_layout`, `OverflowToolbarStyle`) | `UIF-COMP-008`, `UIF-PRIN-003` |
| `pagination` | - | [`composite::pagination`](file:///D:/hoarfrost/src/hoarfrost/composite/pagination.rs) (`PaginationBar`, `PaginationModel`, `pagination_bar`) | `UIF-COMP-014` |
| `pips-pager` | - | [`composite::pips_pager`](file:///D:/hoarfrost/src/hoarfrost/composite/pips_pager.rs) (`PipsPager`, `PipsPagerModel`, `pips_pager`) | `UIF-COMP-011`, `UIF-PRIN-004` |
| `scroll-view` | - | [`composite::scroll_view`](file:///D:/hoarfrost/src/hoarfrost/composite/scroll_view.rs) (`ScrollViewport`, `ScrollIndicator`) | `UIF-COMP-006` |
| `settings-row` | `button` | [`composite::settings_row`](file:///D:/hoarfrost/src/hoarfrost/composite/settings_row.rs) (`solve_settings_row_layout`, `settings_row_layer`) | `UIF-COMP-001` |
| `side-sheet` | `zorder` | [`composite::side_sheet`](file:///D:/hoarfrost/src/hoarfrost/composite/side_sheet.rs) (`SideSheet`, `SideSheetModel`, `side_sheet`) | `UIF-COMP-012`, `UIF-PRIN-006`, `UIF-MOD-002` |
| `skeleton` | - | [`composite::skeleton`](file:///D:/hoarfrost/src/hoarfrost/composite/skeleton.rs) (`Skeleton`, `SkeletonStyle`, `skeleton`) | `UIF-COMP-014`, `UIF-PRIN-003` |
| `split-divider` | `button` | [`composite::split_divider`](file:///D:/hoarfrost/src/hoarfrost/composite/split_divider.rs) (`SplitDivider`, `SplitDividerModel`, `split_divider`) | `UIF-COMP-007` |
| `stepper` | - | [`composite::stepper`](file:///D:/hoarfrost/src/hoarfrost/composite/stepper.rs) (`Stepper`, `StepperModel`, `stepper`) | `UIF-COMP-011`, `UIF-PRIN-004`, `UIF-PRIN-005` |
| `tab-strip` | `menu`, `tooltip` | [`composite::tab_strip`](file:///D:/hoarfrost/src/hoarfrost/composite/tab_strip.rs) (`TabStrip`, `TabStripModel`, `clamp_tab_width`) | `UIF-TAB-001`, `UIF-TAB-002`, `UIF-TAB-003` |
| `table` | `tree-view`, `virtual-list` | [`composite::table_cell`](file:///D:/hoarfrost/src/hoarfrost/composite/table_cell.rs), [`composite::table_column`](file:///D:/hoarfrost/src/hoarfrost/composite/table_column.rs), [`composite::table_header`](file:///D:/hoarfrost/src/hoarfrost/composite/table_header.rs), [`composite::table_view`](file:///D:/hoarfrost/src/hoarfrost/composite/table_view.rs) | `UIF-COMP-007` |
| `toast` | `info-bar`, `notification`, `tooltip`, `zorder` | [`composite::toast`](file:///D:/hoarfrost/src/hoarfrost/composite/toast.rs) (`ToastStack`, `ToastPlacement`) | `UIF-COMP-002` |
| `tree-row` | `button`, `list-tile`, `virtual-list` | [`composite::tree_row`](file:///D:/hoarfrost/src/hoarfrost/composite/tree_row.rs) (`solve_tree_row_layout`, `indent_allowance`) | `UIF-COMP-006` |
| `tree-view` | `virtual-list` | [`composite::tree_view`](file:///D:/hoarfrost/src/hoarfrost/composite/tree_view.rs) (`TreeView`, `render_tree_with_footer`) | `UIF-COMP-006` |
| `virtual-list` | `scroll-view` | [`composite::virtual_list`](file:///D:/hoarfrost/src/hoarfrost/composite/virtual_list.rs) (`visible_range`, `scroll_offset`) | `UIF-COMP-006` |

#### Floating Surfaces (`surface::*`)

Surfaces handle overlays, focus-trapped dialogs, popovers, context menus, and tooltips across spatial Z-bands.

| Feature | Implies | Public Modules & Re-exports | Normative Requirement |
|---|---|---|---|
| `tooltip` | - | [`surface::tooltip`](file:///D:/hoarfrost/src/hoarfrost/surface/tooltip/mod.rs) (`Tooltip`, `tooltip`, `TimeMs`) | `UIF-SURF-001` |
| `menu` | `button`, `tooltip` | [`surface::menu`](file:///D:/hoarfrost/src/hoarfrost/surface/menu/mod.rs) (`Menu`, `MenuCascade`, `solve_menu_placement`) | `UIF-SURF-002` |
| `zorder` | `menu` | [`surface::zorder`](file:///D:/hoarfrost/src/hoarfrost/surface/zorder.rs) (`SurfaceRegistry`, `ZBand`, `escape_with_pre_rank`) | `UIF-ZORD-001` |
| `modal` | `zorder` | [`surface::modal`](file:///D:/hoarfrost/src/hoarfrost/surface/modal.rs), [`surface::modal_tier`](file:///D:/hoarfrost/src/hoarfrost/surface/modal_tier.rs) (`ModalHost`, `ModalLayout`) | `UIF-MOD-001`, `UIF-MOD-002` |
| `dialog` | `button`, `zorder` | [`surface::input_dialog`](file:///D:/hoarfrost/src/hoarfrost/surface/input_dialog.rs), [`surface::dialog_button`](file:///D:/hoarfrost/src/hoarfrost/surface/dialog_button.rs) (`InputLayout`, `InputControl`, `Verdict`) | `UIF-DIA-001` |
| `movable` | `zorder` | [`surface::movable`](file:///D:/hoarfrost/src/hoarfrost/surface/movable.rs), [`surface::drag_tier`](file:///D:/hoarfrost/src/hoarfrost/surface/drag_tier.rs) (`MovableHost`, `MovableGeometry`) | `UIF-MOV-001` |
| `popover` | `zorder` | [`surface::popover`](file:///D:/hoarfrost/src/hoarfrost/surface/popover/mod.rs) (`PopoverHost`, `PopoverModel`, `solve_popover_placement`) | `UIF-POP-001` |

#### Meta and Utility Features

| Feature | Implies | Purpose |
|---|---|---|
| `range-model` | - | Compiles the shared continuous interval math, non-crossing clamp, and drift-free stepping engine backing `slider`, `range-slider`, `number-box`, and `navigation-rail`. Consumers **SHOULD** depend on the specific widget rather than selecting `range-model` directly. |
| `all` | Every widget, composite, and surface feature | The complete framework suite. Used for integration test suites and the demo application. |
| `gallery` | `all` | Compiles the runnable showcase binary ([`bin/gallery.rs`](file:///D:/hoarfrost/src/hoarfrost/bin/gallery.rs)) and the comprehensive visual regression sweep (`UIF-VER-004`). |
| `test-helpers` | - | Exports the headless widget capture harness ([`test_helpers`](file:///D:/hoarfrost/src/hoarfrost/test_helpers.rs)) in the public API, enabling downstream crates to drive real Iced paint paths in their test suites without windowing dependencies. |

---

### Dependency requirements

Hoarfrost integrates with the standard Rust GUI ecosystem through strict, non-optional dependencies:

```toml
[dependencies]
iced = { version = "0.14.0", features = [
    "wgpu",
    "tiny-skia",
    "fira-sans",
    "advanced-shaping",
    "advanced",
] }
iced_futures = "0.14.0"
iced_renderer = "0.14.0"
iced_runtime = "0.14.0"

[dev-dependencies]
iced_test = "0.14.0"
proptest = "1.4"
```

#### Why `iced/advanced` is Mandatory

In `iced 0.14.0`, the `advanced` feature is **mandatory**, not optional:
1. Without `advanced`, `iced::advanced` is compiled out entirely (`iced-0.14.0/src/lib.rs`).
2. Without `advanced`, `iced::widget::{core, renderer, graphics}` are replaced by empty shadow modules, making `Widget`, `Tree`, `layout::Layout`, and `operation::Focusable` unreachable.
3. Custom widgets implementing interactive gestures or keyboard focus routing cannot compile without unlocking this re-export module.

#### Explicit Sub-Crate Dependencies

`iced_runtime`, `iced_renderer`, and `iced_futures` are declared as explicit, non-optional dependencies because the headless test harness requires low-level access that `iced`'s root API deliberately does not re-export:
- `iced_runtime::UserInterface`: Drives the headless event loop and layout/draw tree updates (omitted from `iced::advanced`).
- `iced_renderer::Renderer`: The concrete headless renderer implementation (`iced::advanced::renderer` exposes only the abstract trait).
- `iced_futures::futures::executor::block_on`: Headless renderer initialization is asynchronous, and `iced` provides no synchronous executor.

Because these three crates are already transitively present in the dependency graph of `iced`, declaring them adds zero overhead to `cargo tree` while ensuring that unit tests compile seamlessly under both `cargo test` and `test-helpers`.

---

### Configuration knobs summary

The table below replaces hundreds of lines of scattered constants, persistence seams, and configuration structs with an authoritative reference mapping:

| Area | Knobs & Interfaces | Values & Types | Reference Link |
|---|---|---|---|
| **Theme & Palette** | [`Palette::dark()`](file:///D:/hoarfrost/src/hoarfrost/token/color.rs)<br>*(Note: `Palette::dark()` is the sole framework palette; no light variant exists)*<br>[`EditorPalette::inverted()`](file:///D:/hoarfrost/src/hoarfrost/token/color.rs) | 23 semantic `Color` tokens (`surface_base`, `surface_overlay`, `accent_fill`, `status_danger_text`, etc.)<br>Inverted white editor surface (`background`, `text`, `error`) | [Design Tokens (`token`)](file:///D:/hoarfrost/src/hoarfrost/token/mod.rs) |
| **Timing & Motion** | `DurationToken` (`ms`, `easing`, `name`)<br>`resolve_ms(token, reduced_motion) -> u64`<br>`TimeMs(u64)` timestamp value | `time::INSTANT` (0ms), `time::FAST` (100ms), `time::NORMAL` (180ms), `time::SLOW` (240ms)<br>`time::DEBOUNCE_LIVE` (200ms), `time::DELAY_TOOLTIP_OPEN` (400ms), `time::TOAST_AUTO_DISMISS` (5000ms)<br>Under `reduced_motion: true`, transition rungs resolve to `0ms`; timers retain full duration | [Design Tokens (`token`)](file:///D:/hoarfrost/src/hoarfrost/token/mod.rs) |
| **Persistence Seams** | [`WindowStorage`](file:///D:/hoarfrost/src/hoarfrost/geometry/window_geometry.rs) trait (`load_geometry`, `save_geometry`)<br>[`GeometryStorage`](file:///D:/hoarfrost/src/hoarfrost/surface/movable.rs) trait (`load`, `save`, `clear`) | Seams: [`NoopWindowStorage`](file:///D:/hoarfrost/src/hoarfrost/geometry/window_geometry.rs), [`MemoryWindowStorage`](file:///D:/hoarfrost/src/hoarfrost/geometry/window_geometry.rs)<br>Card storage: `NoopGeometryStorage`, `MemoryGeometryStorage`<br>Storage keys derived via `movable::storage_key(&id)`<br>Tests **MUST NOT** mutate real user paths; use memory seams or `tempfile::TempDir` | [Window Geometry](file:///D:/hoarfrost/src/hoarfrost/geometry/window_geometry.rs) |
| **Window & Pane Defaults** | Minimum & default window dimensions<br>Split pane floor dimensions<br>`UiScale` ladder | `WINDOW_DEFAULT_WIDTH`: 1000.0, `WINDOW_DEFAULT_HEIGHT`: 700.0<br>`WINDOW_MIN_WIDTH`: 640.0, `WINDOW_MIN_HEIGHT`: 480.0<br>`CENTER_PANE_MIN_W`: 240.0, `LEFT_PANE_MIN_W`: 150.0, `RIGHT_PANE_MIN_W`: 120.0<br>`UiScale`: `MIN = 0.75`, `MAX = 2.00`, `STEP = 0.10`, `DEFAULT = 1.00` | [Window Geometry](file:///D:/hoarfrost/src/hoarfrost/geometry/window_geometry.rs) |
| **Surface Geometry Constants** | Compile-time bare scalar carve-outs registered in `token_lint::SURFACE_GEOMETRY_CONSTANTS` | **Modals**: `EDITOR_WIDTH_MIN` (520.0), `EDITOR_WIDTH_MAX` (960.0), `EDITOR_HEIGHT_MIN` (380.0), `EDITOR_HEIGHT_MAX` (760.0), `TOOLS_WIDTH` (720.0), `BATCH_WIDTH` (560.0), `ABOUT_WIDTH` (440.0)<br>**Movable Cards**: `CARD_DEFAULT_WIDTH` (680.0), `CARD_DEFAULT_HEIGHT` (440.0)<br>**Menus & Dialogs**: `MENU_CARD_WIDTH_MIN` (224.0), `MENU_CARD_WIDTH_MAX` (320.0), `CARD_WIDTH` (320.0)<br>**Flyouts & Pickers**: `COMBO_MIN_WIDTH` (120.0), `CALENDAR_FLYOUT_WIDTH` (256.0), `TIME_FLYOUT_WIDTH` (220.0), `COLOR_FLYOUT_WIDTH` (240.0)<br>**Layout**: `TAB_MIN_WIDTH` (96.0), `TAB_MAX_WIDTH` (220.0), `GUTTER_WIDTH` (40.0) | [Token Lint (`token_lint`)](file:///D:/hoarfrost/src/hoarfrost/token_lint.rs) |
| **User-Facing Strings** | Verbatim text catalogue in [`strings`](file:///D:/hoarfrost/src/hoarfrost/strings.rs) | All control labels, action text, header titles, placeholder values, and validation messages **MUST** be referenced as `pub const` items in [`strings`](file:///D:/hoarfrost/src/hoarfrost/strings.rs). Call sites **MUST NOT** restate string literals. | [User-Facing Text](file:///D:/hoarfrost/src/hoarfrost/strings.rs) |
| **Quality Gate Commands** | Workspace verification gate execution before merging or completing changes | `cargo check --workspace --all-features`<br>`cargo nextest run -p hoarfrost --all-features --status-level fail --show-progress none`<br>`cargo clippy --workspace --all-features -- -D warnings`<br>`cargo fmt --check`<br>`cargo doc --workspace --all-features --no-deps --quiet` | [Quality Gates](#build-time-verification-gates) |

---

### Module render strategy matrix

Hoarfrost strictly divides user-interface implementations into two architectural categories:
1. **Live `iced` Widgets (`Element<'a, Message>`)**: Direct widget implementations of `iced::advanced::Widget` or composable view functions that return an active `Element<'a, Message>`.
2. **Headless Models / `*Host` Controllers / Style Resolvers**: Pure mathematical functions `(state, palette) -> Style`, declarative data structs, and linear gesture state machines that the host application or composite controls project into live widgets.

| Module / Control | Render Architecture Category | Primary Types / Constructors | Projection & Paint Details |
|---|---|---|---|
| [`widget::button_widget`](file:///D:/hoarfrost/src/hoarfrost/widget/button_widget.rs) | **Live `iced` Widget** | `Button<'a, Message>`, `button`, `icon_only`, `controlled` | Implements `Widget` and `Into<Element>`. Handles pointer capture, single-flight gating, and drawing. |
| [`widget::button`](file:///D:/hoarfrost/src/hoarfrost/widget/button.rs), `button_model`, `button_controller` | **Headless Resolver / Controller** | `ButtonModel`, `ButtonController`, `ButtonStyle`, `resolve_style` | Pure style resolution and linear gesture suppression logic. |
| [`widget::text_input`](file:///D:/hoarfrost/src/hoarfrost/widget/text_input.rs) | **Live `iced` Widget** | `TextInput<'a, Message>`, `text_input` | Implements `Widget` and `Into<Element>`. Encloses trailing icon slots and focus indicators. |
| [`widget::text_input_model`](file:///D:/hoarfrost/src/hoarfrost/widget/text_input_model.rs), `text_input_controller` | **Headless Model / Controller** | `TextInputModel`, `TextInputController`, `InputKind` | Manages text buffers, edit sessions, and live/commit event tiers. |
| [`widget::password_model`](file:///D:/hoarfrost/src/hoarfrost/widget/password_model.rs), [`password_host`](file:///D:/hoarfrost/src/hoarfrost/widget/password_host.rs) | **Headless Model / Controller** | `SecretInput`, `PasswordHost`, `password_host::mirror` | Headless secret masking and reveal transcript. Projected directly into `TextInput` via `mirror`. |
| [`widget::auto_suggest_model`](file:///D:/hoarfrost/src/hoarfrost/widget/auto_suggest_model.rs), [`auto_suggest_host`](file:///D:/hoarfrost/src/hoarfrost/widget/auto_suggest_host.rs) | **Headless Model / Controller** | `AutoSuggestModel`, `AutoSuggestHost`, `MatchSegment` | Headless completion lifecycle, generation token invalidation, and flyout geometry solver. |
| [`widget::text_area_model`](file:///D:/hoarfrost/src/hoarfrost/widget/text_area_model.rs), [`text_area_host`](file:///D:/hoarfrost/src/hoarfrost/widget/text_area_host.rs) | **Headless Model / Controller** | `TextAreaModel`, `TextAreaHost`, `LineCol` | Multi-line growth solver, scroll extent, and undo/redo ledger. |
| [`widget::toggle_widget`](file:///D:/hoarfrost/src/hoarfrost/widget/toggle_widget.rs) | **Live `iced` Widget** | `Checkbox<'a, Message>`, `RadioGroup<'a, Message, Id>`, `ToggleSwitch<'a, Message>` | Implements `Widget` and `Into<Element>`. Renders checkboxes, roving radio groups, and switches. |
| [`widget::toggle`](file:///D:/hoarfrost/src/hoarfrost/widget/toggle.rs), `toggle_model` | **Headless Resolver / Model** | `CheckboxModel`, `RadioGroupModel`, `ToggleSwitchModel`, `resolve_toggle_style` | Two-row polarity matrix, tri-state boolean transitions, and adaptive focus rings. |
| [`widget::badge`](file:///D:/hoarfrost/src/hoarfrost/widget/badge.rs), `badge_model` | **Headless Resolver / Model** | `BadgeModel`, `BadgeStyle`, `resolve_badge_style`, `badge_quad` | Resolves dot, count, and status badges. Emits quads painted by hosts. |
| [`widget::chip`](file:///D:/hoarfrost/src/hoarfrost/widget/chip.rs), `chip_model` | **Headless Resolver / Model** | `ChipModel<Id>`, `ChipStyle`, `resolve_chip_style`, `chip_layer` | Resolves filter/assist chip geometry and trailing target layers. |
| [`widget::progress`](file:///D:/hoarfrost/src/hoarfrost/widget/progress.rs), `progress_model` | **Headless Resolver / Model** | `ProgressModel`, `ProgressStyle`, `resolve_progress_style`, `progress_track_quad` | Resolves progress bars and spinners into quads; compiled under the `button` feature. |
| [`widget::range_model`](file:///D:/hoarfrost/src/hoarfrost/widget/range_model.rs) | **Headless Model** | `RangeModel` | Pure continuous interval clamping, endpoint policies, and drift-free stepping. |
| [`widget::slider`](file:///D:/hoarfrost/src/hoarfrost/widget/slider.rs), `slider_host` | **Headless Resolver / Host** | `SliderStyle`, `resolve_slider_style`, `SliderHost` | Headless pointer drag capture and three-tier scrub contract. |
| [`widget::number_box`](file:///D:/hoarfrost/src/hoarfrost/widget/number_box.rs), `number_box_host` | **Headless Resolver / Host** | `NumberBoxStyle`, `resolve_number_box_style`, `NumberBoxHost` | Stepper button resolver and repeat-hold cadence host. |
| [`widget::range_slider`](file:///D:/hoarfrost/src/hoarfrost/widget/range_slider.rs) | **Live `iced` Widget** | `RangeSlider<'a, Message>`, `range_slider` | Implements `Widget` and `Into<Element>`. Dual-thumb interval control with non-crossing invariant. |
| [`widget::rating`](file:///D:/hoarfrost/src/hoarfrost/widget/rating.rs) | **Live `iced` Widget** | `RatingControl<'a, Message>`, `rating_control` | Implements `Widget` and `Into<Element>`. Star rating row with hover preview and keyboard navigation. |
| [`widget::search_box`](file:///D:/hoarfrost/src/hoarfrost/widget/search_box.rs) | **Live `iced` Widget** | `SearchBox<'a, Message>` | Converted to `Element<'a, Message>` via `From`. 32px search field with dynamic clear action. |
| [`widget::segmented`](file:///D:/hoarfrost/src/hoarfrost/widget/segmented.rs), `segmented_model` | **Headless Resolver / Model** | `SegmentedModel<Id>`, `resolve_segmented_style` | Multi-segment button strip resolver and shared roving keyboard walk. |
| [`widget::split_button`](file:///D:/hoarfrost/src/hoarfrost/widget/split_button.rs) | **Live `iced` Widget** | `SplitButton<'a, Message>` | Converted to `Element<'a, Message>` via `From`. Two-zone primary action and menu flyout trigger. |
| [`widget::date_picker`](file:///D:/hoarfrost/src/hoarfrost/widget/date_picker.rs) | **Live `iced` Widget** | `DatePicker<'a, Message>`, `DateRangePicker<'a, Message>`, `CalendarView<'a, Message>` | Implements `Widget` and `Into<Element>`. Pure civil calendar arithmetic with fixed 42-cell day grid. |
| [`widget::time_picker`](file:///D:/hoarfrost/src/hoarfrost/widget/time_picker.rs) | **Live `iced` Widget** | `TimePicker<'a, Message>`, `time_picker` | Implements `Widget` and `Into<Element>`. Segmented trigger and civil time drum flyout. |
| [`widget::color_picker`](file:///D:/hoarfrost/src/hoarfrost/widget/color_picker.rs) | **Live `iced` Widget** | `ColorPicker<'a, Message>`, `ColorSwatchButton<'a, Message>` | Implements `Widget` and `Into<Element>`. 2D saturation-value canvas, hue slider, and swatch button. |
| [`widget::avatar`](file:///D:/hoarfrost/src/hoarfrost/widget/avatar.rs) | **Live `iced` Widget** | `Avatar<'a, Message>`, `avatar` | Implements `Widget` and `Into<Element>`. Three-stage cascade (image -> initials -> generic glyph). |
| [`widget::combo_box`](file:///D:/hoarfrost/src/hoarfrost/widget/combo_box.rs) | **Live `iced` Widget** | `Select<'a, Message>`, `ComboBox<'a, Message>`, `select`, `combo_box` | Implements `Widget` and `Into<Element>`. Dropdown select trigger and combo box. |
| [`composite::card`](file:///D:/hoarfrost/src/hoarfrost/composite/card.rs) | **Headless Resolver / Layer** | `CardStyle`, `resolve_style`, `content_bounds`, `card_layer` | Raised container styling; host applications render into `iced::widget::container` using `card_layer`. |
| [`composite::code_block`](file:///D:/hoarfrost/src/hoarfrost/composite/code_block.rs) | **Live `iced` Widget** | `CodeBlock<'a, Message>`, `code_block` | Implements `Widget` and `Into<Element>`. Fenced code container with 40px gutter and copy feedback. |
| [`composite::command_palette`](file:///D:/hoarfrost/src/hoarfrost/composite/command_palette.rs) | **Live `iced` Widget** | `CommandPalette<'a, Message>`, `command_palette` | Implements `Widget` and `Into<Element>`. Quick-open modal card with fuzzy search and prefix modes. |
| [`composite::diff_viewer`](file:///D:/hoarfrost/src/hoarfrost/composite/diff_viewer.rs) | **Live `iced` Widget** | `DiffViewer<'a, Message>`, `diff_viewer` | Implements `Widget` and `Into<Element>`. Unified and side-by-side diff renderer with hunk actions. |
| [`composite::empty_state`](file:///D:/hoarfrost/src/hoarfrost/composite/empty_state.rs) | **Live `iced` Widget** | `EmptyState<'a, Message>`, `empty_state` | Implements `Widget` and `Into<Element>`. Placeholder for empty, zero-match, and load-error states. |
| [`composite::grid_view`](file:///D:/hoarfrost/src/hoarfrost/composite/grid_view.rs) | **Live `iced` Widget** | `GridView<'a, Message, T>`, `grid_view` | Implements `Widget` and `Into<Element>`. 2D virtualized tile and card grid with column solving. |
| [`composite::keybind_recorder`](file:///D:/hoarfrost/src/hoarfrost/composite/keybind_recorder.rs) | **Live `iced` Widget** | `KeybindRecorder<'a, Message>`, `keybind_recorder` | Implements `Widget` and `Into<Element>`. Raw chord keyboard shortcut capture input. |
| [`composite::navbar`](file:///D:/hoarfrost/src/hoarfrost/composite/navbar.rs) | **Live `iced` Widget** | `NavBar<'a, Message>`, `navbar_trigger_element` | Converted to `Element<'a, Message>` via `From`. Shell navigation bar with latching menu triggers. |
| [`composite::navigation_rail`](file:///D:/hoarfrost/src/hoarfrost/composite/navigation_rail.rs) | **Live `iced` Widget** | `NavigationRail<'a, Message>`, `navigation_rail` | Implements `Widget` and `Into<Element>`. Collapsible vertical navigation bar (48px compact / 180 - 320px expanded). |
| [`composite::pagination`](file:///D:/hoarfrost/src/hoarfrost/composite/pagination.rs) | **Live `iced` Widget** | `PaginationBar<'a, Message>`, `pagination_bar` | Implements `Widget` and `Into<Element>`. 32px footer bar with page size options and range summary. |
| [`composite::pips_pager`](file:///D:/hoarfrost/src/hoarfrost/composite/pips_pager.rs) | **Live `iced` Widget** | `PipsPager<'a, Message>`, `pips_pager` | Implements `Widget` and `Into<Element>`. 24px pagination strip with boundary-clamped step buttons. |
| [`composite::side_sheet`](file:///D:/hoarfrost/src/hoarfrost/composite/side_sheet.rs) | **Live `iced` Widget** | `SideSheet<'a, Message>`, `side_sheet` | Implements `Widget` and `Into<Element>`. Contextual inspector drawer with resize dividers. |
| [`composite::skeleton`](file:///D:/hoarfrost/src/hoarfrost/composite/skeleton.rs) | **Live `iced` Widget** | `Skeleton<'a, Message>`, `skeleton` | Implements `Widget` and `Into<Element>`. Loading shimmer placeholder reserving exact dimensions. |
| [`composite::split_divider`](file:///D:/hoarfrost/src/hoarfrost/composite/split_divider.rs) | **Live `iced` Widget** | `SplitDivider<'a, Message>`, `split_divider` | Implements `Widget` and `Into<Element>`. Interactive draggable splitter for split panes. |
| [`composite::stepper`](file:///D:/hoarfrost/src/hoarfrost/composite/stepper.rs) | **Live `iced` Widget** | `Stepper<'a, Message>`, `stepper` | Implements `Widget` and `Into<Element>`. Multi-step workflow header and vertical sequence container. |
| [`composite::tab_strip`](file:///D:/hoarfrost/src/hoarfrost/composite/tab_strip.rs) | **Live `iced` Widget** | `TabStrip<'a, Message>` | Converted to `Element<'a, Message>` via `From`. Scrollable document tab strip with dirty guards. |
| [`composite::tree_view`](file:///D:/hoarfrost/src/hoarfrost/composite/tree_view.rs) | **Live `iced` Widget Container** | `TreeView`, `render_tree_with_footer` | Live composite container enclosing visible virtual rows with a pinned search footer. |
| [`composite::breadcrumb`](file:///D:/hoarfrost/src/hoarfrost/composite/breadcrumb.rs) | **Headless Resolver / Layout** | `solve_breadcrumb_elision`, `BreadcrumbStyle` | Pure width-dependent elision solver; host renders crumbs and overflow menus. |
| [`composite::form_field`](file:///D:/hoarfrost/src/hoarfrost/composite/form_field.rs) | **Headless Resolver / Layout** | `solve_form_field_layout`, `FormFieldStyle` | Pure vertical rhythm solver enforcing reserved validation caption slots. |
| [`composite::list_tile`](file:///D:/hoarfrost/src/hoarfrost/composite/list_tile.rs) | **Headless Resolver / Layout** | `solve_list_tile_layout`, `ListTileStyle` | Pure 3-slot row geometry (leading mark, filling text, trailing action targets). |
| [`composite::overflow_toolbar`](file:///D:/hoarfrost/src/hoarfrost/composite/overflow_toolbar.rs) | **Headless Resolver / Layout** | `solve_overflow_layout`, `OverflowToolbarStyle` | Priority-declared overflow eviction solver; prevents vertical shift. |
| [`composite::table_view`](file:///D:/hoarfrost/src/hoarfrost/composite/table_view.rs), `table_*` | **Headless Model / Resolver** | `TableView`, `solve_columns`, `TableHeaderLayout` | Virtualized grid model, column width constraint solving, and stable sorting. |
| [`composite::toast`](file:///D:/hoarfrost/src/hoarfrost/composite/toast.rs) | **Headless Model / Stack** | `ToastStack`, `ToastModel`, `ToastPlacement` | Positioning queue, auto-dismiss timers, and focus group management. |
| [`composite::tree_row`](file:///D:/hoarfrost/src/hoarfrost/composite/tree_row.rs) | **Headless Resolver / Layout** | `solve_tree_row_layout`, `indent_allowance` | Tree row slot math, depth-indent clamping, and contrast-promoted guide lines. |
| [`composite::virtual_list`](file:///D:/hoarfrost/src/hoarfrost/composite/virtual_list.rs) | **Headless Math / Windowing** | `visible_range`, `scroll_offset` | Fixed-height metric math bounding realized rows independently of total row count. |
| [`surface::tooltip`](file:///D:/hoarfrost/src/hoarfrost/surface/tooltip/mod.rs) | **Live `iced` Widget Wrapper** | `Tooltip<'a, Message>`, `tooltip` | Implements `Widget` and `Into<Element>`. Wraps any child element to show hover callouts. |
| [`surface::menu`](file:///D:/hoarfrost/src/hoarfrost/surface/menu/mod.rs) | **Headless Engine / Cascade** | `MenuCascade`, `Menu`, `solve_menu_placement` | Floating card cascade engine, submenu positioning, and structural validation. |
| [`surface::zorder`](file:///D:/hoarfrost/src/hoarfrost/surface/zorder.rs) | **Headless Manager / Registry** | `SurfaceRegistry`, `ZBand`, `escape_with_pre_rank` | Spatial stacking, pointer pass-through routing, and pre-ranked Escape unwinding. |
| [`surface::modal`](file:///D:/hoarfrost/src/hoarfrost/surface/modal.rs) | **Headless Host / Trap** | `ModalHost`, `ModalLayout`, `ModalOutcome` | Focus-trapping modal card host and scrim geometry calculator. |
| [`surface::input_dialog`](file:///D:/hoarfrost/src/hoarfrost/surface/input_dialog.rs) | **Headless Host / Layout** | `InputLayout`, `InputControl`, `Verdict` | Prompt card geometry with permanently reserved validation error slots. |
| [`surface::movable`](file:///D:/hoarfrost/src/hoarfrost/surface/movable.rs) | **Headless Host / Engine** | `MovableHost`, `MovableGeometry` | Detached card drag engine, clamp-to-viewport physics, and geometry serialization. |
| [`surface::popover`](file:///D:/hoarfrost/src/hoarfrost/surface/popover/mod.rs) | **Headless Host / Placement** | `PopoverHost`, `PopoverModel`, `solve_popover_placement` | Anchor-relative callout placement engine and single-escape unwinding. |

---

### Duplicate symbol names across modules

Because Hoarfrost intentionally avoids glob re-exports, several type and function names recur across distinct subsystems. Call sites **MUST** qualify imports using the specific paths detailed below:

| Colliding Symbol | Module & Qualified Signature | Behavior & Distinguishing Purpose |
|---|---|---|
| `group_for` | [`surface::popover::group_for() -> GroupId`](file:///D:/hoarfrost/src/hoarfrost/surface/popover/model.rs) | Free function taking **zero arguments**; returns the fixed callout focus group identifier (`"popover"`). |
| | [`surface::modal::group_for(id: &SurfaceId) -> GroupId`](file:///D:/hoarfrost/src/hoarfrost/surface/modal.rs) | Free function taking `&SurfaceId`; returns the modal focus trap group (`"modal:{id}"`). |
| | [`surface::input_dialog::group_for(id: &SurfaceId) -> GroupId`](file:///D:/hoarfrost/src/hoarfrost/surface/input_dialog.rs) | Free function taking `&SurfaceId`; returns the input dialog focus trap group (`"dialog:{id}"`). |
| | [`surface::movable::group_for(id: &SurfaceId) -> GroupId`](file:///D:/hoarfrost/src/hoarfrost/surface/movable.rs) | Free function taking `&SurfaceId`; returns the movable utility card focus group (`"movable:{id}"`). |
| | [`composite::toast::ToastStack::group_for(id: &NotificationId) -> GroupId`](file:///D:/hoarfrost/src/hoarfrost/composite/toast.rs) | Associated method on `ToastStack`; returns the notification card focus group (`"toast:{id}"`). |
| | `composite::navigation_rail::NavigationRailModel::group_for` | Derives the roving navigation destination group identifier. |
| `segmented_model` | [`widget::segmented_model`](file:///D:/hoarfrost/src/hoarfrost/widget/segmented_model.rs) | Resides in the **`widget`** module (**NOT** `composite`). Defines `SegmentedModel<Id>`, `SegmentItem<Id>`, and selection transitions. |
| `SelectionMode` | [`widget::segmented_model::SelectionMode`](file:///D:/hoarfrost/src/hoarfrost/widget/segmented_model.rs) | Enum with variants `{ SingleSelect, MultiSelect }` governing joined button strips. |
| | [`composite::tree_view::SelectionMode`](file:///D:/hoarfrost/src/hoarfrost/composite/tree_view.rs) (or `TreeSelectionMode`) | Enum with variants `{ Single, ExtendedMulti }` governing row selection with range-shift anchors. |
| `RoveDirection` | [`widget::rove::RoveDirection`](file:///D:/hoarfrost/src/hoarfrost/widget/rove.rs) | Has variants **`Forward`** and **`Backward`** (**NOT** `Next` / `Previous`). `Forward` advances toward higher indices (`Down`/`Right`); `Backward` steps toward lower indices (`Up`/`Left`). |
| `indent_allowance` | [`composite::tree_row::indent_allowance(row_width: f32) -> f32`](file:///D:/hoarfrost/src/hoarfrost/composite/tree_row.rs) | Pure layout helper calculating available indent floor after subtracting furniture (chevron, icon, label floor, padding). |
| `DismissCause` | [`surface::modal::DismissCause`](file:///D:/hoarfrost/src/hoarfrost/surface/modal.rs) | Variants: `{ Escape, Scrim, CloseButton, Programmatic }`. |
| | [`surface::input_dialog::DismissCause`](file:///D:/hoarfrost/src/hoarfrost/surface/input_dialog.rs) | Variants: `{ Escape, CancelButton, Scrim, Programmatic }`. |
| | [`surface::popover::DismissCause`](file:///D:/hoarfrost/src/hoarfrost/surface/popover/mod.rs) | Variants: `{ PrimaryAction, DismissButton, Escape, ClickOutside, Programmatic }`. |
| `IgnoredReason` | [`surface::menu::cascade::IgnoredReason`](file:///D:/hoarfrost/src/hoarfrost/surface/menu/cascade.rs) | Explains why a cascade navigation request was unserved (`NotOpen`, `NotSubmenu`, `SubmenuDisabled`, etc.). |
| | [`surface::popover::host::IgnoredReason`](file:///D:/hoarfrost/src/hoarfrost/surface/popover/host.rs) | Explains why a popover transition was unserved (`NotOpen`, `AlreadyOpen`, etc.). |
| `clamp_width` | `surface::tooltip::clamp_width(width: f32) -> f32` | Free function clamping proposed tooltip width to `MAX_WIDTH` (280px). |
| | `surface::popover::PopoverStyle::clamp_width(width: f32) -> f32` | Associated function on `PopoverStyle` clamping callout width to `[WIDTH_MIN, WIDTH_MAX]` (280 - 360px). |

---

## Error Handling

### Principles

Hoarfrost adheres to five architectural error-handling tenets across all modules:

1. **Refusals are values, never exceptions**: Interaction rules, focus transitions, and gesture recognizers return structured sum-type variants describing what was refused and why (`Punishment`, `ModalOutcome::DismissRefused`, `Ignored(EscapeRefusal)`, `Activation::Suppressed`). The crate performs no I/O, defines no catchable panics, and never converts input failures into aborts.
2. **Paint and layout paths never panic**: Every layout solver, style resolver, and draw routine is total over all inputs. Non-finite floating-point numbers (`NaN`, `+inf`, `-inf`) and negative measurements are sanitized to boundary clamps before geometry computation.
3. **Degenerate input is sanitized at the perimeter**: Ill-formed inputs are corrected without silent propagation: non-finite UI scales quantize to `1.00`, out-of-order range slider bounds clamp to the minimum step, negative scroll offsets reset to `0.0`, and progress fractions clamp strictly to `[0.0, 1.0]`.
4. **Contrast shields guard development without crashing production**: Local assertion shields (`assert_contrast`, `assert_fill_foreground_lock`) execute exclusively in `debug_assertions` builds, pinning illegal resolver combinations during unit testing while allowing production release builds to paint. Crate-wide compile/test audits verify all reachable states.
5. **Exhaustive pattern matching on refusal variants**: Consumers **MUST** explicitly handle every refusal variant. Employing wildcard catch-all arms (`_ => {}`) on `NoOp` or `Suppressed` variants obscures the specific logic boundary that the refusal was designed to surface.

---

### Refusal & fault taxonomy

The table below catalogs every refusal, suppression, and fault enum in the framework, linking it directly to its subsystem:

| Refusal / Fault Type | Subsystem & API Link | Nature of Fault / Refusal Gated |
|---|---|---|
| [`DeclineReason`](file:///D:/hoarfrost/src/hoarfrost/surface/zorder.rs) / [`EscapeRefusal`](file:///D:/hoarfrost/src/hoarfrost/surface/zorder.rs) | [`surface::zorder`](file:///D:/hoarfrost/src/hoarfrost/surface/zorder.rs), [`surface::modal`](file:///D:/hoarfrost/src/hoarfrost/surface/modal.rs) | Surface declined unwinding (`NotOpen`, `NotFocused`, `CannotUnwind`), stack was empty (`EmptyStack`), or all registered surfaces declined (`AllDeclined`). |
| [`MenuViolation`](file:///D:/hoarfrost/src/hoarfrost/surface/menu/model.rs) / [`IgnoredReason`](file:///D:/hoarfrost/src/hoarfrost/surface/menu/cascade.rs) | [`surface::menu`](file:///D:/hoarfrost/src/hoarfrost/surface/menu/mod.rs) | Structural invariant failure in declarative menu tree (`MissingKey`, `DuplicateKey`, `DestructiveNotLast`, `MixedGroup`, `RadioOverSelected`); cascade transition declined. |
| [`SuppressedReason`](file:///D:/hoarfrost/src/hoarfrost/binding/pending.rs) / [`CancelRefusal`](file:///D:/hoarfrost/src/hoarfrost/binding/pending.rs) | [`binding`](file:///D:/hoarfrost/src/hoarfrost/binding/mod.rs), [`widget::button_controller`](file:///D:/hoarfrost/src/hoarfrost/widget/button_controller.rs) | Action activation suppressed due to active in-flight task (`Busy`) or unobserved state (`AwaitingProjection`); cancellation refused (`NotBusy`, `NotCancellable`). |
| [`GestureStart`](file:///D:/hoarfrost/src/hoarfrost/binding/snapshot.rs) | [`binding::snapshot`](file:///D:/hoarfrost/src/hoarfrost/binding/snapshot.rs) | Linear gesture initiation rejected because a prior gesture remains active and uncommitted (`AlreadyOpen`). |
| [`InvariantViolation`](file:///D:/hoarfrost/src/hoarfrost/invariant_gate.rs) | [`invariant_gate`](file:///D:/hoarfrost/src/hoarfrost/invariant_gate.rs) | Microcopy, casing, or ergonomics invariant violated (`SentenceCase`, `SecretRedaction`, `ActionEllipsis`). |
| [`PunishReason`](file:///D:/hoarfrost/src/hoarfrost/validation/punish.rs) | [`validation`](file:///D:/hoarfrost/src/hoarfrost/validation/mod.rs) | Form field validation state evaluated under the "Reward Early, Punish Late" protocol (`Valid`, `WithheldPristine`, `TouchedInvalid`, `DisallowedImmediate`). |
| `FillLock` / [`ChromeClass`](file:///D:/hoarfrost/src/hoarfrost/token/contrast.rs) | [`token::contrast`](file:///D:/hoarfrost/src/hoarfrost/token/contrast.rs) | WCAG 2.1 contrast floor breached (< 4.5:1 text, < 3.0:1 chrome) or Rule 4 Solid Fill Foreground Lock breached on a saturated background. |
| [`EllipsisViolation`](file:///D:/hoarfrost/src/hoarfrost/microcopy/action_label.rs) | [`composite::navbar`](file:///D:/hoarfrost/src/hoarfrost/composite/navbar.rs), [`microcopy::action_label`](file:///D:/hoarfrost/src/hoarfrost/microcopy/action_label.rs) | Microcopy ellipsis convention breached (`MissingActionEllipsis`, `SpuriousActionEllipsis`, `SpacedActionEllipsis`, `UnicodeEllipsisAsActionSuffix`). |

---

### Validation and recovery pipeline

#### The `punish` Pipeline (`UIF-PRIN-009`)

Validation errors are evaluated through a single deterministic pure function:

```rust
pub const fn punish(stance: EditStance, invalid: bool, disallowed: bool) -> Punishment;
```

The pipeline enforces the specification's ninth pillar: **Reward Early, Punish Late**. While host business logic determines *whether* a field is valid, the framework controls *when* the error is visually exposed:

| `EditStance` | `invalid` | `disallowed` | `reason()` | `invalid()` | `painted()` | `blocks_submit()` |
|---|---|---|---|---|---|---|
| `Pristine` | `false` | `false` | `Valid` | `false` | `false` | `false` |
| `Pristine` | `true` | `false` | `WithheldPristine` | `true` | **`false`** | **`true`** |
| `Pristine` | Any | `true` | `DisallowedImmediate` | `true` | **`true`** | **`true`** |
| `Touched` | `false` | `false` | `Valid` | `false` | `false` | `false` |
| `Touched` | `true` | `false` | `TouchedInvalid` | `true` | **`true`** | **`true`** |
| Any | Any | `true` | `DisallowedImmediate` | `true` | **`true`** | **`true`** |

1. **Visual suppression on pristine fields**: When a pristine field has never been committed or blurred, invalid input produces `WithheldPristine`. The field remains calm (no red border, no error message displayed: `painted() == false`), but form submission is strictly blocked (`blocks_submit() == true`).
2. **Immediate disallowance**: Characters in [`FILENAME_ILLEGAL_CHARS`](file:///D:/hoarfrost/src/hoarfrost/validation/disallowed.rs) (`/ \ : * ? " < > |`) or ASCII control characters trigger `DisallowedImmediate` instantaneously, turning the control invalid regardless of stance.
3. Call sites **MUST** inspect `punishment.painted()` prior to rendering warning icons or error borders, and **MUST** inspect `punishment.blocks_submit()` prior to executing action callbacks.

#### Snapshots and `EditSnapshot::from_text_input`

When an edit session begins, the application captures a snapshot to prepare for potential cancellation or rejected commits:

```rust
pub struct EditSnapshot {
    buffer: String,
    selection: Option<TextRange>,
    scroll: Option<ScrollOffset>,
    was_dirty: bool,
}
```

- **`EditSnapshot::from_text_input(model: &TextInputModel)`**:
  - Accurately captures `buffer: model.value.clone()` and `selection: model.selection`.
  - Sets **`scroll: None`**. Single-line `TextInputModel` instances do not track scroll offsets; Iced internally owns caret panning. Setting `scroll: None` (rather than `Some(ScrollOffset::ZERO)`) guarantees that a subsequent restore never falsely claims to have mutated the viewport.
- **`EditSnapshot::from_text_area(model: &TextAreaModel)`**:
  - Captures `buffer: model.text()`, `selection: model.selection()`, `scroll: Some(ScrollOffset::new(model.scroll_offset()))`, and `was_dirty: model.is_dirty()`.

#### `RecoverySession` and State Reversion

```rust
pub struct RecoverySession { /* linear gesture slot */ }

impl RecoverySession {
    pub fn begin_from_text_input(&mut self, model: &TextInputModel) -> GestureStart;
    pub fn begin_from_text_area(&mut self, model: &TextAreaModel) -> GestureStart;
    pub fn cancel_into_text_input(&mut self, model: &mut TextInputModel) -> Option<RestoredFields>;
    pub fn cancel_into_text_area(&mut self, model: &mut TextAreaModel) -> Option<RestoredFields>;
    pub fn settle(&mut self) -> bool;
}
```

1. **Atomic Revert**: Invoking `cancel_into_*` restores buffer, selection, and scroll position in a single atomic update.
2. **Re-entrancy Protection**: Attempting to invoke `begin_*` while a session is already active returns `GestureStart::AlreadyOpen`. The framework refuses the secondary snapshot to prevent clobbering the original pre-edit state.
3. **Settlement**: Upon a confirmed commit, `settle()` closes the session and drops the snapshot. Calling `cancel_*` on a settled session cleanly returns `None`.

---

### Escape and surface unwinding

#### The Escape Rank Hierarchy (`UIF-WIN-003`)

The framework maintains an absolute separation between spatial Z-order stacking (`ZBand::index`) and keyboard dismissal priority (`ZBand::escape_rank`). When `Escape` is pressed, surfaces unwind strictly by **escape rank**, highest to lowest:

```
[Pre-Rank: Active Pointer Drag (Slider, Splitter, Window Header)]
                       |
                       v
       [Rank 3: Menu (Deepest Submenu -> Root Menu)]
                       |
                       v
            [Rank 2: Modal & Input Dialogs]
                       |
                       v
        [Rank 1: Drawer / Movable Utility Overlay]
                       |
                       v
          [Rank 0: Inline Workspace / Filter Reset]
```

- **Pre-Rank (Pointer Capture)**: If an interactive drag is active (e.g., resizing a `SplitDivider` or scrubbing a `Slider`), the drag gesture captures the pointer. Hosts **MUST** execute [`escape_with_pre_rank`](file:///D:/hoarfrost/src/hoarfrost/surface/zorder.rs) so that `Escape` aborts the live drag and restores pre-drag geometry before any floating surface is dismissed.
- **Rank 3 (`ZBand::Menu`)**: Submenus unwind outward: deepest submenu first, then parent submenu, then root context menu.
- **Rank 2 (`ZBand::Modal`)**: Blocking modal dialogs unwind next, clearing their scrims.
- **Rank 1 (`ZBand::UtilityOverlay`)**: Modeless floating utility overlays (e.g. Logs, Agent Debug) unwind only when focused. An unfocused overlay steps aside (`Declined(DeclineReason::NotFocused)`).
- **Rank 0 (`ZBand::Workspace`)**: Main window inline filters and tree selections clear last.

#### `ModalHost::unwind_answer` Contract

The dismissal behavior of a modal dialog host is strictly codified:

```rust
pub fn unwind_answer(&mut self, focus: &mut FocusState) -> UnwindAnswer {
    if !self.open {
        return UnwindAnswer::Declined(DeclineReason::NotOpen);
    }
    if self.dirty {
        return UnwindAnswer::Declined(DeclineReason::CannotUnwind);
    }
    if self.action.blocks_dismiss() {
        return UnwindAnswer::Declined(DeclineReason::CannotUnwind);
    }

    self.teardown(focus);
    UnwindAnswer::Dismissed
}
```

1. **Why `ModalHost::unwind_answer` returns `Dismissed` when open**: An active modal dialog is a modal focus trap (`UIF-MOD-001`, `UIF-MOD-002`). It completely covers the workspace with a dimmed backdrop scrim and isolates keyboard interaction to its contained group. When the modal is clean and has no blocking in-flight actions, `Escape` consumes the key, dismisses the modal, tears down the focus trap, and returns `UnwindAnswer::Dismissed`.
2. **Why it never returns `NotFocused`**: Unlike modeless overlays (Band 1), a modal dialog never steps aside. Even if keyboard focus rests on a child button or inner input rather than the card container, the modal trap owns the window's top-level event stream. Yielding `Escape` to lower layers via `NotFocused` would breach the modal containment boundary.
3. **Dirty and Busy Refusals**: If the modal holds unsaved edits (`dirty == true`) or an action is currently executing (`action.blocks_dismiss() == true`), it returns `Declined(DeclineReason::CannotUnwind)`, allowing the host to present a confirmation guard without dropping user data.

---

### Build-time verification gates

Hoarfrost enforces architectural correctness through five mandatory verification gates (`UIF-VER-001` .. `UIF-VER-005`):

1. **`UIF-VER-001` - Contrast & Token Discipline**:
   - Validated by [`token::contrast`](file:///D:/hoarfrost/src/hoarfrost/token/contrast.rs) and [`token_lint`](file:///D:/hoarfrost/src/hoarfrost/token_lint.rs).
   - Proves that every sRGB color pair clears WCAG 2.1 contrast minimums (4.50:1 normal text, 3.00:1 active chrome, 4.00:1 - 6.05:1 disabled text band).
   - Enforces the Rule 4 Solid Fill Foreground Lock (pins `color.text.strong` onto solid fills).
   - Verifies that zero raw RGB/hex constructors exist outside [`token/color.rs`](file:///D:/hoarfrost/src/hoarfrost/token/color.rs).
2. **`UIF-VER-002` - Layout & Bounded Geometry**:
   - Validated by pure geometry solvers in [`geometry`](file:///D:/hoarfrost/src/hoarfrost/geometry/mod.rs) and [`composite`](file:///D:/hoarfrost/src/hoarfrost/composite/mod.rs).
   - Proves that all control dimensions meet or exceed hit target minimums (24px compact, 32px standard).
   - Guarantees zero layout shift when validation states transition by verifying reserved slots.
3. **`UIF-VER-003` - State Machine Completeness & Determinism**:
   - Validated by [`state::resolve`](file:///D:/hoarfrost/src/hoarfrost/state.rs) and atomic resolver matrices.
   - Proves that every cell of the `(variant, state, palette)` tensor resolves to a concrete visual style without `NaN`, non-finite measurements, or unhandled combinations.
   - Proves that linear activation gates suppress duplicate submissions.
4. **`UIF-VER-004` - Headless Visual Regression Harness**:
   - Validated by the runnable [`bin/gallery.rs`](file:///D:/hoarfrost/src/hoarfrost/bin/gallery.rs) binary and [`test_helpers`](file:///D:/hoarfrost/src/hoarfrost/test_helpers.rs).
   - Renders every widget across all permutations through the real `iced` draw pipeline into headless pixel buffers, verifying draw layers against reference token quads.
5. **`UIF-VER-005` - Microcopy & Invariant Gates**:
   - Validated by [`invariant_gate`](file:///D:/hoarfrost/src/hoarfrost/invariant_gate.rs) and [`microcopy`](file:///D:/hoarfrost/src/hoarfrost/microcopy/mod.rs).
   - Asserts sentence casing across UI chrome labels, enforces the single-character Unicode ellipsis (`…`) for truncation versus three ASCII periods (`...`) for prompt triggers, and ensures that `Debug` and `Display` implementations redact sensitive credentials (`check_secret_redaction`).

