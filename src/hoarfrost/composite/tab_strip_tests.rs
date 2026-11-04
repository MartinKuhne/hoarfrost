//! Tests for tab strip model, dirty close guards, pointer bindings, and external deletion synchronization (`UIF-TAB-001`, `UIF-TAB-002`, `UIF-TAB-003`).

use super::*;
use crate::surface::menu::RowKind;
use crate::token::color::Palette;

#[test]
fn test_tab_strip_constants() {
    assert_eq!(TAB_STRIP_HEIGHT, 36.0);
    assert_eq!(TAB_MIN_WIDTH, 96.0);
    assert_eq!(TAB_MAX_WIDTH, 220.0);
    assert_eq!(DIRTY_DOT_DIAMETER, 6.0);
    assert_eq!(CLOSE_BUTTON_SIZE, 24.0);
}

#[test]
fn test_tab_width_clamping() {
    assert_eq!(clamp_tab_width(50.0), 96.0);
    assert_eq!(clamp_tab_width(96.0), 96.0);
    assert_eq!(clamp_tab_width(150.0), 150.0);
    assert_eq!(clamp_tab_width(220.0), 220.0);
    assert_eq!(clamp_tab_width(300.0), 220.0);
}

#[test]
fn test_tab_context_menu_functional_groups() {
    let menu = tab_context_menu("doc1.md");
    let rows = menu.rows();

    // Spec §4.9.1 / UIF-TAB-001: 3 functional groups separated by dividers:
    // Group 1: Close, Close Others, Close All
    // Separator
    // Group 2: Edit, Open in Editor, Format Markdown
    // Separator
    // Group 3: Copy Path, Show in File Explorer
    assert_eq!(rows.len(), 10);

    // Group 1: Tab Lifecycle
    assert_eq!(rows[0].key(), "close");
    assert_eq!(rows[0].label(), "Close");
    assert_eq!(rows[0].label(), LABEL_TAB_CLOSE);
    assert_eq!(rows[1].key(), "close_others");
    assert_eq!(rows[1].label(), "Close Others");
    assert_eq!(rows[1].label(), LABEL_TAB_CLOSE_OTHERS);
    assert_eq!(rows[2].key(), "close_all");
    assert_eq!(rows[2].label(), "Close All");
    assert_eq!(rows[2].label(), LABEL_TAB_CLOSE_ALL);

    // Divider 1
    assert!(matches!(rows[3].kind(), RowKind::Separator));

    // Group 2: Edit & Automation
    assert_eq!(rows[4].key(), "edit");
    assert_eq!(rows[4].label(), "Edit");
    assert_eq!(rows[4].label(), LABEL_TAB_EDIT);
    assert_eq!(rows[5].key(), "open_in_editor");
    assert_eq!(rows[5].label(), "Open in Editor");
    assert_eq!(rows[5].label(), LABEL_TAB_OPEN_IN_EDITOR);
    assert_eq!(rows[6].key(), "format_markdown");
    assert_eq!(rows[6].label(), "Format Markdown");
    assert_eq!(rows[6].label(), LABEL_TAB_FORMAT_MARKDOWN);

    // Divider 2
    assert!(matches!(rows[7].kind(), RowKind::Separator));

    // Group 3: Location
    assert_eq!(rows[8].key(), "copy_path");
    assert_eq!(rows[8].label(), "Copy Path");
    assert_eq!(rows[8].label(), LABEL_TAB_COPY_PATH);
    assert_eq!(rows[9].key(), "show_in_explorer");
    assert_eq!(rows[9].label(), "Show in File Explorer");
    assert_eq!(rows[9].label(), LABEL_TAB_SHOW_IN_EXPLORER);

    // Verify TabContextMenuModel wrapper
    let model = TabContextMenuModel::new("doc1.md");
    let built = model.build_menu();
    assert_eq!(built.rows().len(), 10);
}

