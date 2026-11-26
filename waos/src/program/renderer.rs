//! Ruffle's wgpu renderer on a view of WaOS.
//!
//! The invariant: the wgpu instance is `wasdk_wgpu`'s, the only one with a backend in a WaOS
//! program. Ruffle's own constructors make theirs with `wgpu::Instance::new`, which has none
//! here, so the renderer is put together from the pieces those constructors use.

use std::sync::Arc;

use ruffle_render_wgpu::backend::{WgpuRenderBackend, request_adapter_and_device};
use ruffle_render_wgpu::descriptors::Descriptors;
use ruffle_render_wgpu::target::RenderTarget as _;
use wasdk::views::View;

use super::view_target::{DrawnFrame, ViewTarget};

/// Ruffle's renderer on a view, with what the program draws its own controls over the movie with.
pub struct ViewRenderer {
    pub backend: WgpuRenderBackend<ViewTarget>,
    /// The device and queue Ruffle draws with.
    pub descriptors: Arc<Descriptors>,
    /// The format a frame is drawn in.
    pub format: wgpu::TextureFormat,
    /// Each frame Ruffle drew, until the program presents it.
    pub drawn: DrawnFrame,
}

/// Ruffle's renderer drawing on `view`, at the view's size.
#[expect(
    clippy::arc_with_non_send_sync,
    reason = "Ruffle's renderer takes its descriptors in an `Arc`, as its own crate allows itself"
)]
pub async fn on_view(view: &Arc<View>) -> Result<ViewRenderer, String> {
    let instance = wasdk_wgpu::instance();
    let surface = instance
        .create_surface(view.clone())
        .map_err(|error| format!("no surface on the window: {error}"))?;
    let (adapter, device, queue) = request_adapter_and_device(
        wgpu::Backends::all(),
        &instance,
        Some(&surface),
        wgpu::PowerPreference::HighPerformance,
    )
    .await
    .map_err(|error| format!("no graphics device: {error}"))?;
    let descriptors = Descriptors::new(instance, adapter, device, queue);
    let metrics = view.metrics();
    let drawn = DrawnFrame::default();
    let target = ViewTarget::new(
        surface,
        &descriptors.adapter,
        (metrics.width, metrics.height),
        &descriptors.device,
        drawn.clone(),
    );
    let format = target.format();
    let descriptors = Arc::new(descriptors);
    let backend = WgpuRenderBackend::new(Arc::clone(&descriptors), target)
        .map_err(|error| format!("Ruffle's renderer did not start: {error}"))?;
    Ok(ViewRenderer {
        backend,
        descriptors,
        format,
        drawn,
    })
}
