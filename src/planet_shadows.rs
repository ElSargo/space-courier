#![allow(dead_code)]

use bevy_ecs::{
    component::Component,
    system::{In, Res, ResMut, Resource},
};

use bytemuck::{Pod, Zeroable};
use glam::{vec2, Vec2, Vec3};
use wgpu::{
    util::DeviceExt, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BlendComponent, BufferUsages, Color,
    ColorWrites, Device, Extent3d, FrontFace, PipelineLayoutDescriptor, PrimitiveState,
    RenderPipelineDescriptor, ShaderStages, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureViewDescriptor,
};

use crate::{
    camera::CameraUniform,
    ecs::Initailiser,
    gpu::{Packable, ScreenResolution, RD},
    instance_buffer::{InstanceBuffer, RenderPassExt},
    instance_layout,
    planets::Planet,
    shader_manager::{
        PipelineHandle, PipelinesView, ShaderManagerCtrl, UnifiedPipelineModuleResource,
        UnifiedShaders,
    },
    transform::Transform,
    vertex_layout, ViewTextures,
};

#[derive(Resource)]
pub struct PlanetShadows {
    render_texture: wgpu::Texture,
    pub result_texture_view: wgpu::TextureView,
    pub render_texture_bind_group: wgpu::BindGroup,
    pub texture_bind_group_layout: wgpu::BindGroupLayout,
    pipeline: PipelineHandle,
    instance_buffer: InstanceBuffer,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct Vertex {
    length_coef: f32,
    displacment: f32,
}
vertex_layout!(Vertex,[0 => Float32, 1 => Float32]);

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Instance {
    penumbra_origin_length: f32,
    penumbra_cosa: f32,
    umbra_origin_length: f32,
    umbra_cosa: f32,
    umbra_h: f32,
    shadow_dir: Vec3,
    radius: f32,
    back_length: f32,
    back_width: f32,
}
instance_layout!( Instance ,[2 => Float32, 3 => Float32, 4 => Float32, 5 => Float32, 6 => Float32,7 => Float32x3,8=>Float32,9=>Float32,10=>Float32] );

impl PlanetShadows {
    pub fn new(
        device: &Device,
        resolution: &ScreenResolution,
        shader_manager: ShaderManagerCtrl,
        view_textures_bind_group_layout: &BindGroupLayout,
    ) -> Self {
        let camera_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Planet shadows camera"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let pipeline_layout = Box::leak(Box::new(device.create_pipeline_layout(
            &PipelineLayoutDescriptor {
                label: Some("Planet shadow pipeline layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &view_textures_bind_group_layout],
                push_constant_ranges: &[],
            },
        )));

        let shader_handle = shader_manager.load_shader("./src/planet_shadows.wgsl");

        let pipeline = shader_manager.add_render_pipeline(
            UnifiedShaders {
                handle: shader_handle,
            },
            // Box::new(move |__rr, device, module| {
            |In(module): bevy_ecs::system::In<UnifiedPipelineModuleResource>, device: RD| {
                device.create_render_pipeline(&RenderPipelineDescriptor {
                    label: Some("Planet shadow pipeline"),
                    layout: Some(pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: "vs_main",
                        buffers: &[Vertex::desc(), Instance::desc()],
                        compilation_options: Default::default(),
                    },
                    primitive: PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        cull_mode: None,
                        front_face: FrontFace::Cw,
                        ..Default::default()
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState {
                        count: 1,                         // 2.
                        mask: !0,                         // 3.
                        alpha_to_coverage_enabled: false, // 4.
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &module,
                        entry_point: "fs_main",
                        targets: &[Some(wgpu::ColorTargetState {
                            format: TextureFormat::Rg16Float,
                            blend: Some(wgpu::BlendState {
                                color: BlendComponent {
                                    src_factor: wgpu::BlendFactor::One,
                                    dst_factor: wgpu::BlendFactor::One,
                                    operation: wgpu::BlendOperation::Add,
                                },
                                alpha: BlendComponent {
                                    src_factor: wgpu::BlendFactor::One,
                                    dst_factor: wgpu::BlendFactor::One,
                                    operation: wgpu::BlendOperation::Max,
                                },
                            }),
                            write_mask: ColorWrites::RED | ColorWrites::GREEN,
                        })],
                        compilation_options: Default::default(),
                    }),

                    multiview: None,
                    cache: None,
                })
            },
        );

        let instance_buffer = InstanceBuffer::new(
            Some("Planet shadow instance buffer".to_owned()),
            BufferUsages::COPY_DST | BufferUsages::VERTEX,
            &device,
        );

        let texture_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("Planet shadows bind group layout descriptor"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                }],
            });

        let (render_texture, render_texture_view, render_texture_bind_group) =
            make_texture(device, resolution, &texture_bind_group_layout);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Planet shadow volumes vertex buffer"),
            contents: bytemuck::cast_slice(&[
                Vertex {
                    length_coef: 0.0,
                    displacment: 1.0,
                },
                Vertex {
                    length_coef: 1.0,
                    displacment: 1.0,
                },
                Vertex {
                    length_coef: 1.0,
                    displacment: -1.0,
                },
                Vertex {
                    length_coef: 0.0,
                    displacment: -1.0,
                },
            ]),
            usage: BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Planet shadow volumes index buffer"),
            contents: bytemuck::cast_slice::<u16, u8>(&[0, 2, 1, 0, 3, 2]),
            usage: BufferUsages::INDEX,
        });

        Self {
            render_texture,
            pipeline,
            instance_buffer,
            result_texture_view: render_texture_view,
            render_texture_bind_group,
            texture_bind_group_layout,
            vertex_buffer,
            index_buffer,
        }
    }

    pub fn resize(&mut self, resolution: &ScreenResolution, device: &Device) {
        let (result_texture, result_texture_view, result_texture_bind_group) =
            make_texture(device, resolution, &self.texture_bind_group_layout);

        self.render_texture = result_texture;
        self.result_texture_view = result_texture_view;
        self.render_texture_bind_group = result_texture_bind_group;
    }
}

