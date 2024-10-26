#![allow(dead_code)]
use crate::{gpu::Packable, instance_layout};
use bevy_ecs::component::Component;
use glam::{vec3, Mat3, Mat4, Quat, Vec2, Vec3, Vec3Swizzles, Vec4, Vec4Swizzles};

#[derive(Debug, Clone, Component)]
pub struct Transform {
    pub scale: Vec3,
    pub translation: Vec3,
    pub rotation: Quat,
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

    pub fn to_mat_with_origin(&self, origin: Vec3) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation - origin)
    }

    pub fn from_scale_rotation_translation(scale: Vec3, rotation: Quat, translation: Vec3) -> Self {
        Self {
            scale,
            translation,
            rotation,
        }
    }

    pub fn from_mat4(m: &Mat4) -> Self {
        let (scale, rotation, translation) = m.to_scale_rotation_translation();
        Self {
            scale,
            rotation,
            translation,
        }
    }

    pub fn from_position(position: Vec3) -> Self {
        Self {
            translation: position,
            ..Default::default()
        }
    }

    pub fn forward(&self) -> Vec3 {
        self.rotation.forward()
    }

    pub fn up(&self) -> Vec3 {
        self.rotation.up()
    }

    pub fn right(&self) -> Vec3 {
        self.rotation.right()
    }

    pub fn down(&self) -> Vec3 {
        self.rotation.down()
    }

    pub fn left(&self) -> Vec3 {
        self.rotation.left()
    }

    pub fn back(&self) -> Vec3 {
        self.rotation.back()
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

    pub fn lerp(&self, rhs: &Self, t: f32) -> Self {
        Self {
            scale: self.scale.lerp(rhs.scale, t),
            translation: self.translation.lerp(rhs.translation, t),
            rotation: self.rotation.lerp(rhs.rotation, t),
        }
    }

    pub fn lerp_to(&mut self, rhs: &Self, t: f32) {
        self.scale = self.scale.lerp(rhs.scale, t);
        self.translation = self.translation.lerp(rhs.translation, t);
        self.rotation = self.rotation.lerp(rhs.rotation, t);
    }

    pub fn translated(&self, translation: Vec3) -> Self {
        Self {
            translation: self.translation + translation,
            scale: self.scale,
            rotation: self.rotation,
        }
    }
}

impl Packable for RawInstance {
    type Input<'a> = &'a Transform;

    fn pack<'a>(input: Self::Input<'a>, eye: Vec2) -> Option<Self> {
        Some(RawInstance::from_transform(
            &input.translated(-eye.extend(0.0).xzy()),
        ))
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct RawInstance {
    transform: Mat4,
    rotation_only: Mat3,
}
unsafe impl bytemuck::Pod for RawInstance {}
unsafe impl bytemuck::Zeroable for RawInstance {}
instance_layout!(RawInstance, [
     // Model matrix
     5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4,
     // Model rotation matrix
     9 => Float32x3, 10 => Float32x3, 11 => Float32x3,
]);

impl RawInstance {
    pub fn from_transform(t: &Transform) -> Self {
        Self {
            transform: t.to_mat(),
            rotation_only: Mat3::from_quat(t.rotation),
        }
    }
}

pub trait QuatDirections {
    fn forward(self) -> Vec3;

    fn up(self) -> Vec3;

    fn right(self) -> Vec3;

    fn down(self) -> Vec3;

    fn left(self) -> Vec3;

    fn back(self) -> Vec3;
}

// forward = +y, back = -y
// right   = +x, left = -x
// up      = +z, down = -z
impl QuatDirections for Quat {
    fn forward(self) -> Vec3 {
        let v = Vec4::from(self);
        2.0 * vec3(
            v.xw().dot(v.zy()),
            v.y * v.z - v.w * v.x,
            0.5 - v.xy().length_squared(),
        )
    }

    fn up(self) -> Vec3 {
        let v = Vec4::from(self);
        2.0 * vec3(
            v.x * v.y - v.w * v.z,
            0.5 - v.xz().length_squared(),
            v.yw().dot(v.zx()),
        )
    }

    fn right(self) -> Vec3 {
        let v = Vec4::from(self);
        2.0 * vec3(
            0.5 - v.yz().length_squared(),
            v.xw().dot(v.yz()),
            v.x * v.z - v.w * v.y,
        )
    }

    fn down(self) -> Vec3 {
        -self.up()
    }

    fn left(self) -> Vec3 {
        -self.right()
    }

    fn back(self) -> Vec3 {
        -self.forward()
    }
}
