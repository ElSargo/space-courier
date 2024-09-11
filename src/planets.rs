use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    system::{Commands, Query, Res, ResMut, Resource},
};
use glam::{Quat, Vec3};
use rand::{thread_rng, Rng};
use std::{
    default::Default,
    sync::{Arc, Mutex},
};
use wgpu::BindGroupLayout;
use winit::keyboard::Key;

use crate::{
    camera::Camera,
    ecs::Ecs,
    gpu::{DeviceRes, InstanceBuffer, QueueRes},
    gravity::G,
    input::ButtonInput,
    instance_buffer_bytes,
    model::Model,
    physics::Physics,
    resources,
    transform::Transform,
};

#[derive(Default, Debug, Component)]
pub struct Planet {
    pub translation: Vec3,
    pub rotation: Quat,
    pub velocity: Vec3,
    pub radius: f32,
    pub mass: f32,
}

pub fn init(
    ecs: &mut Ecs,
    devicedevice Res<DeviceRes>,
    QueueRes(queue): Res<QueueRes>,
    texture_bind_group_layout: &BindGroupLayout,
    ib: Arc<Mutex<InstanceBuffer>>,
) {
    ecs.init.add_systems(make_planets);
    ecs.update.add_systems(remake_planets);
    ecs.update.add_systems(add_planet);
    ecs.update.add_systems(destroy_planets_in_sun);
    ecs.extract.add_systems(update_instance_buffer);

    ecs.world.insert_resource(PlanetGpuData {
        instance_buffer: ib,
        model: resources::load_model("wallmart.obj", &device, &queue, &texture_bind_group_layout)
            .block_on()
            .unwrap(),
    })

    // planets.scales[1] = Vec3::ONE;
}

fn remake_planets(
    kb: Res<ButtonInput>,
    planets: Query<Entity, With<Planet>>,
    mut commands: Commands,
) {
    if kb.just_down(Key::R) {
        for e in planets.iter() {
            commands.entity(e).despawn();
        }
        make_planets(commands);
    }
}

fn add_planet(kb: Res<ButtonInput>, mut commands: Commands) {
    if kb.is_down(Key::L) {
        let mut rng = thread_rng();
        make_planet(&mut rng, &mut commands);
    }
}

fn make_planets(mut commands: Commands) {
    let mut rng = thread_rng();
    for _ in 0..400 {
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

pub fn update_instance_buffer(
    camera: Res<Camera>,
    transforms: Query<&Transform, With<Planet>>,
    planet_data: ResMut<PlanetGpuData>,
    DeviceRes(device): Res<DeviceRes>,
    QueueRes(queue): Res<QueueRes>,
) {
    {
        let planet_instances = instance_buffer_bytes(
            transforms.iter(), // QueryBuilder::<&Transform, With<Planet>>::new(&mut self.ecs.world)
            camera.eye,        //     .build(){
                               //     .iter(&self.ecs.world),
        );
        planet_data
            .instance_buffer
            .lock()
            .expect("Couldn't lock buffer")
            .write(&device, &planet_instances.as_slice(), &queue);
    }
    // let i = transforms.iter();
    // println!("{:#?}", i.size_hint());
    // let bytes = instance_buffer_bytes(i);
    // gpu.queue.write_buffer(
    //     &planet_data.instance_buffer,
    //     0,
    //     bytemuck::cast_slice(&bytes),
    // );
    // println!("Updated buffer")
}
