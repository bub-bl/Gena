use std::collections::HashSet;

use egui_wgpu::{ScreenDescriptor, wgpu};
use winit::event::DeviceEvent;
use winit::keyboard::KeyCode;
use winit::window::{CursorGrabMode, Window as WinitWindow};

use crate::EguiRenderer;

/// État lié au rendu / egui pour une fenêtre.
///
/// Changements principaux :
/// - Centralise la gestion de l'entrée (pressed keys, mouse capture, mouse delta)
///   dans `WindowState` pour éviter de disperser les verrous / accès entre
///   `App`, `Window` et autres.
/// - Fournit des helpers (press/release key, device event handling, take_mouse_delta)
///   pour que la boucle d'événements / `App` n'ait qu'une API simple à appeler.
/// - NOTE: idéalement la signature de `Window::render` devrait accepter `&mut WindowState`
///   afin que le rendu puisse muter l'état d'entrée (par ex. consommer la mouse delta).
///   Dans le code existant on peut soit lock et muter le state depuis l'appelant, soit
///   appeler `take_mouse_delta()` pour récupérer la delta accumulée.
pub struct WindowState {
    // WGPU
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    pub format: wgpu::TextureFormat,
    pub scale_factor: f32,

    // Entrée (centralisée)
    // - touches pressées
    pub pressed_keys: HashSet<KeyCode>,
    // - accumule la delta souris depuis les DeviceEvent::MouseMotion
    mouse_delta: (f32, f32),
    // - indique si la fenêtre a capturé la souris (cursor grab)
    mouse_captured: bool,

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
            mouse_delta: (0.0, 0.0),
            mouse_captured: false,
            egui_renderer,
        }
    }

    // ------------------------
    // Input / events centralisés
    // ------------------------

    /// Traite un `DeviceEvent` bas niveau (par ex. MouseMotion).
    /// - accumulation rapide et non bloquante de la delta souris.
    /// - ne fait pas d'opération lourde, retourne rapidement.
    pub fn handle_device_event(&mut self, event: &DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            if self.mouse_captured {
                self.mouse_delta.0 += delta.0 as f32;
                self.mouse_delta.1 += delta.1 as f32;
            }
        }
    }

    /// Marquer une touche comme pressée.
    pub fn press_key(&mut self, key: KeyCode) {
        self.pressed_keys.insert(key);
    }

    /// Marquer une touche comme relâchée.
    pub fn release_key(&mut self, key: KeyCode) {
        self.pressed_keys.remove(&key);
    }

    /// Interroger si une touche est pressée.
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.pressed_keys.contains(&key)
    }

    /// Récupère la delta souris accumulée depuis le dernier appel et la remet à zéro.
    /// Utilisé par la boucle principale / la scène pour appliquer un seul delta par frame.
    pub fn take_mouse_delta(&mut self) -> (f32, f32) {
        let d = self.mouse_delta;
        self.mouse_delta = (0.0, 0.0);
        d
    }

    /// Demande/release du capture de la souris et mise à jour du flag interne.
    /// On manipule le `WinitWindow` ici parce que c'est une opération fenêtre-spécifique.
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

    /// Interroger si la souris est capturée (considérer utiliser cette info depuis les Windows)
    pub fn is_mouse_captured(&self) -> bool {
        self.mouse_captured
    }

    // ------------------------
    // Egui / rendering helpers (existants)
    // ------------------------

    /// Commence une frame egui (doit être appelé avec un borrow mut sur self).
    pub fn begin_frame(&mut self, window: &WinitWindow) {
        self.egui_renderer.begin_frame(window);
    }

    /// Récupère le Context egui (clone cheap).
    pub fn egui_context(&self) -> egui::Context {
        self.egui_renderer.context().clone()
    }

    /// Termine la frame egui et dessine dans l'encoder fourni.
    /// Cette version accède à device et queue via self.
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

    // Expose queue/device helpers (petite commodité)
    /// Retourne une référence immuable à la queue.
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Retourne une référence immuable au device.
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }
}
