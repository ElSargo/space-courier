use bytemuck::cast_slice;
use std::mem::size_of;

use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BufferUsages, RenderPass, VertexBufferLayout,
};

use crate::shader_manager::{ShaderHandle, ShaderManager};

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

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x2 ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub struct FullscreenQuad {
    pub mesh_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    pub shader_handle: ShaderHandle,
    pub vertex_format: &'static [VertexBufferLayout<'static>; 1],
}

impl FullscreenQuad {
    pub fn init(device: &wgpu::Device, shader_manager: &mut ShaderManager) -> Self {
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
        // let shader_module = std::boxed::Box::leak(Box::new(
        //     device.create_shader_module(wgpu::include_wgsl!("fullscreen_quad.wgsl")),
        // ));

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

    // pub fn vertex_state<'a>(&'a self) -> VertexState<'a> {
    //     wgpu::VertexState {
    //         module: self.shader_module,
    //         entry_point: "vs_main",       // 1.
    //         buffers: &self.vertex_format, // 2.
    //     }
    // }
}
