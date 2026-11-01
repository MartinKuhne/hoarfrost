//! Tests for the argument parser.
//!
//! Parsing is a pure function of a string slice, so every branch is reachable
//! without a process, a filesystem or an environment (**RUST-006**). The cases
//! below are the ones that matter for a tool whose output is a directory of
//! files: a silently wrong output directory, a silently wrong width, and a
//! `--help` text that has drifted from what the parser accepts.

use std::path::PathBuf;

use super::*;

/// Build an argument vector from string literals.
fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

/// The options inside a `Run` outcome, or a failure naming the outcome.
fn options_of(outcome: Outcome) -> Options {
    match outcome {
        Outcome::Run(options) => options,
        other => panic!("expected a Run outcome, got {other:?}"),
    }
}

/// The reason inside an `Invalid` outcome, or a failure naming the outcome.
fn reason_of(outcome: Outcome) -> String {
    match outcome {
        Outcome::Invalid(reason) => reason,
        other => panic!("expected an Invalid outcome, got {other:?}"),
    }
}

#[test]
fn no_arguments_give_the_defaults() {
    let options = options_of(parse(&[]));

    assert_eq!(options.output_dir, PathBuf::from(DEFAULT_OUTPUT_DIR_TEXT));
    assert_eq!(options.width, DEFAULT_WIDTH);
    assert_eq!(options.scale, DEFAULT_SCALE);
    assert_eq!(options.filter, None);
}

/// The default output directory as text, spelled through `strings` so the test
/// fails if the two ever disagree.
const DEFAULT_OUTPUT_DIR_TEXT: &str = crate::strings::DEFAULT_OUTPUT_DIR;

#[test]
fn a_bare_positional_sets_the_output_directory() {
    let options = options_of(parse(&args(&["out/here"])));
    assert_eq!(options.output_dir, PathBuf::from("out/here"));
}

#[test]
fn the_positional_may_come_after_a_flag() {
    let options = options_of(parse(&args(&["--width", "800", "out"])));
    assert_eq!(options.width, 800.0);
    assert_eq!(options.output_dir, PathBuf::from("out"));
}

#[test]
fn the_positional_may_come_before_a_flag() {
    let options = options_of(parse(&args(&["out", "--width", "800"])));
    assert_eq!(options.width, 800.0);
    assert_eq!(options.output_dir, PathBuf::from("out"));
}

#[test]
fn out_names_the_output_directory() {
    let options = options_of(parse(&args(&["--out", "out/here"])));
    assert_eq!(options.output_dir, PathBuf::from("out/here"));
}

#[test]
fn out_may_come_before_or_after_the_other_flags() {
    let before = options_of(parse(&args(&["--out", "out", "--width", "800"])));
    let after = options_of(parse(&args(&["--width", "800", "--out", "out"])));

    assert_eq!(before.output_dir, PathBuf::from("out"));
    assert_eq!(after, before);
}

#[test]
fn out_tolerates_a_value_that_starts_like_a_flag() {
    // The token after a value flag is a value, not a flag to be second-guessed.
    // A directory called `-weird` is unusual but it is not the parser's place to
    // decide what a person may call a folder.
    let options = options_of(parse(&args(&["--out", "-weird"])));
    assert_eq!(options.output_dir, PathBuf::from("-weird"));
}

#[test]
fn a_repeated_out_is_last_one_wins() {
    let options = options_of(parse(&args(&["--out", "one", "--out", "two"])));
    assert_eq!(options.output_dir, PathBuf::from("two"));
}

#[test]
fn an_empty_out_is_rejected() {
    // An empty directory would resolve to the current working directory, which is
    // never what someone means when they type `--out ""`.
    let reason = reason_of(parse(&args(&[crate::strings::FLAG_OUT, ""])));
    assert!(
        reason.contains(crate::strings::ERR_EMPTY_OUTPUT_DIR),
        "{reason}"
    );
    assert!(
        reason.contains(crate::strings::FLAG_OUT),
        "the message must name the flag: {reason}"
    );
}

#[test]
fn naming_the_output_directory_both_ways_is_refused() {
    // Whichever route wins, a silent choice here moves every file in the run, so
    // the parser refuses to make one.
    for argv in [
        &["out", "--out", "other"][..],
        &["--out", "other", "out"][..],
    ] {
        let reason = reason_of(parse(&args(argv)));
        assert!(
            reason.contains(crate::strings::ERR_OUT_GIVEN_TWICE),
            "{argv:?} must be refused: {reason}"
        );
    }
}

