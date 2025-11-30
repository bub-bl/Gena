use anyhow::{Context, Result, anyhow};
use std::sync::Arc;

use crate::{Texture2D, Vfs};

/// AssetLoader : responsable de transformer bytes en resources concrètes.
/// Exemple courant : charger une `Texture2D` à partir d'un chemin VFS.
/// `AssetLoader` ne possède pas lui-même les GPU handles : ils sont passés à l'appel.
#[derive(Clone)]
pub struct AssetLoader {
    vfs: Arc<Vfs>,
}

impl AssetLoader {
    pub fn new(vfs: Arc<Vfs>) -> Self {
        AssetLoader { vfs }
    }

    /// Charge les bytes d'un path via le VFS.
    pub fn load_bytes(&self, path: &str) -> Result<Vec<u8>> {
        self.vfs.read_bytes(path)
    }

    /// Charge une texture en résolvant les bytes via le VFS puis en appelant
    /// `Texture2D::from_bytes(device, queue, &bytes)`.
    ///
    /// Note: l'appelant doit fournir `device` et `queue`.
    pub fn load_texture(
        &self,
        path: &str,
        device: &egui_wgpu::wgpu::Device,
        queue: &egui_wgpu::wgpu::Queue,
    ) -> Result<Texture2D> {
        let bytes = self
            .load_bytes(path)
            .with_context(|| format!("failed to load texture bytes for path {}", path))?;
        Texture2D::from_bytes(device, queue, &bytes)
            .map_err(|e| anyhow!(format!("failed to decode image {:?}: {}", path, e)))
    }

    /// Ecrit des bytes via le VFS (dans le premier mount writable).
    pub fn write_bytes(&self, path: &str, data: &[u8]) -> Result<()> {
        self.vfs.write_bytes(path, data)
    }
}
