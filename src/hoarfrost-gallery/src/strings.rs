//! Every string this program puts in front of a human, collected in one place
//! so a reader or a translation pass can see the whole CLI surface at once
//! (`HF-001`, `RUST-021`).
//!
//! Scope is text a person reads: usage, flag names, summary labels, refusal
//! reasons and failure messages. Filesystem constants that are *not* prose
//! (the `.hoarfrost` directory marker, the manifest's tab separator) stay with
//! the code that interprets them, because changing one of those changes
//! behaviour rather than wording.

/// The program name, as printed in usage and in the run summary.
pub const PROGRAM: &str = "hoarfrost-gallery";

/// One-line purpose printed at the top of the usage block.
pub const USAGE_PURPOSE: &str =
    "Render each hoarfrost demo section to its own PNG file (UIF-VER-004).";

/// Label above the usage synopsis line.
pub const USAGE_LABEL: &str = "Usage:";

/// The synopsis line: every argument the parser accepts, in one glance.
pub const USAGE_SYNOPSIS: &str = "  hoarfrost-gallery [OUTPUT_DIR] [--out <dir>] [--width <px>] [--scale <f>] [--filter <fragment>] [--help]";

/// Label above the per-flag help list.
pub const FLAGS_LABEL: &str = "Options:";

/// Label above the defaults block.
pub const DEFAULTS_LABEL: &str = "Defaults:";

/// The positional argument's name, as it appears in usage and defaults.
pub const ARG_OUTPUT_DIR: &str = "OUTPUT_DIR";

/// Flag that sets the card width.
pub const FLAG_WIDTH: &str = "--width";

/// Flag that sets the raster scale.
pub const FLAG_SCALE: &str = "--scale";

/// Flag that restricts the sweep to matching section ids.
pub const FLAG_FILTER: &str = "--filter";

/// Flag that prints the usage block and stops.
pub const FLAG_HELP: &str = "--help";

/// Flag that names the output directory.
pub const FLAG_OUT: &str = "--out";

/// Value placeholder shown after `--out`.
pub const PLACEHOLDER_DIR: &str = "<dir>";

/// Help line for `--out`.
pub const HELP_OUT: &str = "where the PNGs and manifest.txt are written";

/// Value placeholder shown after `--width`.
pub const PLACEHOLDER_WIDTH: &str = "<px>";

/// Value placeholder shown after `--scale`.
pub const PLACEHOLDER_SCALE: &str = "<f>";

/// Value placeholder shown after `--filter`.
pub const PLACEHOLDER_FILTER: &str = "<fragment>";

/// Help line for the positional output directory, accepted as the shorter way
/// of typing the same thing as `--out`.
pub const HELP_OUTPUT_DIR: &str = "same as --out, for the habit of typing a path first";

/// Help line for `--width`.
pub const HELP_WIDTH: &str = "card width in logical pixels";

/// Help line for `--scale`.
pub const HELP_SCALE: &str = "physical pixels per logical pixel";

/// Help line for `--filter`.
pub const HELP_FILTER: &str = "only sections whose id contains the fragment";

/// Help line for `--help`.
pub const HELP_HELP: &str = "print this text and exit";

/// The directory a run with no positional argument writes into. It lives under
/// `target/`, which the repository already ignores, so a sweep can never
/// accidentally commit its own output.
pub const DEFAULT_OUTPUT_DIR: &str = "target/gallery-png";

/// Prefix on every message written to stderr.
pub const ERROR_PREFIX: &str = "error: ";

/// Prefix on the refusal message, kept identical to `hoarfrost`'s own gallery
/// binary so the two tools refuse in the same voice.
pub const REFUSAL_PREFIX: &str = "refusing to write output: ";

/// Refusal reason for a path carrying a `.hoarfrost` component.
pub const REFUSAL_APP_COMPONENT: &str =
    "path contains a .hoarfrost application directory component";

/// Refusal reason for a path at or below a protected root. The environment
/// variable's name and the root it resolved to are appended by `output`.
pub const REFUSAL_BELOW_ROOT: &str = "path is at or below the";

/// Parse error for a flag that is not in the flag table.
pub const ERR_UNKNOWN_FLAG: &str = "unknown flag";

/// Parse error for a flag given with no value after it.
pub const ERR_MISSING_VALUE: &str = "expects a value";

/// Parse error for a numeric flag whose value is not a finite number.
pub const ERR_NOT_A_NUMBER: &str = "expects a number, got";

/// Parse error for a numeric flag whose value is zero, negative or infinite.
pub const ERR_NOT_POSITIVE: &str = "expects a value greater than zero, got";

