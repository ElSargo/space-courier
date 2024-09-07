use std::sync::{Arc, Mutex};

use anymap::AnyMap;
use bevy_ecs::{
    component::Component,
    query::With,
    system::{Query, Res, ResMut, Resource},
};

use crate::{
    camera::Camera,
    ecs::Ecs,
    get_all,
    gpu::{Gpu, InstanceBuffer},
    instance_buffer_bytes,
    transform::Transform,
};

#[derive(Resource, Clone)]
pub struct ShipGpuResources {
    pub instance_buffer: Arc<Mutex<InstanceBuffer>>,
}

#[derive(Component)]
pub struct Ship;

pub fn init(ecs: &mut Ecs, rr: &mut AnyMap) {
    ecs.extract.add_systems(update_instance_buffer);
    let (gpu,) = get_all!(rr, Gpu).unwrap();
    let sgr = ShipGpuResources {
        instance_buffer: Arc::new(Mutex::new(InstanceBuffer::new(
            Some("Ship instance Buffer".to_owned()),
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            &gpu.device,
        ))),
    };
    rr.insert(sgr.clone());
    ecs.world.insert_resource(sgr);
}

pub fn update_instance_buffer(
    camera: Res<Camera>,
    transforms: Query<&Transform, With<Ship>>,
    gpu: Res<Gpu>,
    ship_data: ResMut<ShipGpuResources>,
) {
    let ship_bytes = instance_buffer_bytes(transforms.iter(), camera.eye);
    ship_data.instance_buffer.lock().unwrap().write(
        &gpu.device,
        &ship_bytes.as_slice(),
        &gpu.queue,
    );
}