#[test]
fn each_value_flag_sets_its_option() {
    let options = options_of(parse(&args(&[
        "--width", "1024", "--scale", "1.5", "--filter", "tab",
    ])));

    assert_eq!(options.width, 1024.0);
    assert_eq!(options.scale, 1.5);
    assert_eq!(options.filter, Some("tab".to_owned()));
}

#[test]
fn a_repeated_flag_is_last_one_wins() {
    // Repeating a flag is a typo a person makes while typing, not an ambiguity
    // worth failing a run over. The *positional* is the one that must not be
    // silently overridden, because it chooses where files land.
    let options = options_of(parse(&args(&["--width", "100", "--width", "200"])));
    assert_eq!(options.width, 200.0);
}

#[test]
fn help_ends_the_parse_wherever_it_is_reached() {
    assert_eq!(parse(&args(&["--help"])), Outcome::Help);
    assert_eq!(parse(&args(&["out", "--help"])), Outcome::Help);
    // A parse error *before* `--help` still wins. The parser reads left to right
    // and does not guess that a token the user never reached was what they
    // meant; `--help` alone is the way to ask for help.
    assert!(matches!(
        parse(&args(&["--width", "abc", "--help"])),
        Outcome::Invalid(_)
    ));
}

#[test]
fn an_unknown_flag_is_an_error() {
    let reason = reason_of(parse(&args(&["--nope"])));
    assert!(
        reason.contains("--nope"),
        "the message must name the flag: {reason}"
    );
    assert!(
        reason.contains(crate::strings::ERR_UNKNOWN_FLAG),
        "the message must say what is wrong: {reason}"
    );
}

#[test]
fn a_single_dash_token_is_an_error() {
    for token in ["-w", "-", "--"] {
        let reason = reason_of(parse(&args(&[token])));
        assert!(
            reason.contains(crate::strings::ERR_UNKNOWN_FLAG),
            "{token:?} must be refused: {reason}"
        );
    }
}

#[test]
fn a_non_numeric_width_is_an_error() {
    let reason = reason_of(parse(&args(&["--width", "wide"])));
    assert!(
        reason.contains(crate::strings::ERR_NOT_A_NUMBER),
        "the message must say it wanted a number: {reason}"
    );
    assert!(
        reason.contains("wide"),
        "the message must quote it: {reason}"
    );
}

#[test]
fn a_missing_value_after_every_flag_is_an_error() {
    for flag in [
        crate::strings::FLAG_OUT,
        crate::strings::FLAG_WIDTH,
        crate::strings::FLAG_SCALE,
        crate::strings::FLAG_FILTER,
    ] {
        let reason = reason_of(parse(&args(&[flag])));
        assert!(
            reason.contains(crate::strings::ERR_MISSING_VALUE),
            "{flag} with no value must be refused: {reason}"
        );
        assert!(reason.contains(flag), "the message must name it: {reason}");
    }
}

#[test]
fn a_missing_value_is_an_error_even_after_a_positional() {
    let reason = reason_of(parse(&args(&["out", "--scale"])));
    assert!(
        reason.contains(crate::strings::ERR_MISSING_VALUE),
        "{reason}"
    );
}

#[test]
fn zero_and_negative_numbers_are_rejected() {
    for value in ["0", "0.0", "-1", "-0.5"] {
        for flag in [crate::strings::FLAG_WIDTH, crate::strings::FLAG_SCALE] {
            let reason = reason_of(parse(&args(&[flag, value])));
            assert!(
                reason.contains(crate::strings::ERR_NOT_POSITIVE),
                "{flag} {value} must be refused: {reason}"
            );
        }
    }
}

#[test]
fn nan_and_infinity_are_rejected() {
    // Both parse successfully as `f32`, so only an explicit finiteness check
    // stops them reaching a raster call.
    for value in ["nan", "inf", "-inf", "infinity"] {
        let reason = reason_of(parse(&args(&[crate::strings::FLAG_WIDTH, value])));
        assert!(
            reason.contains(crate::strings::ERR_NOT_POSITIVE),
            "{value} must be refused: {reason}"
        );
    }
}

#[test]
fn a_small_positive_scale_is_accepted() {
    // Fractional scales are legitimate: they are how a run asks for a
    // lower-resolution sweep.
    let options = options_of(parse(&args(&["--scale", "0.25"])));
    assert_eq!(options.scale, 0.25);
}

#[test]
fn an_empty_filter_is_rejected() {
    // An empty fragment would match every section, which is the opposite of what
    // someone typing `--filter ""` means.
    let reason = reason_of(parse(&args(&[crate::strings::FLAG_FILTER, ""])));
    assert!(
        reason.contains(crate::strings::ERR_EMPTY_FILTER),
        "{reason}"
    );
}

