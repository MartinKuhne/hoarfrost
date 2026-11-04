//! The RIGHT pane — the composite widgets.
//!
//! Mirror image of [`crate::pane_atoms`]: every composite gets its own
//! [`Section`] built by a small pure function, and the pane is nothing but
//! the ordered list of those builders.
//!
//! # Where the state lives
//!
//! The composites in this group are all **controlled** — each one borrows a
//! host-owned model every frame — so those models live in
//! [`CompositesState`], which [`crate::app::DemoApp`] holds. Every
//! callback folds back through [`CompositesState::apply`]; nothing is
//! persisted and nothing leaves the process.
//!
//! S5 covers the navigation family: tab strip, navigation rail, stepper,
//! pagination bar and pips pager.
//!
//! # The wizard owns its status ladder
//!
//! The step indicator paints from [`StepItem::status`], never from
//! `StepperModel::current_step`, and the framework offers nothing that
//! keeps the two in sync. So the host does it: every wizard transition
//! here rewrites the ladder through `statuses_for`, which is also what
//! makes `linear` navigation semantics real — `can_select` only lets a
//! step through once the ladder has actually marked it `Completed`.
//!
//! Unit tests live in the sibling `pane_composites_tests.rs` sidecar.

use std::collections::BTreeSet;

use iced::widget::Column;
use iced::{Border, Color, Element, Length, Shadow};

use hoarfrost::composite::code_block::{CodeBlock, CodeBlockModel};
use hoarfrost::composite::command_palette::{CommandPalette, CommandPaletteModel};
use hoarfrost::composite::diff_viewer::{DiffDisplayMode, DiffViewer, DiffViewerModel};
use hoarfrost::composite::empty_state::empty_state;
use hoarfrost::composite::form_field;
use hoarfrost::composite::grid_view::{GridView, GridViewModel};
use hoarfrost::composite::keybind_recorder::{KeyChord, KeybindRecorder, KeybindRecorderModel};
use hoarfrost::composite::navigation_rail::{NavigationRail, NavigationRailModel};
use hoarfrost::composite::pagination::{PaginationModel, pagination_bar};
use hoarfrost::composite::pips_pager::{PipsPagerModel, pips_pager};
use hoarfrost::composite::settings_row;
use hoarfrost::composite::side_sheet::{SheetMode, SideSheet, SideSheetModel};
use hoarfrost::composite::skeleton::skeleton;
use hoarfrost::composite::stepper::{
    StepItem, StepStatus, Stepper, StepperModel, StepperOrientation, WizardFooterConfig,
    wizard_footer,
};
use hoarfrost::composite::tab_strip::{
    TabCloseVerdict, TabItem, TabStrip, TabStripModel, tab_context_menu,
};
use hoarfrost::surface::menu::model::Menu;
use hoarfrost::surface::menu::view::MenuRow;
use hoarfrost::token::color::Palette;
use hoarfrost::token::{shape, space, typography};

use crate::app::Message;
use crate::data;
use crate::section::{PaneBody, Section, row_id};

/// Stable scroll id for the right pane, so its scroll position survives a rebuild.
pub const PANE_SCROLL_ID: &str = "demo.composites.scroll";

/// Prefix of every declared settings-row container id.
///
/// See [`settings_rows_section`] for why the row, and not its title text,
/// is what the demo is able to name.
const SETTINGS_ROW_ID_PREFIX: &str = "demo.composites.setting-row.";

/// Prefix of every rendered menu-row id.
///
/// The final segment is the row's own key — declared in
/// [`data::MENU_ROWS`] and carried on the `Row` itself, separators
/// included. See [`menu_section`].
const MENU_ROW_ID_PREFIX: &str = "demo.composites.menu.";

/// Width the grid section lays its tiles out against.
///
/// Fixed rather than measured because the demo has no live viewport to ask,
/// and a grid needs a concrete width to solve its column count.
pub const GRID_AVAILABLE_WIDTH: f32 = 520.0;

/// Host height given to the side sheet so its `Length::Fill` resolves.
///
/// A docked sheet fills its host row; the demo supplies a fixed row so the
/// sheet is visible inside the scrolling pane without going infinite.
pub const SHEET_HOST_HEIGHT: f32 = 260.0;

/// Host height given to the navigation rail, for the same reason.
pub const RAIL_HOST_HEIGHT: f32 = 280.0;

/// Host height given to the command palette, for the same reason.
///
/// Slightly taller than the spec's 380px card so the scrim band around it
/// is visible and the card is not clipped.
pub const PALETTE_HOST_HEIGHT: f32 = 420.0;

/// Title of the dirty tab close confirmation guard card (`UIF-TAB-003`, `UIF-MOD-002`).
pub const GUARD_TITLE: &str = "Unsaved Changes Guard (UIF-TAB-003)";

/// Button label to cancel dirty tab closure (`UIF-TAB-003`).
pub const LABEL_CANCEL: &str = "Cancel";

/// Button label to discard unsaved edits and close a dirty tab (`UIF-TAB-003`).
pub const LABEL_DISCARD_CLOSE: &str = "Discard & Close";

/// Button label to dismiss an open context menu card (`UIF-TAB-001`).
pub const LABEL_DISMISS: &str = "Dismiss";

/// Button label to append dynamic tabs demonstrating horizontal overflow (`UIF-TAB-002`).
pub const LABEL_ADD_OVERFLOW: &str = "+ Add Tab (Overflow)";

/// Button label to simulate external file deletion synchronization (`UIF-TAB-003`).
pub const LABEL_EXTERNAL_DELETE: &str = "Simulate External Delete";

/// Button label to reset tabs to default fixtures (`UIF-TAB-001`, `UIF-TAB-003`).
pub const LABEL_RESET_TABS: &str = "Reset Tabs";

/// Interactive state for every live composite in the right pane.
///
/// Owned by `DemoApp` because the hoarfrost composites borrow their models,
/// so the models have to outlive the frame — exactly like
/// [`crate::pane_atoms::AtomsState`].
#[derive(Debug, Clone)]
pub struct CompositesState {
    /// Open document tabs, including the MRU activation history (`UIF-TAB-001`, `UIF-TAB-003`).
    pub tabs: TabStripModel,
    /// The tab whose close is waiting on a confirming click (`UIF-TAB-003`, `UIF-MOD-002`).
    pub pending_close: Option<String>,
    /// Queue of dirty tab IDs awaiting close confirmation when batch closing (`UIF-TAB-003`).
    pub pending_close_queue: Vec<String>,
    /// Target tab id whose right-click context menu is currently open (`UIF-TAB-001`, §4.9.1).
    pub tab_context_menu_target: Option<String>,
    /// Last action executed from the tab strip or its context menu (`UIF-TAB-001`).
    pub last_tab_action: Option<String>,
    /// Counter for dynamically generated overflow demonstration tabs (`UIF-TAB-002`).
    pub extra_tab_count: usize,
    /// Rail destinations, which one is selected, and whether it is expanded.
    pub rail: NavigationRailModel<String>,
    /// Wizard steps and the current one.
    pub stepper: StepperModel<String>,
    /// Page, page size and total item count of the paginated list.
    pub pagination: PaginationModel,
    /// Page and page count of the pips pager.
    pub pips: PipsPagerModel,
    /// The code block's content and copy state.
    pub code: CodeBlockModel,
    /// The diff hunks and the unified / side-by-side mode.
    pub diff: DiffViewerModel<String>,
    /// The grid tiles and the current selection.
    pub grid: GridViewModel<String>,
    /// The recorded key chord and whether the recorder is listening.
    pub keybind: KeybindRecorderModel,
    /// The side sheet's mode, placement, width and open flag.
    pub sheet: SideSheetModel,
    /// The command palette's groups, query and open flag.
    pub palette: CommandPaletteModel<String>,
    /// Whether the demo form field currently validates.
    pub form_valid: bool,
    /// The three settings-row toggles, in [`data::SETTINGS_ROWS`] order.
    pub settings: Vec<bool>,
    /// The menu tree, whose checkable and radio rows carry their own state.
    pub menu: Menu,
    /// The last empty-state recovery action the user pressed.
    pub last_empty_action: Option<String>,
}

