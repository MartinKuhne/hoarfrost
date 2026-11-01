//! The machinery behind `hoarfrost-gallery`: render each hoarfrost demo section
//! to its own PNG file, for documentation and a widget gallery (`UIF-VER-004`).
//!
//! # What this crate is, and what it is not
//!
//! `hoarfrost::gallery` already enumerates every `(Variant × State)` cell and
//! writes a *text* snapshot of each one. That matrix is the assertion target, and
//! it stays the assertion target: a frozen PNG pins every glyph, antialiasing
//! kernel and rasteriser version in the crate, so as a *test* it fails on
//! unrelated upgrades and gets regenerated until it stops meaning anything.
//!
//! The PNGs here are not assertions. They are documentation artifacts — generated
//! on demand, never compared, never committed, written into a directory named on
//! the command line and defaulted under git-ignored `target/`. Nothing in this
//! crate changes an existing test to look at pixels, and nothing in it claims a
//! widget is correct because a picture looked right.
//!
//! # Layout
//!
//! The library holds every decision and the binary holds none of them, so each
//! module is testable on its own and the entry point only composes.
//!
//! | Module | Job |
//! |--------|-----|
//! | [`cli`] | argument parsing, pure and table-driven |
//! | [`catalog`] | the demo's own sections, enumerated and rebuilt on demand |
//! | [`capture`] | measure one card, then rasterise exactly that box |
//! | [`png`] | an RGBA8 buffer to a PNG file, or a clear refusal |
//! | [`output`] | where a run may write, and what a file may be called |
//! | [`mod@sweep`] | the render loop: one target in, one manifest line out |
//! | [`mod@manifest`] | the index that makes a run inspectable afterwards |
//! | [`strings`] | every word a human sees |
//!
//! # Scope
//!
//! The sweep renders the demo's real sections: the catalog is built from
//! `hoarfrost_demo`, the renderer draws with `demo_theme()`, and the binary
//! writes one PNG per section plus the manifest that says which ones did not
//! make it. Hover, pressed and focused states are not in a static section, and
//! they are not here either.

pub mod capture;
pub mod catalog;
pub mod cli;
pub mod manifest;
pub mod output;
pub mod png;
pub mod strings;
pub mod sweep;

pub use capture::CaptureError;
pub use capture::Renderer as CaptureRenderer;
pub use capture::Shot;
pub use catalog::Catalog;
pub use catalog::Target;
pub use cli::Options;
pub use cli::Outcome;
pub use manifest::Entry;
pub use manifest::manifest;
pub use output::guard;
pub use output::png_file_name;
pub use output::sanitize_stem;
pub use png::write_png;
pub use sweep::sweep;
