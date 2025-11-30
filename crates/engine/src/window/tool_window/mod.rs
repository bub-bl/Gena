//! Minimal, simplified ToolWindow implementation.
//! - Each window owns a `WindowState` (WGPU + egui renderer).
//! - A single egui draw callback can be set; if absent a small default UI is shown.
//! - Keep the API tiny so creating new tool windows is straightforward and maintainable.

use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
};

use egui::Context;
use egui_wgpu::wgpu;
use winit::{event::DeviceEvent, window::Window as WinitWindow};

use crate::{Window, WindowFactory, WindowState};

/// A very small tool window: owns its rendering state and exposes an egui callback.
pub struct ToolWindow {
    window: Arc<WinitWindow>,
    state: Arc<Mutex<WindowState>>,
    /// Optional egui draw callback called every frame with the `egui::Context`.
    draw_callback: Option<Arc<dyn Fn(&Context) + Send + Sync>>,
    mouse_captured: bool,
}

impl ToolWindow {
    const DEFAULT_WIDTH: u32 = 800;
    const DEFAULT_HEIGHT: u32 = 600;

    /// Async constructor which prepares WGPU / egui state for the given winit window.
    pub async fn new(winit_window: winit::window::Window) -> Self {
        // Request an initial size so the surface configuration is sensible.
        let _ = winit_window.request_inner_size(winit::dpi::PhysicalSize::new(
            Self::DEFAULT_WIDTH,
            Self::DEFAULT_HEIGHT,
        ));

        // Create the wgpu instance + surface and initialize WindowState.
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let window = Arc::new(winit_window);
        let surface = instance
            .create_surface(window.clone())
            .expect("create surface");

        let state = WindowState::new(
            &instance,
            surface,
            &window,
            Self::DEFAULT_WIDTH,
            Self::DEFAULT_HEIGHT,
        )
        .await;

        Self {
            window,
            state: Arc::new(Mutex::new(state)),
            draw_callback: None,
            mouse_captured: false,
        }
    }

    /// Set the egui draw callback. Passing `None` clears it.
    pub fn set_draw_callback<F>(&mut self, cb: Option<F>)
    where
        F: Fn(&Context) + Send + Sync + 'static,
    {
        self.draw_callback = cb.map(|f| Arc::new(f) as Arc<dyn Fn(&Context) + Send + Sync>);
    }

    /// Helper to get a clone of the internal WindowState handle.
    pub fn state_handle(&self) -> Arc<Mutex<WindowState>> {
        self.state.clone()
    }

    /// Convenience to toggle mouse capture for this window.
    pub fn set_mouse_capture(&mut self, capture: bool) {
        self.mouse_captured = capture;
        if let Ok(mut s) = self.state.lock() {
            // WindowState knows how to perform cursor grab on the Winit window.
            s.set_mouse_capture(&*self.window, capture);
        }
    }
}

impl Window for ToolWindow {
    fn state(&self) -> &Arc<Mutex<WindowState>> {
        &self.state
    }

    fn window(&self) -> &Arc<winit::window::Window> {
        &self.window
    }

    /// Default render is a no-op. Tools that need GPU work should access `state_handle()`
    /// and perform uploads / render passes before egui draws.
    fn render(
        &mut self,
        _encoder: &mut wgpu::CommandEncoder,
        _surface_view: &wgpu::TextureView,
        _state: &mut WindowState,
    ) {
        // Intentionally empty: keep tool window minimal.
    }

    /// Draw the egui UI. Invoke the user callback if set, otherwise show a tiny default UI.
    fn draw(&mut self, ctx: &egui::Context) {
        if let Some(cb) = &self.draw_callback {
            cb(ctx);
            return;
        }

        // Default minimal UI
        egui::Window::new("Tool")
            .default_open(true)
            .show(ctx, |ui| {
                ui.label("Tool window");
                ui.separator();
                ui.label("No callback provided — use `set_draw_callback` to supply UI.");
            });
    }

    fn is_mouse_captured(&self) -> bool {
        self.mouse_captured
    }

    /// Forward low-level device events to WindowState so it can accumulate mouse delta etc.
    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if let Ok(mut state) = self.state.lock() {
            state.handle_device_event(&event);
        }
    }

    fn on_key_pressed(&mut self, key: winit::keyboard::KeyCode) {
        if let Ok(mut state) = self.state.lock() {
            state.press_key(key);
        }
    }

    fn on_key_released(&mut self, key: winit::keyboard::KeyCode) {
        if let Ok(mut state) = self.state.lock() {
            state.release_key(key);
        }
    }

    fn handle_resized(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        if let Ok(mut state) = self.state.lock() {
            state.resize_surface(width, height);
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
