//! Tests for the capture driver, measured and declared.
//!
//! Most of the cards here are synthetic — a `container` with a known id and a
//! fixed size — so the *machinery* is proven without depending on a particular
//! section: the driver is generic over the element producer precisely so it can
//! be proven here. The two exceptions are deliberate and are the only tests that
//! reach into the demo: the declared-height guard, which has to be run against
//! the real section the declaration was written for, and the theme test, which
//! has to use the real theme.
//!
//! # Cost
//!
//! Building iced's headless renderer takes roughly 620 ms, and `cargo nextest`
//! runs every test in its own process, so a renderer cannot be shared *between*
//! tests. Each test that needs one therefore builds exactly one and runs every
//! raster assertion through it; everything else here is pure arithmetic and runs
//! for free.
//!
//! No test writes to the filesystem (**RUST-006**).

use hoarfrost::test_helpers::KIND_CONTAINER;
use hoarfrost::test_helpers::KIND_TEXT;
use iced::Element;
use iced::Length;
use iced::Point;
use iced::Rectangle;
use iced::Size;
use iced::widget::Id;
use iced::widget::container;
use iced::widget::text;

use crate::catalog;
use crate::catalog::Catalog;
use crate::png::expected_bytes;

use super::*;

/// The id the synthetic card publishes.
const CARD_ID: &str = "gallery.test.card";

/// A pure builder for a fixed-size card carrying [`CARD_ID`].
///
/// Returning a closure rather than an `Element` is the point: the driver calls it
/// once per pass, which is only legal because it is pure.
fn card(width: f32, height: f32) -> impl Fn() -> Element<'static, ()> {
    move || {
        container(text("card"))
            .id(Id::from(CARD_ID.to_owned()))
            .width(Length::Fixed(width))
            .height(Length::Fixed(height))
            .into()
    }
}

/// The same card with fill width and natural height, which is how a real demo
/// section is built.
fn filling_card() -> impl Fn() -> Element<'static, ()> {
    move || {
        container(text("card"))
            .id(Id::from(CARD_ID.to_owned()))
            .width(Length::Fill)
            .into()
    }
}

/// A fill-width card that asks for more height than the probe can give it.
fn too_tall_card() -> impl Fn() -> Element<'static, ()> {
    move || {
        container(text("tall"))
            .id(Id::from(CARD_ID.to_owned()))
            .width(Length::Fill)
            .height(Length::Fixed(PROBE_HEIGHT + 400.0))
            .into()
    }
}

#[test]
fn a_card_is_measured_and_rasterised_at_its_own_size() {
    let mut renderer = Renderer::new();

    // Pass one: the card's own box comes back out of the tree.
    let bounds = renderer
        .measure(CARD_ID, 300.0, card(240.0, 80.0))
        .expect("the card is addressable");
    assert!(
        (bounds.width - 240.0).abs() < 0.01,
        "the card measured {bounds:?}, expected 240 wide"
    );
    assert!(
        (bounds.height - 80.0).abs() < 0.01,
        "the card measured {bounds:?}, expected 80 tall"
    );

    // Pass two: the PNG is the card, at the run's scale, with no crop.
    let shot = renderer
        .shot(CARD_ID, 300.0, 2.0, DEFAULT_BACKGROUND, card(240.0, 80.0))
        .expect("the card rasterises");
    assert_eq!(shot.width, 480, "240 logical at scale 2 is 480 physical");
    assert_eq!(shot.height, 160, "80 logical at scale 2 is 160 physical");
    assert_eq!(shot.rgba.len(), 480 * 160 * 4);
    assert!(shot.is_well_formed());

    // The premise of the whole design: a fill-width card takes the width it is
    // given, so measuring at the run's width is what fixes the image's width.
    let filled = renderer
        .measure(CARD_ID, 640.0, filling_card())
        .expect("the fill card is addressable");
    assert!(
        (filled.width - 640.0).abs() < 0.01,
        "a fill-width card should take the probe width, got {filled:?}"
    );
    assert!(
        filled.height > 0.0,
        "a natural-height card should measure some height, got {filled:?}"
    );

    // A scale of 1 is the same card at one physical pixel per logical one.
    let flat = renderer
        .shot(CARD_ID, 300.0, 1.0, DEFAULT_BACKGROUND, card(240.0, 80.0))
        .expect("scale 1 rasterises");
    assert_eq!((flat.width, flat.height), (240, 80));
    assert!(flat.is_well_formed());
}

