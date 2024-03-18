use bevy_ecs::system::Resource;
use glam::{vec3, Mat4, Vec3, Vec4};
use wgpu::{util::DeviceExt, Queue};

use std::{cell::RefCell, default::Default};

#[derive(Resource)]
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
    view_pos: Vec4,
    view: Mat4,
    view_proj: Mat4,
    inv_proj: Mat4,
    inv_view: Mat4,
}
unsafe impl bytemuck::Pod for CameraUniformData {}
unsafe impl bytemuck::Zeroable for CameraUniformData {}

pub struct CameraUniform {
    pub bind_group: wgpu::BindGroup,
    pub buffer: wgpu::Buffer,
    pub uniform: RefCell<CameraUniformData>,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl CameraUniformData {
    pub fn new() -> Self {
        Self {
            view_proj: Mat4::IDENTITY,
            view_pos: Vec4::ZERO,
            view: Mat4::IDENTITY,
            inv_proj: Mat4::IDENTITY,
            inv_view: Mat4::IDENTITY,
        }
    }

    pub fn update(&mut self, camera: &Camera) {
        let view = camera.view_matrix();
        let proj = camera.projection_matrix();
        let view_proj = Camera::build_view_projection_matrix(view, proj);
        *self = CameraUniformData {
            view_pos: camera.eye.extend(1.0),
            view,
            view_proj,
            inv_proj: proj.inverse(),
            inv_view: view.transpose(),
        }
    }
}

impl Camera {
    pub fn build_view_projection_matrix(view: Mat4, proj: Mat4) -> glam::Mat4 {
        return proj * view;
    }

    pub fn projection_matrix(&self) -> Mat4 {
        glam::Mat4::perspective_infinite_reverse_lh(self.fovy, self.aspect, self.znear)
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_lh(self.eye, self.target, self.up)
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
    pub fn update(&self, camera: &Camera, queue: &Queue) {
        {
            let mut this = self.uniform.borrow_mut();
            this.update(camera);
        };
        queue.write_buffer(
            &self.buffer,
            0,
            bytemuck::cast_slice(&[*self.uniform.borrow()]),
        )
    }

    pub fn setup(camera: &Camera, device: &wgpu::Device) -> Self {
        let camera_uniform = RefCell::new(CameraUniformData::new());
        camera_uniform.borrow_mut().update(camera);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[*camera_uniform.borrow()]),
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
