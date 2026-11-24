//! One H.264 stream of a movie, decoded by WaOS's video decoder.
//!
//! The player hands a stream's chunks over and wants a picture back at once, and WaOS's decoder
//! answers later. So the stream keeps two tasks of the program's executor: one hands the
//! decoder each chunk as it has room, the other reads each frame it puts out and keeps the
//! newest as a picture Ruffle draws (`picture`). A chunk handed over is answered with the
//! newest picture there is, which is of a chunk a little before it: as far behind as the stream
//! reorders its pictures, and a tick or two more. Ruffle's decoder in a browser is behind the
//! same way.
//!
//! The decoder puts frames out in the order the stream shows them, read from the stream itself,
//! so the time each chunk carries is only a label, and the number the player gives a chunk
//! serves.
//!
//! The invariant: only the tasks touch the decoder. The stream tells them what the player asks
//! through a channel, and dropping the stream closes the channel, which ends them and lets the
//! decoder go.

use std::cell::RefCell;
use std::rc::Rc;

use async_channel::{Receiver, Sender};
use ruffle_render::backend::RenderBackend;
use ruffle_render::bitmap::{Bitmap, BitmapHandle, BitmapInfo, PixelRegion};
use ruffle_video::error::Error;
use ruffle_video::frame::{EncodedFrame, FrameDependency};
use wasdk::video_decoders::{
    self, DecoderError, DecodingPurpose, VideoDecoder, VideoDecoderOptions, VideoDecoders,
};
use wasdk::video_frames::VideoFrame;
use wasdk::video_streams::{CodecChoice, EncodedChunk, NalFormat, VideoStreamSetup};
use wasdk::{FutureReader, StreamReader};

use super::picture;

/// The kind of unit that is a key picture: one that decodes with nothing before it.
const KEY_PICTURE: u8 = 5;

/// The frames the program holds at once: the one being copied out, and one waiting behind it.
const FRAME_LIMIT: u32 = 2;

/// What the player asks of the decoder.
enum Work {
    /// The stream's configuration record (`avcC`): its parameter sets, and how its units are
    /// framed.
    Setup(Vec<u8>),
    Chunk(EncodedChunk),
}

/// The newest picture decoded and not yet drawn.
type Newest = Rc<RefCell<Option<Bitmap<'static>>>>;

pub(super) struct H264Stream {
    /// How many bytes hold the length before each unit of a chunk, once the setup said.
    length_size: Option<usize>,
    asked: Sender<Work>,
    newest: Newest,
    /// The picture the player draws, once one was decoded.
    drawn: Option<BitmapInfo>,
}

impl H264Stream {
    /// A stream whose decoder is opened once its setup comes.
    pub(super) fn start() -> Self {
        let (asked, work) = async_channel::unbounded();
        let newest = Newest::default();
        wasdk::spawn(hand_over(work, Rc::clone(&newest)));
        H264Stream {
            length_size: None,
            asked,
            newest,
            drawn: None,
        }
    }

    /// Takes the stream's configuration record, which the decoder is opened with, or goes on
    /// with where it is open.
    pub(super) fn set_up(&mut self, record: &[u8]) -> Result<(), Error> {
        let length_size = length_size_of(record).ok_or_else(|| {
            Error::DecoderError("the H.264 stream's configuration is no avcC record".into())
        })?;
        self.length_size = Some(length_size);
        self.ask(Work::Setup(record.to_vec()));
        Ok(())
    }

    /// Whether the chunk `data` decodes with nothing before it.
    pub(super) fn dependency_of(&self, data: &[u8]) -> FrameDependency {
        if self.is_key_picture(data) {
            FrameDependency::None
        } else {
            FrameDependency::Past
        }
    }

    /// Hands `frame` to the decoder and answers the newest picture there is, drawn on
    /// `renderer`; no picture, where none was decoded yet.
    pub(super) fn decode(
        &mut self,
        frame: EncodedFrame<'_>,
        renderer: &mut dyn RenderBackend,
    ) -> Result<BitmapInfo, Error> {
        self.ask(Work::Chunk(EncodedChunk {
            data: frame.data.to_vec(),
            timestamp: i64::from(frame.frame_id),
            duration: None,
            key_frame: self.is_key_picture(frame.data),
        }));
        let newest = self.newest.borrow_mut().take();
        if let Some(picture) = newest {
            self.drawn = Some(draw(picture, self.drawn.take(), renderer)?);
        }
        self.drawn.clone().ok_or(Error::DecoderNoOutputFrame)
    }

    fn ask(&self, work: Work) {
        // The tasks end only once the decoder cannot go on, which they have said.
        let _ = self.asked.try_send(work);
    }

    fn is_key_picture(&self, data: &[u8]) -> bool {
        let Some(length_size) = self.length_size else {
            return false;
        };
        unit_types(data, length_size).any(|unit_type| unit_type == KEY_PICTURE)
    }
}

