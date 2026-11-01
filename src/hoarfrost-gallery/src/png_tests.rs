//! Tests for the PNG writer.
//!
//! Every file is written inside a `tempfile::TempDir` and nothing else (**RUST-006**).
//! The round-trip case decodes with the same crate's decoder, which proves the
//! bytes are a real PNG rather than something that merely starts like one — the
//! failure this module exists to prevent is a file that looks fine on disk and is
//! garbage to every viewer that opens it.

use std::io::BufReader;
use std::path::Path;

use super::*;

/// The fixed eight bytes every PNG file starts with.
const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// A 2×2 RGBA image with four distinct pixels, so a channel swap or a row-order
/// mistake shows up instead of cancelling out.
fn checker() -> Vec<u8> {
    vec![
        255, 0, 0, 255, // top-left: red
        0, 255, 0, 128, // top-right: half-green
        0, 0, 255, 0, // bottom-left: transparent blue
        16, 32, 64, 255, // bottom-right: dark opaque
    ]
}

/// Decode a PNG file back into (width, height, pixels).
fn decode(path: &Path) -> (u32, u32, Vec<u8>) {
    let file = std::fs::File::open(path).expect("the file exists");
    let mut reader = png::Decoder::new(BufReader::new(file))
        .read_info()
        .expect("a decodable PNG");

    let info = reader.info();
    let (width, height) = (info.width, info.height);
    assert_eq!(info.color_type, png::ColorType::Rgba);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);

    let mut pixels = vec![0u8; expected_bytes(width, height) as usize];
    let frame = reader.next_frame(&mut pixels).expect("the frame decodes");
    assert_eq!(frame.width, width);
    assert_eq!(frame.height, height);

    (width, height, pixels)
}

#[test]
fn a_known_image_round_trips_through_a_temp_directory() {
    let dir = tempfile::TempDir::new().expect("a temp directory");
    let path = dir.path().join("checker.png");
    let rgba = checker();

    write_png(&path, 2, 2, &rgba).expect("the write succeeds");

    let (width, height, pixels) = decode(&path);
    assert_eq!(width, 2);
    assert_eq!(height, 2);
    assert_eq!(pixels, rgba, "every pixel must survive the round trip");
}

#[test]
fn the_file_starts_with_the_png_magic() {
    let dir = tempfile::TempDir::new().expect("a temp directory");
    let path = dir.path().join("magic.png");

    write_png(&path, 2, 2, &checker()).expect("the write succeeds");

    let bytes = std::fs::read(&path).expect("the file was created");
    assert_eq!(&bytes[..PNG_MAGIC.len()], &PNG_MAGIC[..]);
}

#[test]
fn a_non_square_image_round_trips() {
    // A row-stride mistake is invisible in a square image.
    let dir = tempfile::TempDir::new().expect("a temp directory");
    let path = dir.path().join("strip.png");
    let rgba: Vec<u8> = (0..3 * 4).map(|index| (index * 7) as u8).collect();

    write_png(&path, 3, 1, &rgba).expect("the write succeeds");

    let (width, height, pixels) = decode(&path);
    assert_eq!((width, height), (3, 1));
    assert_eq!(pixels, rgba);
}

#[test]
fn an_existing_file_is_replaced_not_appended_to() {
    let dir = tempfile::TempDir::new().expect("a temp directory");
    let path = dir.path().join("twice.png");
    std::fs::write(&path, b"not a png at all").expect("the seed write");

    write_png(&path, 2, 2, &checker()).expect("the write succeeds");

    let (_, _, pixels) = decode(&path);
    assert_eq!(pixels, checker());
}

#[test]
fn a_shorter_buffer_than_the_size_is_refused() {
    let dir = tempfile::TempDir::new().expect("a temp directory");
    let path = dir.path().join("short.png");

    let error = write_png(&path, 2, 2, &[0u8; 7]).expect_err("7 bytes is not 2x2 RGBA");
    let message = error.to_string();
    assert!(
        message.contains(crate::strings::ERR_PNG_BUFFER),
        "the message must say what is wrong: {message}"
    );
    assert!(
        message.contains("16"),
        "the message must give the size: {message}"
    );
    assert!(
        !path.exists(),
        "a refused write must not leave a file behind"
    );
}