#[test]
fn an_unmeasurable_card_is_reported_instead_of_rasterised() {
    let mut renderer = Renderer::new();

    // A card the tree never published: no image, and a reason.
    let missing = renderer
        .shot(
            "gallery.test.absent",
            300.0,
            2.0,
            DEFAULT_BACKGROUND,
            card(100.0, 40.0),
        )
        .expect_err("the id is not in the tree");
    assert_eq!(
        missing,
        CaptureError::CardNotFound {
            card_id: "gallery.test.absent".to_owned(),
        }
    );

    // A card that lays out at nothing: also refused, and refused *before* a
    // raster, so no zero-sized PNG is ever produced.
    let empty = renderer
        .measure(CARD_ID, 300.0, card(0.0, 0.0))
        .expect_err("a zero-sized card is not an image");
    assert_eq!(
        empty,
        CaptureError::CardHasNoArea {
            card_id: CARD_ID.to_owned(),
            width: 0.0,
            height: 0.0,
        }
    );

    // The same refusal has to come from `shot`, not just from `measure`.
    let empty_shot = renderer
        .shot(CARD_ID, 300.0, 2.0, DEFAULT_BACKGROUND, card(0.0, 0.0))
        .expect_err("a zero-sized card is not an image");
    assert!(matches!(empty_shot, CaptureError::CardHasNoArea { .. }));
}

#[test]
fn id_name_reads_a_stable_name_and_rejects_a_generated_one() {
    assert_eq!(
        id_name(&Id::from("demo.section.buttons".to_owned())).as_deref(),
        Some("demo.section.buttons")
    );
    assert_eq!(id_name(&Id::new("plain")), Some("plain".to_owned()));

    // `Id::unique()` is a process-global counter: matching on it would make the
    // manifest change between runs.
    assert_eq!(id_name(&Id::unique()), None);
}

#[test]
fn find_card_only_accepts_the_container_door() {
    use hoarfrost::test_helpers::Candidate;

    let named = |kind: &'static str| Candidate {
        kind,
        id: Some(Id::from(CARD_ID.to_owned())),
        bounds: rect(0.0, 0.0, 10.0, 10.0),
    };

    assert_eq!(
        find_card(&[named(KIND_CONTAINER)], CARD_ID),
        Some(rect(0.0, 0.0, 10.0, 10.0))
    );
    assert_eq!(find_card(&[named(KIND_TEXT)], CARD_ID), None);
    assert_eq!(find_card(&[], CARD_ID), None);
}

/// A rectangle from the four numbers a test thinks in, since iced's own
/// constructor takes a point and a size.
fn rect(x: f32, y: f32, width: f32, height: f32) -> Rectangle {
    Rectangle::new(Point::new(x, y), Size::new(width, height))
}

#[test]
fn validate_accepts_a_card_that_fits_strictly_inside_the_probe() {
    let bounds = rect(0.0, 0.0, 100.0, 40.0);
    assert_eq!(validate("c", bounds), Ok(bounds));

    // One logical pixel below the ceiling is still a measurement.
    let just_under = rect(0.0, 0.0, 100.0, PROBE_HEIGHT - 1.0);
    assert_eq!(validate("c", just_under), Ok(just_under));
}

#[test]
fn validate_refuses_a_card_that_measures_exactly_the_probe_ceiling() {
    // The bug this pins. iced clamps layout to the viewport, so a card that
    // wants 3000px and a card that is genuinely 1600px both report 1600px. A
    // `>` test accepted the clipped one and wrote a short PNG while reporting
    // success, so the ceiling itself has to be refused.
    let flush = rect(0.0, 0.0, 100.0, PROBE_HEIGHT);
    assert_eq!(
        validate("c", flush),
        Err(CaptureError::CardTooTall {
            card_id: "c".to_owned(),
            height: PROBE_HEIGHT,
            probe_height: PROBE_HEIGHT,
        })
    );
}

