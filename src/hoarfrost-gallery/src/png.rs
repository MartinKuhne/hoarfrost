//! Turn an RGBA8 framebuffer into a PNG file on disk.
//!
//! The encoder is the `png` crate rather than a hand-written one (`RUST-022`):
//! it is already in the workspace lock graph, so adding it costs no new
//! dependency weight, and a correct PNG is zlib, CRCs and filter selection —
//! none of which this program needs to get right itself.
//!
//! The one thing this module adds is a *refusal before a write*. A PNG whose
//! declared size and pixel count disagree is not a warning-level problem:
//! encoders accept it and produce a file that image viewers reject or, worse,
//! display as garbage. A documentation artifact that is quietly wrong is worse
//! than a run that stops and says which section failed, so the buffer length and
//! the dimensions are checked here, before a single byte is written.
//!
//! Unit tests live in the sibling `png_tests.rs` sidecar.

use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use crate::strings;

/// Bytes per pixel in the RGBA8 buffers the capture driver produces.
pub const BYTES_PER_PIXEL: u64 = 4;

/// The exact byte count an RGBA8 image of `width` × `height` must carry.
///
/// Computed in `u64` so a large frame cannot wrap a `u32` product into a
/// plausible-looking smaller number, and saturating rather than wrapping even
/// past `u64`: an absurd size then fails the length check with a clear message
/// instead of panicking in a debug build or silently agreeing with a corrupt
/// buffer in a release one.
#[must_use]
pub fn expected_bytes(width: u32, height: u32) -> u64 {
    u64::from(width)
        .saturating_mul(u64::from(height))
        .saturating_mul(BYTES_PER_PIXEL)
}

/// Reject a buffer that cannot describe an image, before anything is written.
///
/// # Errors
///
/// Returns a message naming the mismatch when either dimension is zero or the
/// buffer length is not `width * height * 4`.
fn check_shape(width: u32, height: u32, rgba: &[u8]) -> Result<(), Box<dyn Error>> {
    if width == 0 || height == 0 {
        return Err(strings::ERR_PNG_ZERO_SIZE.into());
    }

    let expected = expected_bytes(width, height);
    if u64::try_from(rgba.len()) != Ok(expected) {
        return Err(format!(
            "{}: expected {expected} bytes, got {}",
            strings::ERR_PNG_BUFFER,
            rgba.len()
        )
        .into());
    }

    Ok(())
}

/// Encode `rgba` as an 8-bit RGBA PNG at `path`, replacing any existing file.
///
/// # Errors
///
/// Returns a message when the shape is not a valid image (see
/// [`expected_bytes`]), when the file cannot be created, or when the encoder fails.
/// A failed call leaves no half-written image behind that it did not already
/// own: the shape is validated before the file is opened.
pub fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) -> Result<(), Box<dyn Error>> {
    check_shape(width, height, rgba)?;

    let file = BufWriter::new(File::create(path)?);
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder.write_header()?;
    writer.write_image_data(rgba)?;
    writer.finish()?;

    Ok(())
}

#[cfg(test)]
#[path = "png_tests.rs"]
mod tests;
