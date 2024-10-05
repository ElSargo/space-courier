use bevy_ecs::prelude::Res;
use bytemuck::Pod;
use std::any::Any;
use std::ops::{DerefMut, Range};
use std::{
    collections::VecDeque,
    num::NonZeroU64,
    ops::Deref,
    sync::{atomic::AtomicU32, Arc},
};
use wgpu::RenderPass;

use arc_swap::ArcSwap;
use bevy_ecs::{
    component::Component,
    prelude::{Query, Resource},
    query::{QueryData, QueryFilter, WorldQuery},
    system::Commands,
};

use glam::{Vec2, Vec3Swizzles};
use wgpu::{
    util::DeviceExt, BindGroupLayout, BufferDescriptor, BufferUsages, Device, Queue, TextureFormat,
    TextureView,
};

use crate::{
    camera::Camera,
    ecs::Initailiser,
    model::{DrawModel, Model},
    model_loader::ModelHandle,
    shader_manager::PipelineHandle,
};

// #[derive(Resource, Clone)]
// pub struct DeviceResource(pub Arc<wgpu::Device>);

macro_rules! make_resource_wrapper {
    ($t:ty,$l:ident,$s:ident) => {
        #[derive(Resource)]
        pub struct $l(pub $t);
        impl Deref for $l {
            type Target = $t;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
        impl DerefMut for $l {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl Clone for $l {
            fn clone(&self) -> Self {
                Self(self.0.clone())
            }
        }

        /// Shorthand for a wrapper of Res<Inner>
        #[allow(dead_code)]
        pub type $s<'a> = bevy_ecs::prelude::Res<'a, $l>;
    };

    ($t:ty,$l:ident,$s:ident !) => {
        #[derive(Resource)]
        pub struct $l(pub $t);
        impl Deref for $l {
            type Target = $t;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
        impl DerefMut for $l {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        /// Shorthand for a wrapper of Res<Inner>
        #[allow(dead_code)]
        pub type $s<'a> = bevy_ecs::prelude::Res<'a, $l>;
    };
}

make_resource_wrapper!(Arc<Device>, DeviceResource, RD);
make_resource_wrapper!(Arc<wgpu::Queue>, QueueResource, RQ);
make_resource_wrapper!(Arc<BindGroupLayout>, TextureBindGroupLayoutResource, RTBL);
make_resource_wrapper!(Arc<TextureView>, OutputTextureResource, RST);
make_resource_wrapper!(wgpu::BindGroupLayout, ViewTexturesBindGroupLayout, VTBGL !);

#[derive(Debug)]
pub struct InstanceBufferDescriptor {
    pub label: Option<String>,
    pub usages: BufferUsages,
    pub mapped_at_creation: bool,
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
            descriptor: InstanceBufferDescriptor {
                label,
                usages,
                mapped_at_creation: false,
            },
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
            descriptor: InstanceBufferDescriptor {
                label,
                usages,
                mapped_at_creation: false,
            },
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
        items: impl ExactSizeIterator<Item = T>,
        device: &Device,
        queue: &Queue,
    ) {
        let width = std::mem::size_of::<T>();
        let Some(required_size) = NonZeroU64::new((items.len() * width) as u64) else {
            return;
        };

        self.grow_at_least(required_size, device);
        let buffer = self.buffer.load_full();
        let mut write_buffer = queue
            .write_buffer_with(&buffer, 0, required_size)
            .expect("Grow at least should have produced a buffer large enough but didn't");

        self.instances.store(
            (items.len() as u32).into(),
            std::sync::atomic::Ordering::SeqCst,
        );

        for (i, v) in items.enumerate() {
            let start = i * width;
            write_buffer[start..start + width].copy_from_slice(bytemuck::bytes_of(&v));
        }
    }
}

// impl DerefMut for InstanceBuffer {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.0
//     }
// }

pub trait RenderPassExt<'a> {
    // render_pass.set_vertex_buffer(1, *ib.buffer_slice());
    fn set_instance_buffer(&mut self, slot: u32, instance_buffer: &InstanceBuffer);
    fn draw_model_instances(
        &mut self,
        slot: u32,
        instance_buffer: &InstanceBuffer,
        model: &'a Model,
    );
}

impl<'a, 'b> RenderPassExt<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn set_instance_buffer(&mut self, slot: u32, instance_buffer: &InstanceBuffer) {
        let b = instance_buffer.buffer.load();
        self.set_vertex_buffer(slot, b.slice(..));
    }

    fn draw_model_instances(
        &mut self,
        slot: u32,
        instance_buffer: &InstanceBuffer,
        model: &'b Model,
    ) {
        self.set_instance_buffer(slot, instance_buffer);
        self.draw_model_instanced(model, 0..instance_buffer.len());
    }
}

pub trait Packable: Pod {
    type Input<'a>: WorldQuery + QueryData;
    fn pack<'a>(
        intput: <<Self::Input<'a> as QueryData>::ReadOnly as WorldQuery>::Item<'_>,
        eye: Vec2,
    ) -> Self;
}