/// Parse error for a `--filter` given an empty fragment.
pub const ERR_EMPTY_FILTER: &str = "expects a non-empty fragment";

/// Parse error for a `--out` given an empty directory.
pub const ERR_EMPTY_OUTPUT_DIR: &str = "expects a non-empty directory";

/// Parse error for naming the output directory twice, once by each route.
pub const ERR_OUT_GIVEN_TWICE: &str =
    "the output directory was given twice: once as a bare path and once with --out";

/// Parse error for a second positional argument.
pub const ERR_EXTRA_ARGUMENT: &str = "unexpected extra argument";

/// Failure recorded when a PNG cannot be written at zero width or height.
pub const ERR_PNG_ZERO_SIZE: &str = "cannot write a PNG with a zero width or height";

/// Failure recorded when the pixel buffer does not match the declared size.
pub const ERR_PNG_BUFFER: &str = "RGBA buffer length does not match width * height * 4";

/// Failure recorded when the measure pass cannot see a section's card.
pub const ERR_CARD_NOT_FOUND: &str = "no container published the id";

/// Failure recorded when a card measures as having no area.
pub const ERR_CARD_NO_AREA: &str = "measured a box with no area";

/// Failure recorded when a card measures as a non-finite rectangle.
pub const ERR_CARD_NOT_FINITE: &str = "measured a non-finite box";

/// Failure recorded when a card does not fit inside the measure probe, which
/// means the height it reported cannot be trusted.
pub const ERR_CARD_TOO_TALL: &str = "does not fit within the measure probe height";

/// Failure recorded when a declared render canvas has a non-finite dimension.
///
/// Worded as a predicate of the section id, like the other `ERR_*` strings, and
/// deliberately about a *declaration* rather than a measurement: the reader has
/// a different file to go and fix.
pub const ERR_DECLARED_NOT_FINITE: &str = "was declared a non-finite box";

/// Failure recorded when a declared render canvas is zero-sized or negative.
pub const ERR_DECLARED_NO_AREA: &str = "was declared a box with no area";

/// Prefix of the detail column on a line whose height was chosen by the catalog
/// rather than measured by the probe, so a declared image is never mistaken for
/// a measured one. The height follows it, separated by a space.
pub const DETAIL_DECLARED_HEIGHT: &str = "declared height";

/// Failure recorded when the rasteriser's buffer does not match the size it was
/// asked to draw.
pub const ERR_PIXEL_COUNT: &str =
    "the rasteriser returned a pixel count that does not match the image size";

/// Name of the index file written next to the PNGs.
pub const MANIFEST_FILE_NAME: &str = "manifest.txt";

/// Title line at the top of the manifest.
pub const MANIFEST_HEADER: &str = "# hoarfrost-gallery manifest (UIF-VER-004)";

/// Column names line, so the manifest is self-describing to a reader and to a
/// script that splits it on tabs.
pub const MANIFEST_COLUMNS: &str = "# status\tsection\tfile\tpixels\tdetail";

/// Status word for a section that produced a PNG.
pub const STATUS_RENDERED: &str = "rendered";

/// Status word for a section that did not.
pub const STATUS_FAILED: &str = "failed";

/// Placeholder for a field a failed entry has no value for.
pub const FIELD_NONE: &str = "-";

/// Label of the rendered-count line in the manifest's trailing summary. The
/// comment prefix is added by `manifest::summary_line`.
pub const SUMMARY_RENDERED: &str = "rendered:";

/// Label of the failed-count line in the manifest's trailing summary.
pub const SUMMARY_FAILED: &str = "failed:";

/// Label of the total-count line in the manifest's trailing summary.
pub const SUMMARY_TOTAL: &str = "total:";

/// Label of the output directory line in the run summary.
pub const LABEL_OUTPUT: &str = "output";

/// Label of the manifest path line in the run summary.
pub const LABEL_MANIFEST: &str = "manifest";

/// Label of the card width line in the run summary.
pub const LABEL_WIDTH: &str = "width";

/// Label of the raster scale line in the run summary.
pub const LABEL_SCALE: &str = "scale";

/// Label of the filter line in the run summary.
pub const LABEL_FILTER: &str = "filter";

/// Label of the rendered-count line in the run summary.
pub const LABEL_RENDERED: &str = "rendered";

/// Label of the failed-count line in the run summary.
pub const LABEL_FAILED: &str = "failed";

#[cfg(test)]
#[path = "strings_tests.rs"]
mod tests;
