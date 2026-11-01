//! Tests for the entry point's composition step.
//!
//! `main` itself cannot be called from a test, so the exit-code decision it makes
//! lives in [`execute`], which returns a code instead of ending the process. That
//! is the whole reason [`execute`] exists: it makes the exit paths observable.
//! Every directory here is a `tempfile::TempDir` child, and the refused-path case
//! is refused *before* anything is written (**RUST-006**).
//!
//! The tests that really sweep use a narrow filter so they render one or a few
//! sections rather than the whole catalog; the full sweep is proven once, in
//! `sweep_tests.rs`.

use std::path::Path;
use std::path::PathBuf;

use hoarfrost_gallery::catalog::Catalog;
use hoarfrost_gallery::cli;
use hoarfrost_gallery::output;
use hoarfrost_gallery::strings;

use super::*;

/// A filter fragment that matches exactly one resolver-only preview section.
const ONE_SECTION: &str = "card-resolver";

/// A filter fragment every resolver-only preview carries and no live section does.
const PREVIEWS: &str = "resolver";

/// Options that sweep a deliberately small slice of the catalog into `target`.
fn sweeping(filter: &str, target: &Path) -> Options {
    Options {
        output_dir: target.to_path_buf(),
        filter: Some(filter.to_owned()),
        ..Options::default()
    }
}

/// The manifest text a run left behind.
fn manifest_text(target: &Path) -> String {
    std::fs::read_to_string(target.join(strings::MANIFEST_FILE_NAME))
        .expect("the run writes its manifest")
}

/// How many manifest lines record a written PNG.
fn rendered_lines(text: &str) -> usize {
    text.lines()
        .filter(|line| line.starts_with(strings::STATUS_RENDERED))
        .count()
}

/// The PNG files in a directory.
fn pngs_in(target: &Path) -> Vec<std::fs::DirEntry> {
    std::fs::read_dir(target)
        .expect("the output directory is readable")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".png"))
        .collect()
}

#[test]
fn a_real_run_writes_its_png_and_a_manifest_and_exits_zero() {
    let dir = tempfile::TempDir::new().expect("a temp directory");
    let target = dir.path().join("pngs");

    assert_eq!(execute(&sweeping(ONE_SECTION, &target)), EXIT_OK);
    assert!(
        target.is_dir(),
        "the output directory must be created: {}",
        target.display()
    );

    let text = manifest_text(&target);
    assert!(
        text.contains(strings::MANIFEST_HEADER),
        "the manifest must carry its header: {text}"
    );
    assert_eq!(
        rendered_lines(&text),
        1,
        "one section was asked for:\n{text}"
    );

    let pngs = pngs_in(&target);
    assert_eq!(pngs.len(), 1, "one accepted section, one PNG");
    assert!(
        pngs.iter()
            .all(|entry| entry.metadata().expect("a file").len() > 0),
        "a rendered PNG is never an empty file"
    );
}

#[test]
fn a_refused_output_directory_exits_with_the_request_code_and_writes_nothing() {
    let Ok(appdata) = std::env::var("APPDATA") else {
        return;
    };
    let target = PathBuf::from(appdata).join("gallery-round-2");

    let options = Options {
        output_dir: target.clone(),
        ..Options::default()
    };

    assert_eq!(execute(&options), EXIT_BAD_REQUEST);
    assert!(
        !target.exists(),
        "a refused path must not be created: {}",
        target.display()
    );
}

#[test]
fn a_directory_that_cannot_be_created_exits_with_the_write_code() {
    // A plain file standing where the output directory should go: the guard is
    // happy with the path, so this is the create step's failure, and it has to
    // read as a write failure rather than as bad arguments.
    let dir = tempfile::TempDir::new().expect("a temp directory");
    let blocker = dir.path().join("blocker");
    std::fs::write(&blocker, b"not a directory").expect("the seed file");

    let options = Options {
        output_dir: blocker.join("pngs"),
        ..Options::default()
    };

    assert_eq!(execute(&options), EXIT_RENDER_FAILED);
}

#[test]
fn a_section_that_cannot_be_written_fails_the_run_without_stopping_the_sweep() {
    // One accepted preview section has a *directory* parked at the name its PNG
    // would be written to, so that one write fails. The run must still render
    // the rest, record the failure in the manifest, and exit non-zero.
    let dir = tempfile::TempDir::new().expect("a temp directory");
    let target = dir.path().join("pngs");
    std::fs::create_dir_all(&target).expect("the output directory");

    let settings = sweeping(PREVIEWS, &target);
    let blocked = Catalog::new()
        .targets()
        .iter()
        .find(|section| cli::accepts(&settings, section.id()))
        .expect("the previews are in the catalog")
        .clone();
    std::fs::create_dir(target.join(output::png_file_name(blocked.id())))
        .expect("a directory parked where a PNG should go");

    assert_eq!(execute(&settings), EXIT_RENDER_FAILED);

    let text = manifest_text(&target);
    assert!(
        text.contains(strings::STATUS_FAILED),
        "the blocked section is recorded as a failure:\n{text}"
    );
    assert!(
        rendered_lines(&text) >= 1,
        "the sections after the blocked one must still be rendered:\n{text}"
    );
}

#[test]
fn a_manifest_that_cannot_be_written_exits_with_the_write_code() {
    // A directory standing where `manifest.txt` goes: the sweep succeeds, the
    // index does not, and a run whose manifest is missing is not a clean run.
    let dir = tempfile::TempDir::new().expect("a temp directory");
    let target = dir.path().join("pngs");
    std::fs::create_dir_all(&target).expect("the output directory");
    std::fs::create_dir(target.join(strings::MANIFEST_FILE_NAME))
        .expect("a directory parked where the manifest should go");

    assert_eq!(execute(&sweeping(ONE_SECTION, &target)), EXIT_RENDER_FAILED);
}

#[test]
fn the_exit_codes_match_the_gallery_binary() {
    // A script driving either hoarfrost tool must be able to read the same
    // numbers, so these are pinned rather than left to drift.
    assert_eq!(EXIT_OK, 0);
    assert_eq!(EXIT_RENDER_FAILED, 1);
    assert_eq!(EXIT_BAD_REQUEST, 2);
}

#[test]
fn unparsable_arguments_are_refused_before_anything_is_written() {
    // The parse itself is proven in `cli_tests.rs`; what matters here is that
    // `main`'s three-way split keeps a bad request out of `execute` entirely.
    assert!(matches!(
        cli::parse(&["--nope".to_owned()]),
        cli::Outcome::Invalid(_)
    ));
}