/// Everything the right pane reports back to the application.
#[derive(Debug, Clone)]
pub enum CompositesMessage {
    /// A tab was activated (`UIF-TAB-001`).
    TabSelected(String),
    /// A tab's close affordance was used (may need confirmation first) (`UIF-TAB-003`).
    TabCloseRequested(String),
    /// Right-click on a tab requested its context menu (`UIF-TAB-001`).
    TabContextMenu(String),
    /// An action from the tab context menu was executed (`UIF-TAB-001`).
    TabMenuAction(String, String),
    /// The open tab context menu was dismissed (`UIF-TAB-001`).
    TabMenuDismissed,
    /// Dynamically add a document tab to demonstrate horizontal overflow (`UIF-TAB-002`).
    TabAddOverflow,
    /// Synchronize external file deletion closing tab and transferring focus to MRU (`UIF-TAB-003`).
    TabExternalDelete,
    /// Reset tab strip back to fixture defaults (`UIF-TAB-001`, `UIF-TAB-003`).
    TabReset,
    /// Confirm discarding edits and closing the dirty tab (`UIF-TAB-003`, `UIF-MOD-002`).
    TabConfirmDiscard,
    /// Cancel discarding edits and dismiss close confirmation (`UIF-TAB-003`, `UIF-MOD-002`).
    TabCancelDiscard,
    /// A rail destination was selected.
    RailSelected(String),
    /// The rail was collapsed or expanded.
    RailExpanded(bool),
    /// A step was clicked directly.
    StepSelected(usize),
    /// The wizard footer's "Next" control.
    StepNext,
    /// The wizard footer's "Back" control.
    StepPrevious,
    /// A page was selected in the pagination bar.
    PageChanged(usize),
    /// The page size was changed in the pagination bar.
    PageSizeChanged(usize),
    /// A pip was selected in the pips pager.
    PipsPageChanged(usize),
    /// The code block's copy button was used.
    CodeCopied(String),
    /// A diff hunk was accepted.
    DiffHunkAccepted(String),
    /// A diff hunk was rejected.
    DiffHunkRejected(String),
    /// The diff viewer switched between unified and side-by-side.
    DiffModeChanged(DiffDisplayMode),
    /// The grid selection changed.
    GridSelectionChanged(BTreeSet<String>),
    /// A grid tile was activated (double-click / Enter).
    GridActivated(String),
    /// The keybind recorder committed a chord.
    KeybindCommitted(Option<KeyChord>),
    /// The keybind recorder cancelled its capture.
    KeybindCancelled,
    /// The side sheet switched between docked and overlay.
    SheetModeToggled(SheetMode),
    /// The side sheet asked to be closed.
    SheetClosed,
    /// The command palette's query text changed.
    PaletteQueryChanged(String),
    /// A command palette item was executed.
    PaletteExecuted(String),
    /// The command palette was dismissed.
    PaletteDismissed,
    /// The form field's validity was flipped.
    FormValidityToggled(bool),
    /// A settings row toggle changed.
    SettingToggled(usize, bool),
    /// A menu row was activated.
    MenuRowActivated(String),
    /// An empty-state recovery action was pressed.
    EmptyAction(String),
    /// A composite event that owns no state of its own.
    Note(String),
}

