//! Measure one card, then rasterise exactly that box — the core of the sweep.
//!
//! # Why two passes
//!
//! A demo section's card is `Length::Fill` width with natural height, so it has
//! no intrinsic size of its own: asked for at any width it becomes that width.
//! A PNG of it therefore needs the width the run was given and the height the
//! card *chose*, and the only honest way to learn the second is to lay the card
//! out and ask.
//!
//! So each section is built twice. Pass one builds it inside a generous probe
//! viewport and reads the card's own rectangle back out of the widget tree.
//! Pass two rebuilds it at precisely that rectangle and rasterises the whole
//! frame. Because the card is fill-width and natural-height, rebuilding it at
//! its own measured size reproduces it exactly — the image is tight with no
//! crop arithmetic, which is the kind of arithmetic that quietly drifts.
//!
//! Two builds rather than one build read twice is forced and harmless:
//! `iced::Element` owns a boxed widget and is not `Clone`, and the builders are
//! pure functions of their inputs, so the two trees are identical. (Same
//! reasoning as the "Why two builds" note in
//! `src/hoarfrost-demo/src/render_section.rs`.)
//!
//! # Why generic over the builder, not over a section type
//!
//! The capture driver takes `impl Fn() -> iced::Element`, not a `Section`. It
//! needs nothing but an element and an id to look for, so it stays independent
//! of whatever the catalog happens to look like and is testable with a
//! hand-built card (`RUST-020`).
//!
//! # One renderer for the whole run
//!
//! Building iced's headless renderer costs roughly 620 ms. A sweep of fifty
//! sections would spend half a minute in renderer construction alone if it
//! built one per section, so [`Renderer`] pays that cost once and reuses it.
//!
//! # Failure is reported, never papered over
//!
//! Every way a measurement can be untrustworthy has its own [`CaptureError`]
//! variant, and a section that hits one produces no image at all. A PNG of the
//! wrong size looks like a PNG; a manifest line that says *why* the section is
//! missing is the difference between a report and a silent lie.
//!
//! Unit tests live in the sibling `capture_tests.rs` sidecar.

use std::error::Error;
use std::fmt;

use iced::Color;
use iced::Element;
use iced::Rectangle;
use iced::Size;
use iced::Theme;
use iced::advanced::renderer;
use iced::advanced::renderer::Headless as _;
use iced::advanced::widget::Id;
use iced::advanced::widget::Operation;
use iced::advanced::widget::operation::Outcome;
use iced::advanced::widget::operation::black_box;
use iced::mouse;

use iced_runtime::UserInterface;
use iced_runtime::user_interface;

use hoarfrost::test_helpers::Candidate;
use hoarfrost::test_helpers::Capture;
use hoarfrost::test_helpers::KIND_CONTAINER;
use hoarfrost::test_helpers::PIXEL_SCALE;
use hoarfrost::test_helpers::RecordCandidates;

use crate::png::expected_bytes;
use crate::strings;

/// The viewport height the measure pass builds at.
///
/// The probe has to be taller than any section, because a card that overflows
/// the viewport is measured *as if* it ended at the viewport edge and the run
/// would emit a clipped image while reporting success. It matches the demo's
/// own `PANE_VIEWPORT` height so a section measured here measures the same size
/// it does in the demo window's layout tests.
///
/// **It is therefore a ceiling, not a measurement.** iced builds a
/// `UserInterface` with layout limits derived from the viewport, so a card that
/// wants more than `PROBE_HEIGHT` does not report a larger number — it is
/// clamped to exactly `PROBE_HEIGHT`. A card that measures *at* the ceiling is
/// indistinguishable from one that was clipped to it, so `validate` refuses
/// `>=` rather than `>`: the honest reading of "exactly as tall as the probe" is
/// "did not fit, and the probe cannot tell me how tall it really is".
///
/// One real section trips this today: `demo.section.empty-states-uif-comp-014`,
/// because `hoarfrost::composite::empty_state` lays its content out at
/// `Length::Fill` height. Such a card has no natural height — it measures the
/// viewport it is given, at *any* probe height, so raising this constant does not
/// help it. That section is drawn through [`Renderer::shot_at`] at a height the
/// catalog declares instead; see the table in [`crate::catalog`].
pub const PROBE_HEIGHT: f32 = 1600.0;

