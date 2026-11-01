//! Tests for the manifest builder.
//!
//! The manifest is the only record of what a sweep was *asked* for, so the cases
//! here are about completeness and order rather than formatting taste: nothing
//! may be dropped, nothing may be reordered, and two runs over the same entries
//! must agree byte for byte.

use super::*;

/// The entry lines of a manifest, with the header, columns and summary removed.
fn entry_lines(text: &str) -> Vec<&str> {
    text.lines()
        .skip(2)
        .filter(|line| !line.is_empty() && !line.starts_with(COMMENT_PREFIX))
        .collect()
}

/// The `# label: count` summary at the end of a manifest.
fn summary(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|line| line.starts_with(COMMENT_PREFIX) && !line.starts_with("# status"))
        .skip(1)
        .collect()
}

#[test]
fn an_empty_sweep_still_produces_a_well_formed_manifest() {
    let text = manifest(&[]);
    let lines: Vec<&str> = text.lines().collect();

    assert_eq!(lines[0], crate::strings::MANIFEST_HEADER);
    assert_eq!(lines[1], crate::strings::MANIFEST_COLUMNS);
    assert_eq!(lines.last(), Some(&"# total: 0"));
    assert!(text.ends_with('\n'), "the file must end with a newline");

    // A reader must be able to tell "nothing was asked for" from "the file was
    // truncated", so the zeros are printed rather than the summary skipped.
    assert!(entry_lines(&text).is_empty());
    assert_eq!(
        summary(&text),
        vec!["# rendered: 0", "# failed: 0", "# total: 0"]
    );
}

#[test]
fn one_rendered_entry_is_one_line_with_five_fields() {
    let entries = [Entry::rendered(
        "demo.section.buttons",
        "buttons.png",
        1400,
        900,
    )];
    let text = manifest(&entries);
    let lines = entry_lines(&text);

    assert_eq!(lines.len(), 1);
    let fields: Vec<&str> = lines[0].split(FIELD_SEPARATOR).collect();
    assert_eq!(
        fields,
        vec![
            "rendered",
            "demo.section.buttons",
            "buttons.png",
            "1400x900",
            "-"
        ]
    );
}

#[test]
fn a_failed_entry_carries_the_reason_and_no_file() {
    let entries = [Entry::failed(
        "demo.section.broken",
        "no container published the id",
    )];
    let text = manifest(&entries);
    let lines = entry_lines(&text);

    assert_eq!(lines.len(), 1);
    let fields: Vec<&str> = lines[0].split(FIELD_SEPARATOR).collect();
    assert_eq!(
        fields,
        vec![
            "failed",
            "demo.section.broken",
            crate::strings::FIELD_NONE,
            "0x0",
            "no container published the id"
        ]
    );
}

#[test]
fn a_mixed_sweep_counts_both_kinds_and_keeps_input_order() {
    let entries = vec![
        Entry::rendered("demo.section.a", "a.png", 10, 20),
        Entry::failed("demo.section.b", "measured a box with no area"),
        Entry::rendered("demo.section.c", "c.png", 30, 40),
    ];
    let text = manifest(&entries);

    let ids: Vec<String> = entry_lines(&text)
        .iter()
        .map(|line| line.split(FIELD_SEPARATOR).nth(1).expect("a section field"))
        .map(str::to_owned)
        .collect();
    assert_eq!(
        ids,
        vec!["demo.section.a", "demo.section.b", "demo.section.c"]
    );

    assert_eq!(
        summary(&text),
        vec!["# rendered: 2", "# failed: 1", "# total: 3"]
    );
}

#[test]
fn a_declared_height_entry_says_so_in_the_detail_column() {
    // The distinction the manifest exists to preserve: a height the tool chose
    // must not look like a height the widget reported.
    let entries = [Entry::rendered_declared(
        "demo.section.empty-states-uif-comp-014",
        "empty-states-uif-comp-014.png",
        1400,
        1336,
        668.0,
    )];
    let text = manifest(&entries);
    let lines = entry_lines(&text);

    assert_eq!(lines.len(), 1);
    let fields: Vec<&str> = lines[0].split(FIELD_SEPARATOR).collect();
    assert_eq!(
        fields,
        vec![
            crate::strings::STATUS_RENDERED,
            "demo.section.empty-states-uif-comp-014",
            "empty-states-uif-comp-014.png",
            "1400x1336",
            "declared height 668",
        ]
    );

    // Declared is still rendered: the counts must not treat it as a failure.
    assert!(entries[0].is_rendered());
    assert_eq!(counts(&entries), (1, 0));
    assert!(text.contains("# rendered: 1"));
    assert!(text.contains("# failed: 0"));
}