impl CompositesState {
    /// Build the right-pane state from the hardcoded fixtures.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tabs: data::tab_strip_model(),
            pending_close: None,
            pending_close_queue: Vec::new(),
            tab_context_menu_target: None,
            last_tab_action: None,
            extra_tab_count: 0,
            rail: data::rail_model(),
            stepper: data::stepper_model(),
            pagination: data::pagination_model(),
            pips: data::pips_model(),
            code: CodeBlockModel::new(data::CODE_SAMPLE).with_language(data::CODE_LANGUAGE),
            diff: data::diff_model(),
            grid: data::grid_model(),
            keybind: data::keybind_model(),
            sheet: data::sheet_model(),
            palette: data::palette_model(),
            form_valid: true,
            settings: data::SETTINGS_ROWS.iter().map(|_| false).collect(),
            menu: data::menu_model(),
            last_empty_action: None,
        }
    }

    /// Number of interactive widgets the right pane currently renders.
    ///
    /// Delegates to [`composites_pane`] so the status bar can never drift
    /// from what is actually on screen, previews included.
    #[must_use]
    pub fn widget_count(&self) -> usize {
        composites_pane(self).widgets
    }

    /// Fold a right-pane message into the underlying hoarfrost models.
    pub fn apply(&mut self, message: &CompositesMessage) {
        match message {
            CompositesMessage::TabSelected(id) => self.tabs.select_tab(id),
            CompositesMessage::TabCloseRequested(id) => self.close_tab(id),
            CompositesMessage::TabContextMenu(id) => {
                self.tab_context_menu_target = Some(id.clone());
                self.last_tab_action = Some(format!("Context menu opened for '{id}'"));
            }
            CompositesMessage::TabMenuAction(action, id) => {
                match action.as_str() {
                    "close" => {
                        self.close_tab(id);
                    }
                    "close_others" => {
                        let other_ids: Vec<String> = self
                            .tabs
                            .tabs
                            .iter()
                            .filter(|t| &t.id != id && !t.is_pinned)
                            .map(|t| t.id.clone())
                            .collect();
                        for other_id in other_ids {
                            let is_dirty = self
                                .tabs
                                .tabs
                                .iter()
                                .find(|t| t.id == other_id)
                                .is_some_and(|t| t.is_dirty);
                            if is_dirty {
                                if self.pending_close.is_none() {
                                    self.pending_close = Some(other_id);
                                } else {
                                    self.pending_close_queue.push(other_id);
                                }
                            } else {
                                self.tabs.confirm_close(&other_id);
                            }
                        }
                        if self.tabs.tabs.iter().any(|t| &t.id == id) {
                            self.tabs.select_tab(id);
                        }
                        self.last_tab_action = Some(format!("Closed other tabs, keeping '{id}'"));
                    }
                    "close_all" => {
                        let cand_ids: Vec<String> = self
                            .tabs
                            .tabs
                            .iter()
                            .filter(|t| !t.is_pinned)
                            .map(|t| t.id.clone())
                            .collect();
                        for cand_id in cand_ids {
                            let is_dirty = self
                                .tabs
                                .tabs
                                .iter()
                                .find(|t| t.id == cand_id)
                                .is_some_and(|t| t.is_dirty);
                            if is_dirty {
                                if self.pending_close.is_none() {
                                    self.pending_close = Some(cand_id);
                                } else {
                                    self.pending_close_queue.push(cand_id);
                                }
                            } else {
                                self.tabs.confirm_close(&cand_id);
                            }
                        }
                        self.last_tab_action = Some("Close all tabs requested".to_string());
                    }
                    _ => {
                        self.last_tab_action = Some(format!("Action '{action}' on {id}"));
                    }
                }
                self.tab_context_menu_target = None;
            }
            CompositesMessage::TabMenuDismissed => {
                self.tab_context_menu_target = None;
                self.last_tab_action = Some("Context menu dismissed".to_string());
            }
            CompositesMessage::TabAddOverflow => {
                self.extra_tab_count += 1;
                let new_id = format!("tab-extra-{}", self.extra_tab_count);
                let new_title = format!("extra-{}.rs", self.extra_tab_count);
                let new_tab = TabItem::new(&new_id, &new_title)
                    .with_tooltip(format!("Dynamic tab: {new_title}"))
                    .with_dirty(false);
                self.tabs.open_tab(new_tab);
                self.last_tab_action = Some(format!("Added overflow tab '{new_title}'"));
            }
            CompositesMessage::TabExternalDelete => {
                let target = if self.tabs.tabs.iter().any(|t| t.id == "tab-todo") {
                    "tab-todo".to_string()
                } else if let Some(active) = &self.tabs.active_id {
                    active.clone()
                } else if let Some(first) = self.tabs.tabs.first() {
                    first.id.clone()
                } else {
                    "tab-todo".to_string()
                };
                if self.pending_close.as_deref() == Some(&target) {
                    self.pending_close = if self.pending_close_queue.is_empty() {
                        None
                    } else {
                        Some(self.pending_close_queue.remove(0))
                    };
                }
                self.pending_close_queue.retain(|id| id != &target);
                self.tabs.external_deletion_sync(&target);
                self.last_tab_action = Some(format!("External deletion sync for '{target}'"));
            }
            CompositesMessage::TabReset => {
                self.tabs = data::tab_strip_model();
                self.pending_close = None;
                self.pending_close_queue.clear();
                self.tab_context_menu_target = None;
                self.last_tab_action = Some("Tabs reset to default fixtures".to_string());
            }
            CompositesMessage::TabConfirmDiscard => {
                if let Some(id) = self.pending_close.take() {
                    self.tabs.confirm_close(&id);
                    self.last_tab_action = Some(format!("Discarded edits and closed '{id}'"));
                }
                if !self.pending_close_queue.is_empty() {
                    self.pending_close = Some(self.pending_close_queue.remove(0));
                }
            }
            CompositesMessage::TabCancelDiscard => {
                if let Some(id) = self.pending_close.take() {
                    self.last_tab_action = Some(format!("Cancelled close for '{id}'"));
                }
                if !self.pending_close_queue.is_empty() {
                    self.pending_close = Some(self.pending_close_queue.remove(0));
                }
            }
            CompositesMessage::RailSelected(id) => self.rail.selected = id.clone(),
            CompositesMessage::RailExpanded(expanded) => self.rail.expanded = *expanded,
            CompositesMessage::StepSelected(index) => self.select_step(*index),
            CompositesMessage::StepNext => self.step_by(1),
            CompositesMessage::StepPrevious => self.step_by(-1),
            CompositesMessage::PageChanged(page) => {
                let last = self.pagination.total_pages().max(1);
                self.pagination.page = (*page).clamp(1, last);
            }
            CompositesMessage::PageSizeChanged(size) => {
                self.pagination.page_size = *size;
                let last = self.pagination.total_pages().max(1);
                self.pagination.page = self.pagination.page.min(last);
            }
            CompositesMessage::PipsPageChanged(page) => {
                let last = self.pips.total_pages.max(1);
                self.pips.page = (*page).clamp(1, last);
            }
            CompositesMessage::Note(_) => {}
            CompositesMessage::CodeCopied(_) => self.code.mark_copied(),
            CompositesMessage::DiffHunkAccepted(id) => self.diff.hunks.retain(|h| &h.id != id),
            CompositesMessage::DiffHunkRejected(id) => self.diff.hunks.retain(|h| &h.id != id),
            CompositesMessage::DiffModeChanged(mode) => self.diff.mode = *mode,
            CompositesMessage::GridSelectionChanged(ids) => self.grid.selected_ids = ids.clone(),
            CompositesMessage::GridActivated(_) => {}
            CompositesMessage::KeybindCommitted(chord) => {
                self.keybind.chord = chord.clone();
                self.keybind.listening = false;
            }
            CompositesMessage::KeybindCancelled => self.keybind.listening = false,
            CompositesMessage::SheetModeToggled(mode) => self.sheet.mode = *mode,
            CompositesMessage::SheetClosed => self.sheet.open = false,
            CompositesMessage::PaletteQueryChanged(query) => {
                self.palette.query = query.clone();
            }
            CompositesMessage::PaletteExecuted(_) => self.palette.open = false,
            CompositesMessage::PaletteDismissed => self.palette.open = false,
            CompositesMessage::FormValidityToggled(valid) => self.form_valid = *valid,
            CompositesMessage::SettingToggled(index, value) => {
                if let Some(slot) = self.settings.get_mut(*index) {
                    *slot = *value;
                }
            }
            CompositesMessage::MenuRowActivated(key) => {
                if let Some(index) = self
                    .menu
                    .rows()
                    .iter()
                    .position(|row| row.key() == key.as_str())
                {
                    self.menu.activate(index);
                }
            }
            CompositesMessage::EmptyAction(id) => self.last_empty_action = Some(id.clone()),
        }
    }

    /// One-line description of the state produced by `message`, for the status bar.
    ///
    /// Read *after* [`apply`](Self::apply) so the line always describes what is
    /// now on screen rather than what was merely requested.
    pub fn describe(&self, message: &CompositesMessage) -> String {
        match message {
            CompositesMessage::TabSelected(id) => describe_tab_selected(&self.tabs, id),
            CompositesMessage::TabCloseRequested(id) => self.describe_close(id),
            CompositesMessage::TabContextMenu(id) => {
                format!("Tab context menu requested: {id}")
            }
            CompositesMessage::TabMenuAction(action, id) => {
                format!("Tab menu action '{action}' executed on {id}")
            }
            CompositesMessage::TabMenuDismissed => "Tab context menu dismissed".to_string(),
            CompositesMessage::TabAddOverflow => format!(
                "Added tab (overflow demonstration, {} tabs total)",
                self.tabs.tabs.len()
            ),
            CompositesMessage::TabExternalDelete => format!(
                "External deletion sync triggered (active: {:?})",
                self.tabs.active_id
            ),
            CompositesMessage::TabReset => "Tab strip reset to default".to_string(),
            CompositesMessage::TabConfirmDiscard => "Discarded edits and closed tab".to_string(),
            CompositesMessage::TabCancelDiscard => "Cancelled close of dirty tab".to_string(),
            CompositesMessage::RailSelected(id) => describe_rail_selected(&self.rail, id),
            CompositesMessage::RailExpanded(expanded) => {
                let mode = if *expanded { "expanded" } else { "collapsed" };
                format!("Navigation rail {mode}")
            }
            CompositesMessage::StepSelected(index) => {
                let label = self
                    .stepper
                    .steps
                    .get(*index)
                    .map(|step| step.label.clone())
                    .unwrap_or_else(|| format!("#{index}"));
                format!("Step selected: {label}")
            }
            CompositesMessage::StepNext => describe_step(&self.stepper, "Next"),
            CompositesMessage::StepPrevious => describe_step(&self.stepper, "Back"),
            CompositesMessage::PageChanged(_) => format!(
                "Page {} of {} ({} per page)",
                self.pagination.page,
                self.pagination.total_pages().max(1),
                self.pagination.page_size
            ),
            CompositesMessage::PageSizeChanged(size) => format!(
                "Page size {size} - {} pages",
                self.pagination.total_pages().max(1)
            ),
            CompositesMessage::PipsPageChanged(_) => {
                format!("Pips page {} of {}", self.pips.page, self.pips.total_pages)
            }
            CompositesMessage::Note(note) => note.clone(),
            CompositesMessage::CodeCopied(language) => {
                format!("Code copied ({})", language.as_str())
            }
            CompositesMessage::DiffHunkAccepted(id) => {
                format!("Hunk '{id}' accepted - {} remain", self.diff.hunks.len())
            }
            CompositesMessage::DiffHunkRejected(id) => {
                format!("Hunk '{id}' rejected - {} remain", self.diff.hunks.len())
            }
            CompositesMessage::DiffModeChanged(mode) => match mode {
                DiffDisplayMode::Unified => "Diff mode: unified".to_string(),
                DiffDisplayMode::SideBySide => "Diff mode: side by side".to_string(),
            },
            CompositesMessage::GridSelectionChanged(ids) => {
                format!("Grid: {} tile(s) selected", ids.len())
            }
            CompositesMessage::GridActivated(id) => format!("Grid tile activated: {id}"),
            CompositesMessage::KeybindCommitted(chord) => match chord {
                Some(chord) => format!("Keybind set to {}", chord.format()),
                None => "Keybind cleared".to_string(),
            },
            CompositesMessage::KeybindCancelled => "Keybind capture cancelled".to_string(),
            CompositesMessage::SheetModeToggled(mode) => match mode {
                SheetMode::Docked => "Side sheet docked".to_string(),
                SheetMode::ModelessOverlay => "Side sheet floating as overlay".to_string(),
                SheetMode::ModalScrim => "Side sheet modal over scrim".to_string(),
            },
            CompositesMessage::SheetClosed => "Side sheet closed".to_string(),
            CompositesMessage::PaletteQueryChanged(query) => {
                format!("Palette query: \"{query}\"")
            }
            CompositesMessage::PaletteExecuted(id) => format!("Command executed: {id}"),
            CompositesMessage::PaletteDismissed => "Palette dismissed".to_string(),
            CompositesMessage::FormValidityToggled(valid) => {
                let state = if *valid { "valid" } else { "invalid" };
                format!("Form field is now {state}")
            }
            CompositesMessage::SettingToggled(index, value) => {
                let name = data::SETTINGS_ROWS
                    .get(*index)
                    .map(|row| row.0)
                    .unwrap_or("setting");
                let state = if *value { "on" } else { "off" };
                format!("{name} {state}")
            }
            CompositesMessage::MenuRowActivated(key) => format!("Menu row: {key}"),
            CompositesMessage::EmptyAction(id) => format!("Empty-state action: {id}"),
        }
        .to_string()
    }

    /// Describe the outcome of a close request from wherever the handshake got to.
    fn describe_close(&self, id: &str) -> String {
        let is_pinned = self
            .tabs
            .tabs
            .iter()
            .find(|tab| tab.id == id)
            .is_some_and(|tab| tab.is_pinned);
        if is_pinned {
            return format!("Cannot close pinned tab '{id}'");
        }

        let title = self
            .tabs
            .tabs
            .iter()
            .find(|tab| tab.id == id)
            .map(|tab| tab.title.clone());
        if self.pending_close.as_deref() == Some(id) {
            let name = title.unwrap_or_else(|| id.to_string());
            return format!("Confirmation required: '{name}' has unsaved changes");
        }
        match title {
            Some(name) => format!("Close requested: '{name}'"),
            None => format!("Closed '{id}'"),
        }
    }

    /// Run the close handshake for one tab (`UIF-TAB-003`).
    ///
    /// A dirty tab asks for confirmation first; the *same* request arriving
    /// while a confirmation is pending is the user confirming.
    fn close_tab(&mut self, id: &str) {
        if self.pending_close.as_deref() == Some(id) {
            self.tabs.confirm_close(id);
            self.pending_close = if self.pending_close_queue.is_empty() {
                None
            } else {
                Some(self.pending_close_queue.remove(0))
            };
            return;
        }

        match self.tabs.request_close(id) {
            TabCloseVerdict::RequiresConfirmation { .. } => {
                self.pending_close = Some(id.to_string());
            }
            TabCloseVerdict::RefusedPinned { .. } => {
                self.last_tab_action = Some(format!("Tab '{id}' is pinned and cannot be closed"));
            }
            TabCloseVerdict::CloseImmediately { .. } => {}
        }
    }

    /// Jump the wizard straight to the step at `index`, wherever it sits.
    ///
    /// The ladder is rewritten for the new position exactly as
    /// [`step_by`](Self::step_by) does it — only the *choice* of target
    /// differs. Out-of-range indices are dropped rather than clamped: the
    /// widget only ever reports a step it actually drew.
    fn select_step(&mut self, index: usize) {
        if self.stepper.steps.get(index).is_none() {
            return;
        }
        self.stepper.current_step = self.stepper.steps[index].id.clone();
        sync_stepper(&mut self.stepper, index);
    }

    /// Move the wizard by `delta` steps, clamped to the ends.
    ///
    /// The status ladder is rewritten on the way through
    /// ([`sync_stepper`]) because the indicator paints from
    /// [`StepItem::status`], never from `current_step` — moving only the
    /// latter changes the model and repaints nothing.
    fn step_by(&mut self, delta: i32) {
        let Some(index) = landed_step(&self.stepper, delta) else {
            return;
        };
        self.stepper.current_step = self.stepper.steps[index].id.clone();
        sync_stepper(&mut self.stepper, index);
    }
}

