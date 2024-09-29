#![allow(dead_code)]
use bevy_ecs::component::Component;
use glam::{Mat3, Mat4, Quat, Vec2, Vec3, Vec3Swizzles};

use crate::{gpu::Packable, RawInstance};

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
        Self::forward_of_quat(self.rotation)
    }

    pub fn forward_of_quat(rotation: Quat) -> Vec3 {
        Vec3 {
            x: 2.0 * (rotation.x * rotation.z + rotation.w * rotation.y),
            y: 2.0 * (rotation.y * rotation.z - rotation.w * rotation.x),
            z: 1.0 - 2.0 * (rotation.x * rotation.x + rotation.y * rotation.y),
        }
    }

    pub fn up(&self) -> Vec3 {
        Self::up_of_quat(self.rotation)
    }

    pub fn up_of_quat(rotation: Quat) -> Vec3 {
        Vec3 {
            x: 2.0 * (rotation.x * rotation.y - rotation.w * rotation.z),
            y: 1.0 - 2.0 * (rotation.x * rotation.x + rotation.z * rotation.z),
            z: 2.0 * (rotation.y * rotation.z + rotation.w * rotation.x),
        }
    }

    pub fn right(&self) -> Vec3 {
        Self::right_of_quat(self.rotation)
    }

    pub fn right_of_quat(rotation: Quat) -> Vec3 {
        Vec3 {
            x: 1.0 - 2.0 * (rotation.y * rotation.y + rotation.z * rotation.z),
            y: 2.0 * (rotation.x * rotation.y + rotation.w * rotation.z),
            z: 2.0 * (rotation.x * rotation.z - rotation.w * rotation.y),
        }
    }

    pub fn down(&self) -> Vec3 {
        -self.up()
    }

    pub fn down_of_quat(rotation: Quat) -> Vec3 {
        -Self::up_of_quat(rotation)
    }

    pub fn left(&self) -> Vec3 {
        -self.right()
    }

    pub fn left_of_quat(rotation: Quat) -> Vec3 {
        -Self::right_of_quat(rotation)
    }

    pub fn back(&self) -> Vec3 {
        -self.forward()
    }

    pub fn back_of_quat(rotation: Quat) -> Vec3 {
        -Self::forward_of_quat(rotation)
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

impl Packable for &Transform {
    type Raw = RawInstance;

    fn pack(&self, eye: Vec2) -> Self::Raw {
        RawInstance::from_transform(&self.translated(-eye.extend(0.0).xzy()))
    }
}
