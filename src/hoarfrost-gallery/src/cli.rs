//! The command line: what the program accepts and what each token means.
//!
//! Hand-rolled and table-driven rather than pulled from a CLI crate, because the
//! grammar is five tokens long and the one thing a framework cannot prevent is
//! the failure this module is specifically built to avoid: a parser that accepts
//! a flag the help text never mentioned, or a help text that advertises a flag
//! the parser rejects. Here [`FLAGS`] is the single source of truth for both —
//! [`usage`] renders itself from the table — so the two cannot drift, and
//! `cli_tests.rs` asserts that every flag in the table appears in the help text
//! and that the advertised defaults are the defaults actually used.
//!
//! Parsing is pure: [`parse`] takes a slice of strings and returns a typed
//! [`Outcome`]. It reads no environment, touches no filesystem and never exits,
//! so every branch is reachable from a test and the entry point stays a composer
//! rather than a second implementation.
//!
//! Unit tests live in the sibling `cli_tests.rs` sidecar.

use std::path::PathBuf;

use crate::strings;

/// The card width a run uses when nothing says otherwise.
///
/// Matches the demo pane's own viewport width, so a PNG measured here is
/// measured at the width the section was designed for.
pub const DEFAULT_WIDTH: f32 = 700.0;

/// The raster scale a run uses when nothing says otherwise.
///
/// Two physical pixels per logical one, so a glyph's landing point resolves to
/// half a logical pixel instead of being rounded away.
pub const DEFAULT_SCALE: f32 = 2.0;

/// The column each option's description starts in, in the usage block.
pub const HELP_COLUMN: usize = 26;

/// What a flag does with the token after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Ends the parse and asks for the usage block to be printed.
    ShowHelp,
    /// Sets [`Options::output_dir`].
    Out,
    /// Sets [`Options::width`].
    Width,
    /// Sets [`Options::scale`].
    Scale,
    /// Sets [`Options::filter`].
    Filter,
}

/// One row of the flag table.
///
/// The table describes the grammar *and* the help text, which is what keeps them
/// from disagreeing.
#[derive(Clone, Copy, Debug)]
pub struct FlagSpec {
    /// The token the user types, including the leading `--`.
    pub name: &'static str,
    /// The placeholder shown after the flag in the usage block; empty when the
    /// flag takes no value.
    pub placeholder: &'static str,
    /// The one-line description shown in the usage block.
    pub help: &'static str,
    /// What the parser does with the token after this flag.
    pub effect: Effect,
}

impl FlagSpec {
    /// The flag as the usage block names it: its token, then its placeholder.
    #[must_use]
    pub fn display(&self) -> String {
        if self.placeholder.is_empty() {
            return self.name.to_owned();
        }
        format!("{} {}", self.name, self.placeholder)
    }
}

/// Every flag the parser accepts. A flag missing from this table is a flag the
/// program does not have.
pub const FLAGS: [FlagSpec; 5] = [
    FlagSpec {
        name: strings::FLAG_OUT,
        placeholder: strings::PLACEHOLDER_DIR,
        help: strings::HELP_OUT,
        effect: Effect::Out,
    },
    FlagSpec {
        name: strings::FLAG_WIDTH,
        placeholder: strings::PLACEHOLDER_WIDTH,
        help: strings::HELP_WIDTH,
        effect: Effect::Width,
    },
    FlagSpec {
        name: strings::FLAG_SCALE,
        placeholder: strings::PLACEHOLDER_SCALE,
        help: strings::HELP_SCALE,
        effect: Effect::Scale,
    },
    FlagSpec {
        name: strings::FLAG_FILTER,
        placeholder: strings::PLACEHOLDER_FILTER,
        help: strings::HELP_FILTER,
        effect: Effect::Filter,
    },
    FlagSpec {
        name: strings::FLAG_HELP,
        placeholder: "",
        help: strings::HELP_HELP,
        effect: Effect::ShowHelp,
    },
];

