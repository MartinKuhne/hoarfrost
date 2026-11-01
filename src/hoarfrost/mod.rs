//! A reusable iced widget library implementing the UI framework
//! specification.
//!
//! This crate realises the design specification in
//! `spec.md` at this crate's root. Requirement IDs of the form `UIF-*` in that
//! document map to the code in this crate; each module cites the IDs it
//! implements in its doc comments.
//!
//! The core rule is Perceptual Harmony Through Tokenization (`UIF-PRIN-001`):
//! every visual property — colour, elevation, spacing, sizing, border radius,
//! stroke weight and typography — is selected from a closed set of semantic
//! design tokens. Raw numeric literals in individual widget definitions are
//! forbidden; widgets resolve their style through the token modules only.
//!
//! Since the INT series (`doc/planning/hoarfrost-tasks.md` §3c) the host
//! application consumes this crate as its UI framework: the tokens, atoms,
//! composites, menu engine, modal frames, movable hosts, z-order band model
//! and geometry solvers are live in `src/app/ui_iced/` (see the integration
//! contract in `doc/technical-context/ARCHITECTURE_C4.md`).
//!
//! The ownership boundary is the one `spec.md` §5 states ("Presentation vs.
//! Lifecycle Ownership Split") and decisions D-2 / D-8 record: this crate
//! supplies sizing, frame and semantics; the host application owns surface
//! lifecycle (open / close / dismiss), keeps the single writer to
//! `SurfaceRegistry`, and injects all persistence. No host type appears
//! in this API — the dependency points one way.

pub mod a11y;
pub mod binding;
pub mod bridge;
pub mod composite;
pub mod focus;
#[cfg(feature = "gallery")]
pub mod gallery;
#[cfg(feature = "gallery")]
pub mod gallery_audit;
#[cfg(feature = "gallery")]
pub mod gallery_badge;
#[cfg(feature = "gallery")]
pub mod gallery_breadcrumb;
#[cfg(feature = "gallery")]
pub mod gallery_button;
#[cfg(feature = "gallery")]
pub mod gallery_chip;
#[cfg(feature = "gallery")]
pub mod gallery_combo;
#[cfg(feature = "gallery")]
pub mod gallery_composite;
#[cfg(feature = "gallery")]
pub mod gallery_editor;
#[cfg(feature = "gallery")]
pub mod gallery_hyperlink;
#[cfg(feature = "gallery")]
pub mod gallery_input;
#[cfg(feature = "gallery")]
pub mod gallery_menu;
#[cfg(feature = "gallery")]
pub mod gallery_modal;
#[cfg(feature = "gallery")]
pub mod gallery_movable;
#[cfg(feature = "gallery")]
pub mod gallery_popover;
#[cfg(feature = "gallery")]
pub mod gallery_progress;
#[cfg(feature = "gallery")]
pub mod gallery_range;
#[cfg(feature = "gallery")]
pub mod gallery_secret;
#[cfg(feature = "gallery")]
pub mod gallery_segmented;
#[cfg(feature = "gallery")]
pub mod gallery_split_button;
#[cfg(feature = "gallery")]
pub mod gallery_suggest;
#[cfg(feature = "gallery")]
pub mod gallery_table;
#[cfg(feature = "gallery")]
pub mod gallery_toggle;
#[cfg(feature = "gallery")]
pub mod gallery_tooltip;
#[cfg(feature = "gallery")]
pub mod gallery_tree;
#[cfg(feature = "gallery")]
pub mod gallery_warm;
pub mod geometry;
pub mod identity;
pub mod invariant_gate;
pub mod layout;
pub mod microcopy;
#[cfg(feature = "gallery")]
pub mod spec_coverage;
pub mod state;
pub mod strings;
pub mod surface;
pub mod token;
pub mod token_lint;
pub mod validation;
pub mod widget;