/// The background a card's uncovered pixels are painted with.
///
/// Black, the same choice `hoarfrost::test_helpers` makes for its own captures,
/// so a PNG and a test frame of the same section agree on what "nothing was
/// painted here" looks like.
pub const DEFAULT_BACKGROUND: Color = Color::BLACK;

/// The scale a run rasterises at when nothing says otherwise.
///
/// Two physical pixels per logical one, shared with the framework's pixel tests
/// so glyph landing points resolve the same way in both.
pub const DEFAULT_SCALE: f32 = PIXEL_SCALE;

/// The theme a [`Renderer`] draws with when nothing says otherwise.
///
/// `Theme::Dark`, matching `hoarfrost::test_helpers` and the demo, which
/// hardcodes a dark palette. A light-theme axis would be a demo change, not a
/// gallery one.
///
/// This is a *default*, not a policy: the theme decides what iced's own
/// primitives (containers, text, scrollables) resolve their colours to, so a
/// sweep that wants to reproduce what the demo window actually shows passes
/// `hoarfrost_demo::theme::demo_theme()` to [`Renderer::with_theme`] instead.
pub const THEME: Theme = Theme::Dark;

/// The stable name inside an [`Id`], or `None` when the id carries no name.
///
/// `iced_core::widget::Id` exposes no accessor for its text — only `Debug` — so
/// the name has to be read off the debug rendering. `Id::new("x")` and
/// `Id::from(String)` both render as a `Custom` variant carrying the quoted
/// name; `Id::unique()` renders as `Unique(n)` over a process-global counter
/// that changes between runs, so it is rejected rather than matched. This is
/// the same reading the demo's own layout probe performs.
fn id_name(id: &Id) -> Option<String> {
    let rendered = format!("{id:?}");
    if !rendered.contains("Custom") {
        return None;
    }
    let start = rendered.find('"')?;
    let end = rendered.rfind('"')?;
    (end > start).then(|| rendered[start + 1..end].to_string())
}

/// Whether every component of a measured rectangle is a real number.
///
/// Checked before the size checks, because every comparison against `NaN` is
/// false: a non-finite width would sail past "is it zero" and "is it too tall"
/// and be rasterised. Position is included as well as size — a card anchored at
/// a `NaN` point is as unusable as one with a `NaN` size, even though the raster
/// itself only reads the size.
fn is_finite(card: &Rectangle) -> bool {
    [card.x, card.y, card.width, card.height]
        .iter()
        .all(|value| value.is_finite())
}

/// Why a `width` × `height` box cannot be rasterised, before anyone knows whether
/// it was measured by the probe or declared by the caller.
///
/// The two checks live here, in this order, rather than in each caller, because
/// the order is load-bearing: finiteness must be tested first, because every
/// comparison against a non-finite value is false and a `NaN` height would pass
/// a "is it greater than zero" test on its way to a nonsense raster.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BoxProblem {
    /// A dimension is `NaN` or infinite.
    NotFinite,
    /// A dimension is zero or negative, so the image would cover no pixels.
    NoArea,
}

/// The first reason `width` × `height` cannot become an image, if there is one.
fn box_problem(width: f32, height: f32) -> Option<BoxProblem> {
    if !(width.is_finite() && height.is_finite()) {
        return Some(BoxProblem::NotFinite);
    }
    if !(width > 0.0 && height > 0.0) {
        return Some(BoxProblem::NoArea);
    }
    None
}

/// The physical pixel size a logical `viewport` rasterises to at `scale`.
fn physical_size(viewport: Size, scale: f32) -> Size<u32> {
    Size::new(
        (viewport.width * scale).round() as u32,
        (viewport.height * scale).round() as u32,
    )
}

/// A rasterised card: its pixel size and the RGBA8 pixels themselves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shot {
    /// Width in physical pixels.
    pub width: u32,
    /// Height in physical pixels.
    pub height: u32,
    /// Row-major RGBA8 pixels, exactly `width * height * 4` of them.
    pub rgba: Vec<u8>,
}

