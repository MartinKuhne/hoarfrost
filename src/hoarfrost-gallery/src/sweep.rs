//! The render loop: one catalog target in, one manifest line out.
//!
//! # Why the loop lives here and not in `main`
//!
//! An entry point composes; it does not implement. Deciding what happens when a
//! section fails is the most consequential decision in a sweep, and it has to be
//! reachable from a test — which it is not if it is written after the first
//! `println!` in `main`.
//!
//! # One failure never stops the run
//!
//! Every target produces exactly one [`crate::manifest::Entry`] and no target can
//! interrupt the loop. A section whose card cannot be found, or whose PNG cannot
//! be written, is recorded with its reason and the sweep moves to the next one.
//!
//! That is deliberate. The point of a sweep is to say which sections produced an
//! image and which did not; a run that aborted at the first bad section would
//! hide every section after it, which is strictly worse than the bug it stopped
//! for. The exit code is where the failure is escalated, not the loop.
//!
//! # What a filtered-out target is
//!
//! A target `cli::accepts` rejects produces **no entry at all** — not a failed
//! one. The manifest describes what the run attempted; a section the operator
//! deliberately excluded was not attempted, and recording it as a failure would
//! make a filtered run look broken.
//!
//! # Measured heights and declared ones
//!
//! Most targets are measured and drawn by [`capture::Renderer::shot`]. A target
//! the catalog declares a height for — because its widget has no natural height
//! to measure, see [`crate::catalog::DECLARED_HEIGHTS`] — is drawn at that height
//! by [`capture::Renderer::shot_at`] instead, and its manifest line says
//! `declared height <n>` in the detail column. A reader can therefore tell, from
//! the manifest alone, which images record what a widget asked for and which
//! record a canvas this tool chose.
//!
//! Unit tests live in the sibling `sweep_tests.rs` sidecar.

use std::path::Path;

use hoarfrost_demo::app::Message;
use iced::Element;

use crate::capture;
use crate::catalog::{Catalog, Target};
use crate::cli::{self, Options};
use crate::manifest::Entry;
use crate::output;
use crate::png;

/// Render every accepted target into `destination` and report each outcome.
///
/// `renderer` is passed in, not built here, because building iced's headless
/// renderer is the expensive part of a run and a caller that sweeps twice — or
/// sweeps and then re-sweeps to check stability — pays it once.
///
/// The returned entries are in catalog order, filtered by
/// [`cli::accepts`], and there is exactly one per target that survived the
/// filter.
pub fn sweep(
    renderer: &mut capture::Renderer,
    catalog: &Catalog,
    options: &Options,
    destination: &Path,
) -> Vec<Entry> {
    catalog
        .targets()
        .iter()
        .filter(|target| cli::accepts(options, target.id()))
        .map(|target| render_one(renderer, catalog, options, destination, target))
        .collect()
}

/// Count the entries that produced an image, and the ones that did not.
#[must_use]
pub fn counts(entries: &[Entry]) -> (usize, usize) {
    let rendered = entries.iter().filter(|entry| entry.is_rendered()).count();
    (rendered, entries.len() - rendered)
}

/// Measure, rasterise and write one target, or say why it did not happen.
///
/// A target the catalog declares a height for goes through
/// [`capture::Renderer::shot_at`] and everything else through
/// [`capture::Renderer::shot`]. The two are not interchangeable: the declared
/// path has no measure pass, so it cannot detect a card that lost its id, and
/// the measured path cannot measure a widget with no natural height. Which one
/// ran is recorded in the entry's detail column, so the manifest says which
/// heights are the widget's and which are the tool's.
fn render_one(
    renderer: &mut capture::Renderer,
    catalog: &Catalog,
    options: &Options,
    destination: &Path,
    target: &Target,
) -> Entry {
    let file_name = output::png_file_name(target.id());
    let declared = crate::catalog::declared_height(target.id());

    let shot = match declared {
        Some(height) => renderer.shot_at(
            target.id(),
            options.width,
            height,
            options.scale,
            capture::DEFAULT_BACKGROUND,
            rebuild(catalog, target),
        ),
        None => renderer.shot(
            target.id(),
            options.width,
            options.scale,
            capture::DEFAULT_BACKGROUND,
            rebuild(catalog, target),
        ),
    };

    let shot = match shot {
        Ok(shot) => shot,
        Err(why) => return Entry::failed(target.id(), why.to_string()),
    };

    if let Err(why) = png::write_png(
        &destination.join(&file_name),
        shot.width,
        shot.height,
        &shot.rgba,
    ) {
        return Entry::failed(target.id(), why.to_string());
    }

    match declared {
        Some(height) => {
            Entry::rendered_declared(target.id(), file_name, shot.width, shot.height, height)
        }
        None => Entry::rendered(target.id(), file_name, shot.width, shot.height),
    }
}

/// The builder the capture driver calls once per pass.
///
/// It re-runs the catalog's source function rather than reusing an element,
/// because `iced::Element` is not `Clone` and the first pass moves its element
/// into the interface that measures it (see the "Why `build` re-runs the source
/// function" note in [`crate::catalog`]).
///
/// A target the catalog can no longer rebuild yields an empty element instead of
/// a panic. The capture driver will then report that no container published the
/// id, which is both true and a line in the manifest — the alternative is a
/// sweep that dies halfway through and reports nothing about the sections it
/// never reached.
fn rebuild<'a>(catalog: &'a Catalog, target: &'a Target) -> impl Fn() -> Element<'a, Message> + 'a {
    move || {
        catalog
            .build(target)
            .unwrap_or_else(|| iced::widget::Space::new().into())
    }
}

#[cfg(test)]
#[path = "sweep_tests.rs"]
mod tests;
