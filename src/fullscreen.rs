use bevy_ecs::system::{Commands, Res, Resource};
use bytemuck::cast_slice;

use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BufferUsages, RenderPass, VertexBufferLayout,
};

use crate::{
    ecs::Initailiser,
    gpu::RD,
    shader_manager::{ShaderHandle, ShaderLoader},
    vertex_layout,
};

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-1.0, -1.0],
    },
    Vertex {
        position: [1.0, -1.0],
    },
    Vertex {
        position: [-1.0, 1.0],
    },
    Vertex {
        position: [1.0, 1.0],
    },
];

const INDICES: &[u16] = &[0, 1, 2, 1, 2, 3];

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Vertex {
    position: [f32; 2],
}

vertex_layout!(Vertex,[0 => Float32x2 ]);

#[derive(Resource)]
pub struct FullscreenQuad {
    pub mesh_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    pub shader_handle: ShaderHandle,
    pub vertex_format: &'static [VertexBufferLayout<'static>; 1],
}

impl FullscreenQuad {
    pub fn init(device: &wgpu::Device, shader_manager: &ShaderLoader) -> Self {
        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("post_buffer_index"),
            contents: cast_slice(INDICES),
            usage: BufferUsages::INDEX,
        });
        let mesh_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("fullscreen_quad_vertex_buffer"),
            contents: cast_slice(&VERTICES),
            usage: BufferUsages::VERTEX,
        });

        let shader_handle = shader_manager.load_shader("./src/fullscreen_quad.wgsl");

        let vertex_format = Box::leak(Box::new([Vertex::desc()]));
        Self {
            mesh_buffer,
            index_buffer,
            vertex_format,
            shader_handle,
        }
    }

    pub fn draw<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        render_pass.set_vertex_buffer(0, self.mesh_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..6, 0, 0..1);
    }
}

pub fn init_fsq(device: RD, shader_manager: Res<ShaderLoader>, mut commands: Commands) {
    let fs = FullscreenQuad::init(&device, shader_manager.as_ref());

    commands.insert_resource(fs);
}

pub fn init_mod(ecs: &mut Initailiser) {
    ecs.init.add_systems(init_fsq);
}