#[test]
fn a_longer_buffer_than_the_size_is_refused() {
    let dir = tempfile::TempDir::new().expect("a temp directory");
    let path = dir.path().join("long.png");

    let error = write_png(&path, 2, 2, &[0u8; 17]).expect_err("17 bytes is not 2x2 RGBA");
    assert!(error.to_string().contains(crate::strings::ERR_PNG_BUFFER));
}

#[test]
fn a_zero_width_is_refused() {
    let dir = tempfile::TempDir::new().expect("a temp directory");
    let path = dir.path().join("zero.png");

    let error = write_png(&path, 0, 4, &[]).expect_err("a zero-width image is not an image");
    assert!(
        error
            .to_string()
            .contains(crate::strings::ERR_PNG_ZERO_SIZE),
        "{error}"
    );
    assert!(!path.exists());
}

#[test]
fn a_zero_height_is_refused() {
    let dir = tempfile::TempDir::new().expect("a temp directory");
    let path = dir.path().join("zero.png");

    let error = write_png(&path, 4, 0, &[]).expect_err("a zero-height image is not an image");
    assert!(
        error
            .to_string()
            .contains(crate::strings::ERR_PNG_ZERO_SIZE),
        "{error}"
    );
}

#[test]
fn the_zero_size_check_runs_before_the_buffer_check() {
    // A zero-sized image with a mismatched buffer reports the *size*, because
    // that is the problem the caller can act on.
    let dir = tempfile::TempDir::new().expect("a temp directory");
    let error = write_png(&dir.path().join("x.png"), 0, 0, &[1, 2, 3]).expect_err("refused");
    assert!(
        error
            .to_string()
            .contains(crate::strings::ERR_PNG_ZERO_SIZE)
    );
}

#[test]
fn writing_into_a_missing_directory_fails_rather_than_creating_one() {
    let dir = tempfile::TempDir::new().expect("a temp directory");
    let path = dir.path().join("absent").join("out.png");

    let error = write_png(&path, 2, 2, &checker()).expect_err("the parent does not exist");
    assert!(
        !path.exists(),
        "no file may appear where the directory is missing"
    );
    assert!(!error.to_string().is_empty(), "the error must say why");
}

#[test]
fn expected_bytes_is_the_rgba_product() {
    assert_eq!(expected_bytes(1, 1), 4);
    assert_eq!(expected_bytes(2, 2), 16);
    assert_eq!(expected_bytes(700, 300), 700 * 300 * 4);
    assert_eq!(expected_bytes(0, 100), 0);
}

#[test]
fn expected_bytes_does_not_wrap_on_a_large_frame() {
    // A `u32` product would wrap here and hand back a plausible-looking small
    // number, which is exactly how a corrupt image would pass a naive check.
    assert!(expected_bytes(100_000, 100_000) > u64::from(u32::MAX));
    assert_eq!(expected_bytes(100_000, 100_000), 100_000 * 100_000 * 4);
}

#[test]
fn an_absurd_size_saturates_instead_of_panicking_or_wrapping() {
    // No real frame is this big, but the arithmetic must not be the thing that
    // fails: it saturates, so the length check below reports a mismatch.
    assert_eq!(expected_bytes(u32::MAX, u32::MAX), u64::MAX);

    let dir = tempfile::TempDir::new().expect("a temp directory");
    let error = write_png(&dir.path().join("huge.png"), u32::MAX, u32::MAX, &[])
        .expect_err("no buffer can match that size");
    assert!(
        error.to_string().contains(crate::strings::ERR_PNG_BUFFER),
        "{error}"
    );
}

#[test]
fn a_one_pixel_image_is_accepted() {
    let dir = tempfile::TempDir::new().expect("a temp directory");
    let path = dir.path().join("one.png");

    write_png(&path, 1, 1, &[10, 20, 30, 40]).expect("the write succeeds");

    let (width, height, pixels) = decode(&path);
    assert_eq!((width, height), (1, 1));
    assert_eq!(pixels, vec![10, 20, 30, 40]);
}