/// Where the run writes and how it draws, once the arguments have parsed.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// The directory the PNGs and the manifest are written into.
    pub output_dir: PathBuf,
    /// Card width in logical pixels.
    pub width: f32,
    /// Physical pixels per logical pixel.
    pub scale: f32,
    /// When set, only sections whose id contains this fragment are rendered.
    pub filter: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from(strings::DEFAULT_OUTPUT_DIR),
            width: DEFAULT_WIDTH,
            scale: DEFAULT_SCALE,
            filter: None,
        }
    }
}

/// The result of a parse.
///
/// Three outcomes because the caller has three genuinely different jobs: print
/// help and succeed, run, or complain and fail. Collapsing them into
/// `Result<Options, String>` would make `--help` an error that the caller has to
/// special-case back out again.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// `--help` was asked for. Print [`usage`] and exit successfully.
    Help,
    /// The arguments parsed into a complete set of options.
    Run(Options),
    /// The arguments do not parse. The payload is the reason, without any
    /// prefix — the caller owns the prefix.
    Invalid(String),
}

/// The flag table entry for `token`, if there is one.
fn flag_of(token: &str) -> Option<&'static FlagSpec> {
    FLAGS.iter().find(|flag| flag.name == token)
}

/// The message for a flag given with nothing after it.
fn missing_value(flag: &str) -> String {
    format!("{flag} {}", strings::ERR_MISSING_VALUE)
}

/// The message for a token that looks like a flag and is not one.
fn unknown_flag(token: &str) -> String {
    format!("{} {token}", strings::ERR_UNKNOWN_FLAG)
}

/// Parse `value` as a finite number greater than zero.
///
/// Rejects `0`, negatives, `inf` and `nan` alike: a zero width produces an empty
/// image and a negative or infinite one produces no image at all, and neither is
/// worth writing a PNG-shaped file for.
fn positive_number(flag: &str, value: &str) -> Result<f32, String> {
    let parsed = value
        .parse::<f32>()
        .map_err(|_| format!("{flag} {}: {value}", strings::ERR_NOT_A_NUMBER))?;

    if !parsed.is_finite() || parsed <= 0.0 {
        return Err(format!("{flag} {}: {value}", strings::ERR_NOT_POSITIVE));
    }

    Ok(parsed)
}

/// Write one flag's value into `options`.
///
/// [`Effect::ShowHelp`] is answered by [`parse`] before a value is ever read;
/// the arm is here so the match covers the enum, and it is a no-op rather than
/// an unreachable panic so the function stays honest when called directly.
fn apply(options: &mut Options, effect: Effect, flag: &str, value: &str) -> Result<(), String> {
    match effect {
        Effect::ShowHelp => Ok(()),
        Effect::Out => {
            if value.is_empty() {
                return Err(format!("{flag} {}", strings::ERR_EMPTY_OUTPUT_DIR));
            }
            options.output_dir = PathBuf::from(value);
            Ok(())
        }
        Effect::Width => {
            options.width = positive_number(flag, value)?;
            Ok(())
        }
        Effect::Scale => {
            options.scale = positive_number(flag, value)?;
            Ok(())
        }
        Effect::Filter => {
            if value.is_empty() {
                return Err(format!("{flag} {}", strings::ERR_EMPTY_FILTER));
            }
            options.filter = Some(value.to_owned());
            Ok(())
        }
    }
}

/// Consume the flag at `index` together with its value.
///
/// Returns the index of the next unconsumed token, or the reason the run cannot
/// continue.
fn consume_flag(
    options: &mut Options,
    effect: Effect,
    token: &str,
    args: &[String],
    index: usize,
) -> Result<usize, String> {
    let value = args.get(index + 1).ok_or_else(|| missing_value(token))?;

    apply(options, effect, token, value)?;
    Ok(index + 2)
}