pub fn draw_planet_shadows(
    encoder: &mut wgpu::CommandEncoder,
    shader_manager: &PipelinesView<wgpu::RenderPipeline>,

    planet_shadows: Res<PlanetShadows>,
    vt: Res<ViewTextures>,
    camera: Res<CameraUniform>,
) -> Option<()> {
    let pipeline = shader_manager.get(&planet_shadows.pipeline)?;
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Planet shadow volumes render pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &planet_shadows.result_texture_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });

    render_pass.set_bind_group(0, &camera.bind_group, &[]);
    render_pass.set_bind_group(1, &vt.bind_group, &[]);

    render_pass.set_vertex_buffer(0, planet_shadows.vertex_buffer.slice(..));
    render_pass.set_instance_buffer(1, &planet_shadows.instance_buffer);

    render_pass.set_index_buffer(
        planet_shadows.index_buffer.slice(..),
        wgpu::IndexFormat::Uint16,
    );

    render_pass.set_pipeline(&pipeline);
    render_pass.draw_indexed(0..6, 0, 0..planet_shadows.instance_buffer.len());

    Some(())
}

fn make_texture(
    device: &Device,
    reolution: &ScreenResolution,
    result_texture_bind_group_layout: &BindGroupLayout,
) -> (wgpu::Texture, wgpu::TextureView, wgpu::BindGroup) {
    let result_texture = device.create_texture(&TextureDescriptor {
        label: Some("Planet shadow texture"),
        size: Extent3d {
            width: reolution.width / 2,
            height: reolution.height / 2,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rg16Float,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });

    let result_texture_view = result_texture.create_view(&TextureViewDescriptor {
        label: Some("Planet shadow volume view"),
        format: Some(TextureFormat::Rg16Float),
        ..Default::default()
    });

    let result_texture_bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: Some("Planet shadows bind group layout"),
        layout: result_texture_bind_group_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&result_texture_view),
        }],
    });
    (
        result_texture,
        result_texture_view,
        result_texture_bind_group,
    )
}

fn resize_planet_shadows(
    planet_shadows: Option<ResMut<PlanetShadows>>,
    device: RD,
    resolution: Res<ScreenResolution>,
) {
    if let Some(mut pl) = planet_shadows {
        pl.resize(&resolution, &device);
    }
}

#[derive(Component)]
struct PlanetShadowsInstanceBuffer;

pub fn init_mod(ecs: &mut Initailiser) {
    ecs.resize.add_systems(resize_planet_shadows);
    // ecs.prepare_buffers.add_systems(
    //     pack_buffers::<(&Transform, &Planet), RawPlanetShadow, With<PlanetShadowsInstanceBuffer>>,
    // );
}

impl Packable for Instance {
    type Input<'a> = (&'a Transform, &'a Planet);

    fn pack<'a>(input: Self::Input<'a>, eye: Vec2) -> Self {
        let sun_radius = 20.0;
        let distance_recip = input.0.translation.length_recip();
        let umbra_gradient = (sun_radius - input.1.radius) * distance_recip;
        let penumbra_gradient = (sun_radius + input.1.radius) * distance_recip;
        let penumbra_origin_length = sun_radius / penumbra_gradient;
        let umbra_origin_length_past_planet = input.1.radius / umbra_gradient;

        let back_length = 20000.0;
        let back_width = back_length * penumbra_gradient + input.1.radius;

        Instance {
            penumbra_origin_length,
            penumbra_cosa: penumbra_origin_length
                / vec2(penumbra_origin_length, sun_radius).length(),
            umbra_origin_length: umbra_origin_length_past_planet + 1. / distance_recip,
            umbra_cosa: umbra_origin_length_past_planet
                / vec2(umbra_origin_length_past_planet, input.1.radius).length(),
            umbra_h: umbra_origin_length_past_planet,
            shadow_dir: input.0.translation.normalize(),
            radius: input.1.radius,
            back_length,
            back_width,
        }
    }
}