#[test]
fn a_card_taller_than_the_probe_is_refused_rather_than_clipped_into_a_png() {
    // The same refusal, proven through the real two-pass path rather than
    // through `validate` alone: a card that asks for more than the probe gets
    // clamped to it by iced, and `shot` must report that instead of rasterising
    // the truncated box.
    let mut renderer = Renderer::new();
    let tall = too_tall_card();

    let measured = renderer
        .measure(CARD_ID, 300.0, &tall)
        .expect_err("a card at the ceiling is not a measurement");
    assert!(
        matches!(measured, CaptureError::CardTooTall { .. }),
        "expected CardTooTall, got {measured:?}"
    );

    let shot = renderer
        .shot(CARD_ID, 300.0, 1.0, DEFAULT_BACKGROUND, &tall)
        .expect_err("no PNG for a clipped card");
    assert!(matches!(shot, CaptureError::CardTooTall { .. }));
}

#[test]
fn a_renderer_takes_the_documented_theme_or_one_you_hand_it() {
    // `new` is the documented default; `with_theme` is how the gallery draws
    // with the demo's own palette instead of iced's stock dark one.
    assert_eq!(Renderer::new().theme(), &THEME);

    let custom = hoarfrost_demo::theme::demo_theme();
    let renderer = Renderer::with_theme(custom.clone());
    assert_eq!(renderer.theme(), &custom);
    assert_ne!(renderer.theme(), &THEME);
}

#[test]
fn validate_rejects_a_non_finite_box_before_the_size_checks() {
    // Every comparison against a non-finite value is false, so a `NaN` height
    // would pass "is it zero" and "is it too tall" and reach the raster.
    assert!(matches!(
        validate("c", rect(f32::NAN, 0.0, f32::NAN, 10.0)),
        Err(CaptureError::CardNotFinite { .. })
    ));

    assert!(matches!(
        validate("c", rect(0.0, 0.0, f32::INFINITY, 10.0)),
        Err(CaptureError::CardNotFinite { .. })
    ));
}

#[test]
fn validate_rejects_a_card_with_no_area() {
    assert!(matches!(
        validate("c", rect(0.0, 0.0, 0.0, 40.0)),
        Err(CaptureError::CardHasNoArea { .. })
    ));
    assert!(matches!(
        validate("c", rect(0.0, 0.0, 40.0, 0.0)),
        Err(CaptureError::CardHasNoArea { .. })
    ));
    assert!(matches!(
        validate("c", rect(0.0, 0.0, -40.0, 10.0)),
        Err(CaptureError::CardHasNoArea { .. })
    ));
}

#[test]
fn validate_rejects_a_card_taller_than_the_probe() {
    let error =
        validate("c", rect(0.0, 0.0, 100.0, PROBE_HEIGHT + 1.0)).expect_err("the probe clipped it");
    assert_eq!(
        error,
        CaptureError::CardTooTall {
            card_id: "c".to_owned(),
            height: PROBE_HEIGHT + 1.0,
            probe_height: PROBE_HEIGHT,
        }
    );
}

#[test]
fn box_problem_reports_the_first_reason_a_box_cannot_be_rasterised() {
    // The two checks every box passes, measured or declared, and the order they
    // run in. A `NaN` dimension outranks "no area" precisely because a `NaN`
    // would sail through the area comparison: `NaN > 0.0` is false, so a
    // finiteness test that ran second would never be reached.
    assert_eq!(box_problem(1.0, 1.0), None);
    assert_eq!(box_problem(0.1, 0.1), None);

    assert_eq!(box_problem(f32::INFINITY, 1.0), Some(BoxProblem::NotFinite));
    assert_eq!(box_problem(1.0, f32::NAN), Some(BoxProblem::NotFinite));
    assert_eq!(box_problem(f32::NAN, 0.0), Some(BoxProblem::NotFinite));

    assert_eq!(box_problem(0.0, 10.0), Some(BoxProblem::NoArea));
    assert_eq!(box_problem(10.0, 0.0), Some(BoxProblem::NoArea));
    assert_eq!(box_problem(-5.0, 10.0), Some(BoxProblem::NoArea));
    assert_eq!(box_problem(10.0, -1.0), Some(BoxProblem::NoArea));
}

