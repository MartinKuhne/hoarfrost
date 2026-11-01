# hoarfrost

A design system and widget framework for [iced](https://github.com/iced-rs/iced): desktop
controls for panes, tabs, menus and dialogs, each implemented against a written
specification whose `UIF-*` requirement IDs its own module cites in its doc comment.

## What it is for

Desktop productivity applications — the kind with panes, tabs, a navbar, context menus,
modal dialogs and a filesystem to navigate. The specification names nine core surfaces as
its scope: main window, navbar, multiple panes, tabbed windows, input dialogs, modal
dialogs, movable overlays, fixed menus and context menus.

It assumes a high-density desktop: controls hold a compact 28–36px height against a
mandatory 24×24px hit target, and every pointer operation has a deterministic keyboard
path.

## What you get

**Your data stays yours.** Widgets read the model you own and cannot write it, so there is
no second copy inside the framework quietly drifting out of sync with yours.

**Callbacks you cannot mix up.** A live tick, a final commit and a cancellation are three
distinct types. Passing one where another is required is a compile error, and a gesture
cannot be settled twice.

**An interface that does not jump.** Hover, focus, busy, disabled and validation changes
never move anything on screen — the failure a desktop user feels constantly and cannot
name.

**Keyboard parity.** Every pointer operation has a deterministic keyboard path, and a large
composite is a single tab stop rather than one press per row.

**Scale by construction.** A million-row list builds only the rows on screen, and the bound
does not depend on how many rows there are.

**Regressions that read as text.** 6,188 tests assert resolved colours and measured
geometry rather than comparing screenshots, so a failure names the token or the rectangle
that moved instead of showing a picture that looks different.

**Straight talk about gaps.** 89 of 92 requirements are implemented and 3 are recorded as
partial, each naming what is missing.

**Small and clean.** Four dependencies, all of them iced. No `unsafe` blocks. MIT.

## The design it holds itself to

Nine principles in `spec.md` are normative rather than aspirational, and each is written as
something a widget can be checked against:

| | Principle | What it rules out |
|---|---|---|
| `UIF-PRIN-001` | Perceptual harmony through tokenization | Raw numeric literals in widget definitions |
| `UIF-PRIN-002` | High-density desktop ergonomics | Controls that trade height for pointer accuracy |
| `UIF-PRIN-003` | Zero layout shift across state transitions | Hover, focus, busy or invalid states that move anything |
| `UIF-PRIN-004` | Non-collapsing chrome | Hiding unavailable actions; empty panels that collapse to zero |
| `UIF-PRIN-005` | Dual-modality access, redundant cueing | Meaning carried by colour alone |
| `UIF-PRIN-006` | Emphasis hierarchy and container separation | Two high-emphasis buttons in one container; containers that touch with no divider |
| `UIF-PRIN-007` | Microcopy and truncation style | Title case; paths truncated at the end instead of the middle |
| `UIF-PRIN-008` | Single tab-stop composites | A 40-row table that eats 40 presses of `Tab` |
| `UIF-PRIN-009` | Reward early, punish late | Painting a pristine field red before the user has committed |

## Controls that look alike and are not

| You need | Use | Not | Why |
|---|---|---|---|
| A state that can be neither | Checkbox | Toggle switch | Checkbox carries `Indeterminate`; the switch is binary and carries `busy` only, with no determinate progress or cancel on a 36×20px track |
| Paged content with a count | Pagination bar | Pips pager | The bar is a 32px footer with a range readout; the pips are dots for a carousel or a tour |
| A command the user types | Command palette | Fixed menu | The palette is a modal overlay card at Escape Rank 2; a fixed menu is anchored to a navbar trigger |
| Content is missing vs still arriving | Empty state / Skeleton | An empty panel | The empty state holds its height instead of collapsing to zero; the skeleton is the loading shimmer |
| An explanation without a click | Tooltip | — | It opens on hover at 400ms and on keyboard focus at 0ms, so the keyboard path gets it too |

Every image below is rendered by `hoarfrost-gallery` — one PNG per demo section, each card
showing one widget across its states at 2× scale.

## Widgets

### Atoms

#### Buttons

Four button variants — Primary, Secondary, Danger and Ghost — across the resolved
interaction state matrix, with busy actions that suppress repeat activation instead of
firing twice.

![Buttons](doc/img/buttons.png)

#### Checkbox

The tri-state tick control — `Unchecked`, `Checked` and `Indeterminate` — which is what makes
it the right choice over a switch for a parent whose children are only partly selected.

![Checkbox](doc/img/checkbox.png)

#### Radio group

A group of mutually exclusive choices sharing one roving focus walk, so the group is a single
stop to the keyboard.

![Radio group](doc/img/radio-group.png)

#### Toggle switch

The binary on/off control, shown live, busy and disabled; it locks out re-toggling while an
async flip settles rather than showing progress on its 36×20px track.

![Toggle switch](doc/img/toggle-switch.png)

#### Segmented

A joined strip of mutually exclusive segments — single selection drawn as one continuous
bar.

![Segmented](doc/img/segmented.png)

#### Split button

The two-zone control: a primary action beside a separate zone that opens its menu, so the
common case is one click and the alternatives are never hidden.

![Split button](doc/img/split-button.png)

#### Hyperlink

Inline interactive text, with its focus state and internal, external and disabled
renderings.

![Hyperlink](doc/img/hyperlink.png)

#### Text input

The single-line entry atom — entry, search and dropdown trigger across the five-state
matrix.

![Text input](doc/img/text-input.png)

#### Password

A secret-entry field whose value cannot print itself: the secret type renders the mask under
`Debug` and `Display`, so a password that reaches a log line is still masked. Revealing it
drops by itself the moment the field loses focus.

![Password](doc/img/password.png)

#### Combo box

The ComboBox / Select trigger — the closed box the user clicks, and the one menu cascade
it opens.

![Combo box](doc/img/combo-box.png)

#### Search box

The pinned search box for tree views and search panels.

![Search box](doc/img/search-box.png)

#### Rating

The star-scale rating control, shown live, read-only and disabled.

![Rating](doc/img/rating.png)

#### Range slider

The dual-thumb slider over a value that is finite and ordered, so a host cannot hand in a
lower bound above the upper one and get a control that agrees with it.

![Range slider](doc/img/range-slider.png)

#### Avatar

The avatar and person-picture widget for representing a person.

![Avatar](doc/img/avatar.png)

#### Date picker

The date picker and the date range picker, each opening a calendar view. The calendar
arithmetic is computed in the crate, so there is no date library to pull in.

![Date picker](doc/img/date-picker.png)

#### Time picker

The segmented time picker, with civil-time arithmetic and a drum flyout.

![Time picker](doc/img/time-picker.png)

#### Colour picker

The colour picker surface and its standalone swatch-button trigger.

![Colour picker](doc/img/colour-picker.png)

#### Tooltip

The framework-owned tooltip, shown on hover and on keyboard focus alike — 400ms on hover,
immediately on keyboard focus, so keyboard users are not made to wait.

![Tooltip](doc/img/tooltip.png)

### Composites

#### Tab strip (UIF-TAB-003)

Open-document tabs that intercept the close of a dirty tab before edits are discarded,
hand focus to the most-recently-used tab, and stay in sync when a file is deleted outside
the application.

![Tab strip](doc/img/tab-strip-uif-tab-003.png)

#### Navigation rail (UIF-COMP-009)

The collapsible navigation rail for shell-level navigation; `Escape` during a drag reverts
the rail to where the resize started.

![Navigation rail](doc/img/navigation-rail-uif-comp-009.png)

#### Stepper + wizard footer (UIF-COMP-011)

The multi-step workflow header and the wizard footer that drives it.

![Stepper and wizard footer](doc/img/stepper-wizard-footer-uif-comp-011.png)

#### Pagination bar (UIF-COMP-012)

The 32px pagination footer for paged content, with a range summary in tabular figures so a
changing count does not shove the text beside it around.

![Pagination bar](doc/img/pagination-bar-uif-comp-012.png)

#### Pips pager (UIF-COMP-013)

Compact pagination as carousel dots, for a tour or a carousel rather than a paged table.

![Pips pager](doc/img/pips-pager-uif-comp-013.png)

#### Code block (UIF-COMP-015)

The code block container and syntax viewer.

![Code block](doc/img/code-block-uif-comp-015.png)

#### Diff viewer (UIF-COMP-015)

The visual diff viewer, in unified and side-by-side modes.

![Diff viewer](doc/img/diff-viewer-uif-comp-015.png)

#### Empty states (UIF-COMP-014)

The placeholder for a view with nothing in it — no results, filtered to nothing, or errored —
which holds its height instead of collapsing to zero and letting the surrounding layout
snap.

![Empty states](doc/img/empty-states-uif-comp-014.png)

#### Grid view (UIF-COMP-013)

The two-dimensional virtualized tile and card grid: only the tiles in view are built, so a
large library costs the same as a small one.

![Grid view](doc/img/grid-view-uif-comp-013.png)

#### Keybind recorder (UIF-COMP-015)

The keyboard shortcut recorder. Each keystroke emits a live chord preview and the finished
chord arrives as one commit, so a settings panel can show the chord building without
keeping its own copy of it.

![Keybind recorder](doc/img/keybind-recorder-uif-comp-015.png)

#### Skeleton (UIF-COMP-015)

The skeleton placeholder and shimmer shown while content loads — the loading case, as
distinct from the empty one.

![Skeleton](doc/img/skeleton-uif-comp-015.png)

#### Side sheet (UIF-COMP-012)

The docked inspector drawer.

![Side sheet](doc/img/side-sheet-uif-comp-012.png)

#### Command palette (UIF-COMP-010)

The command palette and its modal search view: a 560×380px modal overlay card with mode
prefix chips. `Escape` dismisses in two stages, so a flyout stacked above the palette closes
before the palette itself does.

![Command palette](doc/img/command-palette-uif-comp-010.png)

#### Form field (UIF-COMP-016)

The stacked form field — label, control, and a caption slot reserved at full height whether
or not anything is written in it, so validation text cannot make the form jump.

![Form field](doc/img/form-field-uif-comp-016.png)

#### Settings rows (UIF-COMP-017)

The horizontal settings row — a filling text column beside a shrinking control.

![Settings rows](doc/img/settings-rows-uif-comp-017.png)

#### Menu rows (UIF-FMENU / UIF-CMENU)

The row grid that fixed menus and context menus share, across its state matrix — one set of
rows, whether the menu was opened from a navbar trigger or at the pointer.

![Menu rows](doc/img/menu-rows-uif-fmenu-uif-cmenu.png)

## Resolver palettes

These sections have no live widget behind them. Several framework pieces are
resolver-only — the pure `(variant × state × palette) → style` function exists and is
audited, but nothing paints it into a control a user can touch — so each card paints what
the resolver actually returned, naming every channel, and does not respond to hover, press
or focus.

#### Chip resolver (UIF-ATOM-006)

The chip resolver's paint for assist, filter and input chips across polarity, height and
base state. A removable chip reserves its 24×24px remove target whether or not the `x` is
wanted, so the chip is never a different width from the one it will be when the target is
used.

![Chip resolver](doc/img/chip-resolver-uif-atom-006.png)

#### Badge resolver (UIF-ATOM-008)

The badge resolver's paint for the dot, the count pill and the status chip.

![Badge resolver](doc/img/badge-resolver-uif-atom-008.png)

#### Progress resolver (UIF-ATOM-009)

The progress bar and spinner resolvers across their determinate and indeterminate modes and
statuses.

![Progress resolver](doc/img/progress-resolver-uif-atom-009.png)

#### Slider resolver (UIF-ATOM-010)

The slider resolver's track, thumb and focus-ring paint, per variant and state.

![Slider resolver](doc/img/slider-resolver-uif-atom-010.png)

#### Number box resolver (UIF-ATOM-011)

The number box resolver's paint for the entry and its stepper tiers.

![Number box resolver](doc/img/number-box-resolver-uif-atom-011.png)

#### Card resolver (UIF-COMP-005)

The card container's resolved surface and elevation per variant.

![Card resolver](doc/img/card-resolver-uif-comp-005.png)

#### List tile resolver (UIF-COMP-006)

The three-slot list tile's resolved paint — leading mark, filling text, shrinking trailing
targets — with the slot geometry computed here rather than left to whatever lays the row
out, so two hosts cannot end up with two different lists.

![List tile resolver](doc/img/list-tile-resolver-uif-comp-006.png)

#### Table resolver (UIF-COMP-011)

The structured grid's resolved cell, header and column paint. Selection is keyed by row
identity rather than position, so sorting a column cannot change what is selected, and the
sticky header is laid out from the same column numbers as the body.

![Table resolver](doc/img/table-resolver-uif-comp-011.png)

#### Toast / notification resolver (UIF-COMP-009)

The resolved paint behind the transient notification stack and its severity vocabulary.

![Toast notification resolver](doc/img/toast-notification-resolver-uif-comp-009.png)

#### Breadcrumb resolver (UIF-COMP-004)

The breadcrumb trail's crumb-elision solver and resolved paint: a trail that does not fit
collapses its middle crumbs behind an ellipsis, dropping the ones furthest from where the
user currently is.

![Breadcrumb resolver](doc/img/breadcrumb-resolver-uif-comp-004.png)

#### Overflow toolbar resolver (UIF-COMP-008)

The responsive overflow toolbar's resolved paint as items collapse into overflow — actions
move rather than disappear when the window narrows.

![Overflow toolbar resolver](doc/img/overflow-toolbar-resolver-uif-comp-008.png)

---

`hoarfrost` is not on crates.io yet, so add it from this repository.

Regenerate the images with `cargo run -p hoarfrost-gallery -- <directory>`, and see every
section live with `cargo run -p hoarfrost-demo`.
