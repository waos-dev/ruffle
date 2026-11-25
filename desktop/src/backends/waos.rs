//! WaOS port: what the player asks of the system, where WaOS serves it and a desktop's own
//! libraries cannot: sound through WaOS's audio graph, the movie's files, and H.264 video through
//! WaOS's decoders, which are the system's.

mod audio;
mod navigator;
mod video;

pub use audio::WaosAudioBackend;
pub use navigator::WaosNavigatorBackend;
pub use video::WaosVideoBackend;
