use std::sync::Arc;

use bevy_ecs::system::Resource;

#[derive(Resource, Clone)]
pub struct Gpu {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
}

pub struct InstanceBuffer {
    pub buffer: wgpu::Buffer,
    pub instances: u32,
}
