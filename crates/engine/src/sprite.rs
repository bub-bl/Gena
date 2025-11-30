use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use egui_wgpu::wgpu;
use nalgebra::Matrix4;
use wgpu::util::DeviceExt;

use crate::{PassContext, RenderPass, Shader, Texture2D, TextureHandle, Uniforms, Vertex};

/// Per-instance data uploaded to the GPU for instanced draws.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct InstanceData {
    pub model: [[f32; 4]; 4],
}

impl InstanceData {
    pub fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        // A mat4 is 4 vec4 attributes. We expose them as locations 2..5.
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceData>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // model column 0
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // model column 1
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // model column 2
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 4]>() * 2) as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // model column 3
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 4]>() * 3) as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// Sprite descriptor referencing a `Texture2D`.
/// Keeps per-sprite metadata (for now minimal; can be extended: uv rect, tint, pivot, etc.).
#[derive(Clone)]
pub struct Sprite {
    pub texture: Arc<Texture2D>,
    /// UV rectangle in normalized coordinates [u0, v0, u1, v1] referencing the underlying texture.
    /// Defaults to full texture [0,0,1,1].
    pub uv: [f32; 4],
    /// Optional logical size override (if you want sprites to have different logical size than texture)
    pub size: Option<(f32, f32)>,
}

impl Sprite {
    /// Create a sprite that uses the full texture.
    pub fn from_texture(texture: Arc<Texture2D>) -> Self {
        Self {
            texture,
            uv: [0.0, 0.0, 1.0, 1.0],
            size: None,
        }
    }

    /// Convenience: load texture from file and wrap in a Sprite.
    pub fn from_file(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &str,
    ) -> Result<Self, image::ImageError> {
        let tex = Texture2D::from_file(device, queue, path)?;
        Ok(Self::from_texture(Arc::new(tex)))
    }

    /// Convenience: create from bytes and wrap in a Sprite.
    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
    ) -> Result<Self, image::ImageError> {
        let tex = Texture2D::from_bytes(device, queue, bytes)?;
        Ok(Self::from_texture(Arc::new(tex)))
    }

    /// Create the bind group for the underlying texture using the provided layout.
    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        self.texture.create_bind_group(device, bind_group_layout)
    }

    /// Convenience accessor for texture size
    pub fn texture_size(&self) -> (u32, u32) {
        (self.texture.width, self.texture.height)
    }
}

// ============================================================================
// SpriteRenderer (unchanged behavior - still owns pipeline, instance buffer, etc.)
// ============================================================================

pub struct SpriteRenderer {
    pub pipeline: wgpu::RenderPipeline,
    pub texture_bind_layout: wgpu::BindGroupLayout, // @group(1) - texture + sampler
    pub uniform_bind_layout: wgpu::BindGroupLayout, // @group(0) - uniforms
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub quad_vertex: wgpu::Buffer,
    pub quad_index: wgpu::Buffer,

    // Instance buffer for batching
    pub instance_buffer: wgpu::Buffer,
    pub instance_capacity: usize,
}

