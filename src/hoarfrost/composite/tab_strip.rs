//! Tab strip composite with dirty close guards, pointer bindings, MRU focus handoff, and external-deletion sync (`UIF-TAB-001`, `UIF-TAB-002`, `UIF-TAB-003`).
//!
//! The tab strip manages open document tabs, tracking their active state, unsaved-change ("dirty") status,
//! and most-recently-used (MRU) activation history. When a tab is closed, a dirty close guard intercepts
//! tabs with unsaved edits to raise confirmation dialogs, while external file deletions automatically
//! close the corresponding tab and transfer focus to the most-recently-active remaining document.
//! Pointer bindings allow selecting tabs with left-click, closing tabs with middle-click or close button,
//! and opening context menus with right-click. The tabs are wrapped in a horizontal scrollable container
//! respecting [TAB_MIN_WIDTH, TAB_MAX_WIDTH] and 24x24 close button targets.
//!
//! Unit tests live in the sibling `tab_strip_tests.rs` sidecar.

use iced::alignment;
use iced::{Border, Color, Element, Length, Pixels, Shadow};

use crate::surface::menu::{Menu, Row};
use crate::surface::tooltip::Tooltip;
use crate::token::{color::Palette, shape, size, space, typography};

pub use crate::strings::{
    LABEL_TAB_CLOSE, LABEL_TAB_CLOSE_ALL, LABEL_TAB_CLOSE_OTHERS, LABEL_TAB_COPY_PATH,
    LABEL_TAB_EDIT, LABEL_TAB_FORMAT_MARKDOWN, LABEL_TAB_OPEN_IN_EDITOR,
    LABEL_TAB_SHOW_IN_EXPLORER,
};

/// Standard height of a tab strip in pixels (`size.control.lg` - 36px, §4.4.1, `UIF-TAB-001`).
pub const TAB_STRIP_HEIGHT: f32 = size::CONTROL_LG;

/// Minimum width of an individual tab in pixels (96px, §2.2, §4.4.3).
pub const TAB_MIN_WIDTH: f32 = 96.0;

/// Maximum width of an individual tab in pixels (220px, §2.2, §4.4.3).
pub const TAB_MAX_WIDTH: f32 = 220.0;

/// Diameter of the unsaved-state dirty dot in pixels (`size.indicator.dot` - 6px).
pub const DIRTY_DOT_DIAMETER: f32 = size::INDICATOR_DOT;

/// Size of the tab close button target in pixels (`size.hit_target.min` - 24px, §4.4.3).
pub const CLOSE_BUTTON_SIZE: f32 = size::HIT_TARGET_MIN;

/// Width of the leading status slot in pixels (12px, §4.4.1).
const STATUS_SLOT_WIDTH: f32 = 12.0;

/// Glyph size of the close button icon in pixels (`size::ICON_SM` - 14px, §4.4.1).
const CLOSE_ICON_SIZE: f32 = size::ICON_SM;

/// Clamp a proposed tab width into [`TAB_MIN_WIDTH`, `TAB_MAX_WIDTH`] (`UIF-TAB-002`).
#[must_use]
pub fn clamp_tab_width(width: f32) -> f32 {
    width.clamp(TAB_MIN_WIDTH, TAB_MAX_WIDTH)
}

/// Standardized context menu model for document tabs (`UIF-TAB-001`, spec §4.9.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabContextMenuModel {
    /// Identifier / path of the target tab.
    pub tab_id: String,
}

impl TabContextMenuModel {
    /// Create a new tab context menu model for `tab_id`.
    #[must_use]
    pub fn new(tab_id: impl Into<String>) -> Self {
        Self {
            tab_id: tab_id.into(),
        }
    }

    /// Build the standardized [`Menu`] with §4.9.1 functional groups.
    #[must_use]
    pub fn build_menu(&self) -> Menu {
        tab_context_menu(&self.tab_id)
    }
}

