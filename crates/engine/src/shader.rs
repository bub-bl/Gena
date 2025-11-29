use egui_wgpu::wgpu;

pub struct Shader {
    shader: wgpu::ShaderModule,
}

impl Shader {
    pub fn from_wgsl(device: &wgpu::Device, label: &str, path: &str) -> Self {
        let shader_source = std::fs::read_to_string(path).unwrap();

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        Self { shader }
    }

    pub fn module(&self) -> &wgpu::ShaderModule {
        &self.shader
    }
}
