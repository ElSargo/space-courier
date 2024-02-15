use std::mem::size_of;

use glam::{Mat4, Vec4};

use crate::transform::Transform;

pub struct ArrowGizmos {
    pub transforms: Vec<Transform>,
    pub colors: Vec<Vec4>,
}

impl ArrowGizmos {
    pub fn len(&self) -> usize {
        self.transforms.len()
    }

    pub fn to_raw_buffer(&self) -> Vec<RawArrowInstance> {
        self.transforms
            .iter()
            .zip(self.colors.iter())
            .map(|(transform, color)| RawArrowInstance {
                transform: transform.to_mat(),
                color: *color,
            })
            .collect()
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct RawArrowInstance {
    transform: Mat4,
    color: Vec4,
}
unsafe impl bytemuck::Pod for RawArrowInstance {}
unsafe impl bytemuck::Zeroable for RawArrowInstance {}

impl RawArrowInstance {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 5,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 9,
                },
            ],
        }
    }
}