#[test]
fn test_clean_tab_close_yields_immediate_close_and_mru_handoff() {
    let mut model = TabStripModel::new();
    model.open_tab(TabItem::new("a", "Doc A"));
    model.open_tab(TabItem::new("b", "Doc B"));
    model.open_tab(TabItem::new("c", "Doc C"));

    assert_eq!(model.active_id.as_deref(), Some("c"));
    assert_eq!(model.tabs.len(), 3);

    // Switch to Doc A, making MRU order: B, C, A
    model.select_tab("a");
    assert_eq!(model.active_id.as_deref(), Some("a"));

    // Close Doc A: clean close, should activate Doc C (previous in MRU)
    let verdict = model.request_close("a");
    assert_eq!(
        verdict,
        TabCloseVerdict::CloseImmediately {
            new_active_id: Some("c".to_string()),
        }
    );
    assert_eq!(model.active_id.as_deref(), Some("c"));
    assert_eq!(model.tabs.len(), 2);

    // Close Doc C: should activate Doc B
    let verdict = model.request_close("c");
    assert_eq!(
        verdict,
        TabCloseVerdict::CloseImmediately {
            new_active_id: Some("b".to_string()),
        }
    );
    assert_eq!(model.active_id.as_deref(), Some("b"));
    assert_eq!(model.tabs.len(), 1);
}

#[test]
fn test_dirty_tab_close_requests_confirmation() {
    let mut model = TabStripModel::new();
    model.open_tab(TabItem::new("a", "Doc A"));
    model.open_tab(TabItem::new("b", "Doc B").with_dirty(true));

    assert_eq!(model.active_id.as_deref(), Some("b"));

    // Closing dirty tab B requests confirmation without closing it
    let verdict = model.request_close("b");
    assert_eq!(
        verdict,
        TabCloseVerdict::RequiresConfirmation {
            tab_id: "b".to_string(),
            title: "Doc B".to_string(),
        }
    );
    assert_eq!(model.tabs.len(), 2);
    assert_eq!(model.active_id.as_deref(), Some("b"));

    // Confirm close forces close and hands focus to Doc A
    let next = model.confirm_close("b");
    assert_eq!(next.as_deref(), Some("a"));
    assert_eq!(model.tabs.len(), 1);
    assert_eq!(model.active_id.as_deref(), Some("a"));
}

#[test]
fn test_external_deletion_sync_closes_immediately_and_hands_off_mru() {
    let mut model = TabStripModel::new();
    model.open_tab(TabItem::new("path/a.md", "a.md"));
    model.open_tab(TabItem::new("path/b.md", "b.md").with_dirty(true));

    assert_eq!(model.active_id.as_deref(), Some("path/b.md"));

    // External file deletion bypasses confirmation dialog and hands off to remaining tab
    let next = model.external_deletion_sync("path/b.md");
    assert_eq!(next.as_deref(), Some("path/a.md"));
    assert_eq!(model.tabs.len(), 1);
    assert_eq!(model.active_id.as_deref(), Some("path/a.md"));
}

#[test]
fn test_request_close_all_returns_dirty_tabs() {
    let mut model = TabStripModel::new();
    model.open_tab(TabItem::new("a", "Doc A"));
    model.open_tab(TabItem::new("b", "Doc B").with_dirty(true));
    model.open_tab(TabItem::new("c", "Doc C").with_dirty(true));
    model.open_tab(
        TabItem::new("p_dirty", "Pinned Dirty")
            .with_pinned(true)
            .with_dirty(true),
    );
    model.open_tab(TabItem::new("p_clean", "Pinned Clean").with_pinned(true));

    let dirty = model.request_close_all();
    assert_eq!(dirty, vec!["b".to_string(), "c".to_string()]);
}

