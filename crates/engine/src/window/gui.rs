use egui::Context;
use egui_wgpu::wgpu::{self, CommandEncoder, Device, Queue, StoreOp, TextureFormat, TextureView};
use egui_wgpu::{Renderer, ScreenDescriptor};
use egui_winit::{EventResponse, State};
use winit::event::WindowEvent;
use winit::window::Window;

use crate::{PassContext, RenderPass};

pub struct EguiRenderer {
    state: State,
    renderer: Renderer,
    frame_started: bool,
}

impl EguiRenderer {
    pub fn new(
        device: &Device,
        output_color_format: TextureFormat,
        output_depth_format: Option<TextureFormat>,
        msaa_samples: u32,
        window: &Window,
    ) -> Self {
        let egui_context = Context::default();

        // Configuration du style
        // let mut visuals = egui::Visuals::dark();
        // visuals.window_shadow.extrusion = 4.0;

        // egui_context.set_visuals(visuals);

        let egui_state = egui_winit::State::new(
            egui_context,
            egui::viewport::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            Some(1024 * 2),
        );

        let egui_renderer = Renderer::new(
            device,
            output_color_format,
            output_depth_format,
            msaa_samples,
            true,
        );

        Self {
            state: egui_state,
            renderer: egui_renderer,
            frame_started: false,
        }
    }

    pub fn context(&self) -> &Context {
        self.state.egui_ctx()
    }

    pub fn handle_input(&mut self, window: &Window, event: &WindowEvent) -> EventResponse {
        self.state.on_window_event(window, event)
    }

    pub fn begin_frame(&mut self, window: &Window) {
        let raw_input = self.state.take_egui_input(window);
        self.state.egui_ctx().begin_pass(raw_input);
        self.frame_started = true;
    }

    pub fn end_frame_and_draw(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        window: &Window,
        window_surface_view: &TextureView,
        screen_descriptor: ScreenDescriptor,
    ) {
        if !self.frame_started {
            // Avoid panicking if begin_frame wasn't called. Log and return instead to
            // keep the renderer stable if callers forget to start the frame.
            eprintln!(
                "Warning: end_frame_and_draw called without a matching begin_frame; skipping draw."
            );
            return;
        }

        self.context()
            .set_pixels_per_point(screen_descriptor.pixels_per_point);

        let full_output = self.state.egui_ctx().end_pass();

        self.state
            .handle_platform_output(window, full_output.platform_output);

        let tris = self
            .state
            .egui_ctx()
            .tessellate(full_output.shapes, self.state.egui_ctx().pixels_per_point());

        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }

        self.renderer
            .update_buffers(device, queue, encoder, &tris, &screen_descriptor);

        let rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: window_surface_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            label: Some("egui render pass"),
            occlusion_query_set: None,
        });

        self.renderer
            .render(&mut rpass.forget_lifetime(), &tris, &screen_descriptor);

        for id in &full_output.textures_delta.free {
            self.renderer.free_texture(id);
        }

        self.frame_started = false;
    }
}

/// Render pass wrapper to run egui's end-frame drawing within the Pass system.
///
/// This pass assumes the window has already begun the egui frame (`begin_frame`)
/// and the application has already populated the UI via `Window::draw(...)`.
/// The pass performs the final buffer updates and submits the render pass that
/// draws the tessellated egui meshes into the provided render target.
pub struct EguiPass;

impl EguiPass {
    pub fn new() -> Self {
        Self
    }
}

impl RenderPass for EguiPass {
    fn name(&self) -> &str {
        "egui_pass"
    }

    fn prepare(&mut self, _device: &wgpu::Device, _queue: &Queue) {
        // No preparation needed here; EguiRenderer resources are owned by WindowState.
    }

    fn execute(&self, ctx: &mut PassContext) {
        // Build screen descriptor from WindowState + actual window scale.
        let width = ctx.window_state.config.width;
        let height = ctx.window_state.config.height;
        let pixels_per_point = ctx.window.scale_factor() as f32 * ctx.window_state.scale_factor;

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point,
        };

        // Drive egui end-frame + draw using the window's EguiRenderer instance.
        ctx.window_state.egui_renderer.end_frame_and_draw(
            &ctx.window_state.device,
            ctx.queue,
            ctx.encoder,
            ctx.window,
            ctx.target,
            screen_descriptor,
        );
    }
}