impl Shot {
    /// Whether the pixel count matches the declared size.
    ///
    /// The invariant every consumer relies on, exposed so a caller (and the
    /// PNG writer's own check) can state it without recomputing the product.
    #[must_use]
    pub fn is_well_formed(&self) -> bool {
        u64::try_from(self.rgba.len()) == Ok(expected_bytes(self.width, self.height))
    }
}

/// Why one section could not be turned into an image.
///
/// One variant per distinct way a measurement can be untrustworthy, so the
/// manifest can say *which* thing went wrong rather than "it failed".
#[derive(Clone, Debug, PartialEq)]
pub enum CaptureError {
    /// The measure pass never saw a `container` published under this id, so
    /// there is no box to rasterise. Usually means the section's card lost its
    /// `.id(...)`, or the id and the element disagree.
    CardNotFound {
        /// The id that was looked for.
        card_id: String,
    },
    /// The card laid out at zero width or zero height. An image of nothing is
    /// not an image.
    CardHasNoArea {
        /// The id that measured empty.
        card_id: String,
        /// The width that came back.
        width: f32,
        /// The height that came back.
        height: f32,
    },
    /// A [`Renderer::shot_at`] canvas that is not a usable box: zero, negative
    /// or non-finite in either dimension.
    ///
    /// Separate from [`CaptureError::CardHasNoArea`] because the fix is in a
    /// different place. That one means the *measurement* is wrong and points at
    /// the widget; this one means the *declared* size is wrong and points at the
    /// table in [`crate::catalog`]. A manifest reader should not have to guess
    /// which of the two to go and change.
    DeclaredSizeHasNoArea {
        /// The section whose declared canvas was unusable.
        section_id: String,
        /// The width that was declared.
        width: f32,
        /// The height that was declared.
        height: f32,
    },
    /// The card's rectangle carried a non-finite component, so no pixel size
    /// can be derived from it.
    CardNotFinite {
        /// The id that measured non-finite.
        card_id: String,
        /// The width that came back.
        width: f32,
        /// The height that came back.
        height: f32,
    },
    /// A [`Renderer::shot_at`] canvas with a non-finite dimension.
    ///
    /// Split from [`CaptureError::CardNotFinite`] for the same reason
    /// [`CaptureError::DeclaredSizeHasNoArea`] is split from
    /// [`CaptureError::CardHasNoArea`]: the reader needs to know whether to fix
    /// a widget or a table entry.
    DeclaredSizeNotFinite {
        /// The section whose declared canvas was not finite.
        section_id: String,
        /// The width that was declared.
        width: f32,
        /// The height that was declared.
        height: f32,
    },
    /// The card is at or above [`PROBE_HEIGHT`], so the probe clipped it and the
    /// height is a ceiling, not a measurement.
    ///
    /// iced derives a `UserInterface`'s layout limits from the viewport it was
    /// built with, so a card that wants more than the probe gives exactly
    /// `probe_height` back — the same number a card that happens to be exactly
    /// that tall produces. The two cannot be told apart from the measurement, so
    /// both are refused rather than one of them being guessed at.
    CardTooTall {
        /// The id that did not fit the probe.
        card_id: String,
        /// The clipped height that came back.
        height: f32,
        /// The probe height it did not fit inside.
        probe_height: f32,
    },
    /// The rasteriser handed back a buffer whose length disagrees with the size
    /// it was asked for. Reported rather than written out.
    PixelCountMismatch {
        /// The id whose raster was inconsistent.
        card_id: String,
        /// The physical width that was requested.
        width: u32,
        /// The physical height that was requested.
        height: u32,
        /// The byte count that came back.
        actual: usize,
    },
}

