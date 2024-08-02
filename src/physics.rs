use bevy_ecs::{
    component::Component,
    system::{Query, Res},
};
use glam::{Quat, Vec3};

use crate::{ecs::Ecs, time::Time, transform::Transform};

#[derive(Component, Debug, Default)]
pub struct Physics {
    pub velocity: Vec3,
    pub angular_velocity: Quat,
    pub mass: f32,
}

pub fn apply_velocity(mut boddies: Query<(&mut Transform, &Physics)>, time: Res<Time>) {
    for (
        mut transform,
        Physics {
            velocity,
            angular_velocity,
            ..
        },
    ) in boddies.iter_mut()
    {
        transform.translation += *velocity * time.delta_seconds;
        transform.rotation = transform
            .rotation
            .mul_quat(Quat::IDENTITY.slerp(*angular_velocity, time.delta_seconds.min(1.0)));
    }
}

pub async fn init(ecs: &mut Ecs) {
    ecs.update.add_systems(apply_velocity);
}
