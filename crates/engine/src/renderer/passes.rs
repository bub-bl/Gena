// use egui_wgpu::wgpu;
// use egui_wgpu::wgpu::{CommandEncoder, Queue, TextureView};

// use crate::Camera;

// pub trait RenderPass {
//     fn render(
//         &self,
//         encoder: &mut wgpu::CommandEncoder,
//         target: &wgpu::TextureView,
//         queue: &wgpu::Queue,
//         camera: &Camera,
//     );
// }

// pub struct OpaquePass;

// impl RenderPass for OpaquePass {
//     fn render(
//         &self,
//         encoder: &mut CommandEncoder,
//         target: &TextureView,
//         camera: &Camera,
//         queue: &Queue,
//     ) {
//     }
// }

use egui_wgpu::wgpu;
use wgpu::{CommandEncoder, Queue, TextureView};

use crate::Camera2D;

/// Contexte fourni à chaque pass lors de l'exécution.
/// Contient des références vers les ressources par-frame (encoder, target, queue, camera).
pub struct PassContext<'a> {
    pub encoder: &'a mut CommandEncoder,
    pub target: &'a TextureView,
    pub queue: &'a Queue,
    pub camera: &'a Camera2D,
}

/// Trait simple et ergonomique pour une passe de rendu.
/// - `prepare` : appelé occasionnellement (par ex. au chargement ou quand le device change)
/// - `execute` : appelé chaque frame ; doit démarrer ses propres render passes si nécessaire.
pub trait RenderPass {
    /// Nom (utile pour debug/logging).
    fn name(&self) -> &str;

    /// Préparer / créer les ressources GPU (pipelines, bind-groups, buffers).
    /// Par défaut : no-op.
    fn prepare(&mut self, _device: &wgpu::Device, _queue: &Queue) {}

    /// Execute the pass for the current frame. `ctx` contains encoder/target/queue/camera.
    /// A pass is free to begin one or more `RenderPass`es via `ctx.encoder.begin_render_pass(...)`.
    fn execute(&self, ctx: &mut PassContext);
}

/// Gestionnaire de passes. Garde les passes dans un vecteur et les exécute dans l'ordre.
pub struct PassManager {
    passes: Vec<Box<dyn RenderPass + Send + Sync>>,
}

impl PassManager {
    pub fn new() -> Self {
        Self { passes: Vec::new() }
    }

    pub fn add<P: RenderPass + Send + Sync + 'static>(&mut self, pass: P) {
        self.passes.push(Box::new(pass));
    }

    pub fn clear(&mut self) {
        self.passes.clear();
    }

    /// Appel de `prepare` pour toutes les passes (par ex. lors de l'initialisation ou après resize).
    pub fn prepare_all(&mut self, device: &wgpu::Device, queue: &Queue) {
        for p in &mut self.passes {
            p.prepare(device, queue);
        }
    }

    /// Execute toutes les passes dans l'ordre. Le caller doit fournir un `PassContext`.
    pub fn execute_all(&self, ctx: &mut PassContext) {
        for p in &self.passes {
            // éventuel logging :
            // log::debug!("Executing pass: {}", p.name());
            p.execute(ctx);
        }
    }
}
