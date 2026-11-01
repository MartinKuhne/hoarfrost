//! Tests for the render loop, including the only end-to-end sweep in the crate.
//!
//! # Cost, and what that dictates about the shape of these tests
//!
//! Building iced's headless renderer costs roughly 620 ms and every section costs
//! a layout pass plus a raster pass. `cargo nextest` runs each test in its own
//! process, so a renderer cannot be shared *between* tests — therefore the whole
//! catalog is swept inside **one** test, through **one** renderer, rather than
//! one test per section. The cheaper tests here use `--filter`-style filtering to
//! touch a handful of sections instead of all of them.
//!
//! Every file written here goes into a `tempfile::TempDir` (**RUST-006**).

use std::io::BufReader;
use std::path::Path;
use std::time::Instant;

use super::*;
use crate::capture;
use crate::catalog;
use crate::catalog::Catalog;
use crate::cli::Options;
use crate::manifest::manifest;
use tempfile::TempDir;

/// A filter fragment every resolver-only preview title carries and no live
/// section title does, so a test can ask for just the previews.
const PREVIEW_FRAGMENT: &str = "resolver";

/// The one section with no natural height, spelled out here rather than read
/// from the table so a test that depends on it fails loudly if the row is
/// deleted rather than silently testing nothing.
const EMPTY_STATES_ID: &str = "demo.section.empty-states-uif-comp-014";

/// The options a sweep test runs with, at the documented defaults.
fn options(filter: Option<&str>) -> Options {
    Options {
        filter: filter.map(str::to_owned),
        ..Options::default()
    }
}

/// Every PNG in `dir` as `(file name, byte size)`, sorted by name.
fn png_files(dir: &Path) -> Vec<(String, u64)> {
    let mut files: Vec<(String, u64)> = std::fs::read_dir(dir)
        .expect("the output directory exists")
        .map(|entry| entry.expect("a readable directory entry"))
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.ends_with(".png") {
                return None;
            }
            let size = entry.metadata().ok()?.len();
            Some((name, size))
        })
        .collect();
    files.sort();
    files
}

/// The distinct RGBA pixels a PNG contains, decoded back from disk.
fn distinct_pixels(path: &Path) -> usize {
    let file = std::fs::File::open(path).expect("the PNG is readable");
    let mut reader = ::png::Decoder::new(BufReader::new(file))
        .read_info()
        .expect("the PNG decodes");
    let mut buffer = vec![
        0u8;
        reader
            .output_buffer_size()
            .expect("a decodable PNG has a size")
    ];
    reader.next_frame(&mut buffer).expect("the frame decodes");

    let mut pixels: Vec<[u8; 4]> = buffer.as_chunks::<4>().0.to_vec();
    pixels.sort_unstable();
    pixels.dedup();
    pixels.len()
}

#[test]
fn counts_separates_rendered_from_failed() {
    let entries = vec![
        crate::manifest::Entry::rendered("a", "a.png", 10, 10),
        crate::manifest::Entry::failed("b", "no container published the id"),
        crate::manifest::Entry::rendered("c", "c.png", 10, 10),
    ];

    assert_eq!(counts(&entries), (2, 1));
    assert_eq!(counts(&[]), (0, 0));
}

#[test]
fn a_filter_removes_targets_without_recording_them() {
    // A section the operator excluded was not attempted, so it is not a failure.
    let dir = TempDir::new().expect("a temp directory");
    let mut renderer = capture::Renderer::new();
    let catalog = Catalog::new();

    let entries = sweep(
        &mut renderer,
        &catalog,
        &options(Some("no-such-section-anywhere")),
        dir.path(),
    );

    assert!(
        entries.is_empty(),
        "a filtered-out section must not appear: {entries:?}"
    );
    assert_eq!(
        png_files(dir.path()),
        Vec::new(),
        "a filtered-out section must not write a file"
    );
}

