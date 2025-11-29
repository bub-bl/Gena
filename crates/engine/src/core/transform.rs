use crate::{Mat4, Vec3};

#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::zeros(),
            rotation: Vec3::zeros(),
            scale: Vec3::new(1.0, 1.0, 1.0),
        }
    }
}

impl Transform {
    pub fn matrix(&self) -> Mat4 {
        let translation = Mat4::new_translation(&self.position);
        let rotation_x = Mat4::from_euler_angles(self.rotation.x, 0.0, 0.0);
        let rotation_y = Mat4::from_euler_angles(0.0, self.rotation.y, 0.0);
        let rotation_z = Mat4::from_euler_angles(0.0, 0.0, self.rotation.z);
        let scale = Mat4::new_nonuniform_scaling(&self.scale);

        translation * rotation_y * rotation_x * rotation_z * scale
    }
}