#[test]
fn a_second_positional_is_an_error() {
    let reason = reason_of(parse(&args(&["one", "two"])));
    assert!(
        reason.contains(crate::strings::ERR_EXTRA_ARGUMENT),
        "the message must say it is extra: {reason}"
    );
    assert!(reason.contains("two"), "the message must name it: {reason}");
}

#[test]
fn a_filter_keeps_only_ids_containing_the_fragment() {
    let options = Options {
        filter: Some("tab".to_owned()),
        ..Options::default()
    };

    assert!(accepts(&options, "demo.section.tab-strip"));
    assert!(!accepts(&options, "demo.section.buttons"));
}

#[test]
fn no_filter_keeps_everything() {
    let options = Options::default();
    assert!(accepts(&options, "demo.section.buttons"));
    assert!(accepts(&options, ""));
}

#[test]
fn a_filter_is_matched_case_sensitively_and_as_a_substring() {
    let options = Options {
        filter: Some("Strip".to_owned()),
        ..Options::default()
    };

    assert!(accepts(&options, "demo.section.Tab-Strip"));
    assert!(!accepts(&options, "demo.section.tab-strip"));
    assert!(accepts(&options, "xStripx"));
}

#[test]
fn the_flag_table_covers_every_flag_it_declares() {
    for spec in &FLAGS {
        assert_eq!(
            flag_of(spec.name).map(|found| found.name),
            Some(spec.name),
            "every flag must be findable"
        );
    }
    assert!(flag_of("--definitely-not-a-flag").is_none());
    assert!(flag_of("width").is_none());
}

#[test]
fn a_flag_display_shows_its_placeholder_unless_it_takes_no_value() {
    let width = flag_of(crate::strings::FLAG_WIDTH).expect("in the table");
    assert_eq!(width.display(), "--width <px>");

    let help = flag_of(crate::strings::FLAG_HELP).expect("in the table");
    assert_eq!(help.display(), "--help");
}

#[test]
fn usage_lists_every_flag_in_the_table() {
    // The drift this table-driven design exists to prevent.
    let text = usage();
    for spec in &FLAGS {
        assert!(
            text.contains(&spec.display()),
            "the usage block must advertise {}: {:?}",
            spec.display(),
            text
        );
        assert!(
            text.contains(spec.help),
            "the usage block must describe {}: {:?}",
            spec.display(),
            text
        );
    }
}

#[test]
fn usage_advertises_the_defaults_actually_used() {
    let text = usage();
    assert!(
        text.contains(crate::strings::DEFAULT_OUTPUT_DIR),
        "the default directory must be shown"
    );
    assert!(
        text.contains(&format!("{DEFAULT_WIDTH:.1}")),
        "the default width must be shown"
    );
    assert!(
        text.contains(&format!("{DEFAULT_SCALE:.1}")),
        "the default scale must be shown"
    );
    assert!(text.contains(&format!("{} —", crate::strings::PROGRAM)));
    assert!(text.contains(crate::strings::USAGE_SYNOPSIS));
}

#[test]
fn every_usage_row_starts_at_the_help_column() {
    let row = option_row("--x", "does a thing");
    assert!(row.starts_with("  "));
    assert!(row.ends_with("does a thing\n"));
    let padded = row.trim_end().find("does a thing").expect("description");
    assert!(padded >= HELP_COLUMN);
}

#[test]
fn applying_show_help_changes_nothing() {
    // `parse` answers `--help` before a value is read; the arm exists so the
    // match is exhaustive, and this pins that it really is a no-op.
    let mut options = Options::default();
    apply(&mut options, Effect::ShowHelp, "--help", "ignored").expect("no error");
    assert_eq!(options, Options::default());
}

#[test]
fn positive_number_accepts_what_a_raster_can_use() {
    assert_eq!(positive_number("--width", "700"), Ok(700.0));
    assert_eq!(positive_number("--width", "0.5"), Ok(0.5));
    assert!(positive_number("--width", "0").is_err());
    assert!(positive_number("--width", "abc").is_err());
}

#[test]
fn the_default_options_match_the_documented_defaults() {
    let options = Options::default();
    assert_eq!(options.width, DEFAULT_WIDTH);
    assert_eq!(options.scale, DEFAULT_SCALE);
    assert!(options.filter.is_none());
    assert_eq!(options.output_dir, PathBuf::from(DEFAULT_OUTPUT_DIR_TEXT));
}