impl SpriteRenderer {
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        // ========================================================================
        // BIND GROUP 0 : Uniforms (matrice de transformation)
        // ========================================================================
        let uniform_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("uniform_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        // ========================================================================
        // BIND GROUP 1 : Texture + Sampler
        // ========================================================================
        let texture_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        // Shader
        let shader = Shader::from_wgsl(
            device,
            "sprite_shader",
            r"C:\Users\bubbl\Desktop\gena\assets\shader.wgsl",
        );

        // ========================================================================
        // PIPELINE LAYOUT : Déclare les 2 bind groups dans l'ORDRE
        // @group(0) = uniforms, @group(1) = texture
        // ========================================================================
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite_pipeline_layout"),
            bind_group_layouts: &[
                &uniform_bind_layout, // @group(0)
                &texture_bind_layout, // @group(1)
            ],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprite_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader.module(),
                entry_point: Some("vs_main"),
                // include instance attributes as a second buffer
                buffers: &[Vertex::layout(), InstanceData::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader.module(),
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // ========================================================================
        // Créer le buffer d'uniforms et son bind group
        // ========================================================================
        let uniforms = Uniforms {
            model_view_proj: Matrix4::<f32>::identity().into(),
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("uniform_buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("uniform_bind_group"),
            layout: &uniform_bind_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // ========================================================================
        // Quad geometry
        // ========================================================================
        let quad_vertices = Vertex::quad_vertices();
        let quad_indices = Vertex::quad_indices();

        let quad_vertex = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad_vertex"),
            contents: bytemuck::cast_slice(&quad_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let quad_index = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad_index"),
            contents: bytemuck::cast_slice(&quad_indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // ========================================================================
        // Instance buffer (start with a reasonable default capacity)
        // ========================================================================
        let instance_capacity = 1024usize;
        let empty_instances = vec![InstanceData::zeroed(); instance_capacity];
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("instance_buffer"),
            contents: bytemuck::cast_slice(&empty_instances),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            pipeline,
            texture_bind_layout,
            uniform_bind_layout,
            quad_vertex,
            quad_index,
            uniform_buffer,
            uniform_bind_group,
            instance_buffer,
            instance_capacity,
        }
    }

    /// Dessiner des sprites (instanced). `instance_count` indique combien d'instances seront dessinées
    /// à partir de la `instance_buffer` (commençant à 0).
    pub fn draw_instanced<'a>(
        &'a self,
        rpass: &mut wgpu::RenderPass<'a>,
        texture_bind_group: &'a wgpu::BindGroup,
        instance_count: u32,
    ) {
        rpass.set_pipeline(&self.pipeline);
        rpass.set_vertex_buffer(0, self.quad_vertex.slice(..));
        rpass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        rpass.set_index_buffer(self.quad_index.slice(..), wgpu::IndexFormat::Uint16);

        // IMPORTANT : bind les 2 groupes dans l'ordre
        rpass.set_bind_group(0, &self.uniform_bind_group, &[]); // @group(0) = uniforms
        rpass.set_bind_group(1, texture_bind_group, &[]); // @group(1) = texture

        if instance_count == 0 {
            return;
        }

        rpass.draw_indexed(0..6, 0, 0..instance_count);
    }

    /// Mettre à jour la matrice de transformation
    pub fn update_transform(&self, queue: &wgpu::Queue, matrix: Matrix4<f32>) {
        let uniforms = Uniforms {
            model_view_proj: matrix.into(),
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }
}

// ============================================================================
// 4. SPRITE PASS - Une passe concrète qui utilise SpriteRenderer
// ============================================================================

/// Passe de rendu pour afficher des sprites
pub struct SpritePass {
    renderer: SpriteRenderer,
    // now we keep Sprite descriptors together with a precomputed bind group for batching
    sprites: Vec<(Sprite, wgpu::BindGroup)>,
}

impl SpritePass {
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let renderer = SpriteRenderer::new(device, target_format);

        Self {
            renderer,
            sprites: Vec::new(),
        }
    }

    /// Ajouter une sprite à afficher dans cette passe.
    /// The provided `Sprite` references a `Texture2D`; we create a bind group for that texture using
    /// the renderer's `texture_bind_layout` and store the pair for batched rendering.
    pub fn add_sprite(&mut self, sprite: Sprite, device: &wgpu::Device) {
        let bind_group = sprite.create_bind_group(device, &self.renderer.texture_bind_layout);
        self.sprites.push((sprite, bind_group));
    }
}

impl RenderPass for SpritePass {
    fn name(&self) -> &str {
        "sprite_pass"
    }

    fn execute(&self, ctx: &mut PassContext) {
        // Utiliser la matrice view-projection de la caméra 2D
        let view_proj = ctx.camera.view_projection_matrix();
        self.renderer.update_transform(ctx.queue, view_proj);

        // Créer le descripteur de la render pass
        let descriptor = wgpu::RenderPassDescriptor {
            label: Some("sprite_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: ctx.target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Garder ce qui est déjà dessiné
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        };

        // Ouvrir la render pass
        let mut rpass = ctx.encoder.begin_render_pass(&descriptor);

        // Group sprites by bind_group pointer to batch those that share the same texture
        use std::collections::HashMap;

        let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();

        for (i, (_sprite, bind_group)) in self.sprites.iter().enumerate() {
            let key = bind_group as *const _ as usize;
            groups.entry(key).or_default().push(i);
        }

        // For each group, build instance data and draw in a single instanced call
        for (_key, indices) in groups {
            // Build instance data for this group
            let mut instances: Vec<InstanceData> = Vec::with_capacity(indices.len());

            for &i in &indices {
                let (sprite, _bg) = &self.sprites[i];
                // For now, place identity model matrix; you can expand to include position/scale/rotation
                let model = Matrix4::<f32>::identity();
                instances.push(InstanceData {
                    model: model.into(),
                });
            }

            // Ensure capacity: if needed, we would resize the GPU buffer (not implemented auto-resize here)
            if instances.len() > self.renderer.instance_capacity {
                // If we need to support more instances than capacity, we should recreate the buffer.
                // For simplicity, clamp to capacity.
                // In a real implementation, recreate buffer with larger capacity.
                // Log a warning:
                log::warn!(
                    "Instance count {} exceeds buffer capacity {}; clipping.",
                    instances.len(),
                    self.renderer.instance_capacity
                );
            }

            // Upload instance data to the GPU
            let bytes = bytemuck::cast_slice(
                &instances[..std::cmp::min(instances.len(), self.renderer.instance_capacity)],
            );

            ctx.queue
                .write_buffer(&self.renderer.instance_buffer, 0, bytes);

            // Retrieve any bind_group for this group (take first)
            let first_index = indices[0];
            let (_sprite0, bind_group0) = &self.sprites[first_index];

            // Draw instanced for this group's instances
            let instance_count = instances.len().min(self.renderer.instance_capacity) as u32;

            self.renderer
                .draw_instanced(&mut rpass, bind_group0, instance_count);
        }

        // La render pass se termine automatiquement ici
    }
}