/// How many bytes hold the length before each unit, as a configuration record says; none for
/// bytes that are no such record.
fn length_size_of(record: &[u8]) -> Option<usize> {
    let &[1, _, _, _, sizes, ..] = record else {
        return None;
    };
    Some(usize::from(sizes & 0b11) + 1)
}

/// The kind of each unit of `data`, whose units each follow a length of `length_size` bytes.
/// Ends where a length runs past the data.
fn unit_types(data: &[u8], length_size: usize) -> impl Iterator<Item = u8> {
    let mut rest = data;
    std::iter::from_fn(move || {
        let (length, after) = rest.split_at_checked(length_size)?;
        let length = length
            .iter()
            .fold(0usize, |length, byte| (length << 8) | usize::from(*byte));
        let (unit, following) = after.split_at_checked(length)?;
        rest = following;
        Some(unit.first()? & 0x1F)
    })
}

/// `picture` on `renderer`: in the texture already `drawn`, where it is the same size, and in
/// a new one otherwise.
fn draw(
    picture: Bitmap<'static>,
    drawn: Option<BitmapInfo>,
    renderer: &mut dyn RenderBackend,
) -> Result<BitmapInfo, Error> {
    let (width, height) = (picture.width(), picture.height());
    let handle: BitmapHandle = match drawn {
        Some(drawn) if (drawn.width, drawn.height) == (width, height) => {
            let whole = PixelRegion::for_whole_size(width, height);
            renderer.update_texture(&drawn.handle, picture, whole)?;
            drawn.handle
        }
        _ => renderer.register_bitmap(picture)?,
    };
    Ok(BitmapInfo {
        handle,
        width,
        height,
    })
}

/// Hands the decoder each chunk that comes through `work`, as it has room, opening it with the
/// stream's setup; ends once the stream is dropped, or the decoder cannot go on.
async fn hand_over(work: Receiver<Work>, newest: Newest) {
    let Some(decoders) = video_decoders::get_video_decoders() else {
        tracing::warn!("H.264 video is not shown: WaOS has no video decoder on this system");
        return;
    };
    let mut decoder = None;
    while let Ok(work) = work.recv().await {
        let done = match (work, &decoder) {
            (Work::Setup(record), _) => set_up(&decoders, &mut decoder, record, &newest).await,
            (Work::Chunk(chunk), Some(decoder)) => decoder.decode(chunk).await,
            // A chunk before the stream's setup has nothing to decode it.
            (Work::Chunk(_), None) => Ok(()),
        };
        match done {
            Ok(()) => {}
            // The decoder goes on with the next key picture.
            Err(DecoderError::BadData(reason)) => tracing::warn!("an H.264 chunk: {reason}"),
            Err(error) => {
                tracing::error!("H.264 video stops: {error:?}");
                return;
            }
        }
    }
}

fn setup_of(record: Vec<u8>) -> VideoStreamSetup {
    VideoStreamSetup::H264(NalFormat::LengthPrefixed(record))
}

/// Opens `decoder` with the stream's setup, `record`, or has it go on with it where it is open.
async fn set_up(
    decoders: &VideoDecoders,
    decoder: &mut Option<VideoDecoder>,
    record: Vec<u8>,
    newest: &Newest,
) -> Result<(), DecoderError> {
    match decoder {
        Some(open) => open.configure(setup_of(record)).await,
        None => {
            *decoder = Some(opened(decoders, record, newest).await?);
            Ok(())
        }
    }
}

/// A decoder for the stream `record` sets up, its frames read into `newest` from now on.
async fn opened(
    decoders: &VideoDecoders,
    record: Vec<u8>,
    newest: &Newest,
) -> Result<VideoDecoder, DecoderError> {
    let options = VideoDecoderOptions {
        decoder: CodecChoice::Any,
        purpose: DecodingPurpose::Playback,
        frame_limit: FRAME_LIMIT,
        largest_size: None,
        default_color: None,
    };
    let decoder = decoders.open(setup_of(record), options).await?;
    let (frames, ended) = decoder.frames();
    wasdk::spawn(read(frames, ended, Rc::clone(newest)));
    Ok(decoder)
}

/// Keeps each frame the decoder puts out as the newest picture, until its frames end, and says
/// why where it ended by itself.
async fn read(
    mut frames: StreamReader<VideoFrame>,
    ended: FutureReader<Result<(), DecoderError>>,
    newest: Newest,
) {
    while let Some(frame) = frames.next().await {
        match picture::of_frame(&frame).await {
            Ok(picture) => *newest.borrow_mut() = Some(picture),
            Err(reason) => tracing::warn!("an H.264 picture is not shown: {reason}"),
        }
    }
    if let Err(error) = ended.await {
        tracing::error!("the H.264 decoder ended: {error:?}");
    }
}
