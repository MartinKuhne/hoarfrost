//! Tests for the user-facing string table.
//!
//! These are cheap invariant checks rather than behaviour tests: they catch the
//! two ways a string table rots — a constant that gets emptied out, and a
//! placeholder that stops matching the code that fills it in.

use super::*;

/// Every constant that can end up in front of a human.
const USER_FACING: [(&str, &str); 51] = [
    ("PROGRAM", PROGRAM),
    ("USAGE_PURPOSE", USAGE_PURPOSE),
    ("USAGE_LABEL", USAGE_LABEL),
    ("USAGE_SYNOPSIS", USAGE_SYNOPSIS),
    ("FLAGS_LABEL", FLAGS_LABEL),
    ("DEFAULTS_LABEL", DEFAULTS_LABEL),
    ("ARG_OUTPUT_DIR", ARG_OUTPUT_DIR),
    ("FLAG_OUT", FLAG_OUT),
    ("FLAG_WIDTH", FLAG_WIDTH),
    ("FLAG_SCALE", FLAG_SCALE),
    ("FLAG_FILTER", FLAG_FILTER),
    ("FLAG_HELP", FLAG_HELP),
    ("PLACEHOLDER_DIR", PLACEHOLDER_DIR),
    ("PLACEHOLDER_WIDTH", PLACEHOLDER_WIDTH),
    ("PLACEHOLDER_SCALE", PLACEHOLDER_SCALE),
    ("PLACEHOLDER_FILTER", PLACEHOLDER_FILTER),
    ("HELP_OUT", HELP_OUT),
    ("HELP_OUTPUT_DIR", HELP_OUTPUT_DIR),
    ("HELP_WIDTH", HELP_WIDTH),
    ("HELP_SCALE", HELP_SCALE),
    ("HELP_FILTER", HELP_FILTER),
    ("HELP_HELP", HELP_HELP),
    ("DEFAULT_OUTPUT_DIR", DEFAULT_OUTPUT_DIR),
    ("ERROR_PREFIX", ERROR_PREFIX),
    ("REFUSAL_PREFIX", REFUSAL_PREFIX),
    ("REFUSAL_APP_COMPONENT", REFUSAL_APP_COMPONENT),
    ("REFUSAL_BELOW_ROOT", REFUSAL_BELOW_ROOT),
    ("ERR_UNKNOWN_FLAG", ERR_UNKNOWN_FLAG),
    ("ERR_MISSING_VALUE", ERR_MISSING_VALUE),
    ("ERR_NOT_A_NUMBER", ERR_NOT_A_NUMBER),
    ("ERR_NOT_POSITIVE", ERR_NOT_POSITIVE),
    ("ERR_EMPTY_OUTPUT_DIR", ERR_EMPTY_OUTPUT_DIR),
    ("ERR_OUT_GIVEN_TWICE", ERR_OUT_GIVEN_TWICE),
    ("ERR_EMPTY_FILTER", ERR_EMPTY_FILTER),
    ("ERR_EXTRA_ARGUMENT", ERR_EXTRA_ARGUMENT),
    ("ERR_PNG_ZERO_SIZE", ERR_PNG_ZERO_SIZE),
    ("ERR_PNG_BUFFER", ERR_PNG_BUFFER),
    ("ERR_CARD_NOT_FOUND", ERR_CARD_NOT_FOUND),
    ("ERR_CARD_NO_AREA", ERR_CARD_NO_AREA),
    ("ERR_CARD_NOT_FINITE", ERR_CARD_NOT_FINITE),
    ("ERR_CARD_TOO_TALL", ERR_CARD_TOO_TALL),
    ("ERR_PIXEL_COUNT", ERR_PIXEL_COUNT),
    ("ERR_DECLARED_NOT_FINITE", ERR_DECLARED_NOT_FINITE),
    ("ERR_DECLARED_NO_AREA", ERR_DECLARED_NO_AREA),
    ("DETAIL_DECLARED_HEIGHT", DETAIL_DECLARED_HEIGHT),
    ("MANIFEST_FILE_NAME", MANIFEST_FILE_NAME),
    ("MANIFEST_HEADER", MANIFEST_HEADER),
    ("MANIFEST_COLUMNS", MANIFEST_COLUMNS),
    ("STATUS_RENDERED", STATUS_RENDERED),
    ("STATUS_FAILED", STATUS_FAILED),
    ("FIELD_NONE", FIELD_NONE),
];