/// The index the wizard lands on after moving `delta` from where it
/// currently sits, clamped to the ends of the sequence.
///
/// `None` when the current step is not in the list, or the list is empty,
/// which is the caller's cue to leave the model alone.
#[must_use]
fn landed_step(model: &StepperModel<String>, delta: i32) -> Option<usize> {
    let current = model.current_index()?;
    let last = model.steps.len().checked_sub(1)?;
    let clamped = i32::try_from(current)
        .ok()?
        .saturating_add(delta)
        .clamp(0, i32::try_from(last).unwrap_or(i32::MAX));
    usize::try_from(clamped).ok()
}

/// Recomputes the wizard's status ladder for a run parked at `current`.
///
/// Standard wizard semantics (`UIF-COMP-011`): every step *before* the
/// current one reads `Completed`, the step under the cursor reads
/// `Current`, and everything *after* it reads `Pending`.
///
/// **`Error` is sticky, and is released only by re-entering the step that
/// owns it.** This transition is pure navigation — nothing in it has
/// validated anything — so laundering a failed step into `Completed`
/// because the user pressed Next from a neighbour would put a success
/// claim on screen that no check ever earned, and dropping it to
/// `Pending` would hide the failure outright. Keeping the badge in both
/// directions until the user comes back is the honest reading: the error
/// stays visible while they are elsewhere, and re-entering is exactly the
/// moment they get to fix it. The step then reads `Current` like any
/// other arrival, and its next exit is derived from wherever it lands.
#[must_use]
fn statuses_for(current: usize, existing: &[StepItem<String>]) -> Vec<StepStatus> {
    existing
        .iter()
        .enumerate()
        .map(|(index, step)| {
            if index == current {
                StepStatus::Current
            } else if step.status == StepStatus::Error {
                StepStatus::Error
            } else if index < current {
                StepStatus::Completed
            } else {
                StepStatus::Pending
            }
        })
        .collect()
}

/// Rewrite every step of `model` to the ladder for `current`.
fn sync_stepper(model: &mut StepperModel<String>, current: usize) {
    let statuses = statuses_for(current, &model.steps);
    for (step, status) in model.steps.iter_mut().zip(statuses) {
        step.status = status;
    }
}

impl Default for CompositesState {
    fn default() -> Self {
        Self::new()
    }
}

fn describe_tab_selected(model: &TabStripModel, id: &str) -> String {
    let tab = model.tabs.iter().find(|tab| tab.id == id);
    let title = tab
        .map(|tab| tab.title.clone())
        .unwrap_or_else(|| id.to_string());
    let marker = tab.is_some_and(|tab| tab.is_dirty);
    let suffix = if marker { " (unsaved)" } else { "" };
    format!("Tab active: '{title}'{suffix}")
}

