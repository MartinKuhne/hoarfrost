//! Tests for the catalog of demo sections.
//!
//! These are the assertions that catch the two ways a catalog like this goes
//! wrong without anyone noticing: an id that appears twice, which means two
//! sections quietly write over the same PNG, and a target that no longer rebuilds
//! because the demo moved a section, which means a section quietly disappears.
//!
//! Nothing here rasterises anything (**RUST-006**): building an element is a
//! pure, in-memory operation and the raster is [`crate::capture`]'s job to prove.
//! One test does build a headless renderer, because the only honest way to ask
//! "does this section still have a natural height?" is to run the measure pass
//! and see whether a number comes back.

use super::*;

#[test]
fn the_catalog_is_not_empty() {
    let catalog = Catalog::new();
    assert!(
        !catalog.targets().is_empty(),
        "a sweep with nothing to render is a bug, not a success"
    );
}

#[test]
fn every_target_id_is_unique() {
    // The hazard this pins is real, not theoretical: a section id is derived from
    // its title by `section_slug`, so two sections titled the same way produce
    // the same id, the same sanitised stem and the same file name — and the
    // second PNG silently overwrites the first while the manifest reports two
    // successes.
    let catalog = Catalog::new();
    let mut seen: Vec<&str> = Vec::with_capacity(catalog.targets().len());

    for target in catalog.targets() {
        if let Some(first) = seen.iter().position(|id| *id == target.id()) {
            panic!(
                "id {:?} is claimed by target {first} and by target {}: two sections \
                 would write the same PNG",
                target.id(),
                seen.len(),
            );
        }
        seen.push(target.id());
    }
}

#[test]
fn every_target_id_sanitises_to_a_usable_stem() {
    let catalog = Catalog::new();

    for target in catalog.targets() {
        let stem = crate::output::sanitize_stem(target.id());
        assert!(
            !stem.is_empty(),
            "{:?} sanitises to nothing: it would be written as `.png`",
            target.id()
        );
        assert_ne!(
            stem,
            crate::output::FALLBACK_STEM,
            "{:?} sanitises to the fallback stem, so its file name says nothing \
             about which section it is",
            target.id()
        );
        assert!(
            target
                .id()
                .starts_with(hoarfrost_demo::section::SECTION_ID_PREFIX),
            "{:?} is not namespaced like a demo section",
            target.id()
        );
    }
}

#[test]
fn every_target_rebuilds() {
    // `build` re-runs the source function and indexes into it, so this is the
    // assertion that the stored (source, index) pairs still name something. A
    // `None` here means the demo changed shape and the sweep would silently lose
    // a section.
    let catalog = Catalog::new();

    for target in catalog.targets() {
        assert!(
            catalog.build(target).is_some(),
            "{:?} ({}[{}]) no longer rebuilds",
            target.id(),
            target.source_label(),
            target.index(),
        );
    }
}

#[test]
fn rebuilding_the_same_target_twice_gives_two_elements_not_one_shared_one() {
    // The premise the two-pass capture rests on. `Element` is not `Clone`, so
    // `build` has to produce a fresh tree every time it is called.
    let catalog = Catalog::new();
    let target = &catalog.targets()[0];

    let first = catalog.build(target);
    let second = catalog.build(target);
    assert!(first.is_some() && second.is_some());
}

#[test]
fn the_targets_follow_the_order_the_demo_window_shows() {
    let catalog = Catalog::new();
    let labels: Vec<&str> = catalog.targets().iter().map(Target::source_label).collect();

    let mut groups: Vec<&str> = Vec::new();
    for label in &labels {
        if groups.last() != Some(label) {
            groups.push(label);
        }
    }

    assert_eq!(
        groups,
        vec![
            "atoms/live",
            "atoms/previews",
            "composites/group1",
            "composites/group2",
            "composites/previews",
        ],
        "the sweep must walk the panes the way the window lays them out"
    );
}

#[test]
fn each_source_is_walked_from_zero_without_gaps() {
    let catalog = Catalog::new();
    let mut per_source: std::collections::HashMap<&str, Vec<usize>> =
        std::collections::HashMap::new();

    for target in catalog.targets() {
        per_source
            .entry(target.source_label())
            .or_default()
            .push(target.index());
    }

    for (source, indices) in &per_source {
        let expected: Vec<usize> = (0..indices.len()).collect();
        assert_eq!(
            *indices,
            expected,
            "{source} should be walked 0..{}, got {indices:?}",
            indices.len(),
        );
    }

    assert_eq!(per_source.len(), 5, "all five sources should be present");
}

