use egui::{TextureId, ahash::HashMap};

use crate::Texture2D;

pub struct GpuResources {
    textures: HashMap<TextureId, Texture2D>,
}

impl GpuResources {
    pub fn new() -> Self {
        Self {
            textures: HashMap::default(),
        }
    }

    pub fn get_texture(&self, id: TextureId) -> Option<&Texture2D> {
        self.textures.get(&id)
    }

    pub fn add_texture(&mut self, id: TextureId, texture: Texture2D) {
        self.textures.insert(id, texture);
    }

    pub fn remove_texture(&mut self, id: TextureId) {
        self.textures.remove(&id);
    }

    pub fn clear(&mut self) {
        self.textures.clear();
    }
}