/// Helper function creating the standardized context menu for a tab (`UIF-TAB-001`, spec §4.9.1).
#[must_use]
pub fn tab_context_menu(tab_id: &str) -> Menu {
    let menu_id = format!("menu:tab:{tab_id}");
    let rows = vec![
        // Group 1: Tab Lifecycle
        Row::action("close", LABEL_TAB_CLOSE),
        Row::action("close_others", LABEL_TAB_CLOSE_OTHERS),
        Row::action("close_all", LABEL_TAB_CLOSE_ALL),
        // Divider
        Row::separator(),
        // Group 2: Edit & Automation
        Row::action("edit", LABEL_TAB_EDIT),
        Row::action("open_in_editor", LABEL_TAB_OPEN_IN_EDITOR),
        Row::action("format_markdown", LABEL_TAB_FORMAT_MARKDOWN),
        // Divider
        Row::separator(),
        // Group 3: Location
        Row::action("copy_path", LABEL_TAB_COPY_PATH),
        Row::action("show_in_explorer", LABEL_TAB_SHOW_IN_EXPLORER),
    ];

    Menu::new(menu_id, rows)
}

/// Close verdict returned when a tab close is requested (`UIF-TAB-003`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TabCloseVerdict {
    /// The tab was clean and closed immediately, yielding the new active tab ID if any.
    CloseImmediately {
        /// The new active tab ID, chosen from MRU history.
        new_active_id: Option<String>,
    },
    /// The tab has unsaved changes and requires user confirmation before closing (`UIF-MOD-002`).
    RequiresConfirmation {
        /// The ID of the dirty tab.
        tab_id: String,
        /// The title of the dirty tab.
        title: String,
    },
    /// The tab is pinned and cannot be closed (`UIF-TAB-003`).
    RefusedPinned {
        /// The ID of the pinned tab.
        tab_id: String,
    },
}

/// A single tab item within a [`TabStripModel`] (`UIF-TAB-003`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabItem {
    /// Unique identifier for this tab (often a document path or UUID).
    pub id: String,
    /// Display title of the tab.
    pub title: String,
    /// Optional tooltip text displaying full file path or details.
    pub tooltip: Option<String>,
    /// Whether this tab has unsaved changes.
    pub is_dirty: bool,
    /// Whether this tab is pinned to the leading edge of the strip.
    pub is_pinned: bool,
}

impl TabItem {
    /// Create a new tab item with `id` and `title`.
    #[must_use]
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            tooltip: None,
            is_dirty: false,
            is_pinned: false,
        }
    }

    /// Set whether the tab has unsaved modifications.
    #[must_use]
    pub fn with_dirty(mut self, dirty: bool) -> Self {
        self.is_dirty = dirty;
        self
    }

    /// Set an explicit tooltip path or description.
    #[must_use]
    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Set whether the tab is pinned.
    #[must_use]
    pub fn with_pinned(mut self, pinned: bool) -> Self {
        self.is_pinned = pinned;
        self
    }
}

/// Declarative model governing a tab strip's tabs, selection, MRU history, and close guards (`UIF-TAB-003`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TabStripModel {
    /// Ordered list of open tabs.
    pub tabs: Vec<TabItem>,
    /// Identifier of the currently active tab.
    pub active_id: Option<String>,
    /// Most-recently-used activation history, newest at the end.
    pub mru_history: Vec<String>,
}

