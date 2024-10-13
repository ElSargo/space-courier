use bevy_ecs::{
    schedule::IntoSystemConfigs,
    system::{Commands, Res, ResMut, Resource},
};
use glam::{vec3, Mat4, Vec3, Vec4};
use wgpu::{util::DeviceExt, Device};

use std::default::Default;

use crate::{
    ecs::Initailiser,
    gpu::{ScreenResolution, ViewTexturesBindGroupLayout, RD, RQ, VTBGL},
    make_view_texture, make_view_textures_bind_group, texture, ReflectedViewTextures, ViewTextures,
};

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
#[derive(Debug, Copy, Clone)]
pub struct CameraUniformData {
    view_pos: Vec4,
    view: Mat4,
    proj: Mat4,
    view_proj: Mat4,
    inv_proj: Mat4,
    inv_view: Mat4,
}
unsafe impl bytemuck::Pod for CameraUniformData {}
unsafe impl bytemuck::Zeroable for CameraUniformData {}

#[derive(Resource)]
pub struct CameraUniform {
    pub bind_group: wgpu::BindGroup,
    pub buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

#[derive(Resource)]
pub struct ReflectedCameraUniform {
    pub bind_group: wgpu::BindGroup,
    pub buffer: wgpu::Buffer,
}

impl ReflectedCameraUniform {
    fn setup(camera: &Camera, uni: &CameraUniform, device: &Device) -> Self {
        let uniform_data = camera.into_reflected_uniform_data();

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Refelected camera Buffer"),
            contents: bytemuck::cast_slice(&[uniform_data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uni.bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("reflected_camera_bind_group"),
        });

        Self { bind_group, buffer }
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
        Mat4::look_at_lh(
            Vec3::Y * self.eye,
            self.target - self.eye * vec3(1.0, 0.0, 1.0),
            self.up,
        )
    }

    pub fn into_uniform_data(&self) -> CameraUniformData {
        let view = self.view_matrix();
        let proj = self.projection_matrix();
        let view_proj = Self::build_view_projection_matrix(view, proj);
        CameraUniformData {
            view_pos: self.eye.extend(1.0),
            view,
            proj,
            view_proj,
            inv_proj: proj.inverse(),
            inv_view: view.transpose(),
        }
    }

    pub fn into_reflected_uniform_data(&self) -> CameraUniformData {
        let refy = Vec3::X + Vec3::NEG_Y + Vec3::Z;

        let r = Self {
            eye: self.eye * refy,
            target: self.target * refy,
            up: self.up * refy,
            ..*self
        };

        r.into_uniform_data()
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

pub fn update_camera(
    uniform: Res<CameraUniform>,
    reflected_uniform: Res<ReflectedCameraUniform>,
    camera: Res<Camera>,
    queue: RQ,
) {
    queue.write_buffer(
        &uniform.buffer,
        0,
        bytemuck::cast_slice(&[camera.into_uniform_data()]),
    );
    queue.write_buffer(
        &reflected_uniform.buffer,
        0,
        bytemuck::cast_slice(&[camera.into_reflected_uniform_data()]),
    );
}

impl CameraUniform {
    pub fn setup(camera: &Camera, device: &wgpu::Device) -> Self {
        // let camera_uniform = CameraUniformData::new();
        // camera_uniform.lock().unwrap().update(camera);
        let uniform_data = camera.into_uniform_data();

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[uniform_data]),
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
            buffer: camera_buffer,
            bind_group: camera_bind_group,
            bind_group_layout: camera_bind_group_layout,
        }
    }
}

pub fn insert_camera(resolution: Res<ScreenResolution>, device: RD, mut commands: Commands) {
    let camera = Camera {
        aspect: resolution.width as f32 / resolution.height as f32,
        eye: vec3(0.0, 0.0, -200.0),
        target: Vec3::ZERO,
        ..Default::default()
    };

    let camera_uniform = CameraUniform::setup(&camera, &device);
    let ref_cam_uni = ReflectedCameraUniform::setup(&camera, &camera_uniform, &device);

    commands.insert_resource(camera_uniform);
    commands.insert_resource(ref_cam_uni);
    commands.insert_resource(camera);
}

pub fn init_mod(ecs: &mut Initailiser) {
    ecs.init.add_systems(insert_camera);

    ecs.prepare_buffers.add_systems(update_camera);
    ecs.init.add_systems(init_view_textures);
    ecs.resize
        .add_systems(((resize_camera, resize_view_textures), update_camera).chain());
}

fn resize_camera(mut camera: ResMut<Camera>, resolution: Res<ScreenResolution>) {
    camera.aspect = resolution.width as f32 / resolution.height as f32;
}

pub fn resize_view_textures(
    mut v_tex: ResMut<ViewTextures>,
    mut rv_tex: ResMut<ReflectedViewTextures>,
    vtbgl: VTBGL,
    device: RD,
    reslution: Res<ScreenResolution>,
) {
    v_tex.depth_texture =
        texture::Texture::create_depth_texture(&device, &reslution, "depth_texture");
    v_tex.color_texture = make_view_texture(&reslution, &device);
    v_tex.bind_group =
        make_view_textures_bind_group(&device, &vtbgl, &v_tex.depth_texture, &v_tex.color_texture);

    rv_tex.depth_texture =
        texture::Texture::create_depth_texture(&device, &reslution, "reflected_depth_texture");
    rv_tex.color_texture = make_view_texture(&reslution, &device);
    rv_tex.bind_group = make_view_textures_bind_group(
        &device,
        &vtbgl,
        &rv_tex.depth_texture,
        &rv_tex.color_texture,
    );
}

pub fn init_view_textures(device: RD, mut commands: Commands, resolution: Res<ScreenResolution>) {
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Depth,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                // This should match the filterable field of the
                // corresponding Texture entry above.
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
        label: Some("post_texture_bind_group_layout"),
    });

    {
        let color_texture = make_view_texture(&resolution, &device);
        let depth_texture =
            texture::Texture::create_depth_texture(&device, &resolution, "depth_texture");

        commands.insert_resource(ViewTextures {
            bind_group: make_view_textures_bind_group(
                &device,
                &bind_group_layout,
                &depth_texture,
                &color_texture,
            ),
            depth_texture,
            color_texture,
        });
    }
    {
        let color_texture = make_view_texture(&resolution, &device);
        let null_texture = make_view_texture(&resolution, &device);
        let depth_texture =
            texture::Texture::create_depth_texture(&device, &resolution, "reflected_depth_texture");
        commands.insert_resource(ReflectedViewTextures {
            bind_group: make_view_textures_bind_group(
                &device,
                &bind_group_layout,
                &depth_texture,
                &color_texture,
            ),
            null_bind_group: make_view_textures_bind_group(
                &device,
                &bind_group_layout,
                &depth_texture,
                &null_texture,
            ),
            _null_texture: null_texture,
            depth_texture,
            color_texture,
        });
    }

    commands.insert_resource(ViewTexturesBindGroupLayout(bind_group_layout));
}
