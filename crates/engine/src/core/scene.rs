use crate::Camera2D;
use egui_wgpu::wgpu;
use nalgebra::Vector2;

pub struct Scene {
    pub name: String,
    pub camera: Camera2D,

    // Accumulate raw mouse delta between frames (DeviceEvent)
    mouse_delta: Vector2<f32>,
}

impl Scene {
    pub fn new(name: String, camera: Camera2D) -> Self {
        Self {
            name,
            camera,
            mouse_delta: Vector2::new(0.0, 0.0),
        }
    }

    /// Appelé par le handler d'événements bas niveau (DeviceEvent) :
    /// on accumule la delta souris et on retourne rapidement.
    pub fn accumulate_mouse(&mut self, dx: f32, dy: f32) {
        self.mouse_delta.x += dx;
        self.mouse_delta.y += dy;
    }

    pub fn update(&mut self, delta_time: f32) {
        // self.world.update(delta_time);

        // 2) Appliquer la souris accumulée à la caméra
        if self.mouse_delta.norm() > 0.0 {
            // self.camera
            //     .process_mouse(self.mouse_delta.x, self.mouse_delta.y, delta_time);
            self.mouse_delta = Vector2::new(0.0, 0.0);
        }
    }

    /// Prépare et upload les buffers GPU qui doivent être faits avant d'enregistrer le pass.
    /// Cette étape peut être faite dans le thread principal avant `render`.
    pub fn prepare_gpu(&mut self, queue: &wgpu::Queue) {
        // Ex: upload matrices, instance buffers, vertex buffers dynamiques, textures streaming...
        // self.world.upload_gpu_resources(queue);
        // self.camera.upload_uniforms(queue);
    }

    /// Enregistre les passes de rendu et dessine la scène.
    /// Fournir les ressources dont tu as besoin (encoder, vues, etc.).
    pub fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        surface_view: &wgpu::TextureView,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        // Ex: begin render pass, bind pipelines, draw meshes
        // for renderable in self.world.renderables() { ... }
    }
}