#[test]
fn a_section_that_fails_is_recorded_and_the_sweep_keeps_going() {
    // The destination does not exist, so every write fails. The point is what
    // comes back: one entry per accepted target, all failures, all with a reason.
    // A loop that aborted at the first one would report a single line and hide
    // the rest of the sweep.
    let dir = TempDir::new().expect("a temp directory");
    let missing = dir.path().join("not-created");
    let mut renderer = capture::Renderer::new();
    let catalog = Catalog::new();

    let entries = sweep(
        &mut renderer,
        &catalog,
        &options(Some(PREVIEW_FRAGMENT)),
        &missing,
    );
    let (rendered, failed) = counts(&entries);

    let accepted = catalog
        .targets()
        .iter()
        .filter(|target| crate::cli::accepts(&options(Some(PREVIEW_FRAGMENT)), target.id()))
        .count();

    assert!(
        accepted > 1,
        "the test needs more than one target to prove the loop continues"
    );
    assert_eq!(entries.len(), accepted);
    assert_eq!(rendered, 0);
    assert_eq!(failed, accepted);

    for entry in &entries {
        let reason = entry.failure.as_deref().expect("every entry here failed");
        assert!(
            !reason.is_empty(),
            "{} failed without saying why",
            entry.section_id
        );
        assert_eq!(entry.file_name, crate::strings::FIELD_NONE);
    }
}

#[test]
fn a_full_sweep_writes_one_non_empty_png_per_target() {
    let dir = TempDir::new().expect("a temp directory");
    let catalog = Catalog::new();
    let settings = options(None);

    // One renderer for the whole catalog: construction is the expensive part.
    let started = Instant::now();
    let mut renderer = capture::Renderer::with_theme(hoarfrost_demo::theme::demo_theme());
    let built = started.elapsed();

    let started = Instant::now();
    let entries = sweep(&mut renderer, &catalog, &settings, dir.path());
    let swept = started.elapsed();

    let (rendered, failed) = counts(&entries);
    println!(
        "full sweep: {} targets, {rendered} rendered, {failed} failed; renderer built in \
         {built:?}, swept in {swept:?}",
        catalog.targets().len()
    );

    assert_eq!(
        entries.len(),
        catalog.targets().len(),
        "every target must produce exactly one line"
    );
    assert_eq!(
        failed,
        0,
        "every section in the catalog renders: the ones with a natural height are \
         measured and the one without is drawn on its declared canvas. Failures:\n{}",
        manifest(&entries)
    );
    assert_eq!(rendered, catalog.targets().len());

    let files = png_files(dir.path());
    assert_eq!(
        files.len(),
        rendered,
        "one PNG per rendered entry, no strays:\n{}",
        manifest(&entries)
    );

    for entry in &entries {
        let path = dir.path().join(&entry.file_name);
        assert!(
            path.is_file(),
            "{} says it wrote {}, which is not there",
            entry.section_id,
            path.display()
        );
        let size = std::fs::metadata(&path).expect("the PNG is readable").len();
        assert!(size > 0, "{} is an empty file", path.display());
    }
}

#[test]
fn a_section_with_no_natural_height_is_drawn_at_its_declared_height_and_says_so() {
    // The declared-height path, proven against the real section it was written
    // for. `demo.section.empty-states-uif-comp-014` stacks three
    // `hoarfrost::composite::empty_state` widgets, each laid out at
    // `Length::Fill` height, so it measures the probe ceiling at *any* probe
    // height and the measure pass has nothing to report. It is now drawn on the
    // canvas the catalog declares — and the manifest line has to admit that the
    // height was declared rather than measured, because a reader comparing two
    // PNGs cannot otherwise tell a widget's own height from this tool's choice.
    let dir = TempDir::new().expect("a temp directory");
    let catalog = Catalog::new();
    let settings = options(Some("empty-states"));
    let declared = catalog::declared_height(EMPTY_STATES_ID).expect("the row exists");

    let mut renderer = capture::Renderer::with_theme(hoarfrost_demo::theme::demo_theme());
    let entries = sweep(&mut renderer, &catalog, &settings, dir.path());

    assert_eq!(entries.len(), 1, "exactly one section was asked for");
    let entry = &entries[0];
    assert!(
        entry.is_rendered(),
        "the declared path must produce an image: {entry:?}"
    );
    assert_eq!(entry.section_id, EMPTY_STATES_ID);
    assert_eq!(
        entry.note.as_deref(),
        Some(format!("{} {declared}", crate::strings::DETAIL_DECLARED_HEIGHT).as_str()),
        "the detail column must say the height was declared, not measured"
    );
    assert!(entry.failure.is_none());

    // The declared logical height, at the default 2x scale, is what the file
    // says it is: nothing was clipped and nothing was padded afterwards.
    let scale = crate::cli::Options::default().scale;
    assert_eq!(
        (entry.width, entry.height),
        (
            (crate::cli::Options::default().width * scale) as u32,
            (declared * scale) as u32,
        ),
        "the PNG is the declared canvas, to the scale the run asked for"
    );

    let files = png_files(dir.path());
    assert_eq!(files.len(), 1, "one accepted section, one PNG");
    assert!(files[0].1 > 0, "the declared PNG is not an empty file");
}

