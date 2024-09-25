#![allow(dead_code)]

use glam::{Mat4, Vec3};

use crate::transform::Transform;

pub struct GalaticPosition {
    pub translation: Vec3,
}

impl GalaticPosition {
    fn to_matrix_with_transform(self, transform: Transform) -> Mat4 {
        Mat4::from_scale_rotation_translation(transform.scale, transform.rotation, self.translation)
    }

    fn to_matrix(self) -> Mat4 {
        Mat4::from_translation(self.translation)
    }
}