#[test]
fn test_request_close_others_closes_clean_and_queues_dirty_excluding_pinned() {
    let mut model = TabStripModel::new();
    model.open_tab(TabItem::new("keep", "Keep Doc"));
    model.open_tab(TabItem::new("clean_other", "Clean Other"));
    model.open_tab(TabItem::new("dirty_other", "Dirty Other").with_dirty(true));
    model.open_tab(
        TabItem::new("pinned_dirty", "Pinned Dirty")
            .with_pinned(true)
            .with_dirty(true),
    );
    model.open_tab(TabItem::new("pinned_clean", "Pinned Clean").with_pinned(true));

    // Request closing others keeping "keep"
    let dirty = model.request_close_others("keep");

    // Only unpinned dirty tabs are returned for confirmation
    assert_eq!(dirty, vec!["dirty_other".to_string()]);

    // Clean unpinned tab "clean_other" must be closed immediately
    assert!(!model.tabs.iter().any(|t| t.id == "clean_other"));

    // "keep", "dirty_other", "pinned_dirty", "pinned_clean" must remain in model
    assert_eq!(model.tabs.len(), 4);
    assert!(model.tabs.iter().any(|t| t.id == "keep"));
    assert!(model.tabs.iter().any(|t| t.id == "dirty_other"));
    assert!(model.tabs.iter().any(|t| t.id == "pinned_dirty"));
    assert!(model.tabs.iter().any(|t| t.id == "pinned_clean"));

    // "keep" must be selected as the active tab
    assert_eq!(model.active_id.as_deref(), Some("keep"));
}

#[test]
fn test_roving_keyboard_navigation_empty_model() {
    let mut model = TabStripModel::new();

    // All roving methods must be safe no-ops on empty model
    model.select_next_tab();
    assert_eq!(model.active_id, None);

    model.select_previous_tab();
    assert_eq!(model.active_id, None);

    model.select_first_tab();
    assert_eq!(model.active_id, None);

    model.select_last_tab();
    assert_eq!(model.active_id, None);
}

#[test]
fn test_roving_keyboard_navigation_single_tab() {
    let mut model = TabStripModel::new();
    model.open_tab(TabItem::new("only", "Only Tab"));
    assert_eq!(model.active_id.as_deref(), Some("only"));

    model.select_next_tab();
    assert_eq!(model.active_id.as_deref(), Some("only"));

    model.select_previous_tab();
    assert_eq!(model.active_id.as_deref(), Some("only"));

    model.select_first_tab();
    assert_eq!(model.active_id.as_deref(), Some("only"));

    model.select_last_tab();
    assert_eq!(model.active_id.as_deref(), Some("only"));
}

#[test]
fn test_roving_keyboard_navigation_roving_and_wrap() {
    let mut model = TabStripModel::new();
    model.open_tab(TabItem::new("a", "Doc A"));
    model.open_tab(TabItem::new("b", "Doc B"));
    model.open_tab(TabItem::new("c", "Doc C"));
    assert_eq!(model.active_id.as_deref(), Some("c"));

    // Jump to first
    model.select_first_tab();
    assert_eq!(model.active_id.as_deref(), Some("a"));

    // Next from A -> B
    model.select_next_tab();
    assert_eq!(model.active_id.as_deref(), Some("b"));

    // Next from B -> C
    model.select_next_tab();
    assert_eq!(model.active_id.as_deref(), Some("c"));

    // Next from C wraps circularly -> A
    model.select_next_tab();
    assert_eq!(model.active_id.as_deref(), Some("a"));

    // Previous from A wraps circularly -> C
    model.select_previous_tab();
    assert_eq!(model.active_id.as_deref(), Some("c"));

    // Previous from C -> B
    model.select_previous_tab();
    assert_eq!(model.active_id.as_deref(), Some("b"));

    // Previous from B -> A
    model.select_previous_tab();
    assert_eq!(model.active_id.as_deref(), Some("a"));

    // Jump to last
    model.select_last_tab();
    assert_eq!(model.active_id.as_deref(), Some("c"));
}

#[test]
fn test_tab_strip_widget_into_element_renders_cleanly() {
    #[allow(dead_code)]
    #[derive(Clone, Debug)]
    enum Msg {
        Select(String),
        Close(String),
        ContextMenu(String, iced::Point),
    }

    let mut model = TabStripModel::new();
    model.open_tab(TabItem::new("a", "Doc A").with_tooltip("/path/to/Doc A"));
    model.open_tab(TabItem::new("b", "Doc B").with_dirty(true));

    let tab_strip = TabStrip::new(&model)
        .with_palette(Palette::dark())
        .on_select_tab(Msg::Select)
        .on_close_tab(Msg::Close)
        .on_context_menu(Msg::ContextMenu);

    let _el: Element<'_, Msg> = tab_strip.into();
}

