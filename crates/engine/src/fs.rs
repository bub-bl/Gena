use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result, anyhow};

/// Trait minimal pour un filesystem (peut être monté dans le VFS).
/// Tous les chemins passés aux méthodes sont relatifs au "root" du filesystem.
pub trait FileSystem: Send + Sync + 'static {
    /// Lis un fichier en tant que texte UTF-8.
    fn read_to_string(&self, path: &Path) -> Result<String>;

    /// Lis un fichier en tant que bytes bruts.
    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>>;

    /// Ecrit des bytes dans un fichier (crée les dossiers parents si nécessaire).
    fn write_bytes(&self, path: &Path, data: &[u8]) -> Result<()>;

    /// Vérifie si un chemin existe dans ce filesystem.
    fn exists(&self, path: &Path) -> bool;

    /// Nom (pour debug).
    fn name(&self) -> &str;
}

/// Implementation basique qui mappe vers le système de fichiers OS.
/// Le `root` définit le répertoire racine de ce filesystem.
pub struct Ofs {
    root: PathBuf,
    name: String,
}

impl Ofs {
    /// Crée un OsFileSystem pointant vers `root`.
    /// Exemple : `Ofs::new("/home/me/game/assets", "game_assets")`
    pub fn new(root: impl Into<PathBuf>, name: impl Into<String>) -> Self {
        Ofs {
            root: root.into(),
            name: name.into(),
        }
    }

    /// Résout un chemin relatif en chemin absolu sur le FS.
    fn resolve_path(&self, rel: &Path) -> PathBuf {
        if rel.is_absolute() {
            rel.to_path_buf()
        } else {
            self.root.join(rel)
        }
    }
}

impl FileSystem for Ofs {
    fn read_to_string(&self, path: &Path) -> Result<String> {
        let abs = self.resolve_path(path);
        std::fs::read_to_string(&abs)
            .with_context(|| format!("Ofs({}) failed to read_to_string {:?}", self.name, abs))
    }

    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>> {
        let abs = self.resolve_path(path);
        std::fs::read(&abs).with_context(|| format!("Ofs({}) failed to read {:?}", self.name, abs))
    }

    fn write_bytes(&self, path: &Path, data: &[u8]) -> Result<()> {
        let abs = self.resolve_path(path);
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!(
                    "Ofs({}) failed to create parent directories for {:?}",
                    self.name, abs
                )
            })?;
        }
        std::fs::write(&abs, data)
            .with_context(|| format!("Ofs({}) failed to write {:?}", self.name, abs))?;
        Ok(())
    }

    fn exists(&self, path: &Path) -> bool {
        let abs = self.resolve_path(path);
        abs.exists()
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Mount point utilisé par le VFS.
struct Mount {
    /// Préfixe de chemin auquel ce mount répond.
    /// Exemple : "assets", "engine", "" (catch-all)
    prefix: PathBuf,
    fs: Arc<dyn FileSystem>,
    writable: bool,
}

impl Mount {
    fn matches(&self, path: &Path) -> bool {
        // On considère qu'un mount "prefix" correspond si `path` commence par `prefix`.
        // Un prefix vide ("") matchera tout.
        if self.prefix.as_os_str().is_empty() {
            return true;
        }
        path.starts_with(&self.prefix)
    }

    /// Retourne le chemin relatif à donner au filesystem : strip_prefix(prefix).
    fn relative_path<'a>(&self, path: &'a Path) -> PathBuf {
        if self.prefix.as_os_str().is_empty() {
            // path may be absolute or relative; we give it as-is
            path.to_path_buf()
        } else {
            path.strip_prefix(&self.prefix)
                .unwrap_or(Path::new(""))
                .to_path_buf()
        }
    }
}

/// Virtual File System (collection de mounts).
/// Priorité : le dernier mount ajouté a la priorité la plus haute.
#[derive(Clone)]
pub struct Vfs {
    mounts: Arc<std::sync::Mutex<Vec<Mount>>>,
}