#[test]
fn the_target_count_is_the_sum_of_the_five_sources() {
    // The catalog must not drop or duplicate a source function's output.
    let atoms = hoarfrost_demo::pane_atoms::AtomsState::default();
    let composites = hoarfrost_demo::pane_composites::CompositesState::default();

    let expected = hoarfrost_demo::pane_atoms::live_sections(&atoms).len()
        + hoarfrost_demo::previews::atom_preview_sections().len()
        + hoarfrost_demo::pane_composites::group1_sections(&composites).len()
        + hoarfrost_demo::pane_composites::group2_sections(&composites).len()
        + hoarfrost_demo::previews::composite_preview_sections().len();

    let catalog = Catalog::new();
    assert_eq!(catalog.targets().len(), expected);
}

#[test]
fn every_declared_height_names_a_target_that_still_exists() {
    // The table rots in one obvious way: the demo renames or drops a section and
    // the row keeps declaring a height for an id nothing publishes. Nothing would
    // fail at runtime — the row would simply never be consulted — so the only
    // thing that catches it is this assertion. A row that no longer names a real
    // section must be deleted, not left here looking load-bearing.
    let catalog = Catalog::new();

    for (id, _) in DECLARED_HEIGHTS {
        assert!(
            catalog.targets().iter().any(|target| target.id() == id),
            "{id} is declared but the catalog has no such target: the table is \
             stale, delete the row (or the section came back with a natural \
             height and no longer needs one)"
        );
    }
}

#[test]
fn every_declared_height_is_a_box_the_rasteriser_can_be_asked_for() {
    // A zero, negative or non-finite row would be refused at render time by
    // `shot_at`, which is the right behaviour but the wrong place to find out:
    // the table is a literal, so it can be checked here for free.
    for (id, height) in DECLARED_HEIGHTS {
        assert!(
            height.is_finite(),
            "{id} declares a non-finite height {height}"
        );
        assert!(
            height > 0.0,
            "{id} declares a height of {height}: an image of nothing is not an image"
        );
    }
}

#[test]
fn a_declared_height_is_only_declared_for_a_section_that_cannot_be_measured() {
    // The other direction the table can rot: a section that *could* be measured
    // gets a declared height anyway, and the image quietly stops reflecting what
    // the widget asked for. A section with a natural height measures strictly
    // below the probe ceiling, so a declared row whose section now measures
    // honestly is a row that should be deleted.
    //
    // This is asserted through the measure pass itself rather than through the
    // widget's source, because "has a natural height" means exactly "the probe
    // gets a number back".
    let catalog = Catalog::new();
    let mut renderer = crate::capture::Renderer::new();

    for (id, _) in DECLARED_HEIGHTS {
        let target = catalog
            .targets()
            .iter()
            .find(|candidate| candidate.id() == id)
            .unwrap_or_else(|| panic!("{id} is declared but not in the catalog"));

        let measured = renderer.measure(id, crate::cli::Options::default().width, || {
            catalog
                .build(target)
                .unwrap_or_else(|| panic!("{id} no longer rebuilds"))
        });

        assert!(
            measured.is_err(),
            "{id} now measures honestly ({measured:?}) but is still declared: \
             delete its row in crate::catalog::DECLARED_HEIGHTS so the image is measured"
        );
    }
}

#[test]
fn a_default_catalog_is_the_same_catalog_as_a_new_one() {
    let built = Catalog::new();
    let defaulted = Catalog::default();

    assert_eq!(
        built.targets().iter().map(Target::id).collect::<Vec<_>>(),
        defaulted
            .targets()
            .iter()
            .map(Target::id)
            .collect::<Vec<_>>(),
        "the catalog must be reproducible from the command line alone"
    );
}

#[test]
fn a_target_names_its_id_and_its_place() {
    let catalog = Catalog::new();
    let first = &catalog.targets()[0];

    assert!(!first.id().is_empty());
    assert_eq!(first.source_label(), "atoms/live");
    assert_eq!(first.index(), 0);
}
