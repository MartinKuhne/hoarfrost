//! Where a sweep is allowed to write, and what an emitted file may be called.
//!
//! Two independent safety jobs live here. The first is a *refusal*: a mistyped
//! output argument must never land inside the application's own configuration
//! tree, because a documentation sweep that overwrites user state is worse than
//! a sweep that fails. The second is *naming*: a section id is a human-readable
//! string that may carry punctuation, spaces and spec references, and it has to
//! become a filename that is safe on Windows and stable across runs.
//!
//! # Provenance of the guard
//!
//! `FORBIDDEN_ROOT_VARS`, `APP_DIR_MARKER`, `at_or_below`, `refusal` and
//! [`guard`] are a deliberate copy of the guard in
//! `src/hoarfrost/bin/gallery.rs` (lines 36-120), adapted to this crate's error
//! strings. It is duplicated rather than extracted into `hoarfrost` on purpose:
//! moving it into the published library would widen that crate's public surface
//! for the convenience of one downstream tool, and the two binaries are free to
//! word their refusals differently. The behaviour, however, is the same, and
//! `output_tests.rs` pins it here with the same cases `gallery_tests.rs` pins
//! there.
//!
//! The rule is *containment*, not equality: refusing `%APPDATA%` while accepting
//! `%APPDATA%\app-snapshot` protects nothing, because a subdirectory of the
//! config root is the config root for these purposes. Containment is decided
//! component-wise after normalising separators and case, so a path that merely
//! shares a string prefix with a protected directory is not inside it.
//!
//! The system temp tree is carved out before the roots are consulted, because on
//! Windows it lives *inside* `%LOCALAPPDATA%` — without the carve-out the guard
//! would refuse this tool's own default neighbourhood.
//!
//! Unit tests live in the sibling `output_tests.rs` sidecar.

use std::error::Error;
use std::path::Path;
use std::path::PathBuf;

use crate::strings;

/// Environment variables whose exact directories the gallery must never
/// write into.
///
/// These are the roots an application configuration write would land in, so a
/// mistyped output argument that resolved to one of them is refused rather than
/// honoured. A *subdirectory* of the temp directory is fine — the rule is about
/// the app's own config roots, not about the temp tree a default lives in.
const FORBIDDEN_ROOT_VARS: [&str; 3] = ["APPDATA", "LOCALAPPDATA", "USERPROFILE"];

/// The filename fragment that marks a protected application directory.
const APP_DIR_MARKER: &str = ".hoarfrost";

/// The id prefix every demo section carries, so a filename does not repeat a
/// namespace every filename in the same directory already belongs to.
const SECTION_ID_PREFIX: &str = "demo.section.";

/// The stem handed back when an id sanitises down to nothing, so a generated
/// filename is never just an extension.
///
/// Public because a caller that checks generated names ("did this id sanitise to
/// something that actually describes a section?") has to be able to say what the
/// non-answer looks like without restating the literal.
pub const FALLBACK_STEM: &str = "section";

/// The suffix every emitted image carries.
const PNG_SUFFIX: &str = ".png";

/// Is `child` the same path as `root`, or nested beneath it?
///
/// Component-wise, not string-wise: `/tmp/hoarfrost-gallery-evil` is not
/// below `/tmp/hoarfrost-gallery` even though one string prefixes the
/// other. Both sides are expected to be normalised already.
fn at_or_below(child: &str, root: &str) -> bool {
    child == root || child.starts_with(&format!("{root}/"))
}

/// A path as the guard compares it: lower-cased, with POSIX separators and no
/// trailing slash.
///
/// Case-insensitive and separator-normalised, because the caller may pass either
/// Windows or POSIX separators and Windows paths are case-preserving but not
/// case-sensitive.
fn normalised(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_lowercase()
}

/// The system temp tree as the guard compares it, or `None` when it cannot be
/// resolved.
fn temp_root() -> Option<String> {
    let temp = normalised(&std::env::temp_dir());
    (!temp.is_empty()).then_some(temp)
}

/// Whether `path` sits inside the carved-out temp tree.
fn in_temp_tree(path: &str) -> bool {
    temp_root().is_some_and(|temp| at_or_below(path, &temp))
}

/// Whether `path` carries a protected application-directory component anywhere
/// in it.
fn has_app_component(path: &str) -> bool {
    path.split('/')
        .any(|component| component.starts_with(APP_DIR_MARKER))
}

