use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{AssetLoader, Vfs};

/// Engine: structure principale du moteur, contenant le VFS, l'AssetLoader et un cache simple.
///
/// On garde l'impl minimaliste mais pratique: nom de l'app, vfs partagé, loader, et un cache
/// en mémoire (bytes) pour éviter des relectures disques fréquentes.
pub struct Engine {
    pub vfs: Arc<Vfs>,
    pub loader: AssetLoader,
}

impl Default for Engine {
    fn default() -> Self {
        let vfs = Arc::new(Vfs::new());
        // mount a default engine directory (relative). You can remount later.
        // vfs.mount_os("engine", PathBuf::from("engine"), "Engine", false);

        let loader = AssetLoader::new(vfs.clone());
        Engine { vfs, loader }
    }
}

impl Engine {
    pub const NAME: &str = "Gena";

    pub fn init(&mut self) {
        log::info!("Starting engine...");

        self.vfs
            .mount_os("engine", PathBuf::from("engine"), "Engine", false);

        self.vfs
            .mount_os("assets", PathBuf::from("assets"), "Assets", true);

        log::info!("Engine initialization complete.");
    }

    /// Mount an OS directory for the given prefix. `writable` controls whether writes go here.
    pub fn mount_os(
        &self,
        prefix: impl AsRef<Path>,
        root: impl Into<PathBuf>,
        name: impl Into<String>,
        writable: bool,
    ) {
        self.vfs.mount_os(prefix, root, name, writable);
    }

    /// Unmount a prefix.
    pub fn unmount(&self, prefix: impl AsRef<Path>) {
        self.vfs.unmount(prefix);
    }
}