fn describe_rail_selected(model: &NavigationRailModel<String>, id: &str) -> String {
    let label = model
        .top_items
        .iter()
        .chain(model.bottom_items.iter())
        .find(|destination| destination.id == id)
        .map(|destination| destination.label.clone())
        .unwrap_or_else(|| id.to_string());
    format!("Rail destination: {label}")
}

fn describe_step(model: &StepperModel<String>, action: &str) -> String {
    let label = model
        .steps
        .iter()
        .find(|step| step.id == model.current_step)
        .map(|step| step.label.clone())
        .unwrap_or_else(|| "?".to_string());
    let position = model
        .current_index()
        .map(|index| format!("{} of {}", index + 1, model.steps.len()))
        .unwrap_or_else(|| "unpositioned".to_string());
    format!("{action} - step '{label}' ({position})")
}

/// The S5 sections, in display order.
#[must_use]
pub fn group1_sections(state: &CompositesState) -> Vec<Section<'_>> {
    vec![
        tab_strip_section(state),
        navigation_rail_section(state),
        stepper_section(state),
        pagination_section(state),
        pips_pager_section(state),
    ]
}

/// Builds the whole right pane: live sections then resolver previews.
#[must_use]
pub fn composites_pane(state: &CompositesState) -> PaneBody<'_> {
    let mut sections = group1_sections(state);
    sections.extend(group2_sections(state));
    sections.extend(crate::previews::composite_preview_sections());
    PaneBody::new(sections)
}

/// A muted caption line under a section's widgets.
///
/// Returned as an [`Element`] rather than a `Text` because iced's `Text` is
/// *invariant* in its lifetime: a `Text<'static>` pushed into a `Column<'a>`
/// would demand `'static == 'a` and refuse to compile.
#[must_use]
fn note<'a>(text: impl Into<String>) -> Element<'a, Message> {
    crate::section::rung(text, typography::CAPTION, Palette::dark().text_muted).into()
}

// ---------------------------------------------------------------------------
// Tab strip (UIF-TAB-001, UIF-TAB-002, UIF-TAB-003)
// ---------------------------------------------------------------------------

/// A compact styled button adhering to token colors for normal and danger variants.
fn action_button<'a>(
    label: &'static str,
    message: Message,
    palette: Palette,
    is_danger: bool,
) -> Element<'a, Message> {
    iced::widget::button(crate::section::rung(
        label,
        typography::CAPTION,
        if is_danger {
            palette.status_danger_text
        } else {
            palette.text_primary
        },
    ))
    .on_press(message)
    .padding(
        iced::Padding::new(0.0)
            .horizontal(space::SM)
            .vertical(space::XXS),
    )
    .style(move |_theme, status| {
        let is_hovered = matches!(status, iced::widget::button::Status::Hovered);
        let is_pressed = matches!(status, iced::widget::button::Status::Pressed);
        let (bg, text_color, border_color) = if is_danger {
            if is_pressed {
                (
                    palette.status_danger_fill_active,
                    palette.text_strong,
                    palette.status_danger_fill_active,
                )
            } else if is_hovered {
                (
                    palette.status_danger_fill,
                    palette.text_strong,
                    palette.text_strong,
                )
            } else {
                (
                    palette.surface_raised,
                    palette.status_danger_text,
                    palette.status_danger_text,
                )
            }
        } else if is_pressed {
            (
                palette.surface_active,
                palette.text_strong,
                palette.accent_border,
            )
        } else if is_hovered {
            (
                palette.surface_hover,
                palette.text_strong,
                palette.border_strong,
            )
        } else {
            (
                palette.surface_raised,
                palette.text_primary,
                palette.border_subtle,
            )
        };

        iced::widget::button::Style {
            background: Some(bg.into()),
            text_color,
            border: Border::default()
                .color(border_color)
                .width(shape::BORDER_HAIRLINE)
                .rounded(shape::RADIUS_WIDGET),
            shadow: Shadow::default(),
            snap: true,
        }
    })
    .into()
}

/// Active tab content viewport displaying document details and status indicators (`UIF-PRIN-006`).
fn active_tab_viewport<'a>(state: &CompositesState, palette: Palette) -> Element<'a, Message> {
    let content: Element<'a, Message> = if let Some(active_id) = &state.tabs.active_id {
        if let Some(tab) = state.tabs.tabs.iter().find(|t| &t.id == active_id) {
            let status_badge = if tab.is_dirty {
                crate::section::rung(
                    "[Dirty: Unsaved Changes]",
                    typography::CAPTION,
                    palette.status_warning,
                )
            } else if tab.is_pinned {
                crate::section::rung("[Pinned]", typography::CAPTION, palette.accent_text)
            } else {
                crate::section::rung("[Clean]", typography::CAPTION, palette.text_muted)
            };

            let header = iced::widget::Row::new()
                .spacing(space::SM)
                .align_y(iced::Alignment::Center)
                .push(crate::section::rung(
                    format!("{} ({})", tab.title, tab.id),
                    typography::BODY,
                    palette.text_strong,
                ))
                .push(status_badge);

            let excerpt = match tab.id.as_str() {
                "tab-report" => {
                    "Executive summary: All quarterly targets have been achieved with high system reliability."
                }
                "tab-notes" => {
                    "Engineering sync: Unsaved architectural revisions for tab strip dynamic overflow handling."
                }
                "tab-todo" => {
                    "Action items: Verify MRU activation handoff and external file deletion synchronization."
                }
                "tab-archive" => {
                    "Historical log: Pinned archival documentation preserved against accidental closure."
                }
                _ => {
                    "Dynamic buffer: Generated tab content demonstrating overflow scrolling and viewport bridging."
                }
            };

            let body_preview =
                crate::section::rung(excerpt, typography::CAPTION, palette.text_primary);

            Column::new()
                .spacing(space::XXS)
                .push(header)
                .push(body_preview)
                .into()
        } else {
            crate::section::rung(
                "Active document not found in open tabs.",
                typography::CAPTION,
                palette.text_muted,
            )
            .into()
        }
    } else {
        crate::section::rung(
            "No active tab selected.",
            typography::CAPTION,
            palette.text_muted,
        )
        .into()
    };

    iced::widget::container(content)
        .width(Length::Fill)
        .padding(space::SM)
        .style(move |_theme| iced::widget::container::Style {
            background: Some(palette.surface_overlay.into()),
            border: Border::default()
                .color(palette.border_subtle)
                .width(shape::BORDER_HAIRLINE)
                .rounded(shape::RADIUS_WIDGET),
            shadow: Shadow::default(),
            snap: true,
            text_color: Some(palette.text_primary),
        })
        .into()
}

/// Unsaved changes confirmation guard card (`UIF-TAB-003`, `UIF-MOD-002`).
fn dirty_close_guard_card<'a>(id: &str, palette: Palette) -> Element<'a, Message> {
    let warning_title =
        crate::section::rung(GUARD_TITLE, typography::TITLE_SM, palette.status_warning);

    let body = crate::section::rung(
        format!("Document '{id}' has unsaved changes. Discard edits and close?"),
        typography::BODY,
        palette.text_primary,
    );

    let cancel_btn = action_button(
        LABEL_CANCEL,
        Message::Composites(CompositesMessage::TabCancelDiscard),
        palette,
        false,
    );

    let discard_btn = action_button(
        LABEL_DISCARD_CLOSE,
        Message::Composites(CompositesMessage::TabConfirmDiscard),
        palette,
        true,
    );

    let actions = iced::widget::Row::new()
        .spacing(space::SM)
        .push(cancel_btn)
        .push(discard_btn);

    let card_content = Column::new()
        .spacing(space::XS)
        .push(warning_title)
        .push(body)
        .push(actions);

    iced::widget::container(card_content)
        .width(Length::Fill)
        .padding(space::SM)
        .style(move |_theme| iced::widget::container::Style {
            background: Some(palette.surface_raised.into()),
            border: Border::default()
                .color(palette.border_strong)
                .width(shape::BORDER_HAIRLINE)
                .rounded(shape::RADIUS_CARD),
            shadow: Shadow::default(),
            snap: true,
            text_color: Some(palette.text_primary),
        })
        .into()
}

