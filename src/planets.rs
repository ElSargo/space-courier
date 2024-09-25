use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    schedule::IntoSystemConfigs,
    system::{Commands, Query, Res, Resource},
};
use glam::{Quat, Vec3};

use rand::{thread_rng, Rng};
use std::{
    default::Default,
    sync::{Arc, Mutex},
};
use wgpu::BufferUsages;
use winit::keyboard::Key;

use crate::{
    background,
    camera::Camera,
    ecs::Initailiser,
    gpu::{
        pack_buffers, InstanceBuffer, QueueResource, SolidRenderable,
        TextureBindGroupLayoutResource, RD,
    },
    gravity::G,
    input::ButtonInput,
    instance_buffer_bytes,
    model::Model,
    model_loader,
    physics::Physics,
    transform::Transform,
    RawInstance, StandardRenderPipeline,
};

#[derive(Default, Debug, Component)]
pub struct Planet {
    pub translation: Vec3,
    pub rotation: Quat,
    pub velocity: Vec3,
    pub radius: f32,
    pub mass: f32,
}

#[derive(Default, Debug, Component)]
struct PlanetStuff;

fn init(
    device: RD,
    queue: Res<QueueResource>,
    texture_bind_group_layout: Res<TextureBindGroupLayoutResource>,
    nor_pipe: Res<StandardRenderPipeline>,
    mut commands: Commands,
) {
    let ib = InstanceBuffer::new(
        Some("Planet Instance buffer".to_owned()),
        BufferUsages::VERTEX | BufferUsages::COPY_DST,
        &device,
    );

    commands.spawn((
        ib,
        PlanetStuff,
        SolidRenderable {
            pipeline_handle: nor_pipe.handle,
            model: model_loader::load_model(
                "wallmart.obj",
                device.0.clone(),
                queue.0.clone(),
                texture_bind_group_layout.0.clone(),
            ),
        },
    ));
}

pub fn init_mod(ecs: &mut Initailiser) {
    ecs.init.add_systems((
        make_planets,
        init.after(background::init_background)
            .after(crate::solid_pass::init_normal_pipeline),
    ));
    ecs.update
        .add_systems((remake_planets, add_planet, destroy_planets_in_sun));
    // ecs.prepare_buffers.add_systems(update_instance_buffer);
    ecs.prepare_buffers
        .add_systems(pack_buffers::<&Transform, RawInstance, With<Planet>, With<PlanetStuff>>);

    // planets.scales[1] = Vec3::ONE;
}

fn remake_planets(
    kb: Res<ButtonInput>,
    planets: Query<Entity, With<Planet>>,
    mut commands: Commands,
) {
    if kb.just_down(Key::Character("R".into())) {
        for e in planets.iter() {
            commands.entity(e).despawn();
        }
        make_planets(commands);
    }
}

fn add_planet(kb: Res<ButtonInput>, mut commands: Commands) {
    if kb.is_down(Key::Character("p".into())) {
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

#[derive(Resource)]
pub struct PlanetGpuData {
    pub instance_buffer: Arc<Mutex<InstanceBuffer>>,
    pub model: Model,
}
