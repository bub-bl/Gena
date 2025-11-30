use egui::Context;
use egui_wgpu::wgpu::{self, CommandEncoder, Device, Queue, TextureFormat, TextureView};
use egui_wgpu::{Renderer, ScreenDescriptor};
use egui_winit::{EventResponse, State};
use winit::event::WindowEvent;
use winit::window::Window;

use crate::{PassContext, RenderPass};

/// A small, focused wrapper around egui_winit + egui_wgpu renderer.
/// Purpose: provide the minimal API a Window needs to begin an egui frame,
/// feed window events, and finalize/draw the resulting egui output.
pub struct EguiRenderer {
    state: State,
    renderer: Renderer,
    frame_started: bool,
}

impl EguiRenderer {
    /// Create a new renderer instance.
    /// - `device` and `queue` are used later when drawing.
    /// - `output_color_format` is the swapchain format this renderer will target.
    /// - `output_depth_format` currently unused but kept for future compatibility.
    /// - `msaa_samples` controls MSAA used by the egui renderer.
    /// - `window` is required to initialize egui_winit state.
    pub fn new(
        device: &Device,
        output_color_format: TextureFormat,
        _output_depth_format: Option<TextureFormat>,
        msaa_samples: u32,
        window: &Window,
    ) -> Self {
        // Create an egui Context implicitly via State
        let egui_ctx = Context::default();

        // egui_winit::State manages platform-level integration (input, cursor, etc).
        let state = State::new(
            egui_ctx,
            egui::viewport::ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );

        // Renderer handles translating egui tessellations -> wgpu draws.
        let renderer = Renderer::new(
            device,
            output_color_format,
            _output_depth_format,
            msaa_samples,
            true,
        );

        Self {
            state,
            renderer,
            frame_started: false,
        }
    }

    /// Borrow the egui Context for drawing UI.
    /// This is a cheap clone of the handle provided by egui_winit::State.
    pub fn context(&self) -> &Context {
        self.state.egui_ctx()
    }

    /// Feed a winit WindowEvent to the egui integration.
    /// Returns `EventResponse` so callers can decide whether egui consumed the event.
    pub fn handle_input(&mut self, window: &Window, event: &WindowEvent) -> EventResponse {
        self.state.on_window_event(window, event)
    }

    /// Start an egui frame. Must be called before `draw`/user UI code runs.
    pub fn begin_frame(&mut self, window: &Window) {
        let raw_input = self.state.take_egui_input(window);
        // begin_pass accepts the raw input and prepares the internal egui context
        self.state.egui_ctx().begin_pass(raw_input);
        self.frame_started = true;
    }

    /// End the egui frame and render the result into `target_view`.
    ///
    /// - `device` / `queue` are used by the renderer to upload textures/buffers.
    /// - `encoder` is used to record the render pass that draws egui geometry.
    /// - `target_view` is the swapchain texture view to draw into.
    /// - `screen_descriptor` provides size/pixels-per-point information.
    pub fn end_frame_and_draw(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        window: &Window,
        target_view: &TextureView,
        screen_descriptor: ScreenDescriptor,
    ) {
        if !self.frame_started {
            // If begin_frame was not called, skip drawing to avoid panics.
            eprintln!("EguiRenderer: end_frame_and_draw called without begin_frame()");
            return;
        }

        // Ensure egui's internal scale matches the computed pixels per point.
        self.state
            .egui_ctx()
            .set_pixels_per_point(screen_descriptor.pixels_per_point);

        // Finish egui frame and collect output (shapes + textures + platform output).
        let full_output = self.state.egui_ctx().end_pass();

        // Send platform output (e.g. clipboard, cursor changes) back to winit through State helper.
        self.state
            .handle_platform_output(window, full_output.platform_output);

        // Tessellate shapes into paint jobs.
        let paint_jobs = self
            .state
            .egui_ctx()
            .tessellate(full_output.shapes, self.state.egui_ctx().pixels_per_point());

        // Upload any new/changed textures used by egui.
        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }

        // Upload vertex/index buffers for all paint jobs.
        self.renderer
            .update_buffers(device, queue, encoder, &paint_jobs, &screen_descriptor);

        // Begin a render pass that draws into the caller-provided target view.
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("egui main pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        // Render the tessellated paint jobs.
        self.renderer.render(
            &mut rpass.forget_lifetime(),
            &paint_jobs,
            &screen_descriptor,
        );

        // Free textures that egui no longer needs.
        for id in &full_output.textures_delta.free {
            self.renderer.free_texture(id);
        }

        self.frame_started = false;
    }
}

/// Simple RenderPass wrapper that calls `EguiRenderer::end_frame_and_draw`.
/// Kept here for convenience so existing PassManager code can still add an egui pass.
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
        // Nothing to prepare here; resources live per-window in WindowState.
    }

    fn execute(&self, ctx: &mut PassContext) {
        let width = ctx.window_state.config.width;
        let height = ctx.window_state.config.height;
        let pixels_per_point = ctx.window.scale_factor() as f32 * ctx.window_state.scale_factor;

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point,
        };

        // The window_state owns the EguiRenderer instance; call into it to finish the frame.
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