/// Tab context menu card displaying standardized tab actions (`UIF-TAB-001`, §4.9.1).
fn tab_context_menu_card<'a>(target_id: &str, palette: Palette) -> Element<'a, Message> {
    let header_title = crate::section::rung(
        format!("Context Menu: {target_id}"),
        typography::TITLE_SM,
        palette.text_strong,
    );

    let dismiss_btn = action_button(
        LABEL_DISMISS,
        Message::Composites(CompositesMessage::TabMenuDismissed),
        palette,
        false,
    );

    let header_row = iced::widget::Row::new()
        .spacing(space::SM)
        .align_y(iced::Alignment::Center)
        .push(header_title)
        .push(iced::widget::Space::new().width(Length::Fill))
        .push(dismiss_btn);

    let menu = tab_context_menu(target_id);
    let mut menu_column = Column::new().spacing(space::XXS);

    for row in menu.rows() {
        let key = row.key().to_string();
        if row.is_separator() {
            let sep = iced::widget::container(iced::widget::Space::new())
                .height(Length::Fixed(shape::BORDER_HAIRLINE))
                .width(Length::Fill)
                .style(move |_theme| iced::widget::container::Style {
                    background: Some(palette.border_subtle.into()),
                    ..Default::default()
                });
            menu_column = menu_column.push(sep);
        } else {
            let action_key = key.clone();
            let tid = target_id.to_string();
            let label = row.label().to_string();
            let is_destructive = row.is_destructive();

            let btn = iced::widget::button(crate::section::rung(
                label,
                typography::BODY,
                if is_destructive {
                    palette.status_danger_text
                } else {
                    palette.text_primary
                },
            ))
            .on_press(Message::Composites(CompositesMessage::TabMenuAction(
                action_key, tid,
            )))
            .width(Length::Fill)
            .padding(
                iced::Padding::new(0.0)
                    .horizontal(space::SM)
                    .vertical(space::XXS),
            )
            .style(move |_theme, status| {
                let is_hovered = matches!(status, iced::widget::button::Status::Hovered);
                let (bg, text_color) = if is_destructive {
                    if is_hovered {
                        (palette.status_danger_fill, palette.text_strong)
                    } else {
                        (Color::TRANSPARENT, palette.status_danger_text)
                    }
                } else if is_hovered {
                    (palette.surface_hover, palette.text_strong)
                } else {
                    (Color::TRANSPARENT, palette.text_primary)
                };

                iced::widget::button::Style {
                    background: Some(bg.into()),
                    text_color,
                    border: Border::default().rounded(shape::RADIUS_WIDGET),
                    shadow: Shadow::default(),
                    snap: true,
                }
            });
            menu_column = menu_column.push(btn);
        }
    }

    let card_content = Column::new()
        .spacing(space::XS)
        .push(header_row)
        .push(menu_column);

    iced::widget::container(card_content)
        .width(Length::Fill)
        .padding(space::SM)
        .style(move |_theme| iced::widget::container::Style {
            background: Some(palette.surface_raised.into()),
            border: Border::default()
                .color(palette.border_strong)
                .width(shape::BORDER_HAIRLINE)
                .rounded(shape::RADIUS_CARD),
            shadow: Shadow::default(),
            snap: true,
            text_color: Some(palette.text_primary),
        })
        .into()
}

/// Scenario action toolbar for tab overflow, deletion and reset demonstrations (`UIF-TAB-002`, `UIF-TAB-003`).
fn scenario_toolbar<'a>(palette: Palette) -> Element<'a, Message> {
    let add_btn = action_button(
        LABEL_ADD_OVERFLOW,
        Message::Composites(CompositesMessage::TabAddOverflow),
        palette,
        false,
    );
    let del_btn = action_button(
        LABEL_EXTERNAL_DELETE,
        Message::Composites(CompositesMessage::TabExternalDelete),
        palette,
        false,
    );
    let reset_btn = action_button(
        LABEL_RESET_TABS,
        Message::Composites(CompositesMessage::TabReset),
        palette,
        false,
    );

    iced::widget::Row::new()
        .spacing(space::XS)
        .align_y(iced::Alignment::Center)
        .push(add_btn)
        .push(del_btn)
        .push(reset_btn)
        .into()
}

/// Four tabs: two pinned, one active, one carrying unsaved changes (`UIF-TAB-001`, `UIF-TAB-002`, `UIF-TAB-003`).
#[must_use]
pub fn tab_strip_section(state: &CompositesState) -> Section<'_> {
    let palette = Palette::dark();
    let strip = TabStrip::new(&state.tabs)
        .with_palette(palette)
        .on_select_tab(|id| Message::Composites(CompositesMessage::TabSelected(id)))
        .on_close_tab(|id| Message::Composites(CompositesMessage::TabCloseRequested(id)))
        .on_context_menu(|id, _| Message::Composites(CompositesMessage::TabContextMenu(id)));

    let mut body = Column::new()
        .width(Length::Fill)
        .spacing(space::SM)
        .push(strip)
        .push(active_tab_viewport(state, palette));

    if let Some(ref pending_id) = state.pending_close {
        body = body.push(dirty_close_guard_card(pending_id, palette));
    }

    if let Some(ref target_id) = state.tab_context_menu_target {
        body = body.push(tab_context_menu_card(target_id, palette));
    }

    body = body.push(scenario_toolbar(palette));

    let info_text = if let Some(last_action) = &state.last_tab_action {
        format!("{last_action} · {} tabs open", state.tabs.tabs.len())
    } else {
        format!(
            "{} tabs open · right-click for context menu · middle-click or '×' to close",
            state.tabs.tabs.len()
        )
    };
    body = body.push(note(info_text));

    crate::section::section("Tab strip (UIF-TAB-003)", 1, body)
}

// ---------------------------------------------------------------------------
// Navigation rail (UIF-COMP-009)
// ---------------------------------------------------------------------------

/// A collapsible rail: three destinations on top, two pinned at the bottom.
#[must_use]
pub fn navigation_rail_section(state: &CompositesState) -> Section<'_> {
    let rail = NavigationRail::new(state.rail.clone())
        .palette(Palette::dark())
        .on_select(|commit| {
            Message::Composites(CompositesMessage::RailSelected(commit.value().clone()))
        })
        .on_expand_toggle(|commit| {
            Message::Composites(CompositesMessage::RailExpanded(*commit.value()))
        });

    // The rail takes `limits.max().height` verbatim
    // (`navigation_rail.rs:592`), so inside a scrollable's unbounded main
    // axis it grows to infinity and pushes every following sibling's y to
    // infinity with it. Bound the host the way a real app's rail column
    // would. See BUGS.md BUG-01.
    let host = iced::widget::container(rail).height(Length::Fixed(RAIL_HOST_HEIGHT));

    let body = Column::new()
        .width(Length::Fill)
        .spacing(space::XS)
        .push(host)
        .push(note(format!(
            "Mode: {} · expanded width {:.0}",
            if state.rail.expanded {
                "expanded"
            } else {
                "compact"
            },
            state.rail.expanded_width
        )));

    crate::section::section("Navigation rail (UIF-COMP-009)", 1, body)
}