impl Vfs {
    /// Crée un Vfs vide.
    pub fn new() -> Self {
        Vfs {
            mounts: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    /// Monte un filesystem sur un `prefix` (ex: "assets", "engine", "" pour catch-all).
    /// `prefix` est un chemin relatif (pas de leading slash de convention).
    /// Si `writable == true`, les opérations d'écriture pourront utiliser ce mount.
    pub fn mount(&self, prefix: impl AsRef<Path>, fs: Arc<dyn FileSystem>, writable: bool) {
        let mount = Mount {
            prefix: prefix.as_ref().to_path_buf(),
            fs,
            writable,
        };
        let mut mounts = self.mounts.lock().unwrap();
        mounts.push(mount);
    }

    /// Monte un Ofs facilement (convenience).
    pub fn mount_os(
        &self,
        prefix: impl AsRef<Path>,
        root: impl Into<PathBuf>,
        name: impl Into<String>,
        writable: bool,
    ) {
        let os = Ofs::new(root, name);
        self.mount(prefix, Arc::new(os), writable);
    }

    /// Unmount par prefix (supprime toutes les correspondances exactes).
    pub fn unmount(&self, prefix: impl AsRef<Path>) {
        let mut mounts = self.mounts.lock().unwrap();
        mounts.retain(|m| m.prefix != prefix.as_ref());
    }

    /// Résout le premier mount (ordre priorité) qui matche le chemin passé.
    /// Retourne (fs, relative_path, writable) si trouvé.
    fn resolve_mount_for(&self, path: &Path) -> Option<(Arc<dyn FileSystem>, PathBuf, bool)> {
        let mounts = self.mounts.lock().unwrap();
        for m in mounts.iter().rev() {
            if m.matches(path) {
                let rel = m.relative_path(path);
                return Some((m.fs.clone(), rel, m.writable));
            }
        }
        None
    }

    /// Lit des bytes depuis le VFS.
    /// Le `path` est une chaîne de style "prefix/..." ou ""-prefixed selon vos mounts.
    pub fn read_bytes(&self, path: &str) -> Result<Vec<u8>> {
        let pathp = Path::new(path);
        if let Some((fs, rel, _writable)) = self.resolve_mount_for(pathp) {
            return fs
                .read_bytes(&rel)
                .with_context(|| format!("failed to read bytes from vfs path {:?}", path));
        }
        Err(anyhow!("no mount found for path {:?}", path))
    }

    /// Lis un fichier en tant que string.
    pub fn read_to_string(&self, path: &str) -> Result<String> {
        let pathp = Path::new(path);
        if let Some((fs, rel, _writable)) = self.resolve_mount_for(pathp) {
            return fs
                .read_to_string(&rel)
                .with_context(|| format!("failed to read string from vfs path {:?}", path));
        }
        Err(anyhow!("no mount found for path {:?}", path))
    }

    /// Ecrit des bytes dans le premier mount writable qui matche le chemin.
    pub fn write_bytes(&self, path: &str, data: &[u8]) -> Result<()> {
        let pathp = Path::new(path);
        // Cherche le premier mount (par priorité) qui matche ET est writable.
        let mounts = self.mounts.lock().unwrap();
        for m in mounts.iter().rev() {
            if m.matches(pathp) && m.writable {
                let rel = m.relative_path(pathp);
                return m.fs.write_bytes(&rel, data).with_context(|| {
                    format!(
                        "failed to write bytes to vfs path {:?} (mount {:?})",
                        path, m.prefix
                    )
                });
            }
        }
        Err(anyhow!("no writable mount found for path {:?}", path))
    }

    /// Vérifie si un chemin existe dans le VFS (via le premier mount qui matche).
    pub fn exists(&self, path: &str) -> bool {
        let pathp = Path::new(path);
        if let Some((fs, rel, _)) = self.resolve_mount_for(pathp) {
            return fs.exists(&rel);
        }
        false
    }

    /// Retourne les informations de debug sur les mounts (ordre: basse -> haute priorité).
    pub fn debug_list_mounts(&self) -> Vec<(PathBuf, String, bool)> {
        let mounts = self.mounts.lock().unwrap();
        mounts
            .iter()
            .map(|m| (m.prefix.clone(), m.fs.name().to_string(), m.writable))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::Engine;

    use super::*;
    use std::sync::Arc;

    #[test]
    fn mount_and_read_write_osfs() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("root");
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("hello.txt");
        std::fs::write(&file, "world").unwrap();

        let vfs = Arc::new(Vfs::new());
        vfs.mount_os("game", root.clone(), "game_assets", true);

        assert!(vfs.exists("game/hello.txt"));
        let s = vfs.read_to_string("game/hello.txt").unwrap();
        assert_eq!(s, "world");

        // write to game/foo
        vfs.write_bytes("game/new.txt", b"abc").unwrap();
        let got = std::fs::read_to_string(root.join("new.txt")).unwrap();
        assert_eq!(got, "abc");
    }

    #[test]
    fn mount_priority() {
        // mount A then B; B should win because last mounted
        let dir_a = tempdir().unwrap();
        let dir_b = tempdir().unwrap();
        std::fs::write(dir_a.path().join("x.txt"), "from_a").unwrap();
        std::fs::write(dir_b.path().join("x.txt"), "from_b").unwrap();

        let vfs = Arc::new(Vfs::new());
        vfs.mount_os("common", dir_a.path(), "A", false);
        vfs.mount_os("common", dir_b.path(), "B", false);

        // should read from B (last mounted)
        let s = vfs.read_to_string("common/x.txt").unwrap();
        assert_eq!(s, "from_b");
    }

    #[test]
    fn engine_basic_flow() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("root");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.txt"), "hello").unwrap();

        let engine = Engine::default();
        engine.mount_os("game", root.clone(), "game", true);

        // load via engine (cache cold)
        // let b = engine.load_bytes_cached("game/a.txt").unwrap();
        let b = engine.loader.load_bytes("game/a.txt").unwrap();
        assert_eq!(std::str::from_utf8(&b[..]).unwrap(), "hello");

        // write and then read
        engine.loader.write_bytes("game/b.txt", b"xyz").unwrap();
        assert_eq!(std::fs::read_to_string(root.join("b.txt")).unwrap(), "xyz");
    }
}
