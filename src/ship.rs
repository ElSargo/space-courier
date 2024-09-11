use std::sync::{Arc, Mutex};

use bevy_ecs::{
    component::Component,
    query::With,
    system::{Commands, Query, Res, ResMut, Resource},
};

use crate::{
    camera::Camera,
    ecs::Ecs,
    gpu::{DeviceRes, InstanceBuffer, QueueRes},
    instance_buffer_bytes,
    transform::Transform,
};

#[derive(Resource, Clone)]
pub struct ShipGpuResources {
    pub instance_buffer: Arc<Mutex<InstanceBuffer>>,
}

#[derive(Component)]
pub struct Ship;

pub fn init(ecs: &mut Ecs) {
    ecs.extract.add_systems(update_instance_buffer);
}

pub fn init_ship(DeviceRes(device): Res<DeviceRes>, commands: Commands) {
    let sgr = ShipGpuResources {
        instance_buffer: Arc::new(Mutex::new(InstanceBuffer::new(
            Some("Ship instance Buffer".to_owned()),
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            &device,
        ))),
    };
    commands.insert(sgr.clone());
}

pub fn update_instance_buffer(
    camera: Res<Camera>,
    transforms: Query<&Transform, With<Ship>>,
    DeviceRes(device): Res<DeviceRes>,
    QueueRes(queue): Res<QueueRes>,
    ship_data: ResMut<ShipGpuResources>,
) {
    let ship_bytes = instance_buffer_bytes(transforms.iter(), camera.eye);
    ship_data
        .instance_buffer
        .lock()
        .unwrap()
        .write(&device, &ship_bytes.as_slice(), &queue);
}
