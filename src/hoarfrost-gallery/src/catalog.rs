//! What the sweep renders: the demo's own sections, enumerated in window order.
//!
//! # Why this module owns the demo state
//!
//! A hoarfrost demo section is not a value you can keep. `Section` holds an
//! `iced::Element`, which owns a boxed widget, and the live sections borrow the
//! interactive models they paint — the checkbox states, the tab strip model, the
//! command palette — so the models have to outlive the frame that reads them.
//! That is why `AtomsState` and `CompositesState` exist in the demo and why the
//! section functions take them by reference.
//!
//! The catalog is therefore the owner of that state for the whole run: it holds
//! one of each, built from `Default`, and every section it hands out borrows
//! from them. Nothing here mutates the state — a PNG sweep has no events to
//! deliver — so every section is drawn in its resting configuration, which is
//! exactly what a documentation image should show.
//!
//! # Why `build` re-runs the source function
//!
//! `Section` is not `Clone` and borrows from the state, so a target cannot carry
//! its element and hand it out twice. But the capture driver needs the element
//! twice — once to measure, once to rasterise — and it needs it *fresh* each
//! time, because an `Element` is moved into the `UserInterface` that consumes
//! it.
//!
//! So a [`Target`] stores *where* its section comes from rather than the section
//! itself, and [`Catalog::build`] re-runs the source function and indexes into
//! the `Vec` it returns. This is the same "two builds from one pure builder"
//! reasoning already documented in [`crate::capture`] and in
//! `src/hoarfrost-demo/src/render_section.rs`: the demo's section builders are
//! pure functions of their state, so two calls produce two identical trees, and
//! rebuilding is what makes the two-pass capture legal instead of lucky.
//!
//! # Why a target names a source instead of holding a closure
//!
//! There are exactly five source functions and they are known at compile time.
//! A `Source` enum plus an index says "the third element of
//! [`hoarfrost_demo::pane_atoms::live_sections`]" in a form a reader can check,
//! a `Debug` printing can show, and an equality can compare. A `Box<dyn Fn>`
//! would say the same thing in a form none of those things work on, and would
//! make `Target` non-`Clone` for no benefit.
//!
//! # Why one section needs a declared height
//!
//! Almost every section has a natural height the measure pass can read back. One
//! does not, because `hoarfrost::composite::empty_state` sizes itself to the box
//! it is handed. [`DECLARED_HEIGHTS`] is the closed table of those sections and
//! the canvas each is drawn on, and it is documented with the reason, the
//! arithmetic behind the number, and the condition for deleting the row.
//!
//! Unit tests live in the sibling `catalog_tests.rs` sidecar.

use hoarfrost_demo::app::Message;
use hoarfrost_demo::pane_atoms::AtomsState;
use hoarfrost_demo::pane_composites::CompositesState;
use hoarfrost_demo::section::Section;
use iced::Element;

/// The five source functions the demo composes its two panes from, in the order
/// the window shows them.
///
/// Private on purpose: it names an implementation detail of the demo, and a
/// caller has no reason to branch on it. It exists so a [`Target`] can say where
/// it came from in a form that can be compared and printed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Source {
    /// [`hoarfrost_demo::pane_atoms::live_sections`].
    LiveAtoms,
    /// [`hoarfrost_demo::previews::atom_preview_sections`].
    AtomPreviews,
    /// [`hoarfrost_demo::pane_composites::group1_sections`].
    CompositeGroup1,
    /// [`hoarfrost_demo::pane_composites::group2_sections`].
    CompositeGroup2,
    /// [`hoarfrost_demo::previews::composite_preview_sections`].
    CompositePreviews,
}

impl Source {
    /// Every source, in the order the demo window lays its panes out.
    ///
    /// This one list is the sweep order: [`Catalog::enumerate`] walks it and
    /// [`Catalog::build`] reads from it, so the two cannot disagree about what a
    /// `Source` means.
    const ORDER: [Source; 5] = [
        Source::LiveAtoms,
        Source::AtomPreviews,
        Source::CompositeGroup1,
        Source::CompositeGroup2,
        Source::CompositePreviews,
    ];

    /// What a reader (or a `Debug` print) calls this source.
    fn label(self) -> &'static str {
        match self {
            Source::LiveAtoms => "atoms/live",
            Source::AtomPreviews => "atoms/previews",
            Source::CompositeGroup1 => "composites/group1",
            Source::CompositeGroup2 => "composites/group2",
            Source::CompositePreviews => "composites/previews",
        }
    }
}

