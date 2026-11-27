//! A decoded frame of WaOS's as a picture Ruffle draws: its planes copied into the program's
//! memory and laid out as Ruffle's renderer takes them, three planes of 8-bit 4:2:0, the
//! brightness and then each colour difference by itself.
//!
//! A frame is its decoder's own picture, and WaOS says how it is laid out. The layout handed on
//! here is the one the systems' H.264 decoders give: a plane of brightness and one of both
//! colour differences side by side, at half the size each way. Only the part of the frame that
//! is picture is taken, since a decoder's buffer may be larger.
//!
//! Ruffle's renderer reads the samples as BT.601 in video's range, whatever the frame says of
//! its colour, as it does with every decoder's.

use ruffle_render::bitmap::{Bitmap, BitmapFormat};
use wasdk::video_frames::{PlaneContent, PlaneFormat, PlanePixels, VideoFrame, VideoPlane};

/// `frame` as a picture Ruffle draws; why not, for a frame laid out another way, or one WaOS
/// cannot copy out.
pub(super) async fn of_frame(frame: &VideoFrame) -> Result<Bitmap<'static>, String> {
    let planes = frame.planes().ok_or("the frame has no planes to read")?;
    let [luma, chroma] = planes.as_slice() else {
        return Err(format!("a frame of {} planes", planes.len()));
    };
    if !is_brightness(luma) || !is_both_colour_differences(chroma) {
        return Err(format!(
            "a frame of {:?} in {:?} and {:?} in {:?}",
            luma.content, luma.format, chroma.content, chroma.format
        ));
    }
    let copied = frame
        .read_planes()
        .await
        .map_err(|error| format!("its planes were not copied out: {error:?}"))?;
    let [luma, chroma] = copied.as_slice() else {
        return Err(format!("{} planes were copied out of two", copied.len()));
    };
    let visible = frame.visible_rect();
    let picture = Visible {
        x: visible.x as usize,
        y: visible.y as usize,
        width: visible.width as usize,
        height: visible.height as usize,
    };
    let samples = picture
        .in_three_planes(luma, chroma)
        .ok_or("the frame's planes are smaller than its picture")?;
    Ok(Bitmap::new(
        visible.width,
        visible.height,
        BitmapFormat::Yuv420p,
        samples,
    ))
}

fn is_brightness(plane: &VideoPlane) -> bool {
    plane.content == PlaneContent::Luma && plane.format == PlaneFormat::R8
}

fn is_both_colour_differences(plane: &VideoPlane) -> bool {
    plane.content == PlaneContent::Chroma && plane.format == PlaneFormat::Rg8
}

/// The part of a frame's buffer that is picture, in the brightness plane's samples.
struct Visible {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
}

impl Visible {
    /// The picture's samples as three planes one after another: brightness, then the blue
    /// difference, then the red, each row with no padding. None where a plane ends before the
    /// picture does.
    fn in_three_planes(&self, luma: &PlanePixels, chroma: &PlanePixels) -> Option<Vec<u8>> {
        let chroma_width = self.width.div_ceil(2);
        let chroma_height = self.height.div_ceil(2);
        let mut samples =
            Vec::with_capacity(self.width * self.height + 2 * chroma_width * chroma_height);
        for row in rows(luma, self.y, self.height) {
            samples.extend_from_slice(row?.get(self.x..self.x + self.width)?);
        }
        // Each pair of the second plane is a blue difference and then a red one.
        for difference in 0..2 {
            for row in rows(chroma, self.y / 2, chroma_height) {
                let first = self.x / 2 * 2;
                let pairs = row?.get(first..first + chroma_width * 2)?;
                samples.extend(pairs.iter().skip(difference).step_by(2));
            }
        }
        Some(samples)
    }
}

/// `count` rows of `plane` from the row `first`, each none where the plane ends before it.
fn rows(plane: &PlanePixels, first: usize, count: usize) -> impl Iterator<Item = Option<&[u8]>> {
    let row_bytes = plane.row_bytes as usize;
    (first..first + count).map(move |row| plane.bytes.get(row * row_bytes..(row + 1) * row_bytes))
}
