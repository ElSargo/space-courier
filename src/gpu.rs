use std::sync::Arc;

use bevy_ecs::system::Resource;
use wgpu::{util::DeviceExt, BufferDescriptor, BufferUsages, Device, Queue};

#[derive(Resource, Clone)]
pub struct Gpu {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
}

pub struct InstanceBufferDescriptor {
    pub label: Option<String>,
    pub usages: BufferUsages,
    pub mapped_at_creation: bool,
}

pub struct InstanceBuffer {
    pub descriptor: InstanceBufferDescriptor,
    pub buffer: wgpu::Buffer,
    pub instances: u32,
}

#[allow(dead_code)]
impl InstanceBuffer {
    pub fn new<S: ToOwned<Owned = String>>(
        label: Option<S>,
        usages: BufferUsages,
        device: &Device,
    ) -> Self {
        let label = label.map(|s| s.to_owned());
        let buffer = device.create_buffer(&BufferDescriptor {
            label: label.as_ref().map(|x| x.as_str()),
            size: 0,
            usage: usages,
            mapped_at_creation: false,
        });
        Self {
            descriptor: InstanceBufferDescriptor {
                label,
                usages,
                mapped_at_creation: false,
            },
            buffer,
            instances: 0,
        }
    }

    pub fn new_init<T: bytemuck::Pod>(
        label: Option<String>,
        usages: BufferUsages,
        device: &Device,
        instances: &[T],
    ) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: label.as_ref().map(|x| x.as_str()),
            contents: bytemuck::cast_slice(instances),
            usage: usages,
        });
        Self {
            descriptor: InstanceBufferDescriptor {
                label,
                usages,
                mapped_at_creation: false,
            },
            buffer,
            instances: instances.len() as u32,
        }
    }

    pub fn write<T: bytemuck::Pod>(&mut self, device: &Device, instances: &[T], queue: &Queue) {
        let bytes = bytemuck::cast_slice(instances);
        self.instances = instances.len() as u32;
        if self.buffer.size() >= bytes.len() as u64 {
            queue.write_buffer(&self.buffer, 0, bytes);
        } else {
            self.buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: self.descriptor.label.as_ref().map(|x| x.as_str()),
                contents: bytemuck::cast_slice(instances),
                usage: self.descriptor.usages,
            });
        }
    }
}