impl TabStripModel {
    /// Create an empty tab strip model.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a tab to the strip and select it.
    pub fn open_tab(&mut self, tab: TabItem) {
        let id = tab.id.clone();
        if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
            self.tabs[pos] = tab;
        } else {
            self.tabs.push(tab);
        }
        self.select_tab(&id);
    }

    /// Select the tab identified by `id`, recording it in the MRU history.
    pub fn select_tab(&mut self, id: &str) {
        if self.tabs.iter().any(|t| t.id == id) {
            self.active_id = Some(id.to_string());
            self.mru_history.retain(|existing| existing != id);
            self.mru_history.push(id.to_string());
        }
    }

    /// Select the next tab in roving order, circularly wrapping from last to first (`UIF-TAB-001`).
    ///
    /// No-op if the tab strip is empty.
    pub fn select_next_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }

        let next_idx = match &self.active_id {
            Some(active) => self
                .tabs
                .iter()
                .position(|t| &t.id == active)
                .map_or(0, |idx| (idx + 1) % self.tabs.len()),
            None => 0,
        };

        let next_id = self.tabs[next_idx].id.clone();
        self.select_tab(&next_id);
    }

    /// Select the previous tab in roving order, circularly wrapping from first to last (`UIF-TAB-001`).
    ///
    /// No-op if the tab strip is empty.
    pub fn select_previous_tab(&mut self) {
        if self.tabs.is_empty() {
            return;
        }

        let prev_idx = match &self.active_id {
            Some(active) => self
                .tabs
                .iter()
                .position(|t| &t.id == active)
                .map_or(0, |idx| {
                    if idx == 0 {
                        self.tabs.len() - 1
                    } else {
                        idx - 1
                    }
                }),
            None => self.tabs.len() - 1,
        };

        let prev_id = self.tabs[prev_idx].id.clone();
        self.select_tab(&prev_id);
    }

    /// Select the first tab in the strip (`UIF-TAB-001`, Home key).
    ///
    /// No-op if the tab strip is empty.
    pub fn select_first_tab(&mut self) {
        if let Some(first) = self.tabs.first() {
            let first_id = first.id.clone();
            self.select_tab(&first_id);
        }
    }

    /// Select the last tab in the strip (`UIF-TAB-001`, End key).
    ///
    /// No-op if the tab strip is empty.
    pub fn select_last_tab(&mut self) {
        if let Some(last) = self.tabs.last() {
            let last_id = last.id.clone();
            self.select_tab(&last_id);
        }
    }

    /// Request closing a tab by `id` (`UIF-TAB-003`).
    ///
    /// If the tab is pinned (`is_pinned == true`), returns [`TabCloseVerdict::RefusedPinned`]
    /// because pinned tabs cannot be closed.
    /// If the tab is dirty (`is_dirty == true`), returns [`TabCloseVerdict::RequiresConfirmation`]
    /// so the host can display a save/discard modal.
    /// If the tab is clean, it is closed immediately, and focus hands off to the most recently
    /// active remaining tab per MRU history.
    pub fn request_close(&mut self, id: &str) -> TabCloseVerdict {
        let Some(tab) = self.tabs.iter().find(|t| t.id == id) else {
            return TabCloseVerdict::CloseImmediately {
                new_active_id: self.active_id.clone(),
            };
        };

        if tab.is_pinned {
            return TabCloseVerdict::RefusedPinned {
                tab_id: tab.id.clone(),
            };
        }

        if tab.is_dirty {
            TabCloseVerdict::RequiresConfirmation {
                tab_id: tab.id.clone(),
                title: tab.title.clone(),
            }
        } else {
            let new_active_id = self.close_tab_internal(id);
            TabCloseVerdict::CloseImmediately { new_active_id }
        }
    }

    /// Confirm and force-close a tab even if it has unsaved changes (`UIF-TAB-003`).
    pub fn confirm_close(&mut self, id: &str) -> Option<String> {
        self.close_tab_internal(id)
    }

    /// Synchronize external file deletion (`UIF-TAB-003`).
    ///
    /// When an underlying file is externally deleted, its tab closes immediately without
    /// blocking on user confirmation, and activation transfers to the most-recently-active
    /// remaining tab.
    pub fn external_deletion_sync(&mut self, vanished_path: &str) -> Option<String> {
        self.close_tab_internal(vanished_path)
    }

    /// Collect IDs of dirty unpinned tabs that require confirmation before closing all tabs except `keep_id` (`UIF-TAB-003`).
    ///
    /// Clean unpinned tabs other than `keep_id` are closed immediately. Pinned tabs are excluded and remain open.
    pub fn request_close_others(&mut self, keep_id: &str) -> Vec<String> {
        let other_ids: Vec<String> = self
            .tabs
            .iter()
            .filter(|t| t.id != keep_id && !t.is_pinned)
            .map(|t| t.id.clone())
            .collect();

        let mut dirty_ids = Vec::new();
        for other_id in other_ids {
            let is_dirty = self
                .tabs
                .iter()
                .find(|t| t.id == other_id)
                .is_some_and(|t| t.is_dirty);
            if is_dirty {
                dirty_ids.push(other_id);
            } else {
                self.close_tab_internal(&other_id);
            }
        }

        if self.tabs.iter().any(|t| t.id == keep_id) {
            self.select_tab(keep_id);
        }

        dirty_ids
    }

    /// Collect all dirty unpinned tabs that require confirmation before closing all tabs (`UIF-TAB-003`).
    ///
    /// Pinned tabs are excluded because they cannot be closed.
    #[must_use]
    pub fn request_close_all(&self) -> Vec<String> {
        self.tabs
            .iter()
            .filter(|t| !t.is_pinned && t.is_dirty)
            .map(|t| t.id.clone())
            .collect()
    }

    /// Internal close implementation removing the tab and restoring the next MRU active tab.
    fn close_tab_internal(&mut self, id: &str) -> Option<String> {
        self.tabs.retain(|t| t.id != id);
        self.mru_history.retain(|existing| existing != id);

        if self.active_id.as_deref() == Some(id) {
            self.active_id = self
                .mru_history
                .iter()
                .rev()
                .find(|candidate| self.tabs.iter().any(|t| t.id == **candidate))
                .cloned()
                .or_else(|| self.tabs.first().map(|t| t.id.clone()));
        }

        self.active_id.clone()
    }
}

