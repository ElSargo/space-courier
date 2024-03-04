use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{QueryBuilder, With},
    system::{Query, Res, Resource},
};
use glam::{Quat, Vec3};
use rand::{thread_rng, Rng};
use std::default::Default;
use wgpu::{util::DeviceExt, BindGroupLayout};

use crate::{
    ecs::Ecs, gpu::Gpu, gravity::G, instance_buffer_bytes, model::Model, physics::Physics,
    resources, transform::Transform, SUN_MASS,
};

#[derive(Default, Debug, Component)]
pub struct Planet {
    pub translation: Vec3,
    pub rotation: Quat,
    pub velocity: Vec3,
    pub radius: f32,
    pub mass: f32,
}

pub async fn init(ecs: &mut Ecs, gpu: &Gpu, texture_bind_group_layout: &BindGroupLayout) {
    let mut rng = thread_rng();
    for _ in 0..200 {
        make_planet(&mut rng, ecs);
    }
    // ecs.update.add_systems(update_instance_buffer);

    let planet_buffer_bytes = instance_buffer_bytes(
        QueryBuilder::<&Transform, With<Planet>>::new(&mut ecs.world)
            .build()
            .iter(&ecs.world),
    );

    ecs.world.insert_resource(PlanetGpuData {
        instance_buffer: gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&planet_buffer_bytes),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            }),
        model: resources::load_model(
            "wallmart.obj",
            &gpu.device,
            &gpu.queue,
            &texture_bind_group_layout,
        )
        .await
        .unwrap(),
    })

    // planets.scales[1] = Vec3::ONE;
}

fn make_planet(rng: &mut rand::prelude::ThreadRng, ecs: &mut Ecs) -> Entity {
    let distance = rng.gen_range(30.0..300.0);
    let mut transform = Transform {
        rotation: Quat::from_euler(
            glam::EulerRot::XYZ,
            rng.gen_range(-0.3..0.3),
            rng.gen_range(0.0..std::f32::consts::TAU),
            rng.gen_range(-0.3..0.3),
        ),
        ..Default::default()
    };
    let forward = transform.forward();
    let radius = rng.gen_range(0.3..0.9_f32).powi(7) * 10.0;
    transform.translation = forward * distance;
    transform.scale *= radius;
    let mass = radius * radius * radius * 10.0;
    let velocity = transform.left() * 0.003 / transform.translation.length().sqrt() * SUN_MASS * G
        / mass.sqrt();

    ecs.world
        .spawn((
            Planet {
                radius,
                ..Default::default()
            },
            transform,
            Physics {
                velocity,
                angular_velocity: Quat::IDENTITY,
                mass,
            },
        ))
        .id()
}

#[derive(Resource)]
pub struct PlanetGpuData {
    pub instance_buffer: wgpu::Buffer,
    pub model: Model,
}

pub fn update_instance_buffer(
    transforms: Query<&Transform, With<Planet>>,
    gpu: Res<Gpu>,
    planet_data: Res<PlanetGpuData>,
) {
    let i = transforms.iter();
    println!("{:#?}", i.size_hint());
    let bytes = instance_buffer_bytes(i);
    gpu.queue.write_buffer(
        &planet_data.instance_buffer,
        0,
        bytemuck::cast_slice(&bytes),
    );
    println!("Updated buffer")
}
