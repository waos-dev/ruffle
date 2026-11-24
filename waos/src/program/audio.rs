//! Ruffle's sound on WaOS: Ruffle's own mixer, played through a stream input of WaOS's audio
//! graph.
//!
//! The backend keeps the mixer, which the player starts and stops sounds on. A task of the
//! program's executor keeps the audio context, and writes what the mixer mixes into the context's
//! stream input, a block at a time. A write waits while the stream is its latency ahead of what
//! plays, so the task paces itself, and runs whenever the program's main loop awaits; a movie
//! that holds the thread longer than that latency is heard as a gap. While the player is paused
//! the context is suspended, so a paused movie makes no sound and WaOS may freeze it out of sight.
//!
//! The invariant: only the task touches the context and its stream input. The backend tells it
//! what the player asks through a channel, and dropping the backend closes the channel, which
//! ends the task and closes the context.

use std::time::Duration;

use async_channel::{Receiver, Sender, TryRecvError};
use ruffle_core::backend::audio::{
    AudioBackend, AudioMixer, AudioMixerProxy, DecodeError, RegisterError, SoundHandle,
    SoundInstanceHandle, SoundStreamInfo, SoundTransform, swf,
};
use ruffle_core::impl_audio_mixer_backend;
use wasdk::audio::{self, AudioContext, AudioContextOptions, StreamInputNode};

const CHANNELS: u8 = 2;

/// The frames mixed and written at once: eight of the graph's blocks, 21 ms at 48 kHz.
const BLOCK_FRAMES: usize = 1024;

/// What the player asks of the output.
enum Playback {
    Play,
    Pause,
}

pub struct WaosAudioBackend {
    mixer: AudioMixer,
    sample_rate: f32,
    asked_playback: Sender<Playback>,
}

impl WaosAudioBackend {
    /// Opens WaOS's default output, or the one the user picked for the program, and starts the
    /// task that plays the mixer on it.
    pub fn start() -> Result<Self, String> {
        let audio = audio::get_audio().ok_or("WaOS gives this program no sound")?;
        let context = audio
            .new_context(AudioContextOptions::new())
            .map_err(|error| format!("WaOS refused a sound context: {error:?}"))?;
        let sample_rate = context.base().sample_rate();
        let mixer = AudioMixer::new(CHANNELS, sample_rate as u32);
        let (asked_playback, asked) = async_channel::unbounded();
        wasdk::spawn(Output::on(context, mixer.proxy(), asked).run());
        Ok(WaosAudioBackend {
            mixer,
            sample_rate,
            asked_playback,
        })
    }

    fn ask(&self, playback: Playback) {
        // The task is gone only once WaOS refused its sound, which it logged.
        let _ = self.asked_playback.try_send(playback);
    }
}

impl AudioBackend for WaosAudioBackend {
    impl_audio_mixer_backend!(mixer);

    fn play(&mut self) {
        self.ask(Playback::Play);
    }

    fn pause(&mut self) {
        self.ask(Playback::Pause);
    }

    fn position_resolution(&self) -> Option<Duration> {
        Some(Duration::from_secs_f64(
            BLOCK_FRAMES as f64 / f64::from(self.sample_rate),
        ))
    }
}

/// The task's side: the context, its stream input, and what feeds and steers them.
struct Output {
    context: AudioContext,
    stream: StreamInputNode,
    mixer: AudioMixerProxy,
    asked: Receiver<Playback>,
}

impl Output {
    /// An output on `context`, a stream input connected to its destination.
    fn on(context: AudioContext, mixer: AudioMixerProxy, asked: Receiver<Playback>) -> Self {
        let stream = context.create_stream_input(u32::from(CHANNELS));
        stream.node().connect(&context.base().destination().node());
        Output {
            context,
            stream,
            mixer,
            asked,
        }
    }

    /// Plays and pauses as the player asks, until the backend is gone or WaOS refuses the sound.
    async fn run(self) {
        let mut asked = Some(Playback::Play);
        while let Some(playback) = asked {
            asked = match playback {
                Playback::Play => self.write_until_asked().await,
                Playback::Pause => self.suspend_until_asked().await,
            };
        }
        self.context.close().await;
    }

    /// Writes what the mixer mixes, a block at a time, until the player asks to pause; none once
    /// the backend is gone or WaOS refuses a block.
    async fn write_until_asked(&self) -> Option<Playback> {
        loop {
            match self.asked.try_recv() {
                Ok(Playback::Play) | Err(TryRecvError::Empty) => {}
                Ok(Playback::Pause) => return Some(Playback::Pause),
                Err(TryRecvError::Closed) => return None,
            }
            let mut block = vec![0.0; BLOCK_FRAMES * usize::from(CHANNELS)];
            self.mixer.mix::<f32>(&mut block);
            if let Err(error) = self.stream.write(block).await {
                tracing::warn!("WaOS refused the sound written: {error:?}");
                return None;
            }
        }
    }

    /// Keeps the context suspended until the player asks to play; none once the backend is gone.
    async fn suspend_until_asked(&self) -> Option<Playback> {
        if let Err(error) = self.context.suspend().await {
            tracing::warn!("WaOS's sound did not suspend: {error:?}");
        }
        let asked = loop {
            match self.asked.recv().await {
                Ok(Playback::Pause) => {}
                Ok(Playback::Play) => break Some(Playback::Play),
                Err(_) => break None,
            }
        };
        if let Err(error) = self.context.resume().await {
            tracing::warn!("WaOS's sound did not resume: {error:?}");
        }
        asked
    }
}
