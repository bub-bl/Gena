//! Simplified WindowState
//! - conserve l'essentiel : wgpu device/queue/surface & configuration
//! - renderer egui encapsulé (EguiRenderer)
//! - helpers d'entrée (touches pressées, mouse delta, capture souris)
//!
//! L'objectif : petite surface d'état claire et facile à maintenir.

use std::collections::HashSet;

use egui_wgpu::{ScreenDescriptor, wgpu};
use winit::event::DeviceEvent;
use winit::keyboard::KeyCode;
use winit::window::{CursorGrabMode, Window as WinitWindow};

use crate::EguiRenderer;

pub struct WindowState {
    // WGPU core
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    pub format: wgpu::TextureFormat,
    /// multiplier additionnel (optionnel) appliqué au scale factor de la fenêtre
    pub scale_factor: f32,

    // Input (minimal)
    pressed_keys: HashSet<KeyCode>,
    mouse_delta: (f32, f32),
    mouse_captured: bool,

    // Egui renderer wrapper (see engine::window::gui::EguiRenderer)
    pub egui_renderer: EguiRenderer,
}

impl WindowState {
    /// Crée un nouvel état WGPU + Egui pour la surface fournie.
    /// Doit être appelé de manière asynchrone.
    pub async fn new(
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'static>,
        window: &WinitWindow,
        width: u32,
        height: u32,
    ) -> Self {
        // Adapter / device / queue
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("Failed to create device");

        let caps = surface.get_capabilities(&adapter);

        // Choisir un format raisonnable (préférence Bgra8 sRGB quand disponible)
        let preferred = wgpu::TextureFormat::Bgra8UnormSrgb;
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| *f == preferred)
            .unwrap_or(caps.formats[0]);

        let present_mode = if caps.present_modes.contains(&wgpu::PresentMode::Mailbox) {
            wgpu::PresentMode::Mailbox
        } else {
            wgpu::PresentMode::Fifo
        };

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 0,
        };

        surface.configure(&device, &config);

        let egui_renderer = EguiRenderer::new(&device, config.format, None, 1, window);

        Self {
            device,
            queue,
            surface,
            config,
            format,
            scale_factor: 1.0,
            pressed_keys: HashSet::new(),
            mouse_delta: (0.0, 0.0),
            mouse_captured: false,
            egui_renderer,
        }
    }

    // ----------------
    // Input helpers
    // ----------------

    /// Handle low-level device events (e.g. MouseMotion). Accumule la delta quand la souris est capturée.
    pub fn handle_device_event(&mut self, event: &DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            if self.mouse_captured {
                self.mouse_delta.0 += delta.0 as f32;
                self.mouse_delta.1 += delta.1 as f32;
            }
        }
    }

    pub fn press_key(&mut self, key: KeyCode) {
        self.pressed_keys.insert(key);
    }

    pub fn release_key(&mut self, key: KeyCode) {
        self.pressed_keys.remove(&key);
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.pressed_keys.contains(&key)
    }

    /// Retourne la delta souris accumulée et la remet à zéro.
    pub fn take_mouse_delta(&mut self) -> (f32, f32) {
        let d = self.mouse_delta;
        self.mouse_delta = (0.0, 0.0);
        d
    }

    /// Toggle capture de la souris (modifie aussi l'état du Winit `Window`).
    pub fn set_mouse_capture(&mut self, window: &WinitWindow, capture: bool) {
        self.mouse_captured = capture;
        if capture {
            window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined))
                .ok();
            window.set_cursor_visible(false);
        } else {
            window
                .set_cursor_grab(CursorGrabMode::None)
                .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined))
                .ok();
            window.set_cursor_visible(true);
        }
    }

    pub fn is_mouse_captured(&self) -> bool {
        self.mouse_captured
    }

    // ----------------
    // Egui / rendering helpers (thin wrappers)
    // ----------------

    /// Commence une frame egui (proxy vers EguiRenderer).
    pub fn begin_frame(&mut self, window: &WinitWindow) {
        self.egui_renderer.begin_frame(window);
    }

    /// Renvoie un clone cheap du Context egui.
    pub fn egui_context(&self) -> egui::Context {
        self.egui_renderer.context().clone()
    }

    /// Termine la frame egui et effectue les opérations GPU nécessaires.
    /// Cette méthode invoque le renderer egui avec les device/queue/encoder fournis.
    pub fn end_frame_and_draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        window: &WinitWindow,
        window_surface_view: &wgpu::TextureView,
        screen_descriptor: ScreenDescriptor,
    ) {
        self.egui_renderer.end_frame_and_draw(
            &self.device,
            &self.queue,
            encoder,
            window,
            window_surface_view,
            screen_descriptor,
        );
    }

    /// Reconfigure la surface après un resize.
    pub fn resize_surface(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }

    // Petites commodités d'accès
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }
}