pub fn pack_buffers<'a, Raw: Packable, InputFilter, InstanceBufferFilter>(
    camera: Res<Camera>,
    inputs: Query<<Raw as Packable>::Input<'a>, InputFilter>,
    instance_buffer: Query<&InstanceBuffer, InstanceBufferFilter>,
    device: RD,
    queue: RQ,
) where
    InstanceBufferFilter: QueryFilter,
    InputFilter: QueryFilter + bevy_ecs::query::ArchetypeFilter,
    <Raw as Packable>::Input<'a>: QueryData,
{
    match instance_buffer.get_single() {
        Ok(ib) => {
            let i = inputs.iter().map(|i| Raw::pack(i, camera.eye.xz()));
            ib.write_from_iterator(i, &device, &queue);
        }
        Err(_e) => {
            // eprintln!("{_e:?}");
        }
    }

    // instance_buffer.single_mut().
}

#[derive(Component)]
pub struct SolidRenderable {
    pub pipeline_handle: PipelineHandle,
    pub renderable: Box<dyn SolidDrawable + Send + Sync>,
}

pub trait SolidDrawable {
    fn draw<'a: 'b, 'b>(
        &'a self,
        render_pass: &mut RenderPass<'b>,
        instances: Range<u32>,
        asset: &'a dyn Any,
    );
    fn aquire(&self) -> Option<Arc<dyn Any>>;
}

impl SolidDrawable for ModelHandle {
    fn draw<'a: 'b, 'b>(
        &'a self,
        render_pass: &mut RenderPass<'b>,
        instances: Range<u32>,
        asset: &'a dyn Any,
    ) {
        if let Some(model) = asset.downcast_ref::<Model>() {
            render_pass.draw_model_instanced(model, instances);
        }
    }
    fn aquire(&self) -> Option<Arc<dyn std::any::Any + 'static>> {
        self.get().map(|model| model as Arc<dyn Any>)
    }
}

// impl SolidDrawable for ModelHandle

#[derive(Resource)]
pub struct ScreenResolution {
    pub width: u32,
    pub height: u32,
}

#[derive(Resource)]
pub struct ScreenFormat {
    pub format: TextureFormat,
}

impl Deref for ScreenFormat {
    type Target = TextureFormat;

    fn deref(&self) -> &Self::Target {
        &self.format
    }
}

pub fn mk_texture_bind_group_layout_resource(device: RD, mut commands: Commands) {
    let texture_bind_group_layout = Arc::new(device.create_bind_group_layout(
        &wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    // This should match the filterable field of the
                    // corresponding Texture entry above.
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        },
    ));

    commands.insert_resource(TextureBindGroupLayoutResource(texture_bind_group_layout));
}

pub fn init_mod(initialiser: &mut Initailiser) {
    initialiser
        .init
        .add_systems(mk_texture_bind_group_layout_resource);
    initialiser.world.init_resource::<RenderCompleteCallBack>();
}

#[derive(Resource, Default)]
pub struct RenderCompleteCallBack {
    pub callbacks: VecDeque<Box<dyn FnOnce() + Send + Sync>>,
}

impl RenderCompleteCallBack {
    pub fn push(&mut self, cb: Box<dyn FnOnce() + Send + Sync>) {
        self.callbacks.push_back(cb);
    }
    pub fn pop(&mut self) -> Option<Box<dyn FnOnce() + Send + Sync>> {
        self.callbacks.pop_front()
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
            crate::gpu::InstanceBuffer::new(label, usages , $device)

        }
    };
}
