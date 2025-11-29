use engine::WindowManager;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, ElementState, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::KeyCode,
    window::WindowId,
};

use crate::editor_window::EditorWindow;

#[derive(Default)]
pub struct App {
    window_manager: WindowManager,
}

impl App {
    pub fn new() -> Self {
        Self {
            window_manager: WindowManager::default(),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = pollster::block_on(
            self.window_manager
                .create_window::<EditorWindow>(event_loop),
        )
        .unwrap();

        // if let Ok(mut win) = window.lock() {
        //     win.set_mouse_capture(true);
        // }

        self.window_manager.set_active_window(window);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if let Some(window_arc) = self.window_manager.get_window(window_id)
            && let Ok(mut window) = window_arc.lock()
        {
            // CORRECTION 1: Utiliser window() au lieu de instance()
            let wnd = window.window();

            // CORRECTION 2: Accéder à egui_renderer via state()
            let consumed = {
                let mut state = window.state().lock().unwrap();
                state.egui_renderer.handle_input(wnd, &event).consumed
            };

            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::RedrawRequested => window.handle_redraw(),
                WindowEvent::Resized(new_size) => {
                    window.handle_resized(new_size.width, new_size.height)
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    if !consumed
                        && let winit::keyboard::PhysicalKey::Code(keycode) = event.physical_key
                    {
                        match event.state {
                            ElementState::Pressed => {
                                // CORRECTION 3: Accéder à mouse_captured via le trait Window
                                if window.is_mouse_captured() {
                                    if keycode == KeyCode::Escape {
                                        window.set_mouse_capture(false);
                                    } else {
                                        // CORRECTION 4: Accéder à pressed_keys via state()

                                        window.on_key_pressed(keycode);

                                        let mut state = window.state().lock().unwrap();
                                        state.pressed_keys.insert(keycode);

                                        log::info!("Pressed key: {:?}", keycode);
                                    }
                                }
                            }
                            ElementState::Released => {
                                // CORRECTION 5: Accéder à pressed_keys via state()
                                window.on_key_released(keycode);

                                let mut state = window.state().lock().unwrap();
                                state.pressed_keys.remove(&keycode);
                            }
                        }
                    }
                }
                WindowEvent::MouseInput { state, .. } => {
                    if !consumed && state == ElementState::Pressed {
                        window.set_mouse_capture(true);
                    }
                }
                _ => {}
            }
        }
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        // Pour le futur : gestion des mouvements de souris en mode capture
        // if let DeviceEvent::MouseMotion { delta } = event {
        //     if let Some(active_window) = self.window_manager.get_active_window() {
        //         if let Ok(window) = active_window.lock() {
        //             if window.mouse_captured {
        //                 // Traitement du mouvement de souris
        //                 println!("Mouse delta: {:?}", delta);
        //             }
        //         }
        //     }
        // }

        if let Some(active_window) = self.window_manager.get_active_window() {
            let mut window = active_window.lock().unwrap();
            window.device_event(event_loop, device_id, event);
        }
    }
}
