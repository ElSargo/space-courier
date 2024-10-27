use arc_swap::ArcSwap;
use bevy_ecs::component::Component;
use std::{marker::PhantomData, num::NonZeroU64, sync::atomic::AtomicU32};
use wgpu::{util::DeviceExt, BufferDescriptor, BufferUsages, Device, Queue};

#[derive(Debug)]
pub struct InstanceBufferDescriptor {
    pub label: Option<String>,
    pub usages: BufferUsages,
}

#[derive(Component)]
pub struct InstanceBuffer {
    descriptor: InstanceBufferDescriptor,
    buffer: ArcSwap<wgpu::Buffer>,
    instances: AtomicU32,
}

#[allow(dead_code)]
impl InstanceBuffer {
    pub fn new<S: ToOwned<Owned = String>>(
        label: Option<S>,
        extra_usages: BufferUsages,
        device: &Device,
    ) -> Self {
        let label = label.map(|s| s.to_owned());
        let usages = extra_usages | BufferUsages::VERTEX | BufferUsages::COPY_DST;
        let buffer = device.create_buffer(&BufferDescriptor {
            label: label.as_ref().map(|x| x.as_str()),
            size: 0,
            usage: usages,
            mapped_at_creation: false,
        });
        Self {
            descriptor: InstanceBufferDescriptor { label, usages },
            buffer: ArcSwap::new(buffer.into()),
            instances: 0.into(),
        }
    }

    pub fn len(&self) -> u32 {
        self.instances.load(std::sync::atomic::Ordering::SeqCst)
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
            descriptor: InstanceBufferDescriptor { label, usages },
            buffer: ArcSwap::new(buffer.into()),
            instances: (instances.len() as u32).into(),
        }
    }

    pub fn write<T: bytemuck::Pod>(&self, device: &Device, instances: &[T], queue: &Queue) {
        let bytes: &[u8] = bytemuck::cast_slice(instances);
        self.instances.store(
            (instances.len() as u32).into(),
            std::sync::atomic::Ordering::SeqCst,
        );

        if let Some(size) = NonZeroU64::new(bytes.len() as u64) {
            self.grow_at_least(size, device);
            queue
                .write_buffer_with(&self.buffer.load(), 0, size)
                .expect("Grow at least should have produced a buffer large enough but didn't")
                .copy_from_slice(bytes);
        }
    }

    /// Grows the buffer to a size that is at least as large as size
    fn grow_at_least(&self, size: NonZeroU64, device: &Device) {
        let size = size.into();
        let curr_buf = self.buffer.load();
        if curr_buf.size() < size {
            self.buffer.store(
                device
                    .create_buffer(&BufferDescriptor {
                        label: self.descriptor.label.as_ref().map(|x| x.as_str()),
                        size,
                        usage: self.descriptor.usages,
                        mapped_at_creation: false,
                    })
                    .into(),
            );
        }
    }

    pub fn write_from_iterator<'a, T: bytemuck::Pod>(
        &self,
        // instances: &[T],
        items: impl Iterator<Item = T>,
        device: &Device,
        queue: &Queue,
    ) {
        let width = std::mem::size_of::<T>();
        let max_items = items
            .size_hint()
            .1
            .expect("Iterator must provide an upper bound so space can be reserved");
        let Some(required_size) = NonZeroU64::new((max_items * width) as u64) else {
            return;
        };

        self.grow_at_least(required_size, device);
        let buffer = self.buffer.load_full();
        let mut write_buffer = queue
            .write_buffer_with(&buffer, 0, required_size)
            .expect("Grow at least should have produced a buffer large enough but didn't");

        let mut i = 0;
        for v in items {
            let start = i * width;
            write_buffer[start..start + width].copy_from_slice(bytemuck::bytes_of(&v));
            i += 1;
        }

        self.instances
            .store((i as u32).into(), std::sync::atomic::Ordering::SeqCst);
    }
}

pub trait RenderPassExt<'a> {
    // render_pass.set_vertex_buffer(1, *ib.buffer_slice());
    fn set_instance_buffer(&mut self, slot: u32, instance_buffer: &InstanceBuffer);
}

impl<'a, 'b> RenderPassExt<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn set_instance_buffer(&mut self, slot: u32, instance_buffer: &InstanceBuffer) {
        let b = instance_buffer.buffer.load();
        self.set_vertex_buffer(slot, b.slice(..));
    }
}

#[macro_export]
macro_rules! instance_buffer {
    ($(@ $label: expr)? $(; $usages: expr)? , $device: expr ) => {
        {
            #[allow(unused_assignments)]
            let mut label = None;
            $(
                label = Some($label.to_owned());
            )?
            let usages =  BufferUsages::empty() $( | $usages )? ;
            crate::instance_buffer::InstanceBuffer::new(label, usages , $device)

        }
    };
}

#[derive(Component)]
pub struct InstanceBufferMarker<T>(pub PhantomData<T>);