/// Visual styling of a single tab item (`UIF-TAB-003`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TabItemStyle {
    /// Background fill of the tab.
    pub background: Color,
    /// Perimeter border of the tab.
    pub border: Border,
    /// Label text color.
    pub text_color: Color,
    /// Dirty indicator color.
    pub dirty_color: Color,
    /// Top accent indicator color (2px indicator bar, §4.4.1, `UIF-TAB-001`).
    pub top_accent: Color,
}

/// Resolve the styling of a tab item (`UIF-TAB-003`).
#[must_use]
pub fn resolve_tab_style(
    palette: &Palette,
    is_active: bool,
    is_hovered: bool,
    is_dirty: bool,
) -> TabItemStyle {
    let (background, border, text_color, top_accent) = if is_active {
        (
            palette.surface_overlay,
            Border::default()
                .color(palette.border_strong)
                .width(shape::BORDER_HAIRLINE)
                .rounded(shape::RADIUS_WIDGET),
            palette.text_strong,
            palette.accent_border,
        )
    } else if is_hovered {
        (
            palette.surface_hover,
            Border::default()
                .color(palette.border_strong)
                .width(shape::BORDER_HAIRLINE)
                .rounded(shape::RADIUS_WIDGET),
            palette.text_primary,
            palette.border_strong,
        )
    } else {
        (
            palette.surface_raised,
            Border::default()
                .color(palette.border_subtle)
                .width(shape::BORDER_HAIRLINE)
                .rounded(shape::RADIUS_WIDGET),
            palette.text_muted,
            Color::TRANSPARENT,
        )
    };

    let dirty_color = if is_dirty {
        palette.status_warning
    } else {
        Color::TRANSPARENT
    };

    TabItemStyle {
        background,
        border,
        text_color,
        dirty_color,
        top_accent,
    }
}

/// Resolve the title label typography font for a tab (`UIF-TAB-001`, §4.4.2).
///
/// Active tabs render with [`iced::font::Weight::Semibold`] (13px SemiBold, §4.4.2).
/// Inactive tabs render with [`iced::Font::DEFAULT`].
#[must_use]
pub fn resolve_tab_font(is_active: bool) -> iced::Font {
    if is_active {
        iced::Font {
            weight: iced::font::Weight::Semibold,
            ..iced::Font::DEFAULT
        }
    } else {
        iced::Font::DEFAULT
    }
}

/// Tab strip widget rendering the horizontal document tabs (`UIF-TAB-001`, `UIF-TAB-002`, `UIF-TAB-003`).
pub struct TabStrip<'a, Message> {
    model: &'a TabStripModel,
    palette: Palette,
    on_select_tab: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_close_tab: Option<Box<dyn Fn(String) -> Message + 'a>>,
    on_context_menu: Option<Box<dyn Fn(String, iced::Point) -> Message + 'a>>,
}