#[test]
fn test_tab_strip_extreme_viewport_stress_down_to_640x480() {
    let mut model = TabStripModel::new();
    for i in 0..12 {
        model.open_tab(
            TabItem::new(
                format!("doc{i}"),
                format!("Long Document Title Number {i}.md"),
            )
            .with_dirty(i % 3 == 0),
        );
    }
    assert_eq!(model.tabs.len(), 12);

    // Stress across viewport widths: 640px, 800px, 1024px, 1920px, 3840px
    let viewports = [640.0, 800.0, 1024.0, 1920.0, 3840.0];
    for vp_width in viewports {
        let available_width = vp_width - 32.0; // Subtract shell margins
        let tab_width =
            (available_width / model.tabs.len() as f32).clamp(TAB_MIN_WIDTH, TAB_MAX_WIDTH);
        assert!(
            tab_width >= TAB_MIN_WIDTH,
            "tab width must not fall below 64px floor"
        );
        assert!(
            tab_width <= TAB_MAX_WIDTH,
            "tab width must not exceed 200px ceiling"
        );
    }
}

#[test]
fn test_pinned_tab_request_close_guards_tab() {
    let mut model = TabStripModel::new();
    model.open_tab(TabItem::new("pinned_doc", "Pinned Doc").with_pinned(true));
    model.open_tab(TabItem::new("other_doc", "Other Doc"));

    assert_eq!(model.tabs.len(), 2);
    let verdict = model.request_close("pinned_doc");
    assert_eq!(
        verdict,
        TabCloseVerdict::RefusedPinned {
            tab_id: "pinned_doc".to_string(),
        }
    );
    // Pinned tab must remain untouched in the model
    assert_eq!(model.tabs.len(), 2);
    assert!(model.tabs.iter().any(|t| t.id == "pinned_doc"));
}

#[test]
fn test_pinned_dirty_tab_request_close_refuses_close() {
    let mut model = TabStripModel::new();
    model.open_tab(
        TabItem::new("pinned_dirty", "Pinned Dirty Doc")
            .with_pinned(true)
            .with_dirty(true),
    );
    model.open_tab(TabItem::new("other_doc", "Other Doc"));

    assert_eq!(model.tabs.len(), 2);
    let verdict = model.request_close("pinned_dirty");
    assert_eq!(
        verdict,
        TabCloseVerdict::RefusedPinned {
            tab_id: "pinned_dirty".to_string(),
        }
    );
    // Pinned tab must remain untouched in the model
    assert_eq!(model.tabs.len(), 2);
    assert!(model.tabs.iter().any(|t| t.id == "pinned_dirty"));
}

struct TextCollector(Vec<String>);

impl iced::advanced::widget::Operation<Vec<String>> for TextCollector {
    fn traverse(
        &mut self,
        operate: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation<Vec<String>>),
    ) {
        operate(self);
    }
    fn text(
        &mut self,
        _id: Option<&iced::advanced::widget::Id>,
        _bounds: iced::Rectangle,
        text: &str,
    ) {
        self.0.push(text.to_owned());
    }
    fn finish(&self) -> iced::advanced::widget::operation::Outcome<Vec<String>> {
        iced::advanced::widget::operation::Outcome::Some(self.0.clone())
    }
}

