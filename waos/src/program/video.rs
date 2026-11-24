//! Ruffle's video on WaOS: H.264 through WaOS's video decoders, which are the system's, and every
//! other codec through Ruffle's own software decoders.
//!
//! The backend sorts each stream by its codec as the player registers it. An H.264 stream is one
//! of `h264`'s; any other is the software backend's, reached by the handle that backend gave.
//!
//! The invariant: a handle the player holds is this backend's own, never the software backend's:
//! the two are told apart only here.

mod h264;
mod picture;

use ruffle_core::swf::{VideoCodec, VideoDeblocking};
use ruffle_render::backend::RenderBackend;
use ruffle_render::bitmap::BitmapInfo;
use ruffle_video::VideoStreamHandle;
use ruffle_video::backend::VideoBackend;
use ruffle_video::error::Error;
use ruffle_video::frame::{EncodedFrame, FrameDependency};
use ruffle_video_software::backend::SoftwareVideoBackend;
use slotmap::SlotMap;

use h264::H264Stream;

/// Where a stream the player registered is decoded.
enum Stream {
    /// By WaOS's decoder.
    H264(H264Stream),
    /// By Ruffle's software backend, under the handle it gave.
    Software(VideoStreamHandle),
}

pub struct WaosVideoBackend {
    streams: SlotMap<VideoStreamHandle, Stream>,
    software: SoftwareVideoBackend,
}

impl WaosVideoBackend {
    pub fn new() -> Self {
        WaosVideoBackend {
            streams: SlotMap::with_key(),
            software: SoftwareVideoBackend::new(),
        }
    }

    fn stream(&mut self, handle: VideoStreamHandle) -> Result<&mut Stream, Error> {
        self.streams
            .get_mut(handle)
            .ok_or(Error::VideoStreamIsNotRegistered)
    }
}

impl VideoBackend for WaosVideoBackend {
    fn register_video_stream(
        &mut self,
        num_frames: u32,
        size: (u16, u16),
        codec: VideoCodec,
        filter: VideoDeblocking,
    ) -> Result<VideoStreamHandle, Error> {
        let stream = if codec == VideoCodec::H264 {
            Stream::H264(H264Stream::start())
        } else {
            let handle = self
                .software
                .register_video_stream(num_frames, size, codec, filter)?;
            Stream::Software(handle)
        };
        Ok(self.streams.insert(stream))
    }

    fn configure_video_stream_decoder(
        &mut self,
        stream: VideoStreamHandle,
        configuration_data: &[u8],
    ) -> Result<(), Error> {
        match self.stream(stream)? {
            Stream::H264(stream) => stream.set_up(configuration_data),
            Stream::Software(handle) => {
                let handle = *handle;
                self.software
                    .configure_video_stream_decoder(handle, configuration_data)
            }
        }
    }

    fn preload_video_stream_frame(
        &mut self,
        stream: VideoStreamHandle,
        encoded_frame: EncodedFrame<'_>,
    ) -> Result<FrameDependency, Error> {
        match self.stream(stream)? {
            Stream::H264(stream) => Ok(stream.dependency_of(encoded_frame.data)),
            Stream::Software(handle) => {
                let handle = *handle;
                self.software
                    .preload_video_stream_frame(handle, encoded_frame)
            }
        }
    }

    fn decode_video_stream_frame(
        &mut self,
        stream: VideoStreamHandle,
        encoded_frame: EncodedFrame<'_>,
        renderer: &mut dyn RenderBackend,
    ) -> Result<BitmapInfo, Error> {
        match self.stream(stream)? {
            Stream::H264(stream) => stream.decode(encoded_frame, renderer),
            Stream::Software(handle) => {
                let handle = *handle;
                self.software
                    .decode_video_stream_frame(handle, encoded_frame, renderer)
            }
        }
    }
}