#[cfg(any(
    feature = "empty-state",
    feature = "grid-view",
    feature = "navigation-rail"
))]
pub use composite::Icon;
#[cfg(feature = "code-block")]
pub use composite::{CodeBlock, CodeBlockModel, code_block};
#[cfg(feature = "command-palette")]
pub use composite::{
    CommandPalette, CommandPaletteModel, EscapeDisposition, PaletteGroup, PaletteItem, PaletteMode,
    command_palette,
};
#[cfg(feature = "diff-viewer")]
pub use composite::{
    DiffDisplayMode, DiffHunk, DiffLine, DiffLineKind, DiffViewer, DiffViewerModel, diff_viewer,
};
#[cfg(feature = "empty-state")]
pub use composite::{EmptyState, EmptyStateKind, EmptyStateModel, EmptyStateStyle, empty_state};
#[cfg(feature = "grid-view")]
pub use composite::{GridNavKey, GridTile, GridView, GridViewModel, grid_view};
#[cfg(feature = "keybind-recorder")]
pub use composite::{KeyChord, KeybindRecorder, KeybindRecorderModel, keybind_recorder};
#[cfg(feature = "navigation-rail")]
pub use composite::{NavDestination, NavigationRail, NavigationRailModel, navigation_rail};
#[cfg(feature = "pagination")]
pub use composite::{PaginationBar, PaginationModel, pagination_bar};
#[cfg(feature = "pips-pager")]
pub use composite::{PipsPager, PipsPagerModel, pips_pager};
#[cfg(feature = "side-sheet")]
pub use composite::{
    SheetMode, SheetPlacement, SideSheet, SideSheetModel, SideSheetResize, SideSheetState,
    side_sheet,
};
#[cfg(feature = "skeleton")]
pub use composite::{Skeleton, SkeletonStyle, skeleton};
#[cfg(feature = "split-divider")]
pub use composite::{
    SplitAxis, SplitDivider, SplitDividerModel, SplitDividerResize, SplitDividerState,
    SplitSnapshot, split_divider,
};
#[cfg(feature = "stepper")]
pub use composite::{
    StepItem, StepStatus, Stepper, StepperModel, StepperOrientation, WizardFooterConfig, stepper,
    wizard_footer,
};
pub use geometry::{
    CENTER_PANE_MIN_W, LEFT_PANE_MIN_W, MemoryWindowStorage, NoopWindowStorage, RIGHT_PANE_MIN_W,
    SolvedWindowLayout, UiScale, WINDOW_DEFAULT_HEIGHT, WINDOW_DEFAULT_WIDTH, WINDOW_MIN_HEIGHT,
    WINDOW_MIN_WIDTH, WindowGeometry, WindowStorage, auto_fit_pane_width, solve_clamped_pane_width,
    solve_window_layout,
};
#[cfg(feature = "time-picker")]
pub use widget::{AmPm, CivilTime, TimePicker, TimePickerModel, TimeSegment, time_picker};
#[cfg(feature = "avatar")]
pub use widget::{Avatar, AvatarModel, AvatarShape, AvatarSize, avatar};
#[cfg(feature = "date-picker")]
pub use widget::{
    CalendarView, CalendarViewMode, CivilDate, DatePicker, DatePickerModel, DateRangePicker,
    DateSelection, Weekday, YearMonth, calendar_view, date_picker, date_range_picker,
};
#[cfg(feature = "color-picker")]
pub use widget::{
    ColorPicker, ColorPickerModel, ColorSwatchButton, Hsva, Rgba8, color_picker,
    color_swatch_button,
};
#[cfg(feature = "combo-box")]
pub use widget::{
    ComboBox, ComboBoxModel, ComboRung, MIN_WIDTH_COMPACT, Select, SelectOption, combo_box, select,
};
#[cfg(feature = "rating")]
pub use widget::{RatingControl, RatingModel, RatingStep, rating_control};

#[cfg(all(test, feature = "all"))]
pub mod checkbox_ink_probe;
#[cfg(all(test, feature = "all"))]
pub mod geometry_gallery;
/// Helpers shared by this crate's test sidecars. Test-only; see the module doc.
#[cfg(test)]
pub(crate) mod test_support;
// Downstream crates get the same headless capture harness this crate's own
// tests run on, by turning on the `test-helpers` feature. Under `test` it
// is here during tests, and downstream crates can request it explicitly.
// See the `[features]` note in `Cargo.toml` for why the flag pulls in nothing.
#[cfg(any(test, feature = "test-helpers"))]
pub mod test_helpers;
