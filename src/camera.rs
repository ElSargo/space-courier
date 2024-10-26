use bevy_ecs::{
    schedule::IntoSystemConfigs,
    system::{Commands, Res, ResMut, Resource},
};
use glam::{vec3, Mat4, Quat, Vec3, Vec4};
use wgpu::{util::DeviceExt, BindGroup, BindGroupLayout, Device, TextureFormat};

use std::default::Default;

use crate::{
    ecs::Initailiser,
    gpu::{ScreenResolution, ViewTexturesBindGroupLayout, RD, RQ, VTBGL},
    texture,
    transform::QuatDirections,
    Texture,
};

#[derive(Resource)]
pub struct Camera {
    pub eye: Vec3,
    pub orientation: Quat,
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
    pub fn projection_matrix(&self) -> Mat4 {
        glam::Mat4::perspective_infinite_reverse_lh(self.fovy, self.aspect, self.znear)
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_to_lh(
            Vec3::Y * self.eye,
            self.orientation.forward(),
            self.orientation.up(),
        )
    }

    pub fn into_uniform_data(&self) -> CameraUniformData {
        let view = self.view_matrix();
        let proj = self.projection_matrix();
        let view_proj = proj * view;
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
        let eye = self.eye * refy;
        let view = Mat4::look_to_lh(
            self.eye * refy * Vec3::Y,
            self.orientation.forward() * refy,
            self.orientation.up() * refy,
        );

        let proj = self.projection_matrix();
        let view_proj = proj * view;
        CameraUniformData {
            view_pos: eye.extend(1.0),
            view,
            proj,
            view_proj,
            inv_proj: proj.inverse(),
            inv_view: view.transpose(),
        }
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            eye: vec3(0.0, 0.0, 0.0),
            orientation: Quat::IDENTITY,
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

fn make_view_texture(resolution: &ScreenResolution, device: &wgpu::Device) -> Texture {
    let size = wgpu::Extent3d {
        // 2.
        width: resolution.width,
        height: resolution.height,
        depth_or_array_layers: 1,
    };
    let desc = wgpu::TextureDescriptor {
        label: Some("View tex"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT // 3.
        | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    };
    let texture = device.create_texture(&desc);
    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some("View texture descriptor"),
        format: Some(TextureFormat::Rgba16Float),
        ..Default::default()
    });
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        // 4.
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    Texture { view, sampler }
}

fn make_view_textures_bind_group(
    device: &wgpu::Device,
    depth_texture_bind_group_layout: &BindGroupLayout,
    depth_texture: &Texture,
    color_texture: &Texture,
) -> BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &depth_texture_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&color_texture.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&color_texture.sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&depth_texture.view),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::Sampler(&depth_texture.sampler),
            },
        ],
        label: Some("view_bind_group"),
    })
}

#[derive(Resource)]
pub struct ViewTextures {
    pub color_texture: Texture,
    pub depth_texture: Texture,
    pub bind_group: BindGroup,
}

#[derive(Resource)]
pub struct ReflectedViewTextures {
    pub color_texture: Texture,
    pub depth_texture: Texture,
    pub bind_group: BindGroup,
    pub _null_texture: Texture,
    pub null_bind_group: BindGroup,
}
