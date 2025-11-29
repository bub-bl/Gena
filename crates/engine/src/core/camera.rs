use crate::{Mat4, Vec2};
use nalgebra::Matrix4;

/// Caméra 2D pure pour le rendu de sprites
pub struct Camera2D {
    /// Position de la caméra dans le monde 2D
    pub position: Vec2,
    /// Zoom (1.0 = normal, 2.0 = zoomed in 2x, 0.5 = zoomed out 2x)
    pub zoom: f32,
    /// Vitesse de déplacement (pixels par seconde)
    pub speed: f32,
    /// Dimensions du viewport en pixels
    pub viewport_width: f32,
    pub viewport_height: f32,
}

impl Camera2D {
    pub fn new(viewport_width: f32, viewport_height: f32) -> Self {
        Self {
            position: Vec2::new(0.0, 0.0),
            zoom: 1.0,
            speed: 500.0,
            viewport_width,
            viewport_height,
        }
    }

    /// Créer une caméra centrée sur une position donnée
    pub fn new_centered(x: f32, y: f32, viewport_width: f32, viewport_height: f32) -> Self {
        Self {
            position: Vec2::new(x, y),
            zoom: 1.0,
            speed: 500.0,
            viewport_width,
            viewport_height,
        }
    }

    /// Déplacer la caméra
    pub fn translate(&mut self, dx: f32, dy: f32) {
        self.position.x += dx;
        self.position.y += dy;
    }

    /// Déplacer la caméra avec deltatime
    pub fn process_movement(&mut self, direction: CameraMovement2D, dt: f32) {
        let velocity = self.speed * dt;
        match direction {
            CameraMovement2D::Up => self.position.y -= velocity,
            CameraMovement2D::Down => self.position.y += velocity,
            CameraMovement2D::Left => self.position.x -= velocity,
            CameraMovement2D::Right => self.position.x += velocity,
        }
    }

    /// Ajuster le zoom
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.max(0.1); // Éviter les zooms négatifs ou nuls
    }

    /// Zoom progressif (pour scroll de souris par exemple)
    pub fn zoom_by(&mut self, delta: f32) {
        self.zoom = (self.zoom + delta).max(0.1);
    }

    /// Mettre à jour les dimensions du viewport (appeler lors du resize)
    pub fn set_viewport_size(&mut self, width: f32, height: f32) {
        self.viewport_width = width;
        self.viewport_height = height;
    }

    /// Aspect ratio du viewport
    pub fn aspect_ratio(&self) -> f32 {
        self.viewport_width / self.viewport_height
    }

    /// Matrice de projection orthographique 2D
    /// Coordonnées écran : (0, 0) = coin supérieur gauche
    pub fn projection_matrix(&self) -> Mat4 {
        // Transformation pour mapper (0, 0) -> (-1, 1) et (width, height) -> (1, -1)
        Matrix4::new(
            2.0 / self.viewport_width,
            0.0,
            0.0,
            -1.0,
            0.0,
            -2.0 / self.viewport_height,
            0.0,
            1.0,
            0.0,
            0.0,
            1.0,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
        )
    }

    /// Matrice de vue (translation de la caméra + zoom)
    pub fn view_matrix(&self) -> Mat4 {
        Matrix4::new(
            self.zoom,
            0.0,
            0.0,
            -self.position.x * self.zoom,
            0.0,
            self.zoom,
            0.0,
            -self.position.y * self.zoom,
            0.0,
            0.0,
            1.0,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
        )
    }

    /// Matrice combinée : projection * view
    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }

    /// Projection centrée : (0, 0) au centre de l'écran
    /// Coordonnées : (-width/2, -height/2) à (width/2, height/2)
    pub fn projection_matrix_centered(&self) -> Mat4 {
        let half_width = self.viewport_width / 2.0;
        let half_height = self.viewport_height / 2.0;

        Matrix4::new(
            2.0 / self.viewport_width,
            0.0,
            0.0,
            0.0,
            0.0,
            -2.0 / self.viewport_height,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
        )
    }

    /// View-projection centrée avec zoom et position de caméra
    pub fn view_projection_matrix_centered(&self) -> Mat4 {
        self.projection_matrix_centered() * self.view_matrix()
    }

    /// Convertir une position écran (pixels) en position monde
    pub fn screen_to_world(&self, screen_x: f32, screen_y: f32) -> Vec2 {
        Vec2::new(
            (screen_x / self.zoom) + self.position.x,
            (screen_y / self.zoom) + self.position.y,
        )
    }

    /// Convertir une position monde en position écran (pixels)
    pub fn world_to_screen(&self, world_x: f32, world_y: f32) -> Vec2 {
        Vec2::new(
            (world_x - self.position.x) * self.zoom,
            (world_y - self.position.y) * self.zoom,
        )
    }
}

pub enum CameraMovement2D {
    Up,
    Down,
    Left,
    Right,
}

// ============================================================================
// Alias pour compatibilité avec l'ancien code
// ============================================================================

/// Alias pour CameraMovement2D (compatibilité)
pub type CameraMovement = CameraMovement2D;
