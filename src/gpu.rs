use crate::{camera::Camera, ecs::Initailiser, instance_buffer::InstanceBuffer};
use bevy_ecs::prelude::Res;
use bevy_ecs::{
    prelude::{Query, Resource},
    query::{QueryData, QueryFilter, WorldQuery},
    system::Commands,
};
use bytemuck::Pod;
use glam::{Vec2, Vec3Swizzles};
use std::{
    collections::VecDeque,
    ops::{Deref, DerefMut},
    sync::Arc,
};
use wgpu::{BindGroupLayout, Device, TextureFormat, TextureView};

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
macro_rules! vertex_layout {
    ($t: ident, $tok: tt ) => {
        impl $t {
            const ATTRIBS: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array!$tok;

            pub fn desc() -> wgpu::VertexBufferLayout<'static> {
                wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &Self::ATTRIBS,
                }
            }

        }

    };
}

#[macro_export]
macro_rules! instance_layout {
    ($t: ident, $tok: tt ) => {
        impl $t {
            const ATTRIBS: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array!$tok;

            pub fn desc() -> wgpu::VertexBufferLayout<'static> {
                wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &Self::ATTRIBS,
                }
            }

        }

    };
}
