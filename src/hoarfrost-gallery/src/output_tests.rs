//! Tests for the output guard and the filename sanitiser.
//!
//! Everything here exercises pure predicates. The only test that resolves a
//! path against the filesystem reads the current directory; none of them create,
//! modify or delete anything, and none of them touch a real user configuration
//! path (**RUST-006**).
//!
//! The hole these tests exist to keep shut is the one `gallery_tests.rs`
//! documents for the guard this module copied: refusing `%APPDATA%` while
//! accepting `%APPDATA%\app-snapshot` protects nothing. The second hole is the
//! opposite error — a string-prefix comparison that refuses a harmless sibling
//! directory and teaches the user to route around the guard.

use std::path::{Path, PathBuf};

use super::*;

/// The three roots the guard exists to keep writes out of.
const ROOT_VARS: [&str; 3] = FORBIDDEN_ROOT_VARS;

/// An absolute path that is nowhere near a protected root.
///
/// Built from the temp directory rather than spelled out, so it is genuinely
/// absolute on every platform and always inside the carved-out tree.
fn harmless() -> PathBuf {
    std::env::temp_dir().join("hoarfrost-gallery")
}

/// Run `body` with `APPDATA` pointed at `root`, then put the value back.
///
/// Two of the rules below can only be shown against a root the test controls:
/// on Windows every sibling of `%APPDATA%` is still inside `%USERPROFILE%`, so a
/// near-prefix sibling of the real root is refused for a *different* reason and
/// proves nothing.
///
/// # Safety
///
/// `cargo nextest` gives every test its own process, so this cannot race another
/// test's read of the environment. The previous value is restored before
/// returning, so the test stays order-independent even under a runner that shares
/// one process between tests.
fn with_appdata<T>(root: &str, body: impl FnOnce() -> T) -> T {
    let previous = std::env::var("APPDATA").ok();

    // SAFETY: see the note above.
    unsafe { std::env::set_var("APPDATA", root) };

    let outcome = body();

    // SAFETY: as above.
    match previous {
        // SAFETY: as above.
        Some(value) => unsafe { std::env::set_var("APPDATA", value) },
        // SAFETY: as above.
        None => unsafe { std::env::remove_var("APPDATA") },
    }

    outcome
}

#[test]
fn refuses_each_root_exactly() {
    for var in ROOT_VARS {
        let Ok(value) = std::env::var(var) else {
            continue;
        };
        assert!(
            refusal(Path::new(&value)).is_some(),
            "${var} root ({value}) must be refused"
        );
    }
}

#[test]
fn refuses_a_subdirectory_of_each_root() {
    // The regression the copied guard was written for.
    for var in ROOT_VARS {
        let Ok(value) = std::env::var(var) else {
            continue;
        };
        let nested = PathBuf::from(&value).join("gallery-out");
        let reason = refusal(&nested);
        assert!(
            reason.is_some(),
            "${var} subdirectory must be refused, got None for {}",
            nested.display()
        );
        assert!(reason.unwrap().contains(var), "the reason must name ${var}");
    }
}

#[test]
fn refuses_a_deeply_nested_subdirectory_of_each_root() {
    for var in ROOT_VARS {
        let Ok(value) = std::env::var(var) else {
            continue;
        };
        let nested = PathBuf::from(&value).join("a").join("b").join("out");
        assert!(
            refusal(&nested).is_some(),
            "a nested path under ${var} must be refused: {}",
            nested.display()
        );
    }
}

#[test]
fn the_refusal_names_the_variable_it_matched() {
    let Ok(local) = std::env::var("LOCALAPPDATA") else {
        return;
    };
    let reason = refusal(&PathBuf::from(local).join("gallery")).expect("refused");
    assert!(
        reason.contains("LOCALAPPDATA"),
        "the message must name the variable: {reason}"
    );
    assert!(
        reason.contains("path is at or below"),
        "the message must say why: {reason}"
    );
}

#[test]
fn refuses_an_app_component_anywhere() {
    for candidate in [
        "/tmp/.hoarfrost-gallery",
        "/somewhere/.hoarfrostx/out",
        "/data/notes/.hoarfrost",
        "relative/.hoarfrost/out",
    ] {
        assert!(
            refusal(Path::new(candidate)).is_some(),
            "a {APP_DIR_MARKER}* component must be refused: {candidate}"
        );
    }
}

