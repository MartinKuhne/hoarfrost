//! The `hoarfrost-gallery` entry point: parse, guard, sweep, report — and
//! nothing else.
//!
//! An entry point composes rather than implements. Every decision lives in the
//! library, where it is reachable from a test; what is left here is the argument
//! vector, the process exit code, and the side effects a CLI exists to have:
//! create a directory, write files, print a summary.
//!
//! The exit codes mirror `src/hoarfrost/bin/gallery.rs`, so a script driving
//! either tool reads the same numbers: `0` success, `1` a render or write
//! failure, `2` a refused output path or unparsable arguments.
//!
//! # Why the font fallback is registered first
//!
//! `hoarfrost_demo::font_fallback::register()` is called before anything is
//! built, because the demo's own `DemoApp::boot` does the same and glyph
//! fallback resolution depends on the fallback face being in iced's font system
//! *before* the first frame is laid out. A sweep that skipped it would render
//! tofu where the demo window shows a glyph, and the PNGs would look like the
//! framework is broken when only the setup order was.
//!
//! # Why the theme is the demo's
//!
//! The renderer is built with `hoarfrost_demo::theme::demo_theme()` rather than
//! iced's stock dark palette, so iced's own primitives — the containers and text
//! the demo shell is built from — resolve to the same colours the demo window
//! shows. A PNG of a section that does not match the window is not evidence of
//! anything.

use std::path::Path;

use hoarfrost_gallery::capture;
use hoarfrost_gallery::catalog;
use hoarfrost_gallery::cli;
use hoarfrost_gallery::cli::Options;
use hoarfrost_gallery::cli::Outcome;
use hoarfrost_gallery::manifest;
use hoarfrost_gallery::output;
use hoarfrost_gallery::strings;
use hoarfrost_gallery::sweep;

/// Everything the run was asked for was produced.
const EXIT_OK: i32 = 0;

/// A render or a write failed.
const EXIT_RENDER_FAILED: i32 = 1;

/// The output path was refused, or the arguments did not parse.
const EXIT_BAD_REQUEST: i32 = 2;

/// The width of the label column in the run summary.
const LABEL_WIDTH: usize = 9;

/// Write one failure to stderr in the shape hoarfrost's own tools use.
fn fail(reason: &str) {
    eprintln!("{}{reason}", strings::ERROR_PREFIX);
}

/// Print the run summary, in the style of `hoarfrost`'s own gallery binary.
fn report(
    destination: &Path,
    manifest_path: &Path,
    options: &Options,
    rendered: usize,
    failed: usize,
) {
    println!("{} PNG sweep (UIF-VER-004)", strings::PROGRAM);
    println!(
        "  {:<LABEL_WIDTH$}: {}",
        strings::LABEL_OUTPUT,
        destination.display()
    );
    println!(
        "  {:<LABEL_WIDTH$}: {}",
        strings::LABEL_WIDTH,
        options.width
    );
    println!(
        "  {:<LABEL_WIDTH$}: {}",
        strings::LABEL_SCALE,
        options.scale
    );

    if let Some(fragment) = &options.filter {
        println!("  {:<LABEL_WIDTH$}: {fragment}", strings::LABEL_FILTER);
    }

    println!("  {:<LABEL_WIDTH$}: {rendered}", strings::LABEL_RENDERED);
    println!("  {:<LABEL_WIDTH$}: {failed}", strings::LABEL_FAILED);
    println!(
        "  {:<LABEL_WIDTH$}: {}",
        strings::LABEL_MANIFEST,
        manifest_path.display()
    );
}

/// Guard and create the output directory, sweep the catalog, write the manifest.
///
/// Returns the process exit code rather than calling `exit`, so the whole path
/// is observable from a test.
///
/// Any section that failed makes the run fail: the PNGs that *were* written are
/// still there and the manifest says what is missing, but a script should not be
/// able to read this run as a clean one.
fn execute(options: &Options) -> i32 {
    let destination = match output::guard(&options.output_dir) {
        Ok(path) => path,
        Err(error) => {
            fail(&error.to_string());
            return EXIT_BAD_REQUEST;
        }
    };

    if let Err(error) = std::fs::create_dir_all(&destination) {
        fail(&error.to_string());
        return EXIT_RENDER_FAILED;
    }

    hoarfrost_demo::font_fallback::register();

    let mut renderer = capture::Renderer::with_theme(hoarfrost_demo::theme::demo_theme());
    let catalog = catalog::Catalog::new();
    let entries = sweep::sweep(&mut renderer, &catalog, options, &destination);
    let (rendered, failed) = sweep::counts(&entries);

    let manifest_path = destination.join(strings::MANIFEST_FILE_NAME);
    if let Err(error) = std::fs::write(&manifest_path, manifest::manifest(&entries).as_bytes()) {
        fail(&error.to_string());
        return EXIT_RENDER_FAILED;
    }

    report(&destination, &manifest_path, options, rendered, failed);

    if failed > 0 {
        return EXIT_RENDER_FAILED;
    }
    EXIT_OK
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match cli::parse(&args) {
        Outcome::Help => print!("{}", cli::usage()),
        Outcome::Invalid(reason) => {
            fail(&reason);
            eprint!("{}", cli::usage());
            std::process::exit(EXIT_BAD_REQUEST);
        }
        Outcome::Run(options) => {
            let code = execute(&options);
            if code != EXIT_OK {
                std::process::exit(code);
            }
        }
    }
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;