/// One render target: a section, named by where it is rebuilt from.
///
/// Cheap to store and compare, and it survives the source `Vec` being dropped,
/// which is exactly what a two-pass capture needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    /// The section's own stable id, `demo.section.<slug>`.
    id: String,
    /// Which source function rebuilds it.
    source: Source,
    /// Its position in that function's output.
    index: usize,
}

impl Target {
    /// The id the section publishes, and the id the capture driver looks for.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Which source function this target is rebuilt from, as a label.
    #[must_use]
    pub fn source_label(&self) -> &'static str {
        self.source.label()
    }

    /// Its position within its source function's output.
    #[must_use]
    pub fn index(&self) -> usize {
        self.index
    }
}

/// The demo's sections, ready to be rebuilt on demand.
///
/// Owns the state every live section borrows, so it must outlive every element
/// [`Catalog::build`] hands out.
pub struct Catalog {
    /// The atoms pane's interactive models, at their resting values.
    atoms: AtomsState,
    /// The composites pane's interactive models, at their resting values.
    composites: CompositesState,
    /// Every renderable section, in window order.
    targets: Vec<Target>,
}

impl Default for Catalog {
    fn default() -> Self {
        Self::new()
    }
}

impl Catalog {
    /// Build the catalog from the demo's resting state.
    ///
    /// The state is `Default`, not a replayed session: a documentation sweep
    /// shows each widget as it first appears, and a run that depended on some
    /// earlier interaction could not be reproduced from the command line alone.
    #[must_use]
    pub fn new() -> Self {
        let atoms = AtomsState::default();
        let composites = CompositesState::default();
        let targets = enumerate(&atoms, &composites);
        Self {
            atoms,
            composites,
            targets,
        }
    }

    /// Every section the sweep will try to render, in window order.
    #[must_use]
    pub fn targets(&self) -> &[Target] {
        &self.targets
    }

    /// A fresh element for `target`, rebuilt from its source function.
    ///
    /// Returns [`None`] when the source no longer has anything at that index —
    /// which means the demo changed shape underneath a catalog built from it.
    /// `None` rather than a panic: the sweep's contract is that one section that
    /// cannot be produced is recorded as a failure and the run continues, and a
    /// panic would abort the very run that exists to report the problem.
    pub fn build(&self, target: &Target) -> Option<Element<'_, Message>> {
        sections(&self.atoms, &self.composites, target.source)
            .into_iter()
            .nth(target.index)
            .map(|section| section.element)
    }
}

/// The section id of the demo's stacked empty states.
const EMPTY_STATES_ID: &str = "demo.section.empty-states-uif-comp-014";

/// The height `EMPTY_STATES_ID` is drawn at, in logical pixels.
///
/// **668 is the section's own natural height, read off the layout rather than
/// guessed.** Every figure below was taken from the widget tree with the section
/// given far more room than it needs (a 700 × 1200 probe), so nothing in it was
/// squeezed, and then summed:
///
/// | part | px |
/// |------|----|
/// | card padding, top and bottom | 16 + 16 |
/// | the section heading | 24 |
/// | heading to body gap | 8 |
/// | three empty-state boxes, each 140 of content plus 24 padding top and bottom | 3 × 188 |
/// | gaps between them and the note | 3 × 8 |
/// | the trailing note line | 16 |
///
/// 16 + 24 + 8 + 564 + 24 + 16 + 16 = **668**. The 140 of content in each box is
/// a badge (40) plus a title (24) plus a description (18) plus an action button
/// (34) with three 8 px gaps; the 48 of padding belongs to `empty_state` itself
/// (`src/hoarfrost/composite/empty_state.rs`, its 24 px container padding).
///
/// The sum was then checked from the other direction by laying the section out
/// *at* 668: each empty-state box measures exactly 188, its content column
/// exactly 140, and the badge, title, description, button and note measure 40,
/// 24, 18, 34 and 16 — every part at the height it asks for, with the last node
/// ending at 652 and the card closing at 668. Nothing is clipped and nothing is
/// padded out.
///
/// **An earlier value of 524 was wrong, and how it was caught.** 524 came from
/// the same sum with the 48 px of per-box padding left out. It is not a silent
/// clip — no node spills outside its container at 524, so a spill test passes —
/// but iced *shrinks* content to fit rather than overflowing it: at 524 each box
/// is handed 140, its content column is crushed to 92, the description renders
/// 4 px tall and the button 0 px. That is what
/// `capture_tests::a_declared_height_leaves_no_part_of_the_section_squeezed`
/// checks for, by comparing every text line at the declared height against the
/// shortest text line the same section produces with plenty of room.
///
/// At the default 2× scale this rasterises to 1336 physical px: taller than any
/// measured section (the tallest are around 1020), which is honest, because three
/// stacked empty states really are that tall once each of them gets the padding
/// it was designed with.
const EMPTY_STATES_HEIGHT: f32 = 668.0;