#[test]
fn shot_at_rasterises_exactly_the_declared_box_and_needs_no_id() {
    // The declared path has no measure pass, so it cannot fail to find a card:
    // the element here publishes no id at all, and the image still comes out.
    let mut renderer = Renderer::new();
    let anonymous = || -> Element<'static, ()> { container(text("no id anywhere")).into() };

    let shot = renderer
        .shot_at(
            "nothing.publishes.this.id",
            120.0,
            60.0,
            1.0,
            DEFAULT_BACKGROUND,
            anonymous,
        )
        .expect("a declared box needs no measurement to draw");

    assert_eq!(shot.width, 120);
    assert_eq!(shot.height, 60);
    assert_eq!(
        shot.rgba.len(),
        usize::try_from(expected_bytes(120, 60)).expect("a 120x60 buffer fits a usize")
    );
    assert!(shot.is_well_formed());

    // The scale multiplies the declared logical box, exactly as the measured
    // path does: 120 x 60 logical at 2x is 240 x 120 physical.
    let scaled = renderer
        .shot_at(
            "nothing.publishes.this.id",
            120.0,
            60.0,
            2.0,
            DEFAULT_BACKGROUND,
            anonymous,
        )
        .expect("the same box at a different scale");
    assert_eq!((scaled.width, scaled.height), (240, 120));
    assert_eq!(
        scaled.rgba.len(),
        usize::try_from(expected_bytes(240, 120)).expect("a 240x120 buffer fits a usize")
    );
}

#[test]
fn shot_at_refuses_a_declared_box_that_cannot_become_an_image() {
    // Zero, negative and non-finite dimensions are refused *before* anything is
    // rasterised, and with errors that say "declared" rather than "measured" —
    // the reader has the catalog table to fix, not a widget.
    let mut renderer = Renderer::new();
    let card = filling_card();

    let refused: Vec<CaptureError> = [
        renderer
            .shot_at("c", 100.0, 0.0, 1.0, DEFAULT_BACKGROUND, &card)
            .expect_err("zero height is not an image"),
        renderer
            .shot_at("c", 0.0, 100.0, 1.0, DEFAULT_BACKGROUND, &card)
            .expect_err("zero width is not an image"),
        renderer
            .shot_at("c", 100.0, -1.0, 1.0, DEFAULT_BACKGROUND, &card)
            .expect_err("a negative height is not an image"),
        renderer
            .shot_at("c", -100.0, 100.0, 1.0, DEFAULT_BACKGROUND, &card)
            .expect_err("a negative width is not an image"),
        renderer
            .shot_at("c", 100.0, f32::NAN, 1.0, DEFAULT_BACKGROUND, &card)
            .expect_err("a NaN height is not an image"),
        renderer
            .shot_at("c", f32::INFINITY, 100.0, 1.0, DEFAULT_BACKGROUND, &card)
            .expect_err("an infinite width is not an image"),
    ]
    .into_iter()
    .collect();

    let no_area = refused
        .iter()
        .filter(|error| matches!(error, CaptureError::DeclaredSizeHasNoArea { .. }))
        .count();
    let not_finite = refused
        .iter()
        .filter(|error| matches!(error, CaptureError::DeclaredSizeNotFinite { .. }))
        .count();
    assert_eq!(
        no_area, 4,
        "zero and negative in either dimension: {refused:?}"
    );
    assert_eq!(
        not_finite, 2,
        "NaN and infinity in either dimension: {refused:?}"
    );

    // The section is named, because a manifest line that cannot say which
    // section it is about is not actionable.
    for error in &refused {
        assert!(
            error.to_string().contains("\"c\""),
            "{error} must name the section"
        );
    }
}

