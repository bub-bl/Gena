//! Abstractions pour le chargement/écriture de resources avec support de "mounts".
//!
//! Objectif principal :
//! - Permettre de monter plusieurs filesystem (read-only ou read-write) sur des préfixes
//!   (ex: "engine://", "game://", "mods/foo/").
//! - Résoudre les chemins en ordre de priorité (dernier monté = priorité la plus haute).
//! - Fournir un `AssetLoader` capable de récupérer des octets via le VFS et de construire
//!   des resources (ex: textures) à partir de ces octets.
//!
//! Design simplifié et extensible :
//! - `FileSystem` est un trait objet (Send + Sync) qui opère sur des chemins relatifs.
//! - `Vfs` gère une liste de `Mount` et résout quel FS doit être utilisé pour un chemin donné.
//! - `AssetLoader` est responsable de l'étape "bytes -> resource" (ex: appelle `Texture2D::from_bytes`).
//!
//! Remarque : la conversion bytes -> GPU resource (Texture2D) nécessite des objets wgpu (device, queue).
//!           `AssetLoader::load_texture` reçoit ces objets et utilise `Texture2D::from_bytes`.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result, anyhow};

use crate::{Texture2D, Vfs};

/// Petit wrapper représentant une resource "raw" (ex: texture) ; utile pour tests ou pour stocker bytes en mémoire.
pub struct RawResource {
    pub path: String,
    pub data: Vec<u8>,
}

impl RawResource {
    pub fn new(path: impl Into<String>, data: Vec<u8>) -> Self {
        RawResource {
            path: path.into(),
            data,
        }
    }
}