#[test]
fn a_note_never_overrides_a_failure() {
    // Defensive: `entry_line` prefers the failure, so an entry that somehow
    // carries both cannot hide the reason it produced nothing.
    let mut entry = Entry::rendered("s", "s.png", 10, 10);
    entry.note = Some("declared height 668".to_owned());
    entry.failure = Some("the write failed".to_owned());

    let line = entry_line(&entry);
    let fields: Vec<&str> = line.split(FIELD_SEPARATOR).collect();
    assert_eq!(fields[0], crate::strings::STATUS_FAILED);
    assert_eq!(fields[4], "the write failed");
    assert!(!entry.is_rendered());
}

#[test]
fn the_summary_counts_follow_the_entries_not_the_other_way_round() {
    // All-failed and all-rendered sweeps, because a summary computed from the
    // wrong predicate shows up only at the extremes.
    let all_failed: Vec<Entry> = (0..3)
        .map(|index| Entry::failed(format!("s{index}"), "reason"))
        .collect();
    assert_eq!(
        summary(&manifest(&all_failed)),
        vec!["# rendered: 0", "# failed: 3", "# total: 3"]
    );

    let all_rendered: Vec<Entry> = (0..2)
        .map(|index| Entry::rendered(format!("s{index}"), format!("{index}.png"), 1, 1))
        .collect();
    assert_eq!(
        summary(&manifest(&all_rendered)),
        vec!["# rendered: 2", "# failed: 0", "# total: 2"]
    );
}

#[test]
fn two_runs_over_the_same_entries_are_byte_identical() {
    let entries = vec![
        Entry::rendered("demo.section.a", "a.png", 100, 200),
        Entry::failed("demo.section.b", "boom"),
        Entry::rendered("demo.section.c", "c.png", 300, 400),
    ];

    assert_eq!(manifest(&entries), manifest(&entries));
}

#[test]
fn the_manifest_never_names_an_absolute_path() {
    // Determinism across machines: the manifest records file names, not where
    // the run happened to put them.
    let entries = [Entry::rendered("demo.section.a", "a.png", 1, 1)];
    let text = manifest(&entries);

    for forbidden in ["C:\\", "C:/", "\\\\", "/home/", "/Users/"] {
        assert!(
            !text.contains(forbidden),
            "no machine-specific path may appear: {forbidden}"
        );
    }
}

#[test]
fn entry_helpers_report_their_own_status() {
    let rendered = Entry::rendered("a", "a.png", 1, 1);
    assert!(rendered.is_rendered());
    assert_eq!(rendered.failure, None);

    let failed = Entry::failed("a", "why");
    assert!(!failed.is_rendered());
    assert_eq!(failed.failure.as_deref(), Some("why"));
    assert_eq!(failed.file_name, crate::strings::FIELD_NONE);
    assert_eq!((failed.width, failed.height), (0, 0));
}

#[test]
fn a_failure_reason_containing_a_tab_is_not_silently_absorbed() {
    // A tab inside a reason would shift the columns. Recording it as-is is the
    // honest choice, and this pins the behaviour rather than hiding it.
    let entries = [Entry::failed("a", "line one\tline two")];
    let text = manifest(&entries);
    let lines = entry_lines(&text);
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("line one\tline two"));
}

#[test]
fn counts_split_the_entries_exactly() {
    let entries = vec![
        Entry::rendered("a", "a.png", 1, 1),
        Entry::failed("b", "x"),
        Entry::rendered("c", "c.png", 1, 1),
        Entry::failed("d", "y"),
        Entry::rendered("e", "e.png", 1, 1),
    ];
    assert_eq!(counts(&entries), (3, 2));
    assert_eq!(counts(&[]), (0, 0));
}
