use glam::{vec3, Mat4, Vec3};
use wgpu::{util::DeviceExt, Queue};

use crate::Transform;
use std::default::Default;

pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
}
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone)]
pub struct CameraUniformData {
    pub view_proj: Mat4,
    pub world_position: Vec3,
}
unsafe impl bytemuck::Pod for CameraUniformData {}
unsafe impl bytemuck::Zeroable for CameraUniformData {}

pub struct CameraUniform {
    pub bind_group: wgpu::BindGroup,
    pub buffer: wgpu::Buffer,
    pub uniform: CameraUniformData,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl CameraUniformData {
    pub fn new() -> Self {
        Self {
            view_proj: Mat4::IDENTITY,
            world_position: Vec3::ZERO,
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
        self.world_position = camera.eye;
    }
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: glam::Mat4 = glam::Mat4::from_cols_array(&[
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
]);

impl Camera {
    pub fn build_view_projection_matrix(&self) -> glam::Mat4 {
        let view = Mat4::look_at_lh(self.eye, self.target, self.up);
        let pr = glam::Mat4::perspective_infinite_reverse_lh(self.fovy, self.aspect, self.znear);
        return OPENGL_TO_WGPU_MATRIX * pr * view;
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            eye: vec3(200.0, 0.0, 0.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            aspect: 1.0,
            fovy: std::f32::consts::PI * 0.25,
            znear: 0.1,
        }
    }
}

impl CameraUniform {
    pub fn update(&mut self, camera: &Camera, queue: &Queue) {
        self.uniform.update_view_proj(camera);
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]))
    }

    pub fn setup(camera: &Camera, device: &wgpu::Device) -> Self {
        let mut camera_uniform = CameraUniformData::new();
        camera_uniform.update_view_proj(camera);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });
        CameraUniform {
            uniform: camera_uniform,
            buffer: camera_buffer,
            bind_group: camera_bind_group,
            bind_group_layout: camera_bind_group_layout,
        }
    }
}