/// Whether `path` is at or below one of the forbidden roots, with the reason it
/// is refused.
fn below_forbidden_root(path: &str) -> Option<String> {
    for variable in FORBIDDEN_ROOT_VARS {
        let Ok(value) = std::env::var(variable) else {
            continue;
        };
        let root = value
            .replace('\\', "/")
            .trim_end_matches('/')
            .to_lowercase();
        if root.is_empty() {
            continue;
        }
        // Containment, not equality. Refusing only the exact root would let a
        // subdirectory through, and a subdirectory of `%APPDATA%` is exactly
        // the case the rule exists to stop — `%APPDATA%\app-snapshot` is
        // no less the user's config tree than `%APPDATA%` itself.
        if at_or_below(path, &root) {
            return Some(format!(
                "{} ${variable} root ({value})",
                strings::REFUSAL_BELOW_ROOT
            ));
        }
    }
    None
}

/// Why a path was refused, if it was.
fn refusal(path: &Path) -> Option<String> {
    let normalised = normalised(path);

    if has_app_component(&normalised) {
        return Some(strings::REFUSAL_APP_COMPONENT.to_owned());
    }

    // The temp tree is carved out before the roots are consulted, because
    // on Windows it lives *inside* `%LOCALAPPDATA%` — refusing everything
    // below that root would refuse the gallery's own default output. The
    // carve-out is the temp tree only, not its parent: `%LOCALAPPDATA%`
    // itself and everything else under it stay refused.
    if in_temp_tree(&normalised) {
        return None;
    }

    below_forbidden_root(&normalised)
}

/// Refuse `path` or return it as an absolute path.
///
/// A relative path is resolved against the current directory and then checked
/// again, because the first check on a relative path says nothing about where it
/// actually lands: `..\..\AppData\Roaming` is relative and refused.
///
/// # Errors
///
/// Returns a message naming the reason when the path is refused, or when the
/// current directory cannot be read in order to resolve a relative path.
pub fn guard(path: &Path) -> Result<PathBuf, Box<dyn Error>> {
    if let Some(reason) = refusal(path) {
        return Err(format!("{}{reason}", strings::REFUSAL_PREFIX).into());
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    if let Some(reason) = refusal(&absolute) {
        return Err(format!("{}{reason}", strings::REFUSAL_PREFIX).into());
    }
    Ok(absolute)
}

/// Whether a character survives filename sanitisation as itself.
fn is_keepable(ch: char) -> bool {
    ch.is_ascii_lowercase() || ch.is_ascii_digit()
}

/// A section id as a safe, stable file stem.
///
/// The `demo.section.` namespace prefix is dropped first — every file in the
/// output directory is a demo section, so repeating it on every name buys
/// nothing. Then the id is lower-cased and reduced to `[a-z0-9-]`, with every
/// run of rejected characters collapsing to a single `-` and no separator kept
/// at either end. The result is never empty: an id that sanitises away entirely
/// becomes `FALLBACK_STEM`, because a filename of `.png` is not a file anyone
/// can find again.
///
/// Deterministic by construction, which is what lets two runs of the same sweep
/// produce the same set of names.
///
/// ```
/// # use hoarfrost_gallery::output::sanitize_stem;
/// assert_eq!(sanitize_stem("demo.section.buttons"), "buttons");
/// assert_eq!(sanitize_stem("demo.section.Tab strip (UIF-TAB-003)"), "tab-strip-uif-tab-003");
/// assert_eq!(sanitize_stem("!!!"), "section");
/// ```
#[must_use]
pub fn sanitize_stem(section_id: &str) -> String {
    let core = section_id
        .strip_prefix(SECTION_ID_PREFIX)
        .unwrap_or(section_id);

    let mut stem = String::with_capacity(core.len());
    let mut pending_separator = false;

    for ch in core.chars() {
        let lowered = ch.to_ascii_lowercase();
        if is_keepable(lowered) {
            if pending_separator {
                stem.push('-');
                pending_separator = false;
            }
            stem.push(lowered);
        } else {
            // A rejected character (including `-` itself) is a separator, and a
            // separator at the very start is dropped rather than deferred, so
            // the stem can never begin with one.
            pending_separator = !stem.is_empty();
        }
    }

    if stem.is_empty() {
        stem.push_str(FALLBACK_STEM);
    }
    stem
}

/// The file name a section's PNG is written under.
///
/// Built from [`sanitize_stem`], so two sections whose ids differ only in
/// punctuation still differ on disk only in what survived sanitisation — which
/// is why the catalog is asserted duplicate-free upstream rather than here.
#[must_use]
pub fn png_file_name(section_id: &str) -> String {
    format!("{}{PNG_SUFFIX}", sanitize_stem(section_id))
}

#[cfg(test)]
#[path = "output_tests.rs"]
mod tests;