impl fmt::Display for CaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CardNotFound { card_id } => {
                write!(f, "{} \"{card_id}\"", strings::ERR_CARD_NOT_FOUND)
            }
            Self::CardHasNoArea {
                card_id,
                width,
                height,
            } => write!(
                f,
                "{} \"{card_id}\" ({width} x {height})",
                strings::ERR_CARD_NO_AREA
            ),
            Self::DeclaredSizeHasNoArea {
                section_id,
                width,
                height,
            } => write!(
                f,
                "{} \"{section_id}\" ({width} x {height})",
                strings::ERR_DECLARED_NO_AREA
            ),
            Self::CardNotFinite {
                card_id,
                width,
                height,
            } => write!(
                f,
                "{} \"{card_id}\" ({width} x {height})",
                strings::ERR_CARD_NOT_FINITE
            ),
            Self::DeclaredSizeNotFinite {
                section_id,
                width,
                height,
            } => write!(
                f,
                "{} \"{section_id}\" ({width} x {height})",
                strings::ERR_DECLARED_NOT_FINITE
            ),
            Self::CardTooTall {
                card_id,
                height,
                probe_height,
            } => write!(
                f,
                "{} \"{card_id}\" ({height} >= {probe_height})",
                strings::ERR_CARD_TOO_TALL
            ),
            Self::PixelCountMismatch {
                card_id,
                width,
                height,
                actual,
            } => write!(
                f,
                "{} \"{card_id}\" ({width} x {height}, {actual} bytes)",
                strings::ERR_PIXEL_COUNT
            ),
        }
    }
}

impl Error for CaptureError {}