/// Sections that have **no natural height**, and the canvas each is drawn on.
///
/// # Why this table has to exist
///
/// `demo.section.empty-states-uif-comp-014` stacks three
/// `hoarfrost::composite::empty_state` widgets in a column
/// (`src/hoarfrost-demo/src/pane_composites.rs`, `empty_state_section`), and an
/// empty state lays its content out inside a container with
/// `.height(Length::Fill)` — `src/hoarfrost/composite/empty_state.rs:266`. A
/// `Fill`-height widget has no height of its own to report: it answers "whatever
/// box you gave me", at *every* probe height, so the measure pass can never
/// learn anything from it and `Renderer::shot` rightly refuses the result (see
/// the `PROBE_HEIGHT` ceiling).
///
/// The alternative to declaring a height is a gallery with a hole in it and a
/// binary that exits non-zero on every run, which is an artifact nobody can use.
/// So the height is chosen here, in a table a reader can see, and the manifest
/// says `declared height 668` on the line it produced, so a declared image is
/// never mistaken for a measured one.
///
/// # How this table rots, and what stops it
///
/// A stale entry — an id the demo no longer publishes — is caught by
/// `catalog_tests::every_declared_height_names_a_target_that_still_exists`. A
/// height that has become too small is caught by
/// `capture_tests::a_declared_height_leaves_no_part_of_the_section_squeezed`,
/// which compares every text line at the declared height against the same
/// section laid out with plenty of room, and a row whose section has grown a
/// natural height again is caught by
/// `catalog_tests::a_declared_height_is_only_declared_for_a_section_that_cannot_be_measured`.
///
/// # When to delete a row
///
/// When hoarfrost gives `empty_state` a natural height — a `Length::Shrink`
/// container, or a fixed content height — the section becomes measurable, the
/// measure pass starts returning a real number, and this row must be deleted so
/// the image is measured again instead of declared. Nothing here forces the
/// widget to stay broken.
pub const DECLARED_HEIGHTS: [(&str, f32); 1] = [(EMPTY_STATES_ID, EMPTY_STATES_HEIGHT)];

/// The height `section_id` is drawn at because it has no natural one, or `None`
/// when the measure pass can be trusted to find out.
///
/// The one question [`mod@crate::sweep`] asks this table, phrased so the sweep never
/// has to know how many rows there are or what the ids are.
#[must_use]
pub fn declared_height(section_id: &str) -> Option<f32> {
    DECLARED_HEIGHTS
        .iter()
        .find(|(id, _)| *id == section_id)
        .map(|(_, height)| *height)
}

/// The sections one source produces, in its own order.
///
/// The single place a `Source` is turned into sections, so [`enumerate`] and
/// [`Catalog::build`] cannot drift apart about what a target's index means.
fn sections<'a>(
    atoms: &'a AtomsState,
    composites: &'a CompositesState,
    source: Source,
) -> Vec<Section<'a>> {
    match source {
        Source::LiveAtoms => hoarfrost_demo::pane_atoms::live_sections(atoms),
        Source::AtomPreviews => hoarfrost_demo::previews::atom_preview_sections(),
        Source::CompositeGroup1 => hoarfrost_demo::pane_composites::group1_sections(composites),
        Source::CompositeGroup2 => hoarfrost_demo::pane_composites::group2_sections(composites),
        Source::CompositePreviews => hoarfrost_demo::previews::composite_preview_sections(),
    }
}

/// Every section the demo shows, as targets, in window order.
///
/// The id is read off the section itself rather than recomputed here, so the
/// catalog cannot name a section differently from the way the element publishes
/// itself — which is the difference between the capture driver finding its card
/// and reporting `CardNotFound`.
fn enumerate(atoms: &AtomsState, composites: &CompositesState) -> Vec<Target> {
    Source::ORDER
        .iter()
        .flat_map(|source| {
            sections(atoms, composites, *source)
                .into_iter()
                .enumerate()
                .map(move |(index, section)| Target {
                    id: section.id,
                    source: *source,
                    index,
                })
        })
        .collect()
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;
