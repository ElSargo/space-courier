use glam::{Mat3, Mat4, Quat, Vec3};

#[derive(Debug, Clone)]
pub struct Transform {
    pub scale: Vec3,
    pub translation: Vec3,
    pub rotation: Quat,
}

#[derive(Debug, Clone)]
pub struct TransformView<'a> {
    pub scale: &'a Vec3,
    pub translation: &'a Vec3,
    pub rotation: &'a Quat,
}
impl Default for Transform {
    fn default() -> Self {
        Self {
            scale: Vec3::ONE,
            translation: Vec3::ZERO,
            rotation: Quat::default(),
        }
    }
}

impl Transform {
    pub fn to_mat(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    pub fn from_position(position: Vec3) -> Self {
        Self {
            translation: position,
            ..Default::default()
        }
    }

    pub fn forward(&self) -> Vec3 {
        Vec3 {
            x: 2.0 * (self.rotation.x * self.rotation.z + self.rotation.w * self.rotation.y),
            y: 2.0 * (self.rotation.y * self.rotation.z - self.rotation.w * self.rotation.x),
            z: 1.0 - 2.0 * (self.rotation.x * self.rotation.x + self.rotation.y * self.rotation.y),
        }
    }

    pub fn up(&self) -> Vec3 {
        Vec3 {
            x: 2.0 * (self.rotation.x * self.rotation.y - self.rotation.w * self.rotation.z),
            y: 1.0 - 2.0 * (self.rotation.x * self.rotation.x + self.rotation.z * self.rotation.z),
            z: 2.0 * (self.rotation.y * self.rotation.z + self.rotation.w * self.rotation.x),
        }
    }

    pub fn right(&self) -> Vec3 {
        Vec3 {
            x: 1.0 - 2.0 * (self.rotation.y * self.rotation.y + self.rotation.z * self.rotation.z),
            y: 2.0 * (self.rotation.x * self.rotation.y + self.rotation.w * self.rotation.z),
            z: 2.0 * (self.rotation.x * self.rotation.z - self.rotation.w * self.rotation.y),
        }
    }

    pub fn down(&self) -> Vec3 {
        -self.up()
    }

    pub fn left(&self) -> Vec3 {
        -self.right()
    }

    pub fn back(&self) -> Vec3 {
        -self.forward()
    }

    pub fn look_at(&mut self, target: Vec3, up: Vec3) {
        self.look_to(target - self.translation, up);
    }

    pub fn look_to(&mut self, direction: Vec3, up: Vec3) {
        let back = -direction.try_normalize().unwrap_or(Vec3::NEG_Z);
        let up = up.try_normalize().unwrap_or(Vec3::Y);
        let right = up
            .cross(back)
            .try_normalize()
            .unwrap_or_else(|| up.any_orthonormal_vector());
        let up = back.cross(right);
        self.rotation = Quat::from_mat3(&Mat3::from_cols(right, up, back));
    }
}

impl<'a> TransformView<'a> {
    pub fn to_mat(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(*self.scale, *self.rotation, *self.translation)
    }

    pub fn forward(&self) -> Vec3 {
        Vec3 {
            x: 2.0 * (self.rotation.x * self.rotation.z + self.rotation.w * self.rotation.y),
            y: 2.0 * (self.rotation.y * self.rotation.z - self.rotation.w * self.rotation.x),
            z: 1.0 - 2.0 * (self.rotation.x * self.rotation.x + self.rotation.y * self.rotation.y),
        }
    }

    pub fn up(&self) -> Vec3 {
        Vec3 {
            x: 2.0 * (self.rotation.x * self.rotation.y - self.rotation.w * self.rotation.z),
            y: 1.0 - 2.0 * (self.rotation.x * self.rotation.x + self.rotation.z * self.rotation.z),
            z: 2.0 * (self.rotation.y * self.rotation.z + self.rotation.w * self.rotation.x),
        }
    }

    pub fn right(&self) -> Vec3 {
        Vec3 {
            x: 1.0 - 2.0 * (self.rotation.y * self.rotation.y + self.rotation.z * self.rotation.z),
            y: 2.0 * (self.rotation.x * self.rotation.y + self.rotation.w * self.rotation.z),
            z: 2.0 * (self.rotation.x * self.rotation.z - self.rotation.w * self.rotation.y),
        }
    }

    pub fn down(&self) -> Vec3 {
        -self.up()
    }

    pub fn left(&self) -> Vec3 {
        -self.right()
    }

    pub fn back(&self) -> Vec3 {
        -self.forward()
    }

    pub fn look_at(&mut self, target: Vec3, up: Vec3) {
        self.look_to(target - *self.translation, up);
    }

    pub fn look_to(&mut self, direction: Vec3, up: Vec3) {
        let back = -direction.try_normalize().unwrap_or(Vec3::NEG_Z);
        let up = up.try_normalize().unwrap_or(Vec3::Y);
        let right = up
            .cross(back)
            .try_normalize()
            .unwrap_or_else(|| up.any_orthonormal_vector());
        let up = back.cross(right);
        *self.rotation = Quat::from_mat3(&Mat3::from_cols(right, up, back));
    }
}

pub trait AsTransform {
    fn as_transform(&self) -> TransformView;
    fn as_transform_mut(&mut self) -> TransformView;
}