// ---------------------------------------------------------------------------
// Stepper (UIF-COMP-011)
// ---------------------------------------------------------------------------

/// A four-step wizard: the step indicator plus its footer.
///
/// Counts as two widgets because the footer is a second, independently
/// interactive control.
#[must_use]
pub fn stepper_section(state: &CompositesState) -> Section<'_> {
    let lookup = state.stepper.clone();

    let stepper = Stepper::new(state.stepper.clone())
        .orientation(StepperOrientation::Horizontal)
        .on_step_select(move |commit| {
            let index = lookup
                .steps
                .iter()
                .position(|step| step.id == *commit.value())
                .unwrap_or(0);
            Message::Composites(CompositesMessage::StepSelected(index))
        });

    let footer = wizard_footer(
        &state.stepper,
        WizardFooterConfig::new()
            .on_previous(|_| Message::Composites(CompositesMessage::StepPrevious))
            .on_next(|_| Message::Composites(CompositesMessage::StepNext)),
    );

    let body = Column::new()
        .width(Length::Fill)
        .spacing(space::SM)
        .push(stepper)
        .push(footer);

    crate::section::section("Stepper + wizard footer (UIF-COMP-011)", 2, body)
}

// ---------------------------------------------------------------------------
// Pagination bar (UIF-COMP-012)
// ---------------------------------------------------------------------------

/// 480 items at 25 per page, with a 10 / 25 / 50 / 100 page-size selector.
#[must_use]
pub fn pagination_section(state: &CompositesState) -> Section<'_> {
    let bar = pagination_bar(
        state.pagination.clone(),
        |commit| Message::Composites(CompositesMessage::PageChanged(*commit.value())),
        |commit| Message::Composites(CompositesMessage::PageSizeChanged(*commit.value())),
    );

    let body = Column::new()
        .width(Length::Fill)
        .spacing(space::XS)
        .push(bar)
        .push(note(format!(
            "{} items · page {} of {}",
            state.pagination.total_items,
            state.pagination.page,
            state.pagination.total_pages()
        )));

    crate::section::section("Pagination bar (UIF-COMP-012)", 1, body)
}

// ---------------------------------------------------------------------------
// Pips pager (UIF-COMP-013)
// ---------------------------------------------------------------------------

/// A seven-dot pager for carousel-style navigation.
#[must_use]
pub fn pips_pager_section(state: &CompositesState) -> Section<'_> {
    let pager = pips_pager(state.pips, |commit| {
        Message::Composites(CompositesMessage::PipsPageChanged(*commit.value()))
    });

    let body = Column::new()
        .width(Length::Fill)
        .spacing(space::XS)
        .push(pager)
        .push(note(format!(
            "Page {} of {}",
            state.pips.page, state.pips.total_pages
        )));

    crate::section::section("Pips pager (UIF-COMP-013)", 1, body)
}

// ---------------------------------------------------------------------------
// S6 — right pane group 2
// ---------------------------------------------------------------------------

/// The S6 sections, in display order.
#[must_use]
pub fn group2_sections(state: &CompositesState) -> Vec<Section<'_>> {
    vec![
        code_block_section(state),
        diff_viewer_section(state),
        empty_state_section(state),
        grid_view_section(state),
        keybind_section(state),
        skeleton_section(),
        side_sheet_section(state),
        command_palette_section(state),
        form_field_section(state),
        settings_rows_section(state),
        menu_section(state),
    ]
}

/// The code block: a rust listing with a copy affordance.
#[must_use]
pub fn code_block_section(state: &CompositesState) -> Section<'_> {
    let block = CodeBlock::new(state.code.clone()).on_copy(|commit| {
        Message::Composites(CompositesMessage::CodeCopied(commit.value().clone()))
    });

    let copied = if state.code.is_copied() {
        "Copy state: active"
    } else {
        "Copy state: idle"
    };

    let body = Column::new()
        .width(Length::Fill)
        .spacing(space::XS)
        .push(block)
        .push(note(copied));

    crate::section::section("Code block (UIF-COMP-015)", 1, body)
}

/// The diff viewer, unified by default with a mode switch.
#[must_use]
pub fn diff_viewer_section(state: &CompositesState) -> Section<'_> {
    let viewer = DiffViewer::new(state.diff.clone())
        .on_accept_hunk(|commit| {
            Message::Composites(CompositesMessage::DiffHunkAccepted(commit.value().clone()))
        })
        .on_reject_hunk(|commit| {
            Message::Composites(CompositesMessage::DiffHunkRejected(commit.value().clone()))
        })
        .on_mode_change(|commit| {
            Message::Composites(CompositesMessage::DiffModeChanged(*commit.value()))
        });

    let body = Column::new()
        .width(Length::Fill)
        .spacing(space::XS)
        .push(viewer)
        .push(note(format!(
            "{} hunk(s) · mode {:?}",
            state.diff.hunks.len(),
            state.diff.mode
        )));

    crate::section::section("Diff viewer (UIF-COMP-015)", 1, body)
}

/// Three empty states: no items, no matches, load error.
#[must_use]
pub fn empty_state_section(state: &CompositesState) -> Section<'_> {
    let mut rows = Column::new().width(Length::Fill).spacing(space::SM);
    for model in data::empty_state_models() {
        rows = rows.push(empty_state(model, |commit| {
            Message::Composites(CompositesMessage::EmptyAction(commit.value().clone()))
        }));
    }

    let last = match &state.last_empty_action {
        Some(id) => format!("Last recovery action: {id}"),
        None => "No recovery action pressed yet.".to_string(),
    };
    let rows = rows.push(note(last));

    crate::section::section("Empty states (UIF-COMP-014)", 3, rows)
}

/// A six-tile grid laid out against a fixed available width.
#[must_use]
pub fn grid_view_section(state: &CompositesState) -> Section<'_> {
    let grid = GridView::new(state.grid.clone(), GRID_AVAILABLE_WIDTH)
        .on_selection_change(|commit| {
            Message::Composites(CompositesMessage::GridSelectionChanged(
                commit.value().clone(),
            ))
        })
        .on_activate(|commit| {
            Message::Composites(CompositesMessage::GridActivated(commit.value().clone()))
        });

    let body = Column::new()
        .width(Length::Fill)
        .spacing(space::XS)
        .push(grid)
        .push(note(format!(
            "{} tiles · {} selected · {:.0}px available",
            state.grid.items.len(),
            state.grid.selected_ids.len(),
            GRID_AVAILABLE_WIDTH
        )));

    crate::section::section("Grid view (UIF-COMP-013)", 1, body)
}

/// The keybind recorder, showing the fixture chord.
#[must_use]
pub fn keybind_section(state: &CompositesState) -> Section<'_> {
    let recorder = KeybindRecorder::new(state.keybind.clone())
        .on_chord_commit(|commit| {
            Message::Composites(CompositesMessage::KeybindCommitted(commit.value().clone()))
        })
        .on_cancel(|_cancel| Message::Composites(CompositesMessage::KeybindCancelled));

    let shown = state
        .keybind
        .chord
        .as_ref()
        .map(|chord| chord.format())
        .unwrap_or_else(|| "<none>".to_string());

    let body = Column::new()
        .width(Length::Fill)
        .spacing(space::XS)
        .push(recorder)
        .push(note(format!("Recorded chord: {shown}")));

    crate::section::section("Keybind recorder (UIF-COMP-015)", 1, body)
}