#[test]
fn a_declared_height_leaves_no_part_of_the_section_squeezed() {
    // The guard that keeps `catalog::DECLARED_HEIGHTS` honest. A `Fill`-height
    // widget reports *any* height it is handed, so the table cannot be checked by
    // asking the widget what it wants. Nor can it be checked by looking for
    // overflow: laid out at 524 — a height that sums correctly from the parts
    // and clips nothing — every node still sits inside its container, while the
    // description line is crushed to 4 px and the action button to 0 px. iced
    // squeezes content to fit rather than spilling out of it, so a spill test
    // passes on a section that is silently unreadable.
    //
    // The failure mode is therefore *shortening*, and the test measures it
    // directly: a text line laid out with plenty of room is as tall as it will
    // ever be, and any line shorter than that at the declared height has been
    // crushed. The floor is read from the layout rather than hard-coded, so the
    // test follows the theme and typography instead of freezing them.
    let catalog = Catalog::new();
    let mut renderer = Renderer::new();

    // A viewport far taller than any section, so nothing in it is squeezed and
    // every text line reports its own natural height.
    const GENEROUS: f32 = 1200.0;
    // Laid out at the width the sweep actually renders at, so the geometry this
    // test checks is the geometry the PNG is made from.
    let width = crate::cli::Options::default().width;

    for (id, declared) in catalog::DECLARED_HEIGHTS {
        let target = catalog
            .targets()
            .iter()
            .find(|candidate| candidate.id() == id)
            .unwrap_or_else(|| panic!("{id} is in the declared table but not in the catalog"));

        // Built fresh at every `record` call rather than stored in a variable:
        // the driver takes the producer by value, and a stored closure captured by
        // this one could not be moved into two calls.
        const REBUILT: &str = "the table entry rebuilds";

        let mut crushed = |height: f32| -> (f32, Vec<String>) {
            let roomy = renderer.record(Size::new(width, GENEROUS), || {
                catalog.build(target).expect(REBUILT)
            });
            let floor = roomy
                .iter()
                .filter(|candidate| candidate.kind == KIND_TEXT)
                .map(|candidate| candidate.bounds.height)
                .fold(f32::MAX, f32::min);
            assert!(
                floor > 1.0 && floor < f32::MAX,
                "{id} produced no usable text floor at {GENEROUS}: {floor}"
            );

            let squeezed: Vec<String> = renderer
                .record(Size::new(width, height), || {
                    catalog.build(target).expect(REBUILT)
                })
                .iter()
                .filter(|candidate| candidate.kind == KIND_TEXT)
                .filter(|candidate| candidate.bounds.height < floor - 0.5)
                .map(|candidate| {
                    format!(
                        "{id}: a text line is {:.1} px tall at height {height}, below \
                         the {:.1} px it measures with room",
                        candidate.bounds.height, floor
                    )
                })
                .collect();

            (floor, squeezed)
        };

        let (floor, violations) = crushed(declared);
        assert!(
            violations.is_empty(),
            "{id} is squeezed by its declared height {declared} (text floor {floor}):\n{}",
            violations.join("\n")
        );

        // And the check has teeth: 24 logical px short of the declared height
        // crushes a line, so an entry that drifts too low cannot pass silently.
        let too_short = declared - 24.0;
        let (_, missed) = crushed(too_short);
        assert!(
            !missed.is_empty(),
            "{id} laid out at {too_short} produced no crushed text line, so this test \
             would not notice a declared height that had become too small"
        );
    }
}

#[test]
fn physical_size_multiplies_and_rounds() {
    assert_eq!(
        physical_size(Size::new(100.0, 50.0), 2.0),
        Size::new(200u32, 100u32)
    );
    assert_eq!(
        physical_size(Size::new(100.4, 50.6), 1.0),
        Size::new(100u32, 51u32)
    );
    assert_eq!(
        physical_size(Size::new(10.0, 10.0), 0.5),
        Size::new(5u32, 5u32)
    );
}

#[test]
fn a_shot_is_well_formed_only_at_exactly_four_bytes_per_pixel() {
    assert!(
        Shot {
            width: 2,
            height: 2,
            rgba: vec![0; 16],
        }
        .is_well_formed()
    );
    assert!(
        !(Shot {
            width: 2,
            height: 2,
            rgba: vec![0; 15],
        })
        .is_well_formed()
    );
    assert!(
        !(Shot {
            width: 2,
            height: 2,
            rgba: vec![0; 17],
        })
        .is_well_formed()
    );
}

#[test]
fn every_capture_error_names_the_card_and_says_why() {
    let errors = [
        CaptureError::CardNotFound {
            card_id: CARD_ID.to_owned(),
        },
        CaptureError::CardHasNoArea {
            card_id: CARD_ID.to_owned(),
            width: 0.0,
            height: 12.0,
        },
        CaptureError::CardNotFinite {
            card_id: CARD_ID.to_owned(),
            width: f32::NAN,
            height: 12.0,
        },
        CaptureError::CardTooTall {
            card_id: CARD_ID.to_owned(),
            height: 2000.0,
            probe_height: PROBE_HEIGHT,
        },
        CaptureError::PixelCountMismatch {
            card_id: CARD_ID.to_owned(),
            width: 10,
            height: 10,
            actual: 7,
        },
    ];

    for error in &errors {
        let text = error.to_string();
        assert!(
            text.contains(CARD_ID),
            "the message must name the card: {text}"
        );
        assert!(
            text.len() > CARD_ID.len() + 4,
            "the message must say why: {text}"
        );
    }

    // It is a real error type, so a caller can box it like one.
    let boxed: Box<dyn std::error::Error> = Box::new(errors[0].clone());
    assert!(boxed.to_string().contains(CARD_ID));
}