/// Owns one headless renderer and turns card builders into images with it.
///
/// The renderer is the expensive part of a run, so it is built once by
/// [`Renderer::new`] and every section reuses it. Each call rebuilds its own
/// interface from scratch, which is what an application does between `view`
/// calls and what keeps one section's widget state from leaking into the next.
///
/// The [`Theme`] is part of the renderer's identity rather than a per-call
/// argument because a sweep has exactly one of them and every section in it must
/// be drawn with that one: mixing themes across a directory of PNGs would make
/// the set unreadable as a set.
pub struct Renderer {
    session: Capture,
    theme: Theme,
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer {
    /// Build a renderer with the documented [`THEME`] default, paying the
    /// headless-renderer cost once for the run.
    #[must_use]
    pub fn new() -> Self {
        Self::with_theme(THEME)
    }

    /// Build a renderer that draws every card with `theme`.
    ///
    /// The gallery passes `hoarfrost_demo::theme::demo_theme()`, so the PNGs
    /// resolve iced's own primitives the way the demo window does instead of
    /// against iced's stock dark palette.
    #[must_use]
    pub fn with_theme(theme: Theme) -> Self {
        Self {
            session: Capture::new(),
            theme,
        }
    }

    /// The theme this renderer draws with.
    #[must_use]
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// The card `build()` produces, as a rectangle in logical pixels.
    ///
    /// # Errors
    ///
    /// [`CaptureError::CardNotFound`] when no container published `card_id`,
    /// [`CaptureError::CardNotFinite`], [`CaptureError::CardHasNoArea`] or
    /// [`CaptureError::CardTooTall`] when the measurement cannot be trusted.
    pub fn measure<'a, Message: 'static>(
        &mut self,
        card_id: &str,
        width: f32,
        build: impl Fn() -> Element<'a, Message>,
    ) -> Result<Rectangle, CaptureError> {
        let candidates = self.record(Size::new(width, PROBE_HEIGHT), build);
        let card = find_card(&candidates, card_id).ok_or_else(|| CaptureError::CardNotFound {
            card_id: card_id.to_owned(),
        })?;

        validate(card_id, card)
    }

    /// Measure `build()`'s card and rasterise exactly that box.
    ///
    /// `build` is called twice — once per pass — because an `Element` is not
    /// `Clone`. `scale` is physical pixels per logical pixel; `background`
    /// shows through wherever the card paints nothing.
    ///
    /// # Errors
    ///
    /// Anything [`Renderer::measure`] can report, plus
    /// [`CaptureError::PixelCountMismatch`] if the rasteriser's buffer does not
    /// match the size it was asked for.
    pub fn shot<'a, Message: 'static>(
        &mut self,
        card_id: &str,
        width: f32,
        scale: f32,
        background: Color,
        build: impl Fn() -> Element<'a, Message>,
    ) -> Result<Shot, CaptureError> {
        let card = self.measure(card_id, width, &build)?;
        self.rasterise(
            card_id,
            Size::new(card.width, card.height),
            scale,
            background,
            build,
        )
    }

    /// Rasterise `build()` at a size the caller *declared* rather than one the
    /// probe measured.
    ///
    /// # When this is the honest call
    ///
    /// A widget laid out at `Length::Fill` height has no natural height: asked
    /// what it is, it answers "whatever you gave me", at every probe height, so
    /// the measure pass has nothing to report and [`Renderer::shot`] refuses it.
    /// A screenshot tool still has to choose a canvas for it, and this is that
    /// choice made explicitly — the alternative is a gallery with a hole in it,
    /// or a height invented silently somewhere the manifest cannot show it.
    ///
    /// There is no measure pass here, so there is no measurement to distrust:
    /// [`CaptureError::CardNotFound`] and [`CaptureError::CardTooTall`] cannot
    /// occur, and [`PROBE_HEIGHT`] does not apply — a declared height is not a
    /// claim about what the widget wanted, so it cannot be a clipped claim.
    ///
    /// `card_id` is still taken, even though nothing is looked up with it: the
    /// errors this can return name the section they failed on, and the manifest
    /// line has to say *which* section it is about. A placeholder id in those
    /// messages would be a small lie in the one place a reader trusts to be
    /// true, so the id is threaded through for the error text alone.
    ///
    /// # Errors
    ///
    /// [`CaptureError::DeclaredSizeNotFinite`] or
    /// [`CaptureError::DeclaredSizeHasNoArea`] when the declared canvas cannot
    /// become an image, [`CaptureError::PixelCountMismatch`] when the raster's
    /// buffer does not match it.
    pub fn shot_at<'a, Message: 'static>(
        &mut self,
        card_id: &str,
        width: f32,
        height: f32,
        scale: f32,
        background: Color,
        build: impl Fn() -> Element<'a, Message>,
    ) -> Result<Shot, CaptureError> {
        let viewport = validate_declared(card_id, width, height)?;
        self.rasterise(card_id, viewport, scale, background, build)
    }

    /// Lay `build()` out at `viewport` and rasterise the whole frame.
    ///
    /// The shared body of [`Renderer::shot`] and [`Renderer::shot_at`]: the two
    /// differ only in where the box came from, and a rasteriser that behaved
    /// differently depending on that would be a bug waiting to happen.
    ///
    /// # Errors
    ///
    /// [`CaptureError::PixelCountMismatch`] when the buffer's length disagrees
    /// with the size that was asked for.
    fn rasterise<'a, Message: 'static>(
        &mut self,
        card_id: &str,
        viewport: Size,
        scale: f32,
        background: Color,
        build: impl Fn() -> Element<'a, Message>,
    ) -> Result<Shot, CaptureError> {
        let physical = physical_size(viewport, scale);

        let mut ui = UserInterface::build(
            build(),
            viewport,
            user_interface::Cache::default(),
            self.session.renderer_mut(),
        );

        ui.draw(
            self.session.renderer_mut(),
            &self.theme,
            &renderer::Style {
                text_color: Color::WHITE,
            },
            mouse::Cursor::Unavailable,
        );

        let rgba = self
            .session
            .renderer_mut()
            .screenshot(physical, scale, background);

        let shot = Shot {
            width: physical.width,
            height: physical.height,
            rgba,
        };

        if shot.is_well_formed() {
            Ok(shot)
        } else {
            Err(CaptureError::PixelCountMismatch {
                card_id: card_id.to_owned(),
                width: shot.width,
                height: shot.height,
                actual: shot.rgba.len(),
            })
        }
    }

    /// Lay `build()` out at `viewport` and record every widget the tree
    /// publishes, in traversal order.
    ///
    /// The operation is run through its whole `Outcome::Chain`, the way
    /// `iced_winit`'s own widget-action handler does. A single `operate` pass
    /// can stop early: an operation whose `finish` returns `Chain(next)` has a
    /// second half that never runs unless the caller re-runs it, and a recorder
    /// that stopped early would report a tree that is short rather than a tree
    /// that is empty.
    fn record<'a, Message: 'static>(
        &mut self,
        viewport: Size,
        build: impl Fn() -> Element<'a, Message>,
    ) -> Vec<Candidate> {
        let mut ui = UserInterface::build(
            build(),
            viewport,
            user_interface::Cache::default(),
            self.session.renderer_mut(),
        );

        let mut operation: Box<dyn Operation<Vec<Candidate>>> = Box::<RecordCandidates>::default();

        loop {
            ui.operate(self.session.renderer(), &mut black_box(operation.as_mut()));

            match operation.finish() {
                Outcome::Some(candidates) => return candidates,
                Outcome::None => return Vec::new(),
                Outcome::Chain(next) => operation = next,
            }
        }
    }
}

/// The first `container` candidate published under `card_id`.
///
/// Filtered on `kind` as well as id because the operation stream carries several
/// kinds of box under the same id — a card that publishes itself as a container
/// also has text candidates inside it — and the container is the card's own box.
fn find_card(candidates: &[Candidate], card_id: &str) -> Option<Rectangle> {
    candidates
        .iter()
        .filter(|candidate| candidate.kind == KIND_CONTAINER)
        .find(|candidate| {
            candidate
                .id
                .as_ref()
                .and_then(id_name)
                .is_some_and(|name| name == card_id)
        })
        .map(|candidate| candidate.bounds)
}

/// Accept a measured card or say why it cannot be trusted.
///
/// The order of the checks is load-bearing: finiteness first, because every
/// comparison against a non-finite value is false and a `NaN` height would
/// otherwise pass the size checks on its way to a nonsense raster.
///
/// The height test is `>=`, not `>`. iced lays a `UserInterface` out inside
/// limits derived from the viewport, so a card taller than [`PROBE_HEIGHT`] is
/// clamped *to* it and comes back looking exactly like a card that fits. A `>`
/// test would accept the clipped box and emit a short PNG while reporting
/// success — the precise failure this check exists to catch.
///
/// # Errors
///
/// [`CaptureError::CardNotFinite`], [`CaptureError::CardHasNoArea`] or
/// [`CaptureError::CardTooTall`].
fn validate(card_id: &str, card: Rectangle) -> Result<Rectangle, CaptureError> {
    let not_finite = || CaptureError::CardNotFinite {
        card_id: card_id.to_owned(),
        width: card.width,
        height: card.height,
    };
    let no_area = || CaptureError::CardHasNoArea {
        card_id: card_id.to_owned(),
        width: card.width,
        height: card.height,
    };
    let too_tall = || CaptureError::CardTooTall {
        card_id: card_id.to_owned(),
        height: card.height,
        probe_height: PROBE_HEIGHT,
    };

    if !is_finite(&card) {
        return Err(not_finite());
    }
    if let Some(problem) = box_problem(card.width, card.height) {
        return Err(match problem {
            BoxProblem::NotFinite => not_finite(),
            BoxProblem::NoArea => no_area(),
        });
    }
    if card.height >= PROBE_HEIGHT {
        return Err(too_tall());
    }

    Ok(card)
}

/// Accept a caller-declared canvas as a viewport, or say why it cannot be one.
///
/// The same two dimension checks a measured box gets, and deliberately **not**
/// the [`PROBE_HEIGHT`] ceiling. That ceiling exists because a *measurement* at
/// the probe's height is ambiguous — it could be a real height or a clipped one
/// — and a declared height is not a measurement of anything, so there is nothing
/// to disambiguate. A declared canvas taller than the probe is simply a tall
/// image.
///
/// # Errors
///
/// [`CaptureError::DeclaredSizeNotFinite`] or
/// [`CaptureError::DeclaredSizeHasNoArea`].
fn validate_declared(section_id: &str, width: f32, height: f32) -> Result<Size, CaptureError> {
    if let Some(problem) = box_problem(width, height) {
        let section_id = section_id.to_owned();
        return Err(match problem {
            BoxProblem::NotFinite => CaptureError::DeclaredSizeNotFinite {
                section_id,
                width,
                height,
            },
            BoxProblem::NoArea => CaptureError::DeclaredSizeHasNoArea {
                section_id,
                width,
                height,
            },
        });
    }

    Ok(Size::new(width, height))
}

#[cfg(test)]
#[path = "capture_tests.rs"]
mod tests;
