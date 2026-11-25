//! WaOS port: what the player asks of the system, where WaOS serves it and a desktop's own
//! libraries cannot: sound through WaOS's audio graph, and H.264 video through WaOS's decoders,
//! which are the system's.

mod audio;
mod video;

pub use audio::WaosAudioBackend;
pub use video::WaosVideoBackend;