impl<'a, Message> TabStrip<'a, Message> {
    /// Create a new tab strip widget from `model`.
    #[must_use]
    pub fn new(model: &'a TabStripModel) -> Self {
        Self {
            model,
            palette: Palette::dark(),
            on_select_tab: None,
            on_close_tab: None,
            on_context_menu: None,
        }
    }

    /// Set the design token color palette.
    #[must_use]
    pub fn with_palette(mut self, palette: Palette) -> Self {
        self.palette = palette;
        self
    }

    /// Callback invoked when a tab is selected (`UIF-TAB-001`).
    #[must_use]
    pub fn on_select_tab(mut self, f: impl Fn(String) -> Message + 'a) -> Self {
        self.on_select_tab = Some(Box::new(f));
        self
    }

    /// Callback invoked when a tab's close button or middle-click is triggered (`UIF-TAB-001`).
    #[must_use]
    pub fn on_close_tab(mut self, f: impl Fn(String) -> Message + 'a) -> Self {
        self.on_close_tab = Some(Box::new(f));
        self
    }

    /// Callback invoked when a tab's context menu is requested (`UIF-TAB-001`).
    ///
    /// Note: [`iced::Point::ORIGIN`] is passed as the cursor position because
    /// `iced::widget::mouse_area::on_right_press` does not yield cursor coordinates.
    #[must_use]
    pub fn on_context_menu(mut self, f: impl Fn(String, iced::Point) -> Message + 'a) -> Self {
        self.on_context_menu = Some(Box::new(f));
        self
    }
}