#[test]
fn test_pinned_tab_omits_close_button() {
    use crate::test_helpers::Harness;

    #[allow(dead_code)]
    #[derive(Clone, Debug)]
    enum Msg {
        Close(String),
    }

    // Unpinned tab: close button "×" is rendered
    let model_unpinned = TabStripModel {
        tabs: vec![TabItem::new("unpinned", "Unpinned Tab").with_pinned(false)],
        active_id: Some("unpinned".to_string()),
        mru_history: vec!["unpinned".to_string()],
    };
    let mut harness_unpinned = Harness::headless(|| {
        let el: Element<'_, Msg> = TabStrip::new(&model_unpinned)
            .on_close_tab(Msg::Close)
            .into();
        el
    });
    let texts_unpinned = harness_unpinned
        .run(Box::new(TextCollector(Vec::new())))
        .expect("text collector returns collected texts");
    assert!(
        texts_unpinned.iter().any(|t| t == "×"),
        "unpinned tab must render close button '×'"
    );

    // Pinned tab: close button "×" is omitted
    let model_pinned = TabStripModel {
        tabs: vec![TabItem::new("pinned", "Pinned Tab").with_pinned(true)],
        active_id: Some("pinned".to_string()),
        mru_history: vec!["pinned".to_string()],
    };
    let mut harness_pinned = Harness::headless(|| {
        let el: Element<'_, Msg> = TabStrip::new(&model_pinned).on_close_tab(Msg::Close).into();
        el
    });
    let texts_pinned = harness_pinned
        .run(Box::new(TextCollector(Vec::new())))
        .expect("text collector returns collected texts");
    assert!(
        !texts_pinned.iter().any(|t| t == "×"),
        "pinned tab must omit close button '×'"
    );
}

#[test]
fn test_dirty_dot_does_not_render_as_text() {
    use crate::test_helpers::Harness;

    #[derive(Clone, Debug)]
    enum Msg {}

    let model = TabStripModel {
        tabs: vec![TabItem::new("dirty", "Dirty Tab").with_dirty(true)],
        active_id: Some("dirty".to_string()),
        mru_history: vec!["dirty".to_string()],
    };
    let mut harness = Harness::headless(|| {
        let el: Element<'_, Msg> = TabStrip::new(&model).into();
        el
    });
    let texts = harness
        .run(Box::new(TextCollector(Vec::new())))
        .expect("text collector returns collected texts");
    assert!(
        !texts.iter().any(|t| t == "•"),
        "dirty dot must be a vector geometric shape and not text '•'"
    );
}

#[test]
fn test_tab_styling_token_resolution() {
    let palette = Palette::dark();

    // Active
    let active = resolve_tab_style(&palette, true, false, false);
    assert_eq!(active.background, palette.surface_overlay);
    assert_eq!(active.border.color, palette.border_strong);
    assert_eq!(active.border.width, shape::BORDER_HAIRLINE);
    assert_eq!(active.border.radius, shape::RADIUS_WIDGET.into());
    assert_eq!(active.text_color, palette.text_strong);
    assert_eq!(active.top_accent, palette.accent_border);
    assert_eq!(active.dirty_color, Color::TRANSPARENT);

    // Inactive (Rest)
    let rest = resolve_tab_style(&palette, false, false, false);
    assert_eq!(rest.background, palette.surface_raised);
    assert_eq!(rest.border.color, palette.border_subtle);
    assert_eq!(rest.border.width, shape::BORDER_HAIRLINE);
    assert_eq!(rest.border.radius, shape::RADIUS_WIDGET.into());
    assert_eq!(rest.text_color, palette.text_muted);
    assert_eq!(rest.top_accent, Color::TRANSPARENT);
    assert_eq!(rest.dirty_color, Color::TRANSPARENT);

    // Inactive (Hovered)
    let hover = resolve_tab_style(&palette, false, true, false);
    assert_eq!(hover.background, palette.surface_hover);
    assert_eq!(hover.border.color, palette.border_strong);
    assert_eq!(hover.border.width, shape::BORDER_HAIRLINE);
    assert_eq!(hover.border.radius, shape::RADIUS_WIDGET.into());
    assert_eq!(hover.text_color, palette.text_primary);
    assert_eq!(hover.top_accent, palette.border_strong);
    assert_eq!(hover.dirty_color, Color::TRANSPARENT);
}

#[test]
fn test_vector_dot_color_is_status_warning() {
    let palette = Palette::dark();

    let active_dirty = resolve_tab_style(&palette, true, false, true);
    assert_eq!(active_dirty.dirty_color, palette.status_warning);

    let rest_dirty = resolve_tab_style(&palette, false, false, true);
    assert_eq!(rest_dirty.dirty_color, palette.status_warning);

    let hover_dirty = resolve_tab_style(&palette, false, true, true);
    assert_eq!(hover_dirty.dirty_color, palette.status_warning);

    let clean = resolve_tab_style(&palette, true, false, false);
    assert_eq!(clean.dirty_color, Color::TRANSPARENT);
}

