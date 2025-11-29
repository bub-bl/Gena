mod editor_window;
mod engine;

use engine::Engine;

use anyhow::Result;
use winit::event_loop::{ControlFlow, EventLoop};
// use winit::event_loop::{ControlFlow, EventLoop};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Démarrage du moteur..");

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = Engine::new();

    log::info!("Entrée dans la boucle principale...");
    event_loop.run_app(&mut app)?;

    Ok(())
}
