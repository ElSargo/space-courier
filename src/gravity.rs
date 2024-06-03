use crate::{ecs::Ecs, physics::Physics, planets::Planet, time::Time, transform::Transform};
use bevy_ecs::system::{Query, Res};
use glam::Vec3;

pub const G: f32 = -10.0;

pub fn apply_gravity(mut planets: Query<(&mut Physics, &Transform)>, time: Res<Time>) {
    let mut comps = planets.iter_combinations_mut();

    while let Some([(mut physics_a, transform_a), (mut physics_b, transform_b)]) =
        comps.fetch_next()
    {
        // let (physics_a, physics_b) = (&mut *physics_a, &mut *physics_b);
        let magnitude =
            gravity_magnitude_from_zero(transform_a.translation - transform_b.translation);
        let mass_ratio = physics_a.mass / physics_b.mass;
        physics_a.velocity += magnitude / mass_ratio * time.delta_seconds;
        physics_b.velocity += magnitude * mass_ratio * time.delta_seconds;
    }
}

pub fn gravity_magnitude_from_zero(position: Vec3) -> Vec3 {
    G * position.normalize_or_zero() / position.length_squared().max(1.0) // Prevent extreamly high gravty when things are close
}

pub fn gravity_all<'a>(
    position: Vec3,
    boddies: impl Iterator<Item = (&'a Physics, &'a Transform)>,
) -> Vec3 {
    let mut resultant = Vec3::ZERO;
    for (Physics { mass, .. }, Transform { translation, .. }) in boddies {
        let displacement = position - *translation;
        let magnitude = gravity_magnitude_from_zero(displacement) * *mass;
        resultant += displacement.normalize() * magnitude;
    }
    resultant
}

pub(crate) async fn init(ecs: &mut Ecs) {
    ecs.update.add_systems(apply_gravity);
}
