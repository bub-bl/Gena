use egui_wgpu::wgpu;

use crate::Camera2D;

pub trait Renderer2D {
    fn update(&mut self, queue: &wgpu::Queue, camera: &Camera2D, dt: f32);

    fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        camera: &Camera2D,
        queue: &wgpu::Queue,
    );
}
