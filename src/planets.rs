use crate::{prelude::*, standard_object};

#[derive(Default, Debug, Component)]
pub struct Planet {
    pub radius: f32,
}

pub fn init_mod(ecs: &mut Initailiser) {
    ecs.init.add_systems(make_planets);
    ecs.update
        .add_systems((remake_planets, add_planet, destroy_planets_in_sun));
    standard_object!(ecs, "wallmart.obj", Planet, With<Planet>)
}

fn remake_planets(
    kb: Res<ButtonInput>,
    planets: Query<Entity, With<Planet>>,
    mut commands: Commands,
) {
    if kb.just_down(KeyCode::KeyR) {
        for e in planets.iter() {
            commands.entity(e).despawn();
        }
        make_planets(commands);
    }
}

fn add_planet(kb: Res<ButtonInput>, mut commands: Commands) {
    if kb.is_down(KeyCode::KeyP) {
        let mut rng = thread_rng();
        make_planet(&mut rng, &mut commands);
    }
}

fn make_planets(mut commands: Commands) {
    let mut rng = thread_rng();
    for _ in 0..40 {
        make_planet(&mut rng, &mut commands);
    }
}

fn destroy_planets_in_sun(
    planets: Query<(Entity, &Transform), With<Planet>>,
    mut commands: Commands,
) {
    for e in planets
        .iter()
        .filter_map(|(entity, transform)| (transform.translation.length() < 20.).then_some(entity))
    {
        commands.entity(e).despawn();
    }
}

fn make_planet(rng: &mut rand::prelude::ThreadRng, commands: &mut Commands) -> Entity {
    let distance = rng.gen_range(60.0..500.0);
    let mut transform = Transform {
        rotation: Quat::from_euler(
            glam::EulerRot::XYZ,
            rng.gen_range(-0.5..0.5),
            rng.gen_range(0.0..std::f32::consts::TAU),
            rng.gen_range(-0.5..0.5),
            // rng.gen_range(-0.3..0.3),
            // rng.gen_range(0.0..std::f32::consts::TAU),
            // rng.gen_range(-0.3..0.3),
        ),
        ..Default::default()
    };
    let forward = transform.forward();
    let radius = rng.gen_range(0.3..0.9_f32).powi(4) * 10.0;
    transform.translation = forward * distance;
    transform.scale *= radius;
    let mass = radius * radius * radius * 10.0;
    let velocity = transform.left() * 0.0005 / transform.translation.length().sqrt()
        * crate::sun::SUN_MASS
        * G
        / mass.sqrt();

    commands
        .spawn((
            Planet {
                radius,
                ..Default::default()
            },
            transform,
            Physics {
                velocity,
                angular_velocity: Quat::from_rotation_y(std::f32::consts::TAU / radius / 1.),
                mass,
            },
        ))
        .id()
}