impl<'a, Message: 'a + Clone> From<TabStrip<'a, Message>> for Element<'a, Message> {
    fn from(tab_strip: TabStrip<'a, Message>) -> Self {
        let palette = tab_strip.palette;
        let mut row = iced::widget::Row::new()
            .spacing(space::XXS)
            .height(Length::Fill);

        for tab in &tab_strip.model.tabs {
            let is_active = tab_strip.model.active_id.as_deref() == Some(&tab.id);
            let style = resolve_tab_style(&palette, is_active, false, tab.is_dirty);

            let top_accent_bar = iced::widget::container(iced::widget::Space::new())
                .height(Length::Fixed(shape::BORDER_FOCUS))
                .width(Length::Fill)
                .style(move |_theme| iced::widget::container::Style {
                    background: Some(style.top_accent.into()),
                    ..Default::default()
                });

            let mut tab_content = iced::widget::Row::new()
                .spacing(space::XS)
                .padding(iced::Padding::new(0.0).horizontal(space::SM))
                .align_y(alignment::Vertical::Center)
                .height(Length::Fill);

            if tab.is_dirty {
                let dirty_dot = iced::widget::container(
                    iced::widget::Space::new()
                        .width(DIRTY_DOT_DIAMETER)
                        .height(DIRTY_DOT_DIAMETER),
                )
                .width(Length::Fixed(DIRTY_DOT_DIAMETER))
                .height(Length::Fixed(DIRTY_DOT_DIAMETER))
                .style(move |_theme| iced::widget::container::Style {
                    background: Some(style.dirty_color.into()),
                    border: Border::default().rounded(shape::RADIUS_PILL),
                    shadow: Shadow::default(),
                    snap: true,
                    text_color: None,
                });

                let status_slot = iced::widget::container(dirty_dot)
                    .width(Length::Fixed(STATUS_SLOT_WIDTH))
                    .align_x(alignment::Horizontal::Center)
                    .align_y(alignment::Vertical::Center);

                tab_content = tab_content.push(status_slot);
            }

            let title_font = resolve_tab_font(is_active);

            tab_content = tab_content.push(
                iced::widget::text(tab.title.clone())
                    .size(Pixels(typography::UI_COMPACT.size))
                    .line_height(typography::UI_COMPACT.iced_line_height())
                    .font(title_font)
                    .color(style.text_color),
            );

            if !tab.is_pinned
                && let Some(on_close) = &tab_strip.on_close_tab
            {
                let close_msg = on_close(tab.id.clone());
                tab_content = tab_content.push(
                    iced::widget::button(
                        iced::widget::container(
                            iced::widget::text("×")
                                .size(Pixels(CLOSE_ICON_SIZE))
                                .align_x(alignment::Horizontal::Center)
                                .align_y(alignment::Vertical::Center),
                        )
                        .width(Length::Fixed(CLOSE_BUTTON_SIZE))
                        .height(Length::Fixed(CLOSE_BUTTON_SIZE))
                        .align_x(alignment::Horizontal::Center)
                        .align_y(alignment::Vertical::Center),
                    )
                    .on_press(close_msg)
                    .padding(iced::Padding::ZERO)
                    .width(Length::Fixed(CLOSE_BUTTON_SIZE))
                    .height(Length::Fixed(CLOSE_BUTTON_SIZE))
                    .style(move |_theme, status| {
                        let is_hovered = matches!(status, iced::widget::button::Status::Hovered);
                        let bg = if is_hovered {
                            palette.surface_hover
                        } else {
                            Color::TRANSPARENT
                        };
                        let text_color = if is_hovered {
                            palette.status_danger_text
                        } else {
                            style.text_color
                        };
                        iced::widget::button::Style {
                            background: Some(bg.into()),
                            text_color,
                            border: Border::default().rounded(shape::RADIUS_WIDGET),
                            shadow: Shadow::default(),
                            snap: true,
                        }
                    }),
                );
            }

            let tab_id = tab.id.clone();
            let tab_column = iced::widget::column![
                top_accent_bar,
                tab_content,
                iced::widget::Space::new().width(Length::Fixed(TAB_MIN_WIDTH)),
            ]
            .height(Length::Fill);

            // Width clamping: guarantee at least TAB_MIN_WIDTH (96px) and at most TAB_MAX_WIDTH (220px)
            let tab_inner = iced::widget::container(tab_column)
                .height(Length::Fill)
                .max_width(Pixels(TAB_MAX_WIDTH))
                .style(move |_theme| iced::widget::container::Style {
                    background: Some(style.background.into()),
                    border: style.border,
                    shadow: Shadow::default(),
                    snap: true,
                    text_color: Some(style.text_color),
                });

            let mut tab_area = iced::widget::mouse_area(tab_inner);

            if let Some(on_select) = &tab_strip.on_select_tab {
                tab_area = tab_area.on_press(on_select(tab_id.clone()));
            }

            if !tab.is_pinned
                && let Some(on_close) = &tab_strip.on_close_tab
            {
                tab_area = tab_area.on_middle_press(on_close(tab_id.clone()));
            }

            if let Some(on_ctx) = &tab_strip.on_context_menu {
                tab_area = tab_area.on_right_press(on_ctx(tab_id.clone(), iced::Point::ORIGIN));
            }

            let el: Element<'a, Message> = tab_area.into();

            if let Some(tooltip_text) = &tab.tooltip {
                row = row.push(Tooltip::new(el, tooltip_text.clone()).with_palette(palette));
            } else {
                row = row.push(el);
            }
        }

        let scroll = iced::widget::scrollable(row)
            .direction(iced::widget::scrollable::Direction::Horizontal(
                iced::widget::scrollable::Scrollbar::default(),
            ))
            .height(Length::Fill);

        iced::widget::container(scroll)
            .width(Length::Fill)
            .height(Length::Fixed(TAB_STRIP_HEIGHT))
            .padding(iced::Padding::new(0.0).horizontal(space::XS))
            .style(move |_theme| iced::widget::container::Style {
                background: Some(palette.surface_sunken.into()),
                border: Border::default()
                    .color(palette.border_subtle)
                    .width(shape::BORDER_HAIRLINE),
                text_color: Some(palette.text_primary),
                shadow: Shadow::default(),
                snap: true,
            })
            .into()
    }
}

#[cfg(test)]
#[path = "tab_strip_tests.rs"]
mod tab_strip_tests;
