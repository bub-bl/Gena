use engine::{Engine, WindowManager};
use std::{
    any::Any,
    cell::{Ref, RefCell, RefMut},
    collections::HashMap,
    rc::Rc,
};
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::KeyCode,
    window::WindowId,
};

use crate::editor_window::EditorWindow;

// -----------------
// Engine
// -----------------

/// Engine central : orchestre WindowManager et subsystèmes de base.
///
/// Stocke chaque subsystème dans un `Rc<RefCell<Box<dyn Subsystem>>>` afin que
/// d'autres subsystèmes puissent obtenir des références (immuables ou mutables)
/// à runtime via `EngineHandle`.
pub struct App {
    engine: Engine,
    window_manager: WindowManager,
}

impl Default for App {
    fn default() -> Self {
        let app = Self {
            engine: Engine::default(),
            window_manager: WindowManager::default(),
        };

        app
    }
}

use anyhow::Result;

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init(&mut self) -> Result<()> {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

        self.engine.init();

        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(self)?;

        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Crée la fenêtre principale / editor window.
        let window = pollster::block_on(
            self.window_manager
                .create_window::<EditorWindow>(event_loop),
        )
        .unwrap();

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
            let wnd = window.window();

            let consumed = {
                let mut state = window.state().lock().unwrap();
                state.egui_renderer.handle_input(wnd, &event).consumed
            };

            match event {
                WindowEvent::CloseRequested => {
                    // shutdown propre des subsystèmes
                    event_loop.exit();
                }
                WindowEvent::RedrawRequested => {
                    // Mettre à jour les subsystèmes avant redraw
                    window.handle_redraw();
                }
                WindowEvent::Resized(new_size) => {
                    window.handle_resized(new_size.width, new_size.height);
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    if !consumed
                        && let winit::keyboard::PhysicalKey::Code(keycode) = event.physical_key
                    {
                        match event.state {
                            ElementState::Pressed => {
                                if window.is_mouse_captured() {
                                    if keycode == KeyCode::Escape {
                                        window.set_mouse_capture(false);
                                    } else {
                                        window.on_key_pressed(keycode);

                                        let mut state = window.state().lock().unwrap();
                                        state.press_key(keycode);

                                        log::info!("Pressed key: {:?}", keycode);
                                    }
                                }
                            }
                            ElementState::Released => {
                                window.on_key_released(keycode);

                                let mut state = window.state().lock().unwrap();
                                state.release_key(keycode);
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
        _event_loop: &ActiveEventLoop,
        device_id: winit::event::DeviceId,
        event: DeviceEvent,
    ) {
        if let Some(active_window) = self.window_manager.get_active_window() {
            let mut window = active_window.lock().unwrap();
            window.device_event(_event_loop, device_id, event.clone());
        }
    }
}