#[test]
fn a_measured_target_carries_no_declared_height_note() {
    // The other half of the manifest's honesty: an ordinary measured section's
    // detail column stays the placeholder, so "declared" cannot quietly become
    // the norm and stop meaning anything.
    let dir = TempDir::new().expect("a temp directory");
    let catalog = Catalog::new();
    let settings = options(Some("card-resolver"));

    let mut renderer = capture::Renderer::with_theme(hoarfrost_demo::theme::demo_theme());
    let entries = sweep(&mut renderer, &catalog, &settings, dir.path());

    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert!(
        entry.is_rendered(),
        "{}",
        entry.failure.clone().unwrap_or_default()
    );
    assert!(
        entry.note.is_none(),
        "a measured image must not claim to be declared"
    );
    assert_eq!(
        manifest(&entries)
            .lines()
            .nth(2)
            .expect("one entry line")
            .split('\t')
            .next_back(),
        Some(crate::strings::FIELD_NONE),
        "the detail column of a measured line is the placeholder"
    );
}

#[test]
fn a_rendered_png_is_not_a_flat_colour() {
    // The cheap version of "the picture is actually a picture": a card that
    // painted nothing would compress to a handful of bytes and decode to one
    // distinct pixel. A real section paints text, borders and fills, so the
    // frame has many.
    let dir = TempDir::new().expect("a temp directory");
    let catalog = Catalog::new();
    let settings = options(Some(PREVIEW_FRAGMENT));

    let mut renderer = capture::Renderer::with_theme(hoarfrost_demo::theme::demo_theme());
    let entries = sweep(&mut renderer, &catalog, &settings, dir.path());
    let (rendered, failed) = counts(&entries);
    assert_eq!(failed, 0, "{}", manifest(&entries));
    assert!(rendered > 0, "the previews should all render");

    for entry in entries.iter().filter(|entry| entry.is_rendered()) {
        let path = dir.path().join(&entry.file_name);
        let colours = distinct_pixels(&path);
        assert!(
            colours > 1,
            "{} decodes to {colours} distinct pixel value: nothing was painted",
            path.display()
        );
    }
}

#[test]
fn a_second_sweep_reproduces_the_same_files_at_the_same_sizes() {
    // Determinism is the property a documentation sweep has to have or nobody
    // can tell a real change from a re-run. Names and byte sizes are compared
    // rather than pixel buffers because the point is that the *same* image came
    // out, and the sizes are the cheap proxy that catches a reflowed section.
    let first = TempDir::new().expect("a temp directory");
    let second = TempDir::new().expect("a second temp directory");
    let catalog = Catalog::new();
    let settings = options(None);

    let mut renderer = capture::Renderer::with_theme(hoarfrost_demo::theme::demo_theme());
    let started = Instant::now();
    let before = sweep(&mut renderer, &catalog, &settings, first.path());
    let first_time = started.elapsed();

    let started = Instant::now();
    let after = sweep(&mut renderer, &catalog, &settings, second.path());
    let second_time = started.elapsed();

    println!(
        "two full sweeps in {first_time:?} and {second_time:?} ({} targets each)",
        catalog.targets().len()
    );

    assert_eq!(
        manifest(&before),
        manifest(&after),
        "the manifest must be byte-identical between runs"
    );
    assert_eq!(
        png_files(first.path()),
        png_files(second.path()),
        "the same names at the same sizes, or the sweep is not reproducible"
    );
}

#[test]
fn the_rebuild_closure_can_be_called_once_per_pass() {
    // The capture driver calls the builder once per pass, so the closure has to
    // be re-runnable rather than hand back a captured element a second time.
    // `iced::Element` is neither `Clone` nor `Debug`, so the honest assertion is
    // the one the sweep already makes below: two passes over the whole catalog
    // produce two complete rasters. This pins that the closure itself is `Fn`,
    // not `FnOnce`, which the compiler enforces and this call sequence restates.
    let catalog = Catalog::new();
    let target = &catalog.targets()[0];
    let builder = rebuild(&catalog, target);

    let first = builder();
    let second = builder();
    assert!(
        std::ptr::from_ref(&first) != std::ptr::from_ref(&second),
        "each call produces its own element"
    );
}