#[test]
fn the_app_component_check_runs_before_the_temp_carveout() {
    // The temp tree is allowed, but not as a way around the marker rule.
    let inside_temp = std::env::temp_dir().join(".hoarfrost").join("out");
    assert!(
        refusal(&inside_temp).is_some(),
        "a {APP_DIR_MARKER} component inside temp must still be refused: {}",
        inside_temp.display()
    );
}

#[test]
fn a_temp_subdirectory_is_allowed() {
    let temp = std::env::temp_dir().join("hoarfrost-gallery-out");
    assert!(
        refusal(&temp).is_none(),
        "a temp subdirectory must be allowed: {}",
        temp.display()
    );
}

#[test]
fn the_temp_root_itself_is_allowed() {
    let temp = std::env::temp_dir();
    assert!(
        refusal(&temp).is_none(),
        "the temp root itself must be allowed: {}",
        temp.display()
    );
}

#[test]
fn a_target_path_is_allowed() {
    // The default output lives under `target/`, which the repository ignores.
    assert!(refusal(Path::new("target/gallery-png")).is_none());
    assert!(refusal(&harmless()).is_none());
}

#[test]
fn matching_is_case_insensitive_and_separator_normalised() {
    let Ok(appdata) = std::env::var("APPDATA") else {
        return;
    };
    let upper = appdata.to_uppercase();
    assert!(
        refusal(Path::new(&upper)).is_some(),
        "case must not defeat the guard: {upper}"
    );
    let posix = appdata.replace('\\', "/");
    assert!(
        refusal(Path::new(&posix)).is_some(),
        "POSIX separators must not defeat the guard: {posix}"
    );
}

#[test]
fn a_root_with_a_trailing_separator_still_nests_correctly() {
    let Ok(appdata) = std::env::var("APPDATA") else {
        return;
    };
    let nested = format!(
        "{}{}gallery-out",
        appdata.trim_end_matches(['/', '\\']),
        "/x"
    );
    assert!(
        refusal(Path::new(&nested)).is_some(),
        "a nested path must still be refused: {nested}"
    );
}

#[test]
fn containment_is_component_wise_not_string_wise() {
    // A sibling that merely shares a string prefix is not "below".
    assert!(
        !at_or_below("/tmp/hoarfrost-gallery-evil", "/tmp/hoarfrost-gallery"),
        "a string-prefix sibling must not count as nested"
    );
    assert!(at_or_below(
        "/tmp/hoarfrost-gallery",
        "/tmp/hoarfrost-gallery"
    ));
    assert!(at_or_below(
        "/tmp/hoarfrost-gallery/out",
        "/tmp/hoarfrost-gallery"
    ));
    assert!(!at_or_below("/tmp", "/tmp/hoarfrost-gallery"));
}

#[test]
fn a_near_prefix_sibling_of_a_root_is_not_refused() {
    // The same trap against a root that really exists for the guard. On Windows
    // every sibling of the real `%APPDATA%` is inside `%USERPROFILE%` and would
    // be refused for the wrong reason, so the root is supplied by the test.
    with_appdata("D:/guard-root", || {
        assert!(
            refusal(Path::new("D:/guard-root")).is_some(),
            "the root itself is refused"
        );
        assert!(
            refusal(Path::new("D:/guard-root/out")).is_some(),
            "a child of the root is refused"
        );
        assert!(
            refusal(Path::new("D:/guard-root-evil/out")).is_none(),
            "a sibling that merely shares a prefix must be allowed"
        );
    });
}

#[test]
fn guard_returns_an_absolute_path_and_creates_nothing() {
    let resolved = guard(Path::new("target/gallery-png")).expect("a target path is allowed");
    assert!(resolved.is_absolute(), "{resolved:?} is not absolute");
    assert_eq!(
        resolved,
        std::env::current_dir()
            .expect("a current directory")
            .join("target/gallery-png")
    );
}

#[test]
fn guard_passes_an_allowed_absolute_path_through_unchanged() {
    let absolute = harmless();
    assert_eq!(guard(&absolute).expect("allowed"), absolute);
}

#[test]
fn guard_refuses_an_absolute_config_subdirectory() {
    let Ok(appdata) = std::env::var("APPDATA") else {
        return;
    };
    let nested = PathBuf::from(appdata).join("gallery");
    let error = guard(&nested).expect_err("must be refused");
    let message = error.to_string();
    assert!(
        message.starts_with("refusing to write output: "),
        "the refusal must carry the shared prefix: {message}"
    );
    assert!(message.contains("APPDATA"), "the reason: {message}");
}

