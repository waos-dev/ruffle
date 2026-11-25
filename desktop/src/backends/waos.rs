//! WaOS port: what the player asks of the system, where WaOS serves it and a desktop's own
//! libraries cannot: sound through WaOS's audio graph, and the movie's files.

mod audio;
mod navigator;

pub use audio::WaosAudioBackend;
pub use navigator::WaosNavigatorBackend;
