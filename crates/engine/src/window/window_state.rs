use std::collections::HashSet;

use egui_wgpu::{ScreenDescriptor, wgpu};
use winit::window::Window as WinitWindow;

use crate::EguiRenderer;

/// État lié au rendu / egui pour une fenêtre.
pub struct WindowState {
    // WGPU
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    pub format: wgpu::TextureFormat,
    pub scale_factor: f32,

    // Entrée
    pub pressed_keys: HashSet<winit::keyboard::KeyCode>,

    // Renderer egui encapsulé
    pub egui_renderer: EguiRenderer,
}

impl WindowState {
    /// Crée un nouvel état WGPU + Egui pour la surface passée.
    pub async fn new(
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'static>,
        window: &WinitWindow,
        width: u32,
        height: u32,
    ) -> Self {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        let features = wgpu::Features::empty();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: features,
                required_limits: Default::default(),
                memory_hints: Default::default(),
                trace: Default::default(),
            })
            .await
            .expect("Failed to create device");

        let caps = surface.get_capabilities(&adapter);

        // Choisir un format commun (Bgra sRGB si disponible)
        let preferred = wgpu::TextureFormat::Bgra8UnormSrgb;
        let format = caps
            .formats
            .iter()
            .find(|f| **f == preferred)
            .copied()
            .unwrap_or(caps.formats[0]);

        // Choisir un PresentMode supporté (Mailbox si possible, sinon Fifo)
        let present_mode = if caps.present_modes.contains(&wgpu::PresentMode::Mailbox) {
            wgpu::PresentMode::Mailbox
        } else {
            wgpu::PresentMode::Fifo
        };

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 0,
        };

        surface.configure(&device, &surface_config);

        // Crée le renderer egui (ta wrapper EguiRenderer)
        let egui_renderer = EguiRenderer::new(&device, surface_config.format, None, 1, window);

        Self {
            device,
            queue,
            surface,
            config: surface_config,
            format,
            scale_factor: 1.0,
            pressed_keys: HashSet::new(),
            egui_renderer,
        }
    }

    /// Commence une frame egui (doit être appelé avec un borrow mut sur self).
    pub fn begin_frame(&mut self, window: &WinitWindow) {
        self.egui_renderer.begin_frame(window);
    }

    /// Récupère le Context egui (clone cheap).
    pub fn egui_context(&self) -> egui::Context {
        self.egui_renderer.context().clone()
    }

    /// NOUVELLE VERSION: Termine la frame egui et dessine dans l'encoder fourni.
    /// Cette version accède à device et queue via self au lieu de les prendre en paramètres.
    pub fn end_frame_and_draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        window: &WinitWindow,
        window_surface_view: &wgpu::TextureView,
        screen_descriptor: ScreenDescriptor,
    ) {
        self.egui_renderer.end_frame_and_draw(
            &self.device, // Accès via self
            &self.queue,  // Accès via self
            encoder,
            window,
            window_surface_view,
            screen_descriptor,
        )
    }

    /// Reconfigure la surface après resize.
    pub fn resize_surface(&mut self, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
    }
}