#[test]
fn a_relative_path_is_resolved_and_checked_again() {
    // The first check sees `target/gallery-png`, harmless on its own. Only after
    // joining the current directory does it become a protected path, and the
    // guard has to notice there rather than honour it.
    let cwd = std::env::current_dir().expect("a current directory");
    let cwd = cwd.to_string_lossy().into_owned();

    with_appdata(&cwd, || {
        let error = guard(Path::new("target/gallery-png"))
            .expect_err("a relative path that resolves below APPDATA must be refused");
        assert!(
            error.to_string().contains("APPDATA"),
            "the refusal must name the root it hit: {error}"
        );
    });
}

#[test]
fn sanitiser_strips_the_section_namespace_prefix() {
    assert_eq!(sanitize_stem("demo.section.buttons"), "buttons");
    assert_eq!(sanitize_stem("demo.section.tab-strip"), "tab-strip");
}

#[test]
fn sanitiser_leaves_an_already_clean_id_alone() {
    assert_eq!(sanitize_stem("buttons"), "buttons");
    assert_eq!(sanitize_stem("tab-strip-003"), "tab-strip-003");
}

#[test]
fn sanitiser_lowercases() {
    assert_eq!(sanitize_stem("BUTTONS"), "buttons");
    assert_eq!(sanitize_stem("demo.section.TabStrip"), "tabstrip");
}

#[test]
fn sanitiser_turns_punctuation_into_one_separator() {
    // Section titles carry spec references, and those have to survive as
    // readable names rather than as a smear of dropped characters.
    assert_eq!(
        sanitize_stem("demo.section.Tab strip (UIF-TAB-003)"),
        "tab-strip-uif-tab-003"
    );
    assert_eq!(sanitize_stem("a.b.c"), "a-b-c");
    assert_eq!(sanitize_stem("a b"), "a-b");
}

#[test]
fn sanitiser_collapses_runs_of_separators() {
    assert_eq!(sanitize_stem("a---b"), "a-b");
    assert_eq!(sanitize_stem("a -- b"), "a-b");
    assert_eq!(sanitize_stem("a!!!b"), "a-b");
}

#[test]
fn sanitiser_trims_separators_from_both_ends() {
    assert_eq!(sanitize_stem("--buttons--"), "buttons");
    assert_eq!(sanitize_stem("  buttons  "), "buttons");
    assert_eq!(sanitize_stem("...buttons..."), "buttons");
}

#[test]
fn sanitiser_never_returns_an_empty_stem() {
    // A stem of "" would produce a file called `.png`, which nobody can find
    // again. Every degenerate id has to land on the fallback instead.
    assert_eq!(sanitize_stem(""), FALLBACK_STEM);
    assert_eq!(sanitize_stem("..."), FALLBACK_STEM);
    assert_eq!(sanitize_stem("!!!"), FALLBACK_STEM);
    assert_eq!(sanitize_stem("   "), FALLBACK_STEM);
    assert_eq!(sanitize_stem(SECTION_ID_PREFIX), FALLBACK_STEM);
    assert_eq!(sanitize_stem("demo.section."), FALLBACK_STEM);
}

#[test]
fn sanitiser_drops_only_the_leading_prefix() {
    // A `demo.section.` appearing later in the id is content, not namespace.
    assert_eq!(sanitize_stem("x.demo.section.y"), "x-demo-section-y");
}

#[test]
fn sanitiser_is_stable_across_repeated_calls() {
    for id in [
        "demo.section.Tab strip (UIF-TAB-003)",
        "Mixed CASE--id!!",
        "",
        "clean",
    ] {
        assert_eq!(
            sanitize_stem(id),
            sanitize_stem(id),
            "sanitisation must be deterministic: {id:?}"
        );
    }
}

#[test]
fn png_file_name_appends_the_extension() {
    assert_eq!(png_file_name("demo.section.buttons"), "buttons.png");
    assert_eq!(png_file_name(""), format!("{FALLBACK_STEM}{PNG_SUFFIX}"));
}

#[test]
fn png_file_name_contains_no_forbidden_characters() {
    let name = png_file_name("demo.section.Tab strip (UIF-TAB-003)");
    assert!(
        name.chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '.'),
        "a generated name must be safe on Windows: {name}"
    );
}
