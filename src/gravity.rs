use crate::{ecs::Ecs, physics::Physics, time::Time, transform::Transform};
use bevy_ecs::system::{Query, Res};
use glam::Vec3;

pub const G: f32 = -10.0;

pub fn apply_gravity(mut planets: Query<(&mut Physics, &Transform)>, time: Res<Time>) {
    let mut comps = planets.iter_combinations_mut();

    while let Some([(mut physics_a, transform_a), (mut physics_b, transform_b)]) =
        comps.fetch_next()
    {
        let (physics_a, physics_b) = (&mut *physics_a, &mut *physics_b);
        physics_a.velocity += gravity(transform_a.translation - transform_b.translation)
            * physics_b.mass
            / physics_a.mass
            * time.delta_seconds;
        physics_b.velocity += gravity(transform_b.translation - transform_a.translation)
            * physics_a.mass
            / physics_b.mass
            * time.delta_seconds;
    }
}

pub fn gravity(position: Vec3) -> Vec3 {
    G * position.normalize_or_zero() / position.length_squared().max(1.0) // Prevent extreamly high gravty when things are close
}

pub(crate) async fn init(ecs: &mut Ecs) {
    ecs.update.add_systems(apply_gravity);
}
