# Hoarfrost

Hoarfrost is a set of widgets for use with the iced framework. I crated this because I wanted
a set of consistently sized and styled widgets that adhere to well established user interface
design rules, without having to become an expert in said design rules myself.

Specfication AND code written by AI.

## Choose features in `Cargo.toml`

The default feature set is empty, so a plain dependency provides the core
(tokens, state, geometry, identity, strings, focus policy, bindings,
validation, and layout rules), but no widget modules:

```toml
[dependencies]
hoarfrost = "0.14"
```

Select one widget family or combine several.

```toml
[dependencies]
hoarfrost = { version = "0.14", default-features = false, features = [
    "button",
    "text-input",
] }
```

Use `all` to enable the whole public framework.

## Atom features

| Feature | Widget API enabled | Also enables |
|---|---|---|
| `avatar` | Avatar (`widget::avatar`) | `badge` |
| `badge` | Badge (`widget::badge`) | — |
| `button` | Button and Progress (`widget::button`, `widget::progress`) | `tooltip` |
| `chip` | Chip (`widget::chip`) | — |
| `color-picker` | Color picker and color swatch button (`widget::color_picker`) | — |
| `combo-box` | ComboBox and Select (`widget::combo_box`) | `menu`, `text-input` |
| `date-picker` | Date picker, date-range picker, and calendar view (`widget::date_picker`) | — |
| `hyperlink` | Hyperlink (`widget::hyperlink`) | `button` (and its dependencies) |
| `number-box` | Number box (`widget::number_box`) | `button`, `range-model`, `text-input` |
| `password` | Password / secret input (`widget::password_host`, `widget::password_model`) | `text-input` |
| `progress` | Progress (`widget::progress`) | `button`; this selects the same shared cluster as `button` |
| `range-slider` | Range slider (`widget::range_slider`) | `range-model` |
| `rating` | Rating control (`widget::rating`) | — |
| `search-box` | Search box (`widget::search_box`) | — |
| `segmented` | Segmented control (`widget::segmented`) | `button` (and its dependencies) |
| `slider` | Slider (`widget::slider`) | `button`, `range-model` |
| `split-button` | Split button (`widget::split_button`) | `button`, `menu` |
| `text-area` | Text area (`widget::text_area_host`, `widget::text_area_model`) | `text-input` |
| `text-input` | Text input (`widget::text_input`) | `button` (and its dependencies) |
| `time-picker` | Time picker (`widget::time_picker`) | — |
| `toggle` | Checkbox, radio group, and toggle switch (`widget::toggle_widget`) | `button` (and its dependencies) |
| `auto-suggest` | Auto-suggest (`widget::auto_suggest_host`, `widget::auto_suggest_model`) | `menu`, `text-input`, `tooltip` |

## Composite features

| Feature | Widget API enabled | Also enables |
|---|---|---|
| `breadcrumb` | Breadcrumb (`composite::breadcrumb`) | `button`, `menu` |
| `card` | Card (`composite::card`) | `button` |
| `code-block` | Code block (`composite::code_block`) | — |
| `command-palette` | Command palette (`composite::command_palette`) | `text-input` |
| `diff-viewer` | Diff viewer (`composite::diff_viewer`) | — |
| `empty-state` | Empty state (`composite::empty_state`) | `toast` |
| `expander` | Expander (`composite::expander`) | `button` |
| `form-field` | Form field (`composite::form_field`) | `button` |
| `grid-view` | Grid view (`composite::grid_view`) | `tree-view` |
| `info-bar` | Info bar (`composite::info_bar`) | `button`, `notification` |
| `keybind-recorder` | Keybind recorder (`composite::keybind_recorder`) | — |
| `list-tile` | List tile (`composite::list_tile`) | `button` |
| `navbar` | Navigation bar (`composite::navbar`) | `tooltip` |
| `navigation-rail` | Navigation rail (`composite::navigation_rail`) | `badge`, `range-model`, `tooltip` |
| `notification` | Notification model (`composite::notification`) | — |
| `overflow-toolbar` | Overflow toolbar (`composite::overflow_toolbar`) | `button`, `menu` |
| `pagination` | Pagination bar (`composite::pagination`) | — |
| `pips-pager` | Pips pager (`composite::pips_pager`) | — |
| `scroll-view` | Scroll view (`composite::scroll_view`) | — |
| `settings-row` | Settings row (`composite::settings_row`) | `button` |
| `side-sheet` | Side sheet (`composite::side_sheet`) | `zorder` |
| `skeleton` | Skeleton (`composite::skeleton`) | — |
| `split-divider` | Split divider (`composite::split_divider`) | `button` |
| `stepper` | Stepper (`composite::stepper`) | — |
| `tab-strip` | Tab strip (`composite::tab_strip`) | `menu`, `tooltip` |
| `table` | Table (`composite::table_cell`, `composite::table_column`, `composite::table_header`, `composite::table_view`) | `tree-view`, `virtual-list` |
| `toast` | Toast (`composite::toast`) | `info-bar`, `notification`, `tooltip`, `zorder` |
| `tree-row` | Tree row (`composite::tree_row`) | `button`, `list-tile`, `virtual-list` |
| `tree-view` | Tree view (`composite::tree_view`) | `virtual-list` |
| `virtual-list` | Virtual list (`composite::virtual_list`) | `scroll-view` |

## Surface features

| Feature | Surface API enabled | Also enables |
|---|---|---|
| `menu` | Menu model, view, cascade, and row actions (`surface::menu`) | `button`, `tooltip` |
| `modal` | Modal surface (`surface::modal`, `surface::modal_tier`) | `zorder` |
| `dialog` | Input dialog and dialog buttons (`surface::input_dialog`, `surface::dialog_button`) | `button`, `zorder` |
| `movable` | Movable surface and drag tier (`surface::movable`, `surface::drag_tier`) | `zorder` |
| `popover` | Popover surface (`surface::popover`) | `zorder` |
| `tooltip` | Tooltip surface (`surface::tooltip`) | — |
| `zorder` | Shared surface ordering (`surface::zorder`) | `menu` |

Surface features can have transitive widget dependencies. For example,
`modal` enables `zorder`, which enables `menu`, which enables `button` and
`tooltip`. These dependencies are intentional; consumers only need to name
the feature for the surface they want.

## Shared and convenience features

| Feature | What it enables | When to select it |
|---|---|---|
| `range-model` | Shared range / interval model (`widget::range_model`); no standalone widget | Normally leave it to `range-slider`, `number-box`, `slider`, or `navigation-rail` to enable |
| `all` | Every public atom, composite, and surface listed above | Select the complete Hoarfrost widget framework |
| `gallery` | `all` plus the crate's gallery modules and `gallery` binary | Build or use the bundled widget gallery |
| `test-helpers` | Public headless widget test harness (`test_helpers`) | Use Hoarfrost's capture harness in downstream tests; it does not enable widgets |

The demo and gallery workspace crates enable `all` and `test-helpers` so they
can render the complete showcase. For normal applications, select only the
widget features you use. To confirm the current feature names and dependency
relationships, see the `[features]` section of
[`hoarfrost/Cargo.toml`](hoarfrost/Cargo.toml).