/// The summary labels the run report and the manifest are built from.
const LABELS: [(&str, &str); 11] = [
    ("SUMMARY_RENDERED", SUMMARY_RENDERED),
    ("SUMMARY_FAILED", SUMMARY_FAILED),
    ("SUMMARY_TOTAL", SUMMARY_TOTAL),
    ("LABEL_OUTPUT", LABEL_OUTPUT),
    ("LABEL_MANIFEST", LABEL_MANIFEST),
    ("LABEL_WIDTH", LABEL_WIDTH),
    ("LABEL_SCALE", LABEL_SCALE),
    ("LABEL_FILTER", LABEL_FILTER),
    ("LABEL_RENDERED", LABEL_RENDERED),
    ("LABEL_FAILED", LABEL_FAILED),
    ("PROGRAM", PROGRAM),
];

#[test]
fn no_user_facing_string_is_empty() {
    for (name, value) in USER_FACING {
        assert!(
            !value.trim().is_empty(),
            "{name} must carry some text, got {value:?}"
        );
    }
}

#[test]
fn no_user_facing_string_carries_stray_surrounding_whitespace() {
    // A leading or trailing space inside a constant is invisible until it shows
    // up as a ragged column in the report. Two kinds of constant are the
    // deliberate exception, and both are pinned rather than skipped.
    let joined_without_space = ["ERROR_PREFIX", "REFUSAL_PREFIX"];
    let intentionally_indented = ["USAGE_SYNOPSIS"];

    for (name, value) in USER_FACING {
        if joined_without_space.contains(&name) {
            assert!(
                value.ends_with(' '),
                "{name} is concatenated directly onto a message: {value:?}"
            );
            continue;
        }
        if intentionally_indented.contains(&name) {
            assert!(
                value.starts_with("  ") && value == value.trim_end(),
                "{name} is an indented block line: {value:?}"
            );
            continue;
        }
        assert_eq!(value, value.trim(), "{name} has stray whitespace");
    }
}

#[test]
fn every_label_is_non_empty_and_distinct() {
    let mut seen: Vec<&str> = Vec::new();
    for (name, value) in LABELS {
        assert!(!value.is_empty(), "{name} must carry text");
        assert!(
            !seen.contains(&value),
            "{name} duplicates the text of an earlier label"
        );
        seen.push(value);
    }
}

#[test]
fn the_manifest_lines_are_self_describing() {
    assert!(MANIFEST_HEADER.starts_with('#'));
    assert!(MANIFEST_COLUMNS.starts_with('#'));
    assert!(
        MANIFEST_COLUMNS.contains("status") && MANIFEST_COLUMNS.contains("section"),
        "the column line must name the columns: {MANIFEST_COLUMNS}"
    );
    assert_eq!(MANIFEST_FILE_NAME, "manifest.txt");
}

#[test]
fn the_flag_names_all_use_the_double_dash_form() {
    for flag in [FLAG_OUT, FLAG_WIDTH, FLAG_SCALE, FLAG_FILTER, FLAG_HELP] {
        assert!(
            flag.starts_with("--"),
            "a flag must be spelled the way the parser looks it up: {flag}"
        );
    }
}

#[test]
fn the_default_output_directory_lives_under_the_ignored_target_tree() {
    // The plan's output-handling decision depends on this: the default has to be
    // somewhere git already ignores, so a sweep can never commit its own output.
    assert!(
        DEFAULT_OUTPUT_DIR.starts_with("target/"),
        "{DEFAULT_OUTPUT_DIR} must live under target/"
    );
}

#[test]
fn the_summary_labels_are_ready_to_be_prefixed_by_the_comment_marker() {
    for label in [SUMMARY_RENDERED, SUMMARY_FAILED, SUMMARY_TOTAL] {
        assert!(
            !label.starts_with('#'),
            "the manifest adds the comment marker itself: {label:?}"
        );
        assert!(
            label.ends_with(':'),
            "a summary label reads as `# {label} <count>`: {label:?}"
        );
    }
}
