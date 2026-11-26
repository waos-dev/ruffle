//! The program: one movie played in one window, from choosing the movie to the window's end.
//!
//! Ruffle's player is built on WaOS's own: its wgpu renderer on the window's view (`renderer`),
//! (`renderer`, `view_target`), with the program's own controls drawn over the movie in egui
//! (`gui`), its mixer played through WaOS's audio graph (`audio`), its H.264 video decoded by
//! WaOS's decoders (`video`), and the small backends the player asks its platform for (`navigator`,
//! `storage`, `ui`). `playback` then runs the player from the
//! view's events.
//!
//! The invariant: everything runs on the program's one thread, as tasks of wasdk's executor, where
//! a task runs only while the others await. Ruffle keeps its player behind a `Mutex`, which the
//! loads it spawns lock too (`navigator`), so the lock is never held across an `await`: a load
//! that ran meanwhile could never take it.

mod audio;
mod gui;
mod keyboard;
mod keys;
mod movie;
mod navigator;
mod playback;
mod pointer;
mod renderer;
mod storage;
mod ui;
mod video;
mod view_target;
mod window;

use std::path::Path;
use std::sync::{Arc, Mutex};

use ruffle_core::tag_utils::SwfMovieData;
use ruffle_core::{Player, PlayerBuilder};
use ruffle_render_wgpu::backend::WgpuRenderBackend;
use tracing_subscriber::EnvFilter;
use wasdk::views::View;
use wasdk::windows::Size;

use audio::WaosAudioBackend;
use gui::Gui;
use navigator::WaosNavigatorBackend;
use storage::WaosStorageBackend;
use ui::WaosUiBackend;
use video::WaosVideoBackend;
use view_target::ViewTarget;

/// What the log shows unless `RUST_LOG` says otherwise: warnings, and what movies trace.
const DEFAULT_LOG_FILTER: &str = "warn,avm_trace=info";

/// Plays the movie the program was started with, or the one its user picks, until its window is
/// gone. Ends at once when the user picks none.
pub async fn run() -> Result<(), String> {
    write_log_to_stdout();
    let Some(movie_path) = movie::chosen_path().await? else {
        return Ok(());
    };
    let movie = movie::read(&movie_path)?;
    let view = window::open(&movie::name(&movie_path), stage_size(&movie)).await?;
    let renderer = renderer::on_view(&view).await?;
    let descriptors = Arc::clone(&renderer.descriptors);
    let gui = Gui::new(&view, Arc::clone(&descriptors), renderer.format);
    let drawn = renderer.drawn.clone();
    let player = build_player(
        movie,
        &movie_path,
        &view,
        renderer.backend,
        &descriptors.device,
    );
    playback::play(&player, &view, gui, drawn, &descriptors.queue).await
}

/// Sends `tracing`'s events, Ruffle's log, to the program's output, which `waos run` and
/// `waos attach` print, filtered as `RUST_LOG` says.
fn write_log_to_stdout() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_FILTER));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false)
        .init();
}

/// The size `movie` was drawn for, which its window opens at.
fn stage_size(movie: &SwfMovieData) -> Size {
    Size {
        width: movie.width().to_pixels(),
        height: movie.height().to_pixels(),
    }
}

/// Ruffle's player for `movie`, read from `movie_path`, drawing on `view` through `renderer`, whose
/// GPU is `device`'s.
fn build_player(
    movie: SwfMovieData,
    movie_path: &Path,
    view: &Arc<View>,
    renderer: WgpuRenderBackend<ViewTarget>,
    device: &wgpu::Device,
) -> Arc<Mutex<Player>> {
    let metrics = view.metrics();
    let builder = PlayerBuilder::new()
        .with_movie(movie)
        .with_renderer(renderer)
        .with_viewport_dimensions(metrics.width, metrics.height, metrics.device_pixel_ratio)
        .with_navigator(WaosNavigatorBackend::beside(movie_path))
        .with_storage(Box::new(WaosStorageBackend::new()))
        .with_ui(WaosUiBackend::new(Arc::clone(view)))
        .with_video(WaosVideoBackend::new(device.clone()))
        .with_autoplay(true);
    with_sound(builder).build()
}

/// `builder` playing through WaOS's sound; unchanged, so silent, where WaOS gives none.
fn with_sound(builder: PlayerBuilder) -> PlayerBuilder {
    match WaosAudioBackend::start() {
        Ok(audio) => builder.with_audio(audio),
        Err(reason) => {
            tracing::warn!("playing without sound: {reason}");
            builder
        }
    }
}
