use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use egui::Context;
use egui_wgpu::wgpu;
use winit::{dpi::PhysicalSize, event::DeviceEvent, keyboard::KeyCode, window::CursorGrabMode};

use crate::{PassContext, Window, WindowFactory, WindowState};

/// ToolWindow: a lightweight, reusable window used by the editor tools.
///
/// Characteristics:
/// - Provides an egui `Context` so tools may draw UI.
/// - Owns (`WindowState`) so it has optional access to wgpu `Device`/`Queue`/`Surface`.
/// - Minimal `render` implementation: tools that need GPU may override `draw` to show GPU resources.
/// - Supports a user-settable egui draw callback so callers can dynamically supply UI code.
///
/// Usage:
/// - Create via `ToolWindow::create` (implements `WindowFactory`) which returns an async constructor.
/// - Optionally call `set_draw_callback(...)` to provide a closure that will be invoked each frame
///   with the egui `Context`.
pub struct ToolWindow {
    window: Arc<winit::window::Window>,
    pub state: Arc<Mutex<WindowState>>,
    /// Optional closure invoked during `draw` with the egui context.
    /// Stored inside an Arc<Mutex<...>> so the callback can be updated from other threads
    /// or while the window is held behind a trait object.
    draw_callback: Arc<Mutex<Option<Arc<dyn Fn(&egui::Context) + Send + Sync>>>>,

    // Local input state
    pressed_keys: Vec<KeyCode>,
    mouse_captured: bool,
}

impl ToolWindow {
    const INITIAL_WIDTH: u32 = 800;
    const INITIAL_HEIGHT: u32 = 600;

    /// Asynchronous constructor used by `WindowFactory::create`.
    /// Builds a `WindowState` (which includes wgpu device/queue/surface) and an egui renderer.
    pub async fn new(winit_window: winit::window::Window) -> Self {
        // Request an initial size so surface configuration uses sensible defaults.
        let _ = winit_window
            .request_inner_size(PhysicalSize::new(Self::INITIAL_WIDTH, Self::INITIAL_HEIGHT));

        // Create wgpu instance & surface and initialize WindowState.
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let window = Arc::new(winit_window);
        let surface = instance.create_surface(window.clone()).unwrap();

        let state = WindowState::new(
            &instance,
            surface,
            &window,
            Self::INITIAL_WIDTH,
            Self::INITIAL_HEIGHT,
        )
        .await;

        Self {
            window,
            state: Arc::new(Mutex::new(state)),
            draw_callback: Arc::new(Mutex::new(None)),
            pressed_keys: Vec::new(),
            mouse_captured: false,
        }
    }

    /// Set or replace the egui draw callback. The closure will be called every frame in `draw`.
    pub fn set_draw_callback<F>(&mut self, cb: F)
    where
        F: Fn(&egui::Context) + Send + Sync + 'static,
    {
        let mut guard = self.draw_callback.lock().unwrap();
        *guard = Some(Arc::new(cb));
    }

    /// Clear the draw callback.
    pub fn clear_draw_callback(&mut self) {
        let mut guard = self.draw_callback.lock().unwrap();
        *guard = None;
    }

    /// Convenience: get a clone of the internal state Arc so external systems can inspect device/queue.
    pub fn state_handle(&self) -> Arc<Mutex<WindowState>> {
        self.state.clone()
    }

    /// Returns the underlying winit window id.
    pub fn id(&self) -> winit::window::WindowId {
        self.window.id()
    }

    /// Toggle mouse capture convenience.
    pub fn set_mouse_capture(&mut self, capture: bool) {
        self.mouse_captured = capture;
        if capture {
            self.window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| self.window.set_cursor_grab(CursorGrabMode::Confined))
                .ok();
            self.window.set_cursor_visible(false);
        } else {
            self.window
                .set_cursor_grab(CursorGrabMode::None)
                .or_else(|_| self.window.set_cursor_grab(CursorGrabMode::Confined))
                .ok();
            self.window.set_cursor_visible(true);
        }
    }

    // Hook for tools that want to perform GPU rendering prior to egui drawing.
    // By default this is a no-op. If a tool needs to render textures or other GPU resources,
    // override this behavior by storing state externally and using `state_handle()` to access
    // `device`/`queue` and perform uploads/encode render passes in `render`.
    fn do_render_gpu(
        &mut self,
        _encoder: &mut wgpu::CommandEncoder,
        _surface_view: &wgpu::TextureView,
    ) {
        // Default: nothing to render.
    }
}

impl Window for ToolWindow {
    fn state(&self) -> &Arc<Mutex<WindowState>> {
        &self.state
    }

    fn window(&self) -> &Arc<winit::window::Window> {
        &self.window
    }

    /// Render step for the window. Called before `begin_frame` / egui draw.
    /// Tools that need GPU rendering can use `self.state` to access device/queue and emit draws.
    fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &wgpu::TextureView,
        _state: &mut WindowState,
    ) {
        // Default: no GPU rendering. Expose a small hook for future extensions.
        self.do_render_gpu(encoder, surface_view);
    }

    /// The egui drawing callback. This either invokes the user-specified draw callback
    /// or draws a simple placeholder UI.
    fn draw(&mut self, ctx: &egui::Context) {
        // If a draw callback is set, call it.
        if let Some(cb) = &*self.draw_callback.lock().unwrap() {
            cb(ctx);
            return;
        }

        // Default UI if no callback provided.
        egui::Window::new("Tool")
            .default_open(true)
            .show(ctx, |ui| {
                ui.label("Tool window");
                ui.separator();
                ui.label("No draw callback set. Use `set_draw_callback` to provide UI.");
            });
    }

    fn is_mouse_captured(&self) -> bool {
        self.mouse_captured
    }

    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            // Tools may choose to capture mouse and act on delta; we only store it locally as needed.
            if self.mouse_captured {
                // For a simple tool window we don't accumulate; a tool can access WindowState::take_mouse_delta.
                // If desired, you can extend ToolWindow to accumulate and expose the delta.
                let _ = delta;
            }
        }
    }

    fn on_key_pressed(&mut self, key: KeyCode) {
        if !self.pressed_keys.contains(&key) {
            self.pressed_keys.push(key);
        }
    }

    fn on_key_released(&mut self, key: KeyCode) {
        self.pressed_keys.retain(|k| *k != key);
    }

    fn handle_resized(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            if let Ok(mut s) = self.state.lock() {
                s.resize_surface(width, height);
            }
        }
    }
}

impl WindowFactory for ToolWindow {
    fn create(
        winit_window: winit::window::Window,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Box<dyn std::error::Error>>> + Send>>
    where
        Self: Sized,
    {
        Box::pin(async move {
            let win = ToolWindow::new(winit_window).await;
            Ok(win)
        })
    }
}

impl PartialEq for ToolWindow {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}