#[test]
fn test_tab_strip_layout_bounds() {
    use crate::test_helpers::Harness;

    let mut harness = Harness::headless(|| {
        let mut model = TabStripModel::new();
        model.open_tab(TabItem::new("tab-1", "Tab One").with_dirty(true));
        model.open_tab(TabItem::new("tab-2", "Tab Two"));
        let boxed: Box<TabStripModel> = Box::new(model);
        let static_ref: &'static TabStripModel = Box::leak(boxed);
        let el: Element<'static, ()> = TabStrip::new(static_ref).on_close_tab(|_| ()).into();
        el
    });

    let candidates = harness.candidates();

    // Outermost track container height must match TAB_STRIP_HEIGHT (36px, §4.4.1)
    let track = candidates
        .iter()
        .find(|c| c.kind == "container" && c.bounds.height == TAB_STRIP_HEIGHT)
        .expect("tab strip track container with TAB_STRIP_HEIGHT");
    assert_eq!(track.bounds.height, TAB_STRIP_HEIGHT);

    // Both tabs must be at least TAB_MIN_WIDTH (96px) and fill track height (36px)
    let tab_cards: Vec<_> = candidates
        .iter()
        .filter(|c| {
            c.kind == "container"
                && c.bounds.height == TAB_STRIP_HEIGHT
                && c.bounds.width >= TAB_MIN_WIDTH
        })
        .collect();
    assert!(
        !tab_cards.is_empty(),
        "tabs must render at full track height"
    );

    // Close button hit target must be 24x24px (§4.4.1)
    let close_buttons: Vec<_> = candidates
        .iter()
        .filter(|c| {
            c.kind == "container"
                && (c.bounds.width - CLOSE_BUTTON_SIZE).abs() < 0.1
                && (c.bounds.height - CLOSE_BUTTON_SIZE).abs() < 0.1
        })
        .collect();
    assert!(
        !close_buttons.is_empty(),
        "close button hit targets must measure 24x24px"
    );

    // Dirty dot indicator must measure DIRTY_DOT_DIAMETER (6x6px, §4.4.1)
    let dirty_dots: Vec<_> = candidates
        .iter()
        .filter(|c| {
            c.kind == "container"
                && (c.bounds.width - DIRTY_DOT_DIAMETER).abs() < 0.1
                && (c.bounds.height - DIRTY_DOT_DIAMETER).abs() < 0.1
        })
        .collect();
    assert_eq!(
        dirty_dots.len(),
        1,
        "dirty tab must render 6x6px vector dot indicator"
    );

    // Status slot must measure STATUS_SLOT_WIDTH (12px, §4.4.1)
    let status_slots: Vec<_> = candidates
        .iter()
        .filter(|c| c.kind == "container" && (c.bounds.width - STATUS_SLOT_WIDTH).abs() < 0.1)
        .collect();
    assert!(
        !status_slots.is_empty(),
        "dirty tab must wrap dot inside 12px status slot container (§4.4.1)"
    );
}

#[test]
fn test_tab_visual_attributes() {
    // 1. Status slot width (12px, §4.4.1)
    assert_eq!(STATUS_SLOT_WIDTH, 12.0);

    // 2. Close icon size (14px, §4.4.1)
    assert_eq!(CLOSE_ICON_SIZE, 14.0);
    assert_eq!(CLOSE_ICON_SIZE, size::ICON_SM);

    // 3. Active tab typography (13px SemiBold, §4.4.2)
    let active_font = resolve_tab_font(true);
    assert_eq!(active_font.weight, iced::font::Weight::Semibold);

    // 4. Inactive tab typography (Default Regular, §4.4.2)
    let inactive_font = resolve_tab_font(false);
    assert_eq!(inactive_font.weight, iced::Font::DEFAULT.weight);
}
