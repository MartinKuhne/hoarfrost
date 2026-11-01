# hoarfrost

A reusable [iced](https://github.com/iced-rs/iced) widget library implementing the UI
framework specification in `src/hoarfrost/spec.md`. The `UIF-*` requirement IDs in that
document map to the code, and every image below is rendered by `hoarfrost-gallery` — one
PNG per demo section, each card showing one widget across its states at 2× scale.

## Widgets

### Atoms

#### Buttons

Four button variants — Primary, Secondary, Danger and Ghost — across the resolved
interaction state matrix.

![Buttons](doc/img/buttons.png)

#### Checkbox

The Boolean tick control, one of three toggle-family controls alongside the radio group
and the toggle switch.

![Checkbox](doc/img/checkbox.png)

#### Radio group

A group of mutually exclusive choices that share a single roving focus walk.

![Radio group](doc/img/radio-group.png)

#### Toggle switch

The on/off switch, the third member of the toggle family, shown live, busy and disabled.

![Toggle switch](doc/img/toggle-switch.png)

#### Segmented

A joined strip of mutually exclusive segments — single selection drawn as one continuous
bar.

![Segmented](doc/img/segmented.png)

#### Split button

The two-zone control: a primary action beside a separate zone that opens its menu.

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

A secret-entry field whose value refuses to print itself, and whose reveal policy keeps a
shown secret inside the one control allowed to show it.

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

The dual-thumb slider, with the invariants that keep the lower thumb below the upper.

![Range slider](doc/img/range-slider.png)

#### Avatar

The avatar and person-picture widget for representing a person.

![Avatar](doc/img/avatar.png)

#### Date picker

The date picker and the date range picker, each opening a calendar view.

![Date picker](doc/img/date-picker.png)

#### Time picker

The segmented time picker, with civil-time arithmetic and a drum flyout.

![Time picker](doc/img/time-picker.png)

#### Colour picker

The colour picker surface and its standalone swatch-button trigger.

![Colour picker](doc/img/colour-picker.png)

#### Tooltip

The framework-owned tooltip, shown on hover and on keyboard focus alike.

![Tooltip](doc/img/tooltip.png)

### Composites

#### Tab strip (UIF-TAB-003)

The tab strip, with dirty-close guards, pointer bindings and most-recently-used focus
handoff.

![Tab strip](doc/img/tab-strip-uif-tab-003.png)

#### Navigation rail (UIF-COMP-009)

The collapsible navigation rail for shell-level navigation.

![Navigation rail](doc/img/navigation-rail-uif-comp-009.png)

#### Stepper + wizard footer (UIF-COMP-011)

The multi-step workflow header and the wizard footer that drives it.

![Stepper and wizard footer](doc/img/stepper-wizard-footer-uif-comp-011.png)

#### Pagination bar (UIF-COMP-012)

The pagination footer bar for paged content.

![Pagination bar](doc/img/pagination-bar-uif-comp-012.png)

#### Pips pager (UIF-COMP-013)

Compact pagination as carousel dots.

![Pips pager](doc/img/pips-pager-uif-comp-013.png)

#### Code block (UIF-COMP-015)

The code block container and syntax viewer.

![Code block](doc/img/code-block-uif-comp-015.png)

#### Diff viewer (UIF-COMP-015)

The visual diff viewer, in unified and side-by-side modes.

![Diff viewer](doc/img/diff-viewer-uif-comp-015.png)

#### Empty states (UIF-COMP-014)

The empty-state placeholder, which holds its height instead of collapsing.

![Empty states](doc/img/empty-states-uif-comp-014.png)

#### Grid view (UIF-COMP-013)

The two-dimensional virtualized tile and card grid.

![Grid view](doc/img/grid-view-uif-comp-013.png)

#### Keybind recorder (UIF-COMP-015)

The keyboard shortcut recorder and key-chord interceptor.

![Keybind recorder](doc/img/keybind-recorder-uif-comp-015.png)

#### Skeleton (UIF-COMP-015)

The skeleton placeholder and shimmer shown while content loads.

![Skeleton](doc/img/skeleton-uif-comp-015.png)

#### Side sheet (UIF-COMP-012)

The docked inspector drawer.

![Side sheet](doc/img/side-sheet-uif-comp-012.png)

#### Command palette (UIF-COMP-010)

The command palette and its modal search view.

![Command palette](doc/img/command-palette-uif-comp-010.png)

#### Form field (UIF-COMP-016)

The stacked form field — label, control, and a caption slot that is always reserved.

![Form field](doc/img/form-field-uif-comp-016.png)

#### Settings rows (UIF-COMP-017)

The horizontal settings row — a filling text column beside a shrinking control.

![Settings rows](doc/img/settings-rows-uif-comp-017.png)

#### Menu rows (UIF-FMENU / UIF-CMENU)

The row grid that fixed menus and context menus share, across its state matrix.

![Menu rows](doc/img/menu-rows-uif-fmenu-uif-cmenu.png)

## Resolver palettes

These sections have no live widget behind them. Several framework pieces are
resolver-only — the pure `(variant × state × palette) → style` function exists and is
audited, but nothing paints it into a control a user can touch — so each card paints what
the resolver actually returned, naming every channel, and does not respond to hover, press
or focus.

#### Chip resolver (UIF-ATOM-006)

The chip resolver's paint for assist, filter and input chips across polarity, height and
base state.

![Chip resolver](doc/img/chip-resolver-uif-atom-006.png)

#### Badge resolver (UIF-ATOM-008)

The badge resolver's paint for the dot, the count pill and the status chip.

![Badge resolver](doc/img/badge-resolver-uif-atom-008.png)

#### Progress resolver (UIF-ATOM-009)

The progress bar and spinner resolvers across their modes and statuses.

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

The three-slot list tile's resolved paint — leading mark, filling text, shrinking
trailing targets.

![List tile resolver](doc/img/list-tile-resolver-uif-comp-006.png)

#### Table resolver (UIF-COMP-011)

The structured grid's resolved cell, header and column paint.

![Table resolver](doc/img/table-resolver-uif-comp-011.png)

#### Toast / notification resolver (UIF-COMP-009)

The resolved paint behind the transient notification stack and its severity vocabulary.

![Toast notification resolver](doc/img/toast-notification-resolver-uif-comp-009.png)

#### Breadcrumb resolver (UIF-COMP-004)

The breadcrumb trail's crumb-elision solver and resolved paint.

![Breadcrumb resolver](doc/img/breadcrumb-resolver-uif-comp-004.png)

#### Overflow toolbar resolver (UIF-COMP-008)

The responsive overflow toolbar's resolved paint as items collapse into overflow.

![Overflow toolbar resolver](doc/img/overflow-toolbar-resolver-uif-comp-008.png)

---

Regenerate the images with `cargo run -p hoarfrost-gallery -- <directory>`.