/// Loading placeholders. Non-interactive by definition, so the section
/// counts zero widgets.
#[must_use]
pub fn skeleton_section() -> Section<'static> {
    let row = iced::widget::Row::new()
        .spacing(space::SM)
        .align_y(iced::Alignment::Center)
        .push(skeleton(160.0, 16.0))
        .push(skeleton(96.0, 16.0))
        .push(skeleton(128.0, 16.0));

    let body = Column::new()
        .width(Length::Fill)
        .spacing(space::XS)
        .push(row)
        .push(skeleton(320.0, 48.0))
        .push(note(
            "Non-interactive loading placeholder — 0 interactive widgets.",
        ));

    crate::section::section("Skeleton (UIF-COMP-015)", 0, body)
}

/// The side sheet, docked inline so the demo never covers its own panes.
#[must_use]
pub fn side_sheet_section(state: &CompositesState) -> Section<'_> {
    let content = Column::new()
        .width(Length::Fill)
        .spacing(space::XS)
        .push(note(data::SHEET_BODY))
        .push(note(format!(
            "open={} dirty={} width={:.0}",
            state.sheet.open, state.sheet.dirty, state.sheet.width
        )));

    let sheet = SideSheet::new(state.sheet.clone(), content)
        .on_mode_toggle(|commit| {
            Message::Composites(CompositesMessage::SheetModeToggled(*commit.value()))
        })
        .on_close_request(|_cancel| Message::Composites(CompositesMessage::SheetClosed));

    // The sheet declares `Length::Fill` for its height (`side_sheet.rs:534`),
    // which is correct for the bounded host row it is designed to sit in.
    // This pane is a `scrollable`, whose main axis is unbounded, so an
    // un-wrapped `Fill` resolves to infinity and poisons every following
    // sibling's y. Give the sheet the bounded host height a real dock
    // would supply. See BUGS.md BUG-01.
    let host = iced::widget::container(sheet).height(Length::Fixed(SHEET_HOST_HEIGHT));

    crate::section::section("Side sheet (UIF-COMP-012)", 1, Column::new().push(host))
}

/// The command palette rendered inline as a card.
#[must_use]
pub fn command_palette_section(state: &CompositesState) -> Section<'_> {
    let palette = CommandPalette::new(state.palette.clone())
        .on_query_change(|live| {
            let (_mode, query, _revision) = live.value().clone();
            Message::Composites(CompositesMessage::PaletteQueryChanged(query))
        })
        .on_execute(|commit| {
            Message::Composites(CompositesMessage::PaletteExecuted(commit.value().clone()))
        })
        .on_dismiss(|_cancel| Message::Composites(CompositesMessage::PaletteDismissed));

    let body = Column::new()
        .width(Length::Fill)
        .spacing(space::XS)
        // Same unbounded-axis problem as the side sheet: the open palette
        // lays out at `limits.max()` (`command_palette.rs:744`), which is
        // infinite inside a scrollable. Bound the host so the card gets a
        // real height. See BUGS.md BUG-01.
        .push(iced::widget::container(palette).height(Length::Fixed(PALETTE_HOST_HEIGHT)))
        .push(note(format!(
            "query \"{}\" · {} group(s)",
            state.palette.query,
            state.palette.groups.len()
        )));

    crate::section::section("Command palette (UIF-COMP-010)", 1, body)
}

/// A stacked form field: header band, control slot, caption band.
#[must_use]
pub fn form_field_section(state: &CompositesState) -> Section<'_> {
    let palette = Palette::dark();
    let header = form_field::resolve_header(data::FORM_LABEL, true, Some("required"), &palette);
    let caption = if state.form_valid {
        form_field::resolve_caption(true, Some(data::FORM_HELPER), None, &palette)
    } else {
        form_field::resolve_caption(false, None, Some(data::FORM_ERROR), &palette)
    };

    let control = iced::widget::text_input("path", "<no value>").into();
    let field = form_field::stacked(header, control, caption);

    let toggle = hoarfrost::widget::toggle_widget::ToggleSwitch::standalone(
        "valid",
        state.form_valid,
        "demo.composites.form.valid",
        move |next| {
            Some(Message::Composites(CompositesMessage::FormValidityToggled(
                next,
            )))
        },
    );

    let body = Column::new()
        .width(Length::Fill)
        .spacing(space::XS)
        .push(field)
        .push(
            iced::widget::Row::new()
                .spacing(space::XS)
                .align_y(iced::Alignment::Center)
                .push(toggle)
                .push(note(
                    "Flip to move the caption band between helper and error.",
                )),
        );

    crate::section::section("Form field (UIF-COMP-016)", 2, body)
}

/// Three settings rows, each with its own toggle control.
///
/// Each row is wrapped in a container carrying a
/// [`ROW_ID_SUFFIX`](crate::section::ROW_ID_SUFFIX) id. The wrapper is
/// not decoration: `settings_row::horizontal` builds its own title and
/// description `Text` widgets, and iced 0.14 throws a `Text`'s identity
/// away at `operate` time, so the demo can name the *row* but can never
/// name the words inside it. Declaring the row is what lets the
/// `declared_rows` module of the (test-only) `alignment_audit` subsystem
/// bind the title/subtitle block to the one toggle it belongs to.
#[must_use]
pub fn settings_rows_section(state: &CompositesState) -> Section<'_> {
    let palette = Palette::dark();
    let mut rows = Column::new().width(Length::Fill).spacing(space::XS);

    for (index, (title, description, disabled)) in data::SETTINGS_ROWS.iter().enumerate() {
        let value = state.settings.get(index).copied().unwrap_or(false);
        let control = hoarfrost::widget::toggle_widget::ToggleSwitch::standalone(
            "enabled",
            value,
            format!("demo.composites.setting.{index}"),
            move |next| {
                Some(Message::Composites(CompositesMessage::SettingToggled(
                    index, next,
                )))
            },
        );
        let control: Element<'_, Message> = if *disabled {
            control.disabled(true).into()
        } else {
            control.into()
        };
        let style = settings_row::resolve_style(*disabled, &palette);
        let row = settings_row::horizontal(title, Some(description), control, &style);
        rows = rows.push(declared_row(
            row,
            &row_id(SETTINGS_ROW_ID_PREFIX, &index.to_string()),
        ));
    }

    crate::section::section("Settings rows (UIF-COMP-017)", 3, rows)
}

/// Wraps a composite row in the padding-free container that declares it
/// to the probe.
///
/// Padding-free is the whole requirement: any inset would move the row's
/// recorded bounds away from the bounds the composite itself drew, and
/// every measurement the audit takes inside the row would inherit the
/// offset.
fn declared_row<'a>(row: Element<'a, Message>, id: &str) -> Element<'a, Message> {
    iced::widget::container(row)
        .id(iced::widget::Id::from(id.to_string()))
        .into()
}

/// The menu's rows, rendered as a flat stack of real menu rows.
///
/// Each row is published under its own key: every `Row` kind carries one,
/// the separators through `Row::with_key`, so the id's final segment is
/// never the empty string a keyless separator used to leave behind.
#[must_use]
pub fn menu_section(state: &CompositesState) -> Section<'_> {
    let mut rows = Column::new().width(Length::Fill);

    for row in state.menu.rows().to_vec() {
        let key = row.key().to_string();
        let menu_row = MenuRow::new(row, format!("{MENU_ROW_ID_PREFIX}{key}"), move |_event| {
            Message::Composites(CompositesMessage::MenuRowActivated(key.clone()))
        })
        .palette(Palette::dark());
        rows = rows.push(menu_row);
    }

    crate::section::section("Menu rows (UIF-FMENU / UIF-CMENU)", 5, rows)
}

#[cfg(test)]
#[path = "pane_composites_tests.rs"]
mod tests;
