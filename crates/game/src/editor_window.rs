use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use egui_wgpu::wgpu::{self};
use engine::{
    Camera2D, CameraMovement, DeltaTimer, PassContext, PassManager, Scene, Sprite, SpritePass,
    Window, WindowFactory, WindowState,
};
use nalgebra::Point3;
use winit::{dpi::PhysicalSize, event::DeviceEvent, keyboard::KeyCode, window::CursorGrabMode};

pub struct EditorWindow {
    window: Arc<winit::window::Window>,
    pub scene: Arc<Mutex<Scene>>,
    pub state: Arc<Mutex<WindowState>>,
    pub mouse_captured: bool,
    pub delta_timer: DeltaTimer,
    pressed_keys: HashSet<KeyCode>,
    pass_manager: Arc<Mutex<PassManager>>,

    // NEW: accumulate raw mouse delta here too (optional),
    // mais on peut aussi appeler scene.accumulate_mouse directement depuis device_event.
    pending_mouse_dx: f32,
    pending_mouse_dy: f32,
}

impl EditorWindow {
    const INITIAL_WIDTH: u32 = 1280;
    const INITIAL_HEIGHT: u32 = 720;

    pub async fn new(window: winit::window::Window) -> Self {
        let _ =
            window.request_inner_size(PhysicalSize::new(Self::INITIAL_WIDTH, Self::INITIAL_HEIGHT));

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let window = Arc::new(window);
        let surface = instance.create_surface(window.clone()).unwrap();

        let window_width = window.inner_size().width;
        let window_height = window.inner_size().height;

        let state = WindowState::new(
            &instance,
            surface,
            &window,
            Self::INITIAL_WIDTH,
            Self::INITIAL_HEIGHT,
        )
        .await;

        let device = &state.device;
        let surface_format = state.config.format;
        let queue = &state.queue;

        let camera = Camera2D::new(window_width as f32, window_height as f32);
        let scene = Scene::new(camera);
        let mut pass_manager = PassManager::new();

        let mut sprite_pass = SpritePass::new(&device, surface_format);

        // let test_sprite = Sprite::from_file(
        //     device,
        //     &queue,
        //     r"C:\Users\bubbl\Desktop\gena\assets\sprites\texture.png",
        // )
        // .unwrap();

        let test_sprite = Sprite::from_file(
            device,
            queue,
            r"C:\Users\bubbl\Desktop\gena\assets\sprites\texture.png",
        )
        .unwrap_or_else(|err| {
            eprintln!("Failed to load sprite: {}", err);
            std::process::exit(1);
        });
        sprite_pass.add_sprite(test_sprite, device);

        pass_manager.add(sprite_pass);

        Self {
            window,
            state: Arc::new(Mutex::new(state)),
            scene: Arc::new(Mutex::new(scene)),
            pass_manager: Arc::new(Mutex::new(pass_manager)),
            mouse_captured: false,
            delta_timer: DeltaTimer::new(),
            pressed_keys: HashSet::new(),
            pending_mouse_dx: 0.0,
            pending_mouse_dy: 0.0,
        }
    }

    pub fn id(&self) -> winit::window::WindowId {
        self.window.id()
    }

    // // AJOUT: Méthodes pour gérer les touches pressées
    // pub fn add_pressed_key(&mut self, key: KeyCode) {
    //     self.pressed_keys.insert(key);
    // }

    // pub fn remove_pressed_key(&mut self, key: KeyCode) {
    //     self.pressed_keys.remove(&key);
    // }

    // AJOUT: Traitement continu du mouvement basé sur les touches pressées
    fn process_continuous_movement(&mut self, delta_time: f32) {
        if self.pressed_keys.is_empty() {
            return;
        }

        let mut scene = self.scene.lock().unwrap();

        // Traiter chaque direction pressée
        for key in &self.pressed_keys {
            let direction = match key {
                KeyCode::KeyW => Some(CameraMovement::Up),
                KeyCode::KeyS => Some(CameraMovement::Down),
                KeyCode::KeyA => Some(CameraMovement::Left),
                KeyCode::KeyD => Some(CameraMovement::Right),
                _ => None,
            };

            if let Some(dir) = direction {
                scene.camera.process_movement(dir, delta_time);
            }
        }
    }
}

impl Window for EditorWindow {
    fn state(&self) -> &Arc<Mutex<WindowState>> {
        &self.state
    }

    fn window(&self) -> &Arc<winit::window::Window> {
        &self.window
    }

    fn draw(&mut self, ctx: &egui::Context) {
        egui::Window::new("Editor Window")
            .resizable(true)
            .default_open(true)
            .show(ctx, |ui| {
                if ui.button("Click me").clicked() {
                    println!("Editor UI clicked");
                }
                ui.label("Editor tools...");
            });
    }

    fn is_mouse_captured(&self) -> bool {
        self.mouse_captured
    }

    fn set_mouse_capture(&mut self, capture: bool) {
        self.mouse_captured = capture;

        if capture {
            self.window()
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| self.window().set_cursor_grab(CursorGrabMode::Confined))
                .ok();
            self.window().set_cursor_visible(false);
        } else {
            self.window()
                .set_cursor_grab(CursorGrabMode::None)
                .or_else(|_| self.window().set_cursor_grab(CursorGrabMode::Confined))
                .ok();
            self.window().set_cursor_visible(true);
        }
    }

    fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &wgpu::TextureView,
        window_state: &WindowState,
    ) {
        let delta_time = self.delta_timer.update();

        self.process_continuous_movement(delta_time);

        if self.mouse_captured {
            // soit on a accumulé dans pending_mouse_* localement,
            // soit on appelle directement scene.accumulate_mouse depuis device_event.
            if self.pending_mouse_dx != 0.0 || self.pending_mouse_dy != 0.0 {
                let mut scene = self.scene.lock().unwrap();
                scene.accumulate_mouse(self.pending_mouse_dx, self.pending_mouse_dy);

                self.pending_mouse_dx = 0.0;
                self.pending_mouse_dy = 0.0;
            }
        }

        let mut scene = self.scene.lock().unwrap();
        scene.update(delta_time);

        // 5) Prepare GPU uploads
        scene.prepare_gpu(&window_state.queue);

        scene.render(
            encoder,
            surface_view,
            &window_state.device,
            &window_state.queue,
        );

        let mut pass_ctx = PassContext {
            encoder,
            target: &surface_view,
            queue: &window_state.queue,
            camera: &scene.camera,
        };

        let pass_manager = self.pass_manager.lock().unwrap();
        pass_manager.execute_all(&mut pass_ctx);

        // 7) UI / egui -> handle ensuite
    }

    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event
            && self.mouse_captured
        {
            // Accumulation locale très rapide, on ne doit pas faire d'update lourd ici.
            self.pending_mouse_dx += delta.0 as f32;
            self.pending_mouse_dy += delta.1 as f32;
        }
    }

    fn on_key_pressed(&mut self, key: KeyCode) {
        self.pressed_keys.insert(key);
    }

    fn on_key_released(&mut self, _key: KeyCode) {
        self.pressed_keys.remove(&_key);
    }

    fn handle_resized(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            let mut state = self.state().lock().unwrap();
            state.resize_surface(width, height);

            let mut scene = self.scene.lock().unwrap();
            scene.camera.set_viewport_size(width as f32, height as f32);
        }
    }
}

impl WindowFactory for EditorWindow {
    async fn create(winit_window: winit::window::Window) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized,
    {
        Ok(EditorWindow::new(winit_window).await)
    }
}

impl PartialEq for EditorWindow {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}
