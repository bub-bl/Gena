use egui_wgpu::{ScreenDescriptor, wgpu};
use std::sync::{Arc, Mutex};
use winit::{
    error::ExternalError, event::DeviceEvent, event_loop::ActiveEventLoop, keyboard::KeyCode,
    window::CursorGrabMode,
};

use crate::WindowState;

pub trait Window {
    fn state(&self) -> &Arc<Mutex<WindowState>>;
    fn window(&self) -> &Arc<winit::window::Window>;
    fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &wgpu::TextureView,
        state: &WindowState,
    );
    fn draw(&mut self, ctx: &egui::Context);
    fn is_mouse_captured(&self) -> bool;
    fn device_event(&mut self, _: &ActiveEventLoop, _: winit::event::DeviceId, event: DeviceEvent);

    fn id(&self) -> winit::window::WindowId {
        self.window().id()
    }

    fn scale_factor(&self) -> f64 {
        self.window().scale_factor()
    }

    fn request_redraw(&self) {
        self.window().request_redraw();
    }

    fn is_minimized(&self) -> bool {
        self.window().is_minimized().unwrap_or(false)
    }

    fn is_maximized(&self) -> bool {
        self.window().is_maximized()
    }

    fn is_focused(&self) -> bool {
        self.window().has_focus()
    }

    fn set_cursor_grab(&self, mode: CursorGrabMode) -> Result<(), ExternalError> {
        self.window().set_cursor_grab(mode)
    }

    fn set_cursor_visible(&self, visible: bool) {
        self.window().set_cursor_visible(visible)
    }

    fn set_mouse_capture(&mut self, capture: bool) {
        if capture {
            self.set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| self.set_cursor_grab(CursorGrabMode::Confined))
                .ok();
            self.set_cursor_visible(false);
        } else {
            self.set_cursor_grab(CursorGrabMode::None)
                .or_else(|_| self.set_cursor_grab(CursorGrabMode::Confined))
                .ok();
            self.set_cursor_visible(true);
        }
    }

    fn handle_resized(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            let mut state = self.state().lock().unwrap();
            state.resize_surface(width, height);
        }
    }

    fn handle_redraw(&mut self) {
        let window_arc = Arc::clone(self.window());

        if window_arc.is_minimized().unwrap_or(false) {
            return;
        }

        let state_arc = Arc::clone(self.state());

        let (width, height, scale_factor) = {
            let state = state_arc.lock().unwrap();
            (state.config.width, state.config.height, state.scale_factor)
        };

        let surface_texture = {
            let state = state_arc.lock().unwrap();

            match state.surface.get_current_texture() {
                Ok(tex) => tex,
                Err(wgpu::SurfaceError::Outdated) => return,
                Err(wgpu::SurfaceError::Lost) => {
                    drop(state);
                    let mut state = state_arc.lock().unwrap();
                    state.resize_surface(width, height);
                    return;
                }
                Err(wgpu::SurfaceError::OutOfMemory) => {
                    eprintln!("Surface out of memory!");
                    return;
                }
                Err(e) => {
                    eprintln!("Surface error: {:?}", e);
                    return;
                }
            }
        };

        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let pixels_per_point = window_arc.scale_factor() as f32 * scale_factor;

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point,
        };

        {
            let mut state = state_arc.lock().unwrap();

            let mut encoder =
                state
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Render Encoder"),
                    });

            self.render(&mut encoder, &surface_view, &state);

            let ctx = {
                state.begin_frame(&window_arc);
                state.egui_context()
            };

            self.draw(&ctx);

            state.end_frame_and_draw(&mut encoder, &window_arc, &surface_view, screen_descriptor);
            state.queue.submit(Some(encoder.finish()));
        }

        surface_texture.present();
        window_arc.request_redraw();
    }

    fn on_key_pressed(&mut self, key: KeyCode) {}
    fn on_key_released(&mut self, key: KeyCode) {}
}