/// Parse command-line arguments into an [`Outcome`].
///
/// `args` is the argument vector *without* the program name. The output directory
/// may be named by `--out` or, as the shorter habit, by the first bare token;
/// naming it both ways is an error rather than a silent precedence rule, because
/// a run that writes somewhere other than the path a person typed is exactly the
/// mistake this tool cannot afford. A second bare token is refused for the same
/// reason. Repeating a value flag keeps the last value, as shells and compilers
/// conventionally do.
///
/// ```
/// # use hoarfrost_gallery::cli::{Outcome, parse};
/// let outcome = parse(&["--out".to_owned(), "out".to_owned(), "--width".to_owned(), "800".to_owned()]);
/// assert!(matches!(outcome, Outcome::Run(options) if options.width == 800.0));
/// ```
#[must_use]
pub fn parse(args: &[String]) -> Outcome {
    let mut options = Options::default();
    let mut positional: Option<String> = None;
    let mut out_by_flag = false;
    let mut index = 0;

    while index < args.len() {
        let token = args[index].as_str();

        if let Some(spec) = flag_of(token) {
            if spec.effect == Effect::ShowHelp {
                return Outcome::Help;
            }
            if spec.effect == Effect::Out {
                out_by_flag = true;
            }
            index = match consume_flag(&mut options, spec.effect, token, args, index) {
                Ok(next) => next,
                Err(reason) => return Outcome::Invalid(reason),
            };
            continue;
        }

        if token.starts_with('-') {
            return Outcome::Invalid(unknown_flag(token));
        }

        if positional.is_some() {
            return Outcome::Invalid(format!("{token} {}", strings::ERR_EXTRA_ARGUMENT));
        }

        positional = Some(token.to_owned());
        index += 1;
    }

    if let Some(directory) = positional {
        if out_by_flag {
            return Outcome::Invalid(strings::ERR_OUT_GIVEN_TWICE.to_owned());
        }
        options.output_dir = PathBuf::from(directory);
    }

    Outcome::Run(options)
}

/// Whether `section_id` survives `options`' filter.
///
/// Substring containment, not a glob: the ids are already slugs, so a fragment
/// like `tab` is the whole of what a person types to isolate the tab sections.
/// No filter means everything survives.
#[must_use]
pub fn accepts(options: &Options, section_id: &str) -> bool {
    options
        .filter
        .as_ref()
        .is_none_or(|fragment| section_id.contains(fragment.as_str()))
}

/// One line of the usage block: the option, padded, then its description.
fn option_row(name: &str, help: &str) -> String {
    format!("  {:width$}{help}\n", name, width = HELP_COLUMN)
}

/// The `--help` text, rendered from [`FLAGS`] and the defaults.
///
/// Built from the table rather than written out by hand so a flag can never be
/// accepted in silence or advertised in vain.
#[must_use]
pub fn usage() -> String {
    let mut text = format!("{} — {}\n\n", strings::PROGRAM, strings::USAGE_PURPOSE);

    text.push_str(strings::USAGE_LABEL);
    text.push('\n');
    text.push_str(strings::USAGE_SYNOPSIS);
    text.push('\n');

    text.push('\n');
    text.push_str(strings::FLAGS_LABEL);
    text.push('\n');
    text.push_str(&option_row(
        strings::ARG_OUTPUT_DIR,
        strings::HELP_OUTPUT_DIR,
    ));
    for flag in &FLAGS {
        text.push_str(&option_row(&flag.display(), flag.help));
    }

    text.push('\n');
    text.push_str(strings::DEFAULTS_LABEL);
    text.push('\n');
    text.push_str(&option_row(
        strings::ARG_OUTPUT_DIR,
        &format!("= {}", strings::DEFAULT_OUTPUT_DIR),
    ));
    text.push_str(&option_row(
        strings::FLAG_WIDTH,
        &format!("= {DEFAULT_WIDTH:.1}"),
    ));
    text.push_str(&option_row(
        strings::FLAG_SCALE,
        &format!("= {DEFAULT_SCALE:.1}"),
    ));

    text
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
