//! The index a sweep leaves beside its images: one line per section, in the
//! order the catalog produced them.
//!
//! The manifest is what makes a run inspectable after the fact. A directory of
//! PNGs says what succeeded; only the manifest says what was *asked for*, so a
//! section that silently disappeared is visible as a `failed` line rather than
//! as an absence nobody notices. It is also where the gap the plan refuses to
//! hide is recorded: hover, pressed and focused states are not in a static
//! section, and the failure lines say so.
//!
//! # Determinism
//!
//! No clock, no hash-ordered iteration, no absolute paths. The text is a pure
//! function of the entries handed in, in the order they were handed in, which
//! is what lets two runs of the same sweep be compared byte for byte.
//!
//! Unit tests live in the sibling `manifest_tests.rs` sidecar.

use crate::strings;

/// The character that separates one manifest field from the next.
///
/// A tab rather than aligned spaces, so the file is trivially splittable by a
/// script and cannot be broken by a section id that happens to contain a space.
const FIELD_SEPARATOR: char = '\t';

/// The comment prefix every non-entry line carries.
const COMMENT_PREFIX: &str = "#";

/// What happened to one section.
///
/// A rendered entry carries its file name and pixel size; a failed one carries
/// the reason and nothing else, so a reader never has to guess which fields are
/// meaningful for which status.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The section's stable id, exactly as the catalog named it.
    pub section_id: String,
    /// The file the PNG was written under, or [`strings::FIELD_NONE`] when
    /// nothing was written.
    pub file_name: String,
    /// Physical pixel width, or `0` when nothing was written.
    pub width: u32,
    /// Physical pixel height, or `0` when nothing was written.
    pub height: u32,
    /// Why the section produced no image, if it did not.
    pub failure: Option<String>,
    /// Something a reader needs to know about an image that *was* written, and
    /// which the pixel size alone cannot tell them — today, that the height was
    /// declared by the catalog rather than measured by the probe.
    ///
    /// Kept apart from `failure` because the two are opposite outcomes: an entry
    /// that rendered *and* failed would make the status column a lie.
    pub note: Option<String>,
}

impl Entry {
    /// A section that produced `file_name` at `width` × `height` pixels.
    #[must_use]
    pub fn rendered(
        section_id: impl Into<String>,
        file_name: impl Into<String>,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            section_id: section_id.into(),
            file_name: file_name.into(),
            width,
            height,
            failure: None,
            note: None,
        }
    }

    /// A section that produced an image at a height somebody *chose* rather than
    /// one the probe measured.
    ///
    /// The detail column says so, because a declared height is a different kind
    /// of evidence: it is the tool's decision, not the widget's measurement, and
    /// a reader comparing two PNGs needs to know which is which.
    #[must_use]
    pub fn rendered_declared(
        section_id: impl Into<String>,
        file_name: impl Into<String>,
        width: u32,
        height: u32,
        declared_height: f32,
    ) -> Self {
        Self {
            section_id: section_id.into(),
            file_name: file_name.into(),
            width,
            height,
            failure: None,
            note: Some(format!(
                "{} {declared_height}",
                strings::DETAIL_DECLARED_HEIGHT
            )),
        }
    }

    /// A section that produced no image, with the reason a reader can act on.
    ///
    /// There is no file name and no size to report, so both are filled with the
    /// placeholder rather than left empty — an empty field in a tab-separated
    /// line reads as a parsing bug.
    #[must_use]
    pub fn failed(section_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            section_id: section_id.into(),
            file_name: strings::FIELD_NONE.to_owned(),
            width: 0,
            height: 0,
            failure: Some(reason.into()),
            note: None,
        }
    }

    /// Whether this line records a PNG that was actually written.
    #[must_use]
    pub fn is_rendered(&self) -> bool {
        self.failure.is_none()
    }
}

/// How many entries rendered, and how many did not.
#[must_use]
fn counts(entries: &[Entry]) -> (usize, usize) {
    let rendered = entries.iter().filter(|entry| entry.is_rendered()).count();
    (rendered, entries.len() - rendered)
}

/// One entry as a manifest line.
///
/// The detail column carries whichever of the two there is: a failure reason, or
/// a note about an image that was written. A rendered entry with nothing to
/// qualify it gets the placeholder, so the column is never empty.
fn entry_line(entry: &Entry) -> String {
    let status = if entry.is_rendered() {
        strings::STATUS_RENDERED
    } else {
        strings::STATUS_FAILED
    };
    let detail = entry
        .failure
        .as_deref()
        .or(entry.note.as_deref())
        .unwrap_or(strings::FIELD_NONE);

    [
        status.to_owned(),
        entry.section_id.clone(),
        entry.file_name.clone(),
        format!("{}x{}", entry.width, entry.height),
        detail.to_owned(),
    ]
    .join(&FIELD_SEPARATOR.to_string())
}

/// A trailing summary line: `# <label> <count>`.
fn summary_line(label: &str, count: usize) -> String {
    format!("{COMMENT_PREFIX} {label} {count}")
}

/// The manifest text for `entries`, in the order given.
///
/// A header, a column-names line, one line per entry, then the counts. Empty
/// input still produces a well-formed manifest: the header and a summary of
/// zeros, so a reader can tell "nothing was asked for" from "the file is
/// truncated".
#[must_use]
pub fn manifest(entries: &[Entry]) -> String {
    let mut lines: Vec<String> = Vec::with_capacity(entries.len() + 5);

    lines.push(strings::MANIFEST_HEADER.to_owned());
    lines.push(strings::MANIFEST_COLUMNS.to_owned());
    lines.extend(entries.iter().map(entry_line));

    let (rendered, failed) = counts(entries);
    lines.push(String::new());
    lines.push(summary_line(strings::SUMMARY_RENDERED, rendered));
    lines.push(summary_line(strings::SUMMARY_FAILED, failed));
    lines.push(summary_line(strings::SUMMARY_TOTAL, entries.len()));

    let mut text = lines.join("\n");
    text.push('\n');
    text
}

#[cfg(test)]
#[path = "manifest_tests.rs"]
mod tests;
