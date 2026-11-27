//! The program's own controls, drawn with egui over the movie: a bar to pause the movie and set
//! its volume.
//!
//! The view's events reach egui through `wasdk-adapter-egui`'s `State`, as a winit window's do
//! through `egui-winit`'s in Ruffle's desktop program, and egui's own wgpu renderer draws the bar
//! into the frame Ruffle drew the movie into, on the device Ruffle draws with.
//!
//! The invariant: an event egui used up does not reach the movie, so a click on the bar is not a
//! click in the movie under it.

use std::sync::Arc;

use ruffle_core::Player;
use ruffle_render_wgpu::descriptors::Descriptors;
use wasdk::views::{View, ViewEvent};
use wasdk_adapter_egui::State;

pub struct Gui {
    egui_ctx: egui::Context,
    state: State,
    renderer: egui_wgpu::Renderer,
    descriptors: Arc<Descriptors>,
    /// The format the movie's frame is drawn in, which the bar is drawn in over it.
    format: wgpu::TextureFormat,
    /// Whether egui has something new to show: an event it took, or a pass that asked for the
    /// next one at once, as it does while something animates.
    wants_repaint: bool,
}

impl Gui {
    /// The controls on `view`, drawn on Ruffle's device into frames of `format`.
    pub fn new(
        view: &Arc<View>,
        descriptors: Arc<Descriptors>,
        format: wgpu::TextureFormat,
    ) -> Gui {
        let egui_ctx = egui::Context::default();
        let mut state = State::new(egui_ctx.clone(), view.clone());
        state.set_max_texture_side(descriptors.limits.max_texture_dimension_2d as usize);
        let options = egui_wgpu::RendererOptions::default();
        let renderer = egui_wgpu::Renderer::new(&descriptors.device, format, options);
        Gui {
            egui_ctx,
            state,
            renderer,
            descriptors,
            format,
            wants_repaint: true,
        }
    }

    /// Takes one of the view's events in, and answers it back unless the controls used it up.
    pub fn on_view_event(&mut self, event: ViewEvent) -> Option<ViewEvent> {
        let response = self.state.on_view_event(event);
        self.wants_repaint |= response.repaint;
        response.event
    }

    /// Whether the controls have something new to show, so the frame is worth drawing though
    /// the movie's picture did not change.
    pub fn wants_repaint(&self) -> bool {
        self.wants_repaint
    }

    /// Lays the controls out for `player` and draws them into `frame`, over the movie Ruffle
    /// drew there.
    pub fn draw_over(&mut self, player: &mut Player, frame: &wgpu::SurfaceTexture) {
        let raw_input = self.state.take_egui_input();
        let full_output = self.egui_ctx.run_ui(raw_input, |ui| controls(ui, player));
        self.state
            .handle_platform_output(full_output.platform_output);
        self.wants_repaint = full_output
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .is_some_and(|root| root.repaint_delay.is_zero());
        let pixels_per_point = full_output.pixels_per_point;
        let primitives = self
            .egui_ctx
            .tessellate(full_output.shapes, pixels_per_point);
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [frame.texture.width(), frame.texture.height()],
            pixels_per_point,
        };
        let (device, queue) = (&self.descriptors.device, &self.descriptors.queue);
        // egui refuses to have a pass's textures dropped unapplied, so they are taken out.
        let mut textures = full_output.textures_delta;
        for (id, deltas) in textures.set.drain() {
            for delta in &deltas {
                self.renderer.update_texture(device, queue, id, delta);
            }
        }
        let target = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.format),
            ..Default::default()
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.renderer
            .update_buffers(device, queue, &mut encoder, &primitives, &screen);
        let mut pass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("controls"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target,
                    depth_slice: None,
                    resolve_target: None,
                    // Over the movie, which Ruffle drew there.
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            })
            .forget_lifetime();
        self.renderer.render(&mut pass, &primitives, &screen);
        drop(pass);
        queue.submit([encoder.finish()]);
        for id in textures.free.drain() {
            self.renderer.free_texture(&id);
        }
    }
}

/// The bar: the movie paused or playing, and its volume.
fn controls(ui: &mut egui::Ui, player: &mut Player) {
    egui::Panel::top("controls").show(ui, |ui| {
        ui.horizontal(|ui| {
            let playing = player.is_playing();
            let label = if playing { "Pause" } else { "Play" };
            if ui.button(label).clicked() {
                player.set_is_playing(!playing);
            }
            let mut volume = player.volume();
            let slider = egui::Slider::new(&mut volume, 0.0..=1.0).text("Volume");
            if ui.add(slider).changed() {
                player.set_volume(volume);
            }
        });
    });
}
