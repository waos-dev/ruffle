//! Where Ruffle draws the movie: the view's surface, as Ruffle's own swap chain target, except
//! that a frame is not shown as Ruffle submits it. It waits ([`DrawnFrame`]) for the program to
//! draw its controls over the movie (`gui`) and present it.
//!
//! The invariant: a frame Ruffle drew is taken once, and whoever takes it presents it; one left
//! untaken is let go as the next arrives, which shows nothing.

use std::cell::RefCell;
use std::rc::Rc;

use ruffle_render_wgpu::target::{RenderTarget, RenderTargetFrame};

/// The frame Ruffle drew the movie into last and nobody has presented yet. Every clone reaches
/// the same one.
#[derive(Clone, Debug, Default)]
pub struct DrawnFrame(Rc<RefCell<Option<wgpu::SurfaceTexture>>>);

impl DrawnFrame {
    /// The frame, for whoever presents it; none when Ruffle drew none since the last was taken.
    pub fn take(&self) -> Option<wgpu::SurfaceTexture> {
        self.0.borrow_mut().take()
    }

    fn put(&self, frame: wgpu::SurfaceTexture) {
        *self.0.borrow_mut() = Some(frame);
    }
}

#[derive(Debug)]
pub struct ViewTarget {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    drawn: DrawnFrame,
}

#[derive(Debug)]
pub struct ViewTargetFrame {
    texture: wgpu::SurfaceTexture,
    view: wgpu::TextureView,
}

impl RenderTargetFrame for ViewTargetFrame {
    fn into_view(self) -> wgpu::TextureView {
        self.view
    }

    fn view(&self) -> &wgpu::TextureView {
        &self.view
    }
}

impl ViewTarget {
    /// `surface` configured at `size`, in a format Flash's colours go into unchanged, as Ruffle's
    /// swap chain target chooses one; each frame drawn waits in `drawn`.
    pub fn new(
        surface: wgpu::Surface<'static>,
        adapter: &wgpu::Adapter,
        (width, height): (u32, u32),
        device: &wgpu::Device,
        drawn: DrawnFrame,
    ) -> ViewTarget {
        let capabilities = surface.get_capabilities(adapter);
        let format = capabilities
            .formats
            .iter()
            .find(|format| {
                matches!(
                    format,
                    wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Bgra8Unorm
                )
            })
            .or_else(|| capabilities.formats.first())
            .copied()
            .unwrap_or(wgpu::TextureFormat::Rgba8Unorm);
        let linear_format = format.remove_srgb_suffix();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: if linear_format == format {
                vec![format]
            } else {
                vec![format, linear_format]
            },
        };
        surface.configure(device, &config);
        ViewTarget {
            surface,
            config,
            drawn,
        }
    }
}

impl RenderTarget for ViewTarget {
    type Frame = ViewTargetFrame;

    fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(device, &self.config);
    }

    /// The format the movie is drawn in: the surface's, read without the sRGB encoding.
    fn format(&self) -> wgpu::TextureFormat {
        self.config.format.remove_srgb_suffix()
    }

    fn width(&self) -> u32 {
        self.config.width
    }

    fn height(&self) -> u32 {
        self.config.height
    }

    fn get_next_texture(&mut self) -> Option<ViewTargetFrame> {
        let texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture)
            | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
            state => {
                tracing::warn!("the window gave no frame to draw into: {state:?}");
                return None;
            }
        };
        let view = texture.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.format()),
            ..Default::default()
        });
        Some(ViewTargetFrame { texture, view })
    }

    /// Submits what Ruffle drew, and keeps the frame for whoever presents it.
    fn submit<I: IntoIterator<Item = wgpu::CommandBuffer>>(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        command_buffers: I,
        frame: ViewTargetFrame,
    ) -> wgpu::SubmissionIndex {
        let index = queue.submit(command_buffers);
        self.drawn.put(frame.texture);
        index
    }
}
